# A2A (Agent-to-Agent) 协议实现文档

## 概述

本文档记录了 A2A (Agent-to-Agent) 协议的实现变更，基于https://github.com/zeroclaw-labs/zeroclaw/pulls/4166进行的二次开发。

**对比基准**: `zeroclaw-labs/master`
**当前分支**: `A2A-v2`

---

## 一、A2A 协议实现

### 1.1 新增核心文件

| 文件路径 | 说明 | 代码行数 |
|---------|------|---------|
| `src/gateway/a2a.rs` | A2A 服务端实现 | ~1089 行 |
| `src/tools/a2a.rs` | A2A 客户端工具 | ~1118 行 |

---

## 二、服务端实现 (`src/gateway/a2a.rs`)

### 2.1 功能概述

实现了 A2A 协议的 MVP 版本，包括：

- **Agent Card 发现**: `GET /.well-known/agent-card.json`
- **JSON-RPC 2.0 端点**: `POST /a2a`
- **Bearer Token 认证**
- **任务状态管理**: 内存存储，最大 10,000 任务

### 2.2 支持的 JSON-RPC 方法

| 方法 | 说明 |
|-----|------|
| `message/send` | 同步请求/响应，发送消息并返回结果 |
| `tasks/get` | 任务状态轮询，查询任务执行状态 |

### 2.3 核心类型定义

```rust
/// 任务状态存储
pub struct TaskStore {
    tasks: RwLock<HashMap<String, TaskState>>,
}

/// 任务状态
pub struct TaskState {
    pub id: String,
    pub status: TaskStatus,
    pub artifacts: Vec<serde_json::Value>,
}

/// 任务状态枚举
pub enum TaskStatus {
    Submitted,
    Working,
    Completed,
    Failed,
}

/// JSON-RPC 请求/响应结构
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: serde_json::Value,
}

pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}
```

### 2.4 Agent Card 生成

```rust
pub fn generate_agent_card(config: &crate::config::Config) -> serde_json::Value {
    // 生成的 Agent Card 包含:
    // - name: 代理名称
    // - description: 代理描述
    // - version: 版本号
    // - url: 公开 URL
    // - capabilities: 能力配置 (streaming: false, pushNotifications: false)
    // - skills: 技能列表
    // - authentication: 认证方案 (schemes: ["bearer"])
}
```

### 2.5 认证机制

```rust
fn require_a2a_auth(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, Json<Value>)> {
    // 1. 提取 Bearer Token
    // 2. 优先检查专用 A2A Bearer Token
    // 3. 回退到 Gateway Pairing 认证
    // 4. 使用常量时间比较防止时序攻击
}
```

### 2.6 Telegram 通知

当配置了 `notify_chat_id` 时，A2A 任务会发送通知到 Telegram 群组：

```rust
async fn notify_telegram_chat(bot_token: &str, chat_id: i64, text: &str) {
    // 发送 Markdown 格式的通知消息
}
```

### 2.7 安全措施

- **容量限制**: 最大 10,000 个并发任务
- **认证验证**: Bearer Token + 常量时间比较
- **错误消息**: 不回显用户输入（防止信息泄露）

---

## 三、客户端工具 (`src/tools/a2a.rs`)

### 3.1 功能概述

客户端工具用于发现远程 A2A 代理并发送/检索任务。

### 3.2 支持的动作

| 动作 | 说明 | 必需参数 |
|-----|------|---------|
| `discover` | 发现远程代理的能力 | `url` |
| `send` | 发送消息（非流式） | `url`, `message` |
| `stream` | 发送消息（SSE 流式响应） | `url`, `message` |
| `status` | 查询任务状态 | `url`, `task_id` |
| `result` | 获取任务结果 | `url`, `task_id` |

### 3.3 工具定义

```rust
pub struct A2aTool {
    security: Arc<SecurityPolicy>,
    timeout_secs: u64,
    default_url: Option<String>,      // 默认远程代理 URL
    pat_token: Option<String>,        // PAT Token
    location_id: Option<String>,      // Location ID
    client_token: Option<String>,     // API Key
}
```

### 3.4 参数 Schema

