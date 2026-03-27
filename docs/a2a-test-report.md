# ZeroClaw A2A 功能说明与跨 Agent 通信指南

> 版本：v0.6.4
> 协议：A2A (Agent-to-Agent) Protocol MVP

---

## 一、已实现功能说明

ZeroClaw 当前实现了 A2A 协议的核心子集，包含以下五项功能。

---

### 1. Agent Card 发现

**端点：** `GET /.well-known/agent-card.json`

无需认证的公开端点。任何调用方可通过该端点获取此 Agent 的能力声明，是 A2A 协议中 Agent 互相"认识"的第一步。

返回内容包含：Agent 名称与版本、对外服务 URL、支持的输入输出格式、认证方式、技能列表（skills）。

**示例响应：**
```json
{
  "name": "ZeroClaw Local Agent",
  "version": "0.6.4",
  "url": "http://127.0.0.1:42617",
  "capabilities": { "streaming": false, "pushNotifications": false },
  "defaultInputModes": ["text"],
  "defaultOutputModes": ["text"],
  "authentication": { "schemes": ["bearer"] },
  "skills": [
    {
      "id": "general",
      "name": "General",
      "description": "General-purpose autonomous agent",
      "tags": ["general"]
    }
  ]
}
```

---

### 2. 消息发送（`message/send`）

**端点：** `POST /a2a`，需携带 Bearer Token

调用方将消息通过 JSON-RPC 2.0 格式发送给此 Agent，Agent 同步处理后直接返回结果。任务生命周期为 `working` → `completed` / `failed`。

支持两种消息格式：

**标准 parts 格式（推荐）：**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "message/send",
  "params": {
    "message": {
      "role": "user",
      "parts": [{ "kind": "text", "text": "你的问题" }],
      "messageId": "msg-001"
    }
  }
}
```

**简化字符串格式（兼容）：**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "message/send",
  "params": { "message": "你的问题" }
}
```

**成功响应：**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "id": "37ac8d08-e47c-42ef-8bd7-776b70bf42f3",
    "status": { "state": "completed" },
    "artifacts": [
      {
        "artifactId": "0c3968e8-ebad-44e1-8687-b00c743499f6",
        "name": "response",
        "parts": [{ "kind": "text", "text": "Agent 的回复内容" }]
      }
    ]
  }
}
```

`result.id` 是任务 ID（UUID），可用于后续 `tasks/get` 查询。

---

### 3. 任务状态查询（`tasks/get`）

**端点：** `POST /a2a`，需携带 Bearer Token

通过任务 ID 查询任务的当前状态和输出内容。由于 `message/send` 是同步执行的，调用完成时任务已处于终态，`tasks/get` 主要用于事后回查或确认。

**请求：**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tasks/get",
  "params": { "id": "37ac8d08-e47c-42ef-8bd7-776b70bf42f3" }
}
```

**响应结构与 `message/send` 完全一致**，包含 `status` 和 `artifacts`。

任务状态枚举：

| 状态 | 含义 |
|---|---|
| `submitted` | 已接收，等待处理 |
| `working` | 处理中 |
| `completed` | 处理完成，artifacts 已就绪 |
| `failed` | 处理失败（如 LLM 调用错误） |

---

### 4. Bearer Token 认证

所有 `POST /a2a` 请求必须携带 `Authorization: Bearer <token>` 头。

认证优先级：
1. 配置项 `a2a.bearer_token`（专用 A2A Token，使用常数时间比较防止时序攻击）
2. 回退到 Gateway Pairing Token
3. 两者均未配置时放行全部请求

`GET /.well-known/agent-card.json` 无需认证，任何调用方均可访问。

**认证失败响应（HTTP 401）：**
```json
{
  "jsonrpc": "2.0",
  "id": null,
  "error": { "code": -32000, "message": "Unauthorized" }
}
```

---

### 5. 出站 A2A 工具（Agent 主动调用远端 Agent）

当 `a2a.enabled = true` 时，ZeroClaw Agent 自身会注册一个名为 `a2a` 的工具。Agent 在执行任务时可调用此工具，主动向远端 A2A Agent 发起通信，实现 Agent 之间的协作链路。

工具支持四个 action：

| Action | 说明 |
|---|---|
| `discover` | 拉取远端 Agent 的 Agent Card |
| `send` | 向远端 Agent 发送消息并获取结果 |
| `status` | 查询远端 Agent 上的任务状态 |
| `result` | 获取远端 Agent 任务的输出 artifacts |

