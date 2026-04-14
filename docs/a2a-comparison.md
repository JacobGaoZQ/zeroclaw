# A2A 代码对比文档

## 对比范围

- **上游 PR**: [zeroclaw-labs/zeroclaw#4166](https://github.com/zeroclaw-labs/zeroclaw/pull/4166) (已关闭)
- **当前代码**: 本地 `A2A` 分支 (https://github.com/JacobGaoZQ/zeroclaw/tree/A2A-v2)

---

## 一、PR #4166 原始改动概览

PR #4166 是 ZeroClaw 的 A2A (Agent-to-Agent) 协议的**初始实现**，共修改 **12 个文件**，新增 **2191 行**代码。

### 1.1 文件清单

| 文件 | 新增行 | 删除行 | 说明 |
|------|--------|--------|------|
| `src/gateway/a2a.rs` | 1092 | 0 | A2A 服务端（agent card 发现 + JSON-RPC 2.0 处理器） |
| `src/tools/a2a.rs` | 846 | 0 | A2A 客户端工具（discover/send/status/result 动作） |
| `src/config/schema.rs` | 54 | 0 | `A2aConfig` 结构体定义 |
| `src/gateway/mod.rs` | 64 | 1 | A2A 模块集成到 gateway |
| `src/tools/mod.rs` | 18 | 0 | A2A 工具注册 |
| `src/agent/loop_.rs` | 7 | 0 | A2A 工具描述加入 bootstrap prompt |
| `src/gateway/api.rs` | 4 | 0 | A2A 路由注册 |
| `src/config/mod.rs` | 1 | 1 | `A2aConfig` 导出 |
| `src/onboard/wizard.rs` | 2 | 0 | 引导向导添加 A2A 字段 |
| `docs/reference/api/config-reference.md` | 35 | 0 | 英文配置文档 |
| `docs/vi/config-reference.md` | 34 | 0 | 越南语配置文档 |
| `docs/i18n/zh-CN/reference/api/config-reference.zh-CN.md` | 34 | 0 | 中文配置文档 |

### 1.2 PR #4166 包含的功能

#### 服务端 (`src/gateway/a2a.rs`)
- `GET /.well-known/agent-card.json` — 未认证的 agent card 发现端点
- `POST /a2a` — 基于 bearer token 认证的 JSON-RPC 2.0 端点
- 支持的 RPC 方法：`message/send`、`tasks/get`
- 内存任务存储（上限 10,000 条）
- 双重认证：专用 `bearer_token` → 回退到 gateway pairing
- Telegram 群通知（`notify_chat_id`）

#### 客户端工具 (`src/tools/a2a.rs`)
- `discover` — 获取远程 agent 的能力卡片
- `send` — 发送任务消息（使用 `message/send`，非流式）
- `status` — 查询任务进度
- `result` — 获取任务输出
- SSRF 防护：阻止私有/本地 IP、DNS 解析验证、重定向验证

#### 配置 (`A2aConfig`)
- `enabled`、`agent_name`、`description`、`public_url`、`bearer_token`、`version`、`capabilities`、`notify_chat_id`

### 1.3 PR #4166 明确未实现的功能

```
// gateway/a2a.rs 注释中明确标注：
// - message/stream (SSE)
// - tasks/cancel
// - input-required state / multi-turn conversations (contextId)
// - Push notifications
// - Structured/binary message parts (data, raw)
// - Async task execution
// - Task persistence
```

---

## 二、当前代码相对于 PR #4166 的新增改动

当前 `A2A` 分支在 PR #4166 的基础上进行了**扩展和增强**。

### 2.1 `A2aConfig` 配置结构 — 新增字段

| 字段 | PR #4166 | 当前代码 | 说明 |
|------|----------|----------|------|
| `enabled` | ✅ | ✅ | 启用 A2A 协议 |
| `agent_name` | ✅ | ✅ | Agent 显示名称 |
| `description` | ✅ | ✅ | Agent 描述 |
| `public_url` | ✅ | ✅ | 公共 URL |
| `bearer_token` | ✅ | ✅ | Bearer Token |
| `version` | ✅ | ✅ | 协议版本 |
| `capabilities` | ✅ | ✅ | 能力标签 |
| `notify_chat_id` | ✅ | ✅ | Telegram 通知 |
| `allow_local` | ❌ | ✅ **新增** | 允许本地/私有 IP（SSRF 旁路） |
| `strands_agent_url` | ❌ | ✅ **新增** | 默认远程 Strands Agent URL |
| `pat_token` | ❌ | ✅ **新增** | Strands Agent PAT Token |
| `location_id` | ❌ | ✅ **新增** | Strands Agent Location ID |

### 2.2 A2A 客户端工具 (`src/tools/a2a.rs`) — 新增功能

| 功能 | PR #4166 | 当前代码 | 说明 |
|------|----------|----------|------|
| `discover` 动作 | ✅ | ✅ | 获取远程 agent 卡片 |
| `send` 动作 | ✅ | ✅ | 非流式 `message/send` |
| `status` 动作 | ✅ | ✅ | 查询任务状态 |
| `result` 动作 | ✅ | ✅ | 获取任务结果 |
| `stream` 动作 | ❌ | ✅ **新增** | 流式 `message/stream` (SSE) |
| `parse_sse_stream` 方法 | ❌ | ✅ **新增** | SSE 事件流解析器 |
| `pat_token` 注入 | ❌ | ✅ **新增** | 在 JSON-RPC params 中附加 PAT |
| `location_id` 注入 | ❌ | ✅ **新增** | 在 JSON-RPC params 中附加 Location ID |
| `default_url` 回退 | ❌ | ✅ **新增** | 使用 `strands_agent_url` 作为默认 URL |
| `with_config` 构造函数 | ❌ | ✅ **新增** | 支持完整配置初始化 |
| SSE 依赖 `futures_util` | ❌ | ✅ **新增** | StreamExt trait 支持 |

#### 新增 `action_stream` 方法核心实现

```rust
async fn action_stream(&self, url: &str, bearer_token: Option<&str>, message: &str) -> anyhow::Result<ToolResult> {
    // 使用 message/stream 方法（流式）
    let body = json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": "message/stream",  // ← 流式
        "params": {
            "message": { "role": "user", "parts": [...], "messageId": ... },
            "configuration": { "accepted_output_modes": ["text"] },
            "pat_token": ...,
            "location_id": ...
        }
    });
    // 解析 SSE 流，提取 artifact-update 事件中的 text
    let text = self.parse_sse_stream(resp).await?;
    ...
}
```

#### 新增 `parse_sse_stream` 方法

- 解析 `data:` 行中的 JSON 事件
- 提取 `artifact-update` 类型事件的 `artifact.parts[].text`
- 收集所有文本片段为完整响应
- 处理缓冲区、行分割、空行等边缘情况

### 2.3 A2A 服务端 (`src/gateway/a2a.rs`) — 保持不变

当前代码的服务端实现与 PR #4166 **基本一致**，仍不支持 `message/stream`（SSE 服务端）。注释中仍标注：

```
// **Not yet implemented (see issue #3566):**
// - `message/stream` (SSE)
```

### 2.4 前端页面 — 全新添加

| 文件 | 说明 |
|------|------|
| `web/src/pages/A2aTest.tsx` | A2A 测试页面 UI（PR #4166 中不存在） |

前端新增功能：
- Action 选择器：`discover` / `send` / `stream` / `status` / `result`
- 不同动作对应不同图标（Send / Waves / Search / ListCheck 等）
- `stream` 动作使用 `message/stream` 方法调用
- `send` 动作使用 `message/send` 方法调用
- 支持勾选 "Use backend A2A tool" 使用后端工具

### 2.5 文档 — 新增

当前分支包含了大量的 A2A 相关文档（PR #4166 中不存在或内容不同）：

- A2A 测试报告（多 Agent Codespaces 设置）
- A2A 协议实现状态文档
- Strands Agent 部署指南
- A2A 配置说明表
- Strands Server 脚本
- CORS 支持文档
- 截图和测试证据

---

## 三、差异总结

### PR #4166 有的，当前代码也有的

- A2A 服务端（agent card + JSON-RPC 2.0）
- A2A 客户端工具（discover/send/status/result）
- `A2aConfig` 基础配置
- SSRF 防护
- Bearer token 认证
- 任务存储（10,000 上限）
- Telegram 群通知
- 配置文档（en/vi/zh-CN）

### 当前代码有，PR #4166 没有的

1. **`stream` 动作** — 基于 `message/stream` 的 SSE 流式响应
2. **`parse_sse_stream` 解析器** — SSE 事件流解析
3. **`strands_agent_url`** — 默认远程 Agent URL 配置
4. **`pat_token`** — Strands Agent PAT Token 配置
5. **`location_id`** — Strands Agent Location ID 配置
6. **`allow_local`** — SSRF 本地访问开关
7. **前端 A2A 测试页面** — `web/src/pages/A2aTest.tsx`
8. **完整的 A2A 文档体系** — 测试报告、部署指南、配置说明等

### PR #4166 明确标注未实现，当前代码仍为未实现的

- 服务端 `message/stream` (SSE) — 服务端仍不支持流式响应
- `tasks/cancel` — 取消任务
- `input-required` 状态 / 多轮对话 (`contextId`)
- Push notifications
- Structured/binary message parts
- Async task execution
- Task persistence（持久化存储）

---

## 四、关键架构差异

```
┌─────────────────────────────────────────────────────────────────┐
│                        PR #4166                                  │
├─────────────────────────────────────────────────────────────────┤
│  客户端工具 (a2a.rs)          服务端 (gateway/a2a.rs)            │
│  ┌────────────────────┐       ┌──────────────────────────┐       │
│  │ discover           │       │ GET /.well-known/...     │       │
│  │ send (message/send)│──────>│ POST /a2a (message/send) │       │
│  │ status (tasks/get) │──────>│ POST /a2a (tasks/get)    │       │
│  │ result             │       │                          │       │
│  └────────────────────┘       └──────────────────────────┘       │
│  配置: enabled, bearer_token, agent_name, ...                    │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     当前代码 (A2A 分支)                           │
├─────────────────────────────────────────────────────────────────┤
│  客户端工具 (a2a.rs)          服务端 (gateway/a2a.rs)            │
│  ┌────────────────────┐       ┌──────────────────────────┐       │
│  │ discover           │       │ GET /.well-known/...     │       │
│  │ send (message/send)│──────>│ POST /a2a (message/send) │       │
│  │ stream (message/   │       │ POST /a2a (tasks/get)    │       │
│  │   stream) [SSE]    │────X─▶│ ❌ message/stream 不支持  │       │
│  │ status (tasks/get) │──────>│                          │       │
│  │ result             │       │                          │       │
│  └────────────────────┘       └──────────────────────────┘       │
│  配置: +strands_agent_url, +pat_token, +location_id, +allow_local│
│  前端: A2aTest.tsx (discover/send/stream/status/result)          │
└─────────────────────────────────────────────────────────────────┘
```

**核心差异**: 当前代码的客户端 `stream` 动作调用 `message/stream`，但目标服务端（如 Strands Agent at `120.26.206.98:8000`）支持 SSE 流式响应，因此客户端可以正确解析。而 ZeroClaw 自身的服务端尚未实现 `message/stream`。