```json
{
    "type": "object",
    "properties": {
        "action": {
            "type": "string",
            "enum": ["discover", "send", "stream", "status", "result"]
        },
        "url": {
            "type": "string",
            "description": "远程代理的基础 URL"
        },
        "bearer_token": {
            "type": "string",
            "description": "认证 Bearer Token"
        },
        "task_id": {
            "type": "string",
            "description": "任务 ID (status/result 需要)"
        },
        "message": {
            "type": "string",
            "description": "发送的消息内容"
        }
    },
    "required": ["action"]
}
```

### 3.5 SSRF 防护

```rust
/// 验证 URL 指向公开主机
fn validate_url(&self, url: &str) -> anyhow::Result<reqwest::Url> {
    // 1. 只允许 http/https 协议
    // 2. 阻止私有/本地主机
    // 3. DNS 解析后再次验证 IP 地址
}

/// 检测私有/本地主机
fn is_private_or_local_host(host: &str) -> bool {
    // 阻止:
    // - localhost, *.localhost, *.local
    // - 127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
    // - 169.254.0.0/16 (云元数据)
    // - ::1, fc00::/7, fe80::/10
}

/// 重定向策略
fn build_client(&self) -> anyhow::Result<reqwest::Client> {
    // 最多 10 次重定向
    // 重定向时再次检查目标主机
}
```

### 3.6 SSE 流式解析

`stream` 动作支持解析 SSE (Server-Sent Events) 响应：

```rust
async fn parse_sse_stream(&self, resp: reqwest::Response) -> anyhow::Result<String> {
    // 解析 SSE 事件:
    // - 提取 task_id 和 context_id
    // - 从 artifact-update 事件中提取文本内容
    // - 收集所有文本部分返回
}
```

---

## 四、配置支持

### 4.1 A2A 配置结构

```rust
pub struct A2aConfig {
    /// 是否启用 A2A 协议
    pub enabled: bool,

    /// 代理名称（显示在 Agent Card 中）
    pub agent_name: Option<String>,

    /// 代理描述
    pub description: Option<String>,

    /// 版本号
    pub version: Option<String>,

    /// 公开 URL（用于生成 Agent Card）
    pub public_url: Option<String>,

    /// 能力标签列表
    pub capabilities: Vec<String>,

    /// 专用 Bearer Token（可选）
    pub bearer_token: Option<String>,

    /// Telegram 通知聊天 ID
    pub notify_chat_id: Option<i64>,

    /// 默认远程代理 URL（客户端工具使用）
    pub strands_agent_url: Option<String>,

    /// PAT Token
    pub pat_token: Option<String>,

    /// Location ID
    pub location_id: Option<String>,

    /// Client Token (x-api-key)
    pub client_token: Option<String>,
}
```

### 4.2 配置示例

```toml
[a2a]
enabled = true
agent_name = "ZeroClaw Agent"
description = "智能家居控制代理"
public_url = "https://agent.example.com"
bearer_token = "your-secret-token"
notify_chat_id = -1001234567890
strands_agent_url = "http://remote-agent.example.com:8080"
client_token = "api-key-for-remote-agent"
```

### 4.3 AppState 扩展

```rust
pub struct AppState {
    // ... 其他字段

    /// A2A Agent Card（启用时）
    pub a2a_agent_card: Option<Arc<serde_json::Value>>,

    /// A2A 任务存储
    pub a2a_task_store: Option<Arc<TaskStore>>,
}
```

---

## 五、API 端点

### 5.1 Agent Card 发现

```http
GET /.well-known/agent-card.json
```

**响应示例**:
```json
{
    "name": "ZeroClaw Agent",
    "description": "ZeroClaw autonomous agent",
    "version": "0.6.4",
    "url": "http://localhost:8080",
    "capabilities": {
        "streaming": false,
        "pushNotifications": false
    },
    "defaultInputModes": ["text"],
    "defaultOutputModes": ["text"],
    "skills": [
        {
            "id": "general",
            "name": "General",
            "description": "General-purpose autonomous agent",
            "tags": ["general"],
            "examples": ["Help me with a task"]
        }
    ],
    "authentication": {
        "schemes": ["bearer"]
    }
}
```

### 5.2 JSON-RPC 端点

```http
POST /a2a
Authorization: Bearer <token>
Content-Type: application/json
```

**message/send 请求示例**:
```json
{
    "jsonrpc": "2.0",
    "id": "req-001",
    "method": "message/send",
    "params": {
        "message": {
            "role": "user",
            "parts": [{"kind": "text", "text": "打开客厅的灯"}],
            "messageId": "msg-001"
        }
    }
}
```