这是实现跨 Agent 通信的关键机制：**本 Agent 既是服务端（接受其他 Agent 调用），也是客户端（通过工具调用其他 Agent）。**

---

## 二、跨 Agent 通信：A2A 协议的本质

### 为什么不是"调接口"

直接调 REST 接口和 A2A 协议的根本区别，不在于 HTTP 请求本身，而在于**通信双方的身份和协作模型**。

| 维度 | 普通 REST 调用 | A2A 协议 |
|---|---|---|
| 通信双方 | 人/系统 → 服务 | **Agent → Agent**（双方都是自主体） |
| 交互单位 | 请求/响应 | **Task（任务）**，有状态、有生命周期 |
| 能力发现 | 靠人工文档 | **Agent Card 标准化声明**，机器可读 |
| 结果形式 | HTTP 响应体（一次性） | **Artifacts**（结构化产物，可被下一个 Agent 继续处理） |
| 协作模式 | 主从调用 | **任务委托**：A 把自己不擅长的子任务交给 B，B 完成后 A 继续推进 |

**A2A 的核心是任务委托，而不是接口调用。** 当 Agent A 向 Agent B 发送 `message/send`，它传递的不是一个 HTTP 请求，而是一个**带有语义的任务**：由 B 的 LLM 和工具链来自主理解、决策、执行，最终以 artifacts 的形式返回结果。A 拿到 artifacts 后，可以继续将其作为输入传给下一个 Agent 或工具，形成 Agent 协作链。

---

### ZeroClaw 实现的双重角色

ZeroClaw 同时承担两个角色，这是 A2A 协议下真正跨 Agent 通信的基础：

```
┌─────────────────────────────────────────────────────┐
│                  ZeroClaw Instance                  │
│                                                     │
│  ┌─────────────────┐       ┌─────────────────────┐  │
│  │   A2A 服务端     │       │   A2A 客户端工具     │  │
│  │                 │       │                     │  │
│  │ /.well-known/   │       │  a2a tool           │  │
│  │ agent-card.json │       │  action: discover   │  │
│  │                 │       │  action: send       │  │
│  │ POST /a2a       │       │  action: status     │  │
│  │ message/send    │       │  action: result     │  │
│  │ tasks/get       │       │                     │  │
│  └────────┬────────┘       └──────────┬──────────┘  │
│           │ 接受其他 Agent 的委托       │ 委托任务给其他 Agent  │
└───────────┼────────────────────────────┼─────────────┘
            ▲                            │
     其他 Agent 调用我              我调用其他 Agent
```

**服务端**（`/a2a` 端点）：接受来自任何符合 A2A 协议的 Agent 的任务委托，由本实例的 LLM 执行后返回 artifacts。

**客户端**（`a2a` 工具）：当 Agent 在执行自己的任务时，如果需要将子任务委托出去，通过此工具以 A2A 协议向远端 Agent 发起通信。

---

### 跨 Agent 任务委托的完整流程

以下展示两个 ZeroClaw 实例之间的真实 A2A 协作过程。Agent A 收到用户任务后，判断其中有子任务适合委托给专门的 Agent B 完成。

**关于"发现"的准确含义**

A2A 协议中的 `discover`（`GET /.well-known/agent-card.json`）**不是自动找到对方的机制**，而是"已知地址、读取能力"。Agent A 要与 Agent B 通信，前提是**通过外部途径提前知道 Agent B 的 URL**——可以是运维配置、用户在指令中直接提供、或注册中心（A2A 协议本身不包含注册中心，属于上层扩展）。

`discover` 的价值在于：拿到地址后，Agent A 可以**机器可读地**了解 B 支持哪些 skills、需要什么认证、支持哪些输入格式，从而自主决定是否委托、如何构造消息，而不需要人工查文档。

---

**环境准备：启动两个实例**

Agent A 配置（`~/.zeroclaw/config.toml`，已有实例，端口 42617）：
```toml
default_provider = "qwen"
default_model = "qwen-plus"
api_key = "<your-key>"

[gateway]
port = 42617
require_pairing = false

[a2a]
enabled = true   # 开启后 Agent A 才拥有 a2a 出站工具
```

Agent B 配置（`~/.zeroclaw-b/config.toml`，新实例，端口 42618）：
```toml
default_provider = "qwen"
default_model = "qwen-plus"
api_key = "<your-key>"

[gateway]
port = 42618
require_pairing = false

[a2a]
enabled = true
agent_name = "ZeroClaw Agent B"
description = "专注代码生成的 Agent"
public_url = "http://localhost:42618"
bearer_token = "agent-b-secret"
```

