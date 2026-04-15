# A2A 代码对比文档

## 对比范围

- **上游 PR**: [zeroclaw-labs/zeroclaw#4166](https://github.com/zeroclaw-labs/zeroclaw/pull/4166) (已关闭)
- **当前代码**: 本地 `A2A` 分支 (https://github.com/JacobGaoZQ/zeroclaw/tree/A2A-v2)

---

## 一、环境配置 (.env)

项目启动时会自动加载 `.env` 文件（通过 `dotenvy` 库），以下是 A2A 相关的环境变量配置：

### 1.1 完整配置示例

```bash
# ── A2A (Agent-to-Agent) Protocol ───────────────────────────────
# 启用 A2A 协议服务端和客户端工具
A2A_ENABLED=true

# Agent 显示名称（在 agent card 中展示）
A2A_AGENT_NAME=ZeroClaw Agent

# Agent 描述信息
A2A_DESCRIPTION=ZeroClaw autonomous agent

# 公共 URL（用于 agent card，如不设置则使用 gateway 绑定地址）
A2A_PUBLIC_URL=https://agent.example.com

# Bearer Token（用于认证入站 A2A 请求）
A2A_BEARER_TOKEN=your-secret-token-here

# 协议版本（默认为 crate 版本）
A2A_VERSION=1.0.0

# 能力标签（逗号分隔）
A2A_CAPABILITIES=code,search,general

# Telegram 群 ID（用于 A2A 活动通知）
A2A_NOTIFY_CHAT_ID=-1001234567890

# 允许本地/私有 IP 访问（仅开发环境使用）
A2A_ALLOW_LOCAL=false

# 远程 Strands Agent URL（A2A 工具默认目标地址）
A2A_STRANDS_AGENT_URL=http://120.26.206.98:8000/a2a

# Strands Agent PAT Token（注入到 JSON-RPC params 中）
A2A_PAT_TOKEN=c062fe5c-2a8f-4d38-a655-ab7046eb5bb3

# Strands Agent Location ID（注入到 JSON-RPC params 中）
A2A_LOCATION_ID=20273f50-0fe4-4eff-b896-8b86bfa13b00

# API Key（x-api-key header，用于认证远程 Strands Agent）
A2A_CLIENT_TOKEN=your-api-key-here
```

### 1.2 配置项说明

| 环境变量 | 必需 | 默认值 | 说明 |
|----------|------|--------|------|
| `A2A_ENABLED` | 否 | `false` | 启用 A2A 协议 |
| `A2A_AGENT_NAME` | 否 | `ZeroClaw Agent` | Agent 名称 |
| `A2A_DESCRIPTION` | 否 | `ZeroClaw autonomous agent` | Agent 描述 |
| `A2A_PUBLIC_URL` | 否 | 自动推导 | Agent card 公共 URL |
| `A2A_BEARER_TOKEN` | 否 | - | 入站请求认证 Token |
| `A2A_VERSION` | 否 | crate 版本 | 协议版本 |
| `A2A_CAPABILITIES` | 否 | - | 能力标签（逗号分隔） |
| `A2A_NOTIFY_CHAT_ID` | 否 | - | Telegram 通知群 ID |
| `A2A_ALLOW_LOCAL` | 否 | `false` | 允许本地 IP（开发用） |
| `A2A_STRANDS_AGENT_URL` | 否 | - | 默认远程 Agent URL |
| `A2A_PAT_TOKEN` | 否 | - | PAT Token（JSON-RPC params） |
| `A2A_LOCATION_ID` | 否 | - | Location ID（JSON-RPC params） |
| `A2A_CLIENT_TOKEN` | 否 | - | API Key（x-api-key header） |

### 1.3 快速启动

1. 复制 `.env.example` 到 `.env`：
   ```bash
   cp .env.example .env
   ```

2. 编辑 `.env`，设置必需的 A2A 配置：
   ```bash
   # 启用 A2A
   A2A_ENABLED=true
   
   # 配置远程 Strands Agent
   A2A_STRANDS_AGENT_URL=http://your-strands-agent:8000/a2a
   A2A_PAT_TOKEN=your-pat-token
   A2A_LOCATION_ID=your-location-id
   A2A_CLIENT_TOKEN=your-api-key
   ```

3. 启动 Gateway：
   ```bash
   cargo build --release
   ./target/release/zeroclaw gateway start
   ```

4. 访问 A2A 测试页面：`http://localhost:42617/` → A2A Test

---

## 二、PR #4166 原始改动概览

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
| `client_token` | ❌ | ✅ **新增** | API Key（x-api-key header） |

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
6. **`client_token`** — API Key（x-api-key header）认证配置
7. **`allow_local`** — SSRF 本地访问开关
8. **`dotenvy` 依赖** — 自动加载 `.env` 文件
9. **前端 A2A 测试页面** — `web/src/pages/A2aTest.tsx`
10. **完整的 A2A 文档体系** — 测试报告、部署指南、配置说明等

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
│  配置: strands_agent_url, pat_token, location_id, client_token,  │
│        allow_local (通过 .env 文件加载)                           │
│  前端: A2aTest.tsx (discover/send/stream/status/result)          │
└─────────────────────────────────────────────────────────────────┘
```

**核心差异**: 当前代码的客户端 `stream` 动作调用 `message/stream`，但目标服务端（如 Strands Agent at `120.26.206.98:8000`）支持 SSE 流式响应，因此客户端可以正确解析。而 ZeroClaw 自身的服务端尚未实现 `message/stream`。

---

## 五、.env 自动加载实现

当前代码通过 `dotenvy` 库实现了 `.env` 文件的自动加载，无需手动导出环境变量。

### 5.1 技术实现

**Cargo.toml** 新增依赖：
```toml
# Environment variables from .env file
dotenvy = "0.15"
```

**src/main.rs** 启动时加载：
```rust
async fn main() -> Result<()> {
    // Load environment variables from .env file (if present).
    // This allows users to configure A2A and other settings via .env.
    if let Err(e) = dotenvy::dotenv() {
        tracing::debug!("No .env file loaded: {}", e);
    }
    // ... 后续代码
}
```

### 5.2 配置加载流程

```
┌─────────────────────────────────────────────────────────────────┐
│                      配置加载优先级                               │
├─────────────────────────────────────────────────────────────────┤
│  1. 环境变量 (最高优先级)                                         │
│     ↓ (如未设置)                                                 │
│  2. .env 文件 (dotenvy 加载到环境变量)                            │
│     ↓ (如未设置)                                                 │
│  3. config.toml 配置文件                                         │
│     ↓ (如未设置)                                                 │
│  4. 默认值                                                        │
└─────────────────────────────────────────────────────────────────┘
```

### 5.3 A2A 环境变量覆盖代码

**src/config/schema.rs** 中的 `apply_env_overrides` 方法：

```rust
// A2A environment variable overrides
if let Ok(enabled) = std::env::var("A2A_ENABLED") {
    match enabled.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => self.a2a.enabled = true,
        "0" | "false" | "no" | "off" => self.a2a.enabled = false,
        _ => tracing::warn!("Ignoring invalid A2A_ENABLED"),
    }
}
if let Ok(agent_name) = std::env::var("A2A_AGENT_NAME") {
    self.a2a.agent_name = Some(agent_name);
}
// ... 其他 A2A_* 环境变量
if let Ok(client_token) = std::env::var("A2A_CLIENT_TOKEN") {
    self.a2a.client_token = Some(client_token);
}
```

### 5.4 客户端认证流程

A2A 客户端工具在调用远程 Agent 时，会使用以下认证方式：

```rust
// src/tools/a2a.rs
// 优先使用 bearer_token，否则使用 client_token
if let Some(ref token) = self.bearer_token {
    req = req.header("Authorization", format!("Bearer {}", token));
} else if let Some(ref token) = self.client_token {
    req = req.header("x-api-key", token);
}
```

**请求示例**：
```http
POST /a2a HTTP/1.1
Host: 120.26.206.98:8000
Content-Type: application/json
x-api-key: your-api-key-here

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "message/send",
  "params": {
    "message": { ... },
    "pat_token": "c062fe5c-2a8f-4d38-a655-ab7046eb5bb3",
    "location_id": "20273f50-0fe4-4eff-b896-8b86bfa13b00"
  }
}
```

---

## 六、前端 A2A 测试页面

### 6.1 页面功能

当前 A2A 测试页面已简化，默认使用后端 A2A 工具配置：

- **Local Gateway** — 固定显示，无需选择
- **自动使用配置** — 直接使用 `.env` 中的 `strands_agent_url`、`pat_token`、`location_id`、`client_token`
- **Action 选择** — `discover` / `send` / `stream` / `status` / `result`

### 6.2 API 调用流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    前端 A2A 测试页面                              │
├─────────────────────────────────────────────────────────────────┤
│  用户选择 Action: discover / send / stream / status / result    │
│                           ↓                                      │
│  POST /api/a2a/outbound (使用 gateway pairing token)            │
│                           ↓                                      │
│  后端 A2A 工具 (src/tools/a2a.rs)                                │
│    - 读取 A2aConfig 配置                                         │
│    - 使用 strands_agent_url 作为默认 URL                         │
│    - 注入 pat_token / location_id 到 JSON-RPC params            │
│    - 使用 client_token 作为 x-api-key header                     │
│                           ↓                                      │
│  远程 Strands Agent (120.26.206.98:8000/a2a)                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## 七、常见问题

### Q1: 修改 .env 后配置不生效？

**A**: 需要重启 Gateway：
```bash
pkill -f "zeroclaw gateway"
./target/release/zeroclaw gateway start
```

### Q2: A2A 调用返回 401 Unauthorized？

**A**: 检查以下配置：
1. `A2A_CLIENT_TOKEN` 是否正确设置
2. 远程 Agent 是否需要 `x-api-key` header
3. `A2A_PAT_TOKEN` 和 `A2A_LOCATION_ID` 是否正确

### Q3: 如何调试 A2A 请求？

**A**: 查看日志：
```bash
RUST_LOG=debug ./target/release/zeroclaw gateway start
```

### Q4: 如何允许本地测试？

**A**: 设置 `A2A_ALLOW_LOCAL=true`（仅开发环境）：
```bash
A2A_ALLOW_LOCAL=true
```