**响应示例**:
```json
{
    "jsonrpc": "2.0",
    "id": "req-001",
    "result": {
        "id": "task-uuid",
        "status": {"state": "completed"},
        "artifacts": [
            {
                "artifactId": "artifact-uuid",
                "name": "response",
                "parts": [{"kind": "text", "text": "已打开客厅的灯"}]
            }
        ]
    }
}
```

**tasks/get 请求示例**:
```json
{
    "jsonrpc": "2.0",
    "id": "req-002",
    "method": "tasks/get",
    "params": {"id": "task-uuid"}
}
```

---

## 六、测试覆盖

### 6.1 服务端测试

| 测试用例 | 说明 |
|---------|------|
| `agent_card_generation_defaults` | 默认 Agent Card 生成 |
| `agent_card_generation_custom` | 自定义配置 Agent Card |
| `rpc_rejects_missing_bearer_when_token_configured` | 缺少 Token 时拒绝 |
| `rpc_rejects_wrong_bearer_token` | 错误 Token 时拒绝 |
| `rpc_accepts_correct_bearer_token` | 正确 Token 时接受 |
| `rpc_allows_unauthenticated_when_no_auth_configured` | 无配置时允许匿名 |
| `task_store_lifecycle` | 任务存储生命周期 |
| `task_store_capacity_limit_enforced` | 容量限制 |
| `message_send_missing_text_returns_invalid_params` | 缺少消息参数 |
| `message_send_accepts_simple_text_fallback` | 简单文本格式 |
| `message_send_accepts_parts_format` | Parts 格式 |
| `message_send_rejects_when_store_full` | 存储满时拒绝 |

### 6.2 客户端测试

| 测试用例 | 说明 |
|---------|------|
| `validate_url_accepts_public_http` | 公开 URL 验证通过 |
| `validate_url_rejects_non_http` | 拒绝非 HTTP 协议 |
| `validate_url_rejects_private_hosts` | 拒绝私有主机 |
| `ssrf_helpers_block_cloud_metadata` | 阻止云元数据访问 |
| `discover_fetches_agent_card_from_server` | 发现 Agent Card |
| `send_dispatches_jsonrpc_and_returns_response` | 发送 JSON-RPC |
| `send_includes_bearer_token_when_provided` | 包含 Bearer Token |
| `read_only_autonomy_blocks_execution` | 只读模式阻止执行 |

---

## 七、提交历史

| 提交 | 说明 |
|-----|------|
| 5bd24680 | feat(interop): add A2A (Agent-to-Agent) protocol support |
| 6f45104f | fix(security): harden A2A protocol endpoints |
| 637f8905 | docs(config): add [a2a] section to config reference |
| 04da7de4 | fix(a2a): add tool description to bootstrap prompt |
| 846d426a | fix(a2a): fix CI — update validate_url tests |
| 9406a106 | feat(a2a): add Telegram group notifications |
| 3622fa40 | fix(a2a): add missing fields after rebase |
| faf2a668 | docs(a2a): mark implementation as MVP |
| 760dac97 | docs(a2a): add A2A protocol implementation status |
| 66efa982 | feat(a2a): add CORS support and web dashboard tester UI |
| a7acb764 | feat(a2a): add Strands Agents A2A instance |
| e5956b0f | feat(a2a): add LangChain A2A agent |
| fe010e19 | feat: add A2A outbound API endpoint |
| 4e9c0d07 | feat(a2a): add stream action for A2A tool |
| 38bec945 | feat(a2a): add Agent Card cache |
| fa34602e | refactor(a2a): simplify A2A tool |
| 7ceba455 | chore: remove A2A descriptions, add device-control skill |

---

## 八、待实现功能

根据代码注释，以下功能尚未实现（参见 issue #3566）：

- [ ] `message/stream` (SSE 服务端流式响应)
- [ ] `tasks/cancel` (取消任务)
- [ ] `input-required` 状态 / 多轮对话 (`contextId`)
- [ ] 推送通知
- [ ] 结构化/二进制消息部分 (`data`, `raw`)
- [ ] 异步任务执行
- [ ] 任务持久化

---

*文档生成时间: 2026-04-17*