启动 Agent B：
```bash
./zeroclaw gateway start --config-dir ~/.zeroclaw-b
```

此时 Agent A（42617）和 Agent B（42618）各自独立运行，互相不知道对方的存在。**Agent B 的地址（`http://localhost:42618`）需要由外部（用户指令或配置）告知 Agent A**，这是通信的前提。

---

**第一步：Agent A 读取 Agent B 的能力声明（discover）**

Agent A **已知** Agent B 的地址是 `http://localhost:42618`，通过 `discover` 读取其 Agent Card，了解对方能做什么、需要什么认证，从而决定如何构造委托消息。

```bash
curl http://localhost:42618/.well-known/agent-card.json
```

响应中的关键字段：
- `skills` — Agent B 声明自己擅长什么（如"代码生成"）
- `authentication.schemes` — 需要 Bearer Token
- `url` — 后续发送任务的 RPC 地址

Agent A 基于这些信息自主判断：Agent B 有能力处理代码生成任务，且需要携带 Bearer Token。

---

**第二步：Agent A 将子任务委托给 Agent B（message/send）**

Agent A 将任务封装后发送。**发送的不是函数调用，而是需要 B 的 LLM 自主理解和执行的任务**。

```bash
curl -X POST http://localhost:42618/a2a \
  -H "Authorization: Bearer agent-b-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "message/send",
    "params": {
      "message": {
        "role": "user",
        "parts": [{ "kind": "text", "text": "请用 Python 写一个解析 JSON 的函数，并附带单元测试" }],
        "messageId": "task-delegate-001"
      }
    }
  }'
```

Agent B 接收到任务后，由自己的 LLM（Qwen）独立理解需求、生成代码，与 Agent A 使用哪个模型、如何执行无关。这体现了 A2A 的核心价值：**Agent 之间通过标准协议协作，内部实现互相透明**。

响应：
```json
{
  "result": {
    "id": "b1c2d3e4-...",
    "status": { "state": "completed" },
    "artifacts": [{
      "name": "response",
      "parts": [{ "kind": "text", "text": "```python\ndef parse_json(s): ...\n```" }]
    }]
  }
}
```

---

**第三步：Agent A 确认任务结果（tasks/get）**

用 task_id 回查，确认终态并取回 artifacts：

```bash
curl -X POST http://localhost:42618/a2a \
  -H "Authorization: Bearer agent-b-secret" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tasks/get",
    "params": { "id": "b1c2d3e4-..." }
  }'
```

---

**第四步：Agent A 将 artifacts 继续使用**

Agent A 拿到 Agent B 返回的 artifacts（代码片段）后，可以：
- 将其作为上下文继续执行自己的任务（如运行代码、做 Code Review）
- 通过 A2A 再委托给第三个 Agent C 做进一步处理
- 直接返回给最终用户

这就是 A2A 协议的完整协作链：**每个 Agent 只做自己专长的部分，通过标准化的任务委托和 artifacts 传递，组成多 Agent 协作流水线。**

---

### 通过 Agent Chat 触发自动委托

上述四步也可以通过自然语言驱动。在 Web Dashboard 的 Agent Chat 中，用户直接告知 Agent B 的地址：

```
http://localhost:42618 上有另一个 Agent，使用 a2a 工具了解它的能力，
然后委托它帮我用 Python 写一个解析 JSON 的函数，获取它的回复后告诉我结果。
bearer token 是 agent-b-secret。
```

Agent A 收到指令后，自动调用 `a2a` 工具依次执行 discover → send → result，整个跨 Agent 委托过程由 LLM 驱动，无需人工构造请求。**Agent B 的地址和 Token 由用户在指令中提供**——这是当前实现下 Agent A "找到" Agent B 的方式。

> 注意：由于 SSRF 防护拦截 localhost，当前 `a2a` 出站工具默认不允许访问本地地址。两实例通信场景需要 Agent B 暴露在公网地址，或在工具注册时开启 `allow_local` 配置（开发环境专用）。

---

## 三、JSON-RPC 错误码参考

| 错误码 | HTTP 状态 | 含义 |
|---|---|---|
| `-32000` | 401 | 未认证或 Token 错误 |
| `-32001` | 200 | Task ID 不存在 |
| `-32600` | 400 | JSON-RPC 版本非 `"2.0"` |
| `-32601` | 200 | 调用了未实现的方法 |
| `-32602` | 200 | 参数缺失或格式错误 |
