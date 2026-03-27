# ZeroClaw A2A 协议实现状态文档

> 版本：v0.6.4 | 追踪 Issue：#3566

---

## 一、已实现功能

### 1. Agent Card 发现

**端点：** `GET /.well-known/agent-card.json`（无需认证）

返回 Agent 能力声明，包含名称、版本、URL、支持的认证方式、技能列表等。

**出处：**
- 路由注册：`src/gateway/mod.rs:1001-1003`
- 处理逻辑：`src/gateway/a2a.rs:166-176`（`handle_agent_card`）
- Card 生成：`src/gateway/a2a.rs:97-162`（`generate_agent_card`）

---

### 2. 同步任务发送 `message/send`

**端点：** `POST /a2a`

接收消息后**同步阻塞**调用 Agent 执行，完成后直接返回结果。任务在整个生命周期内状态路径为：`working` → `completed` / `failed`，不会出现中间异步等待状态。

支持两种消息格式：
- 标准 parts 格式：`{"parts": [{"kind": "text", "text": "..."}]}`
- 简化字符串格式：`{"message": "..."}`（向后兼容回退）

**出处：**
- 方法路由分支：`src/gateway/a2a.rs:209`
- 完整处理逻辑：`src/gateway/a2a.rs:276-411`（`handle_message_send`）

---

### 3. 任务状态查询 `tasks/get`

**端点：** `POST /a2a`，method 为 `tasks/get`

按 `task_id` 查询任务当前状态与产物（artifacts）。由于 `message/send` 是同步的，调用完成时任务已是终态，`tasks/get` 主要用于事后回查。

任务状态枚举（`src/gateway/a2a.rs:58-65`）：

```
Submitted | Working | Completed | Failed
```

**出处：**
- 方法路由分支：`src/gateway/a2a.rs:212`
- 完整处理逻辑：`src/gateway/a2a.rs:413-445`（`handle_tasks_get`）

---

### 4. Bearer Token 认证

对 `POST /a2a` 的所有请求执行认证，优先级顺序：

1. 配置项 `a2a.bearer_token`（使用常数时间比较防时序攻击）
2. 回退到 Gateway Pairing 认证
3. 若两者均未配置，放行所有请求（启动时输出警告）

**出处：** `src/gateway/a2a.rs:227-272`（`require_a2a_auth`）

---

### 5. 客户端出站工具（A2A Tool）

Agent 可通过 `a2a` 工具主动调用远程 A2A Agent，支持四个 action：

| Action | 说明 |
|---|---|
| `discover` | 拉取远端 Agent Card |
| `send` | 向远端发送任务 |
| `status` | 查询任务状态 |
| `result` | 获取任务产物 |

工具注册条件：`config.a2a.enabled = true`。

**出处：**
- 工具实现：`src/tools/a2a.rs:83-245`
- 工具注册：`src/tools/mod.rs:993-1008`

---

### 6. SSRF 防护

对出站请求的目标 URL 进行校验，拦截对私有/本地网络的请求：

- 仅允许 `http` / `https` 协议
- 拦截 `localhost`、`*.localhost`、`*.local`
- 拦截私有 IP 段：`127.x`、`10.x`、`192.168.x`、`172.16-31.x`、`169.254.x`（AWS 元数据）、`::1`、`fc00::/7`、`fe80::/10`
- DNS 解析后二次校验所有解析到的 IP
- 重定向时阻止跳转到私有地址

**出处：** `src/tools/a2a.rs:387-478`

---

### 7. 内存任务存储

任务以 `HashMap` 存储于内存，设有容量上限防止内存耗尽。

```rust
const MAX_TASKS: usize = 10_000;  // src/gateway/a2a.rs:35
```

**出处：** `src/gateway/a2a.rs:34-48`（`TaskStore`）

---

### 8. Telegram 通知

当配置 `a2a.notify_chat_id` 且 Telegram bot token 存在时，每次 A2A 任务完成后向指定群组推送通知。

**出处：** `src/gateway/a2a.rs:341-365`

---

## 二、未实现功能

> 来源：`src/gateway/a2a.rs:9-16` 及 `src/tools/a2a.rs:6-7`，追踪 Issue #3566

| 功能 | 协议含义 | 出处 |
|---|---|---|
| `message/stream` (SSE) | 流式逐步返回结果，而非等待完成后一次性响应 | `src/gateway/a2a.rs:10` |
| `tasks/cancel` | 客户端取消进行中的任务 | `src/gateway/a2a.rs:11` |
| `input-required` / `contextId` | 多轮对话状态，Agent 可中途向调用方追问 | `src/gateway/a2a.rs:12` |
| Push notifications | 任务完成后主动回调调用方，无需轮询 | `src/gateway/a2a.rs:13` |
| Structured/binary parts（`data`、`raw`）| 消息携带结构化数据或二进制内容（图片、文件等） | `src/gateway/a2a.rs:14` |
| Async task execution | 真正异步队列：提交后立即返回 taskId，稍后查询 | `src/gateway/a2a.rs:15` |
| Task persistence | 任务状态持久化，重启后不丢失（当前为纯内存） | `src/gateway/a2a.rs:16` |

---

## 三、已知局限

| 问题 | 说明 | 出处 |
|---|---|---|
| DNS rebinding TOCTOU | 验证时与实际连接时各自解析 DNS，存在时间差，攻击者可利用 DNS 翻转绕过 SSRF 检查 | `src/tools/a2a.rs:56-63` |
| `streaming: false` 硬编码 | Agent Card 中流式能力始终声明为不支持 | `src/gateway/a2a.rs:149` |
| `pushNotifications: false` 硬编码 | Agent Card 中推送能力始终声明为不支持 | `src/gateway/a2a.rs:150` |
| 任务同步执行 | `message/send` 阻塞等待，长任务会占用连接 | `src/gateway/a2a.rs:349-354` |
