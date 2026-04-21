# A2A-v2 分支与 v0.6.5 代码改动文档

**对比基准**: `v0.6.5` (zeroclaw-labs/zeroclaw)
**当前分支**: `A2A-v2`
**变更规模**: 25 个文件，+3208 / -31 行

---

## 一、新增核心模块（A2A 协议实现）

| 文件路径 | 说明 | 代码行数 |
|---------|------|---------|
| `src/gateway/a2a.rs` | A2A 服务端（入站） | ~1088 行 |
| `src/tools/a2a.rs` | A2A 客户端工具（出站） | ~1081 行 |

### 1.1 服务端 (`src/gateway/a2a.rs`)

- **Agent Card 发现**: `GET /.well-known/agent-card.json`
- **JSON-RPC 2.0 端点**: `POST /a2a`
- **支持方法**:
  - `message/send` — 同步请求/响应，发送消息并返回结果
  - `tasks/get` — 任务状态轮询，查询任务执行状态
- **认证机制**: Bearer Token + 常量时间比较（防时序攻击），回退 Gateway Pairing 认证
- **任务管理**: 内存存储（`TaskStore`），上限 10,000 并发任务，状态枚举 `Submitted / Working / Completed / Failed`
- **Telegram 通知**: 配置 `notify_chat_id` 后，任务完成时推送 Markdown 格式通知到 Telegram 群组
- **安全措施**: 容量限制、认证验证、错误消息不回显用户输入

**未实现**: `message/stream` (SSE)、`tasks/cancel`、多轮对话 (`contextId`)、Push 通知、结构化/二进制消息部分、异步任务执行、任务持久化

### 1.2 客户端工具 (`src/tools/a2a.rs`)

- **支持动作**:

| 动作 | 说明 | 必需参数 |
|-----|------|---------|
| `discover` | 发现远程 Agent Card | `url` |
| `send` | 发送一次性任务消息（用于设备控制） | `message`, `pat_token` |
| `stream` | 流式文本响应 | `message`, `pat_token` |
| `status` | 查询任务状态 | `task_id` |
| `result` | 获取任务结果 | `task_id` |

- **SSRF 防护**: 禁止请求私有/本地 IP，重定向跟踪限制 10 跳
- **安全策略**: 只读模式下拒绝执行
- **`pat_token`**: 作为每次调用参数传入（非全局配置），通过 `x-user-token` header 传递给远端

---

## 二、配置系统

### 2.1 `src/config/schema.rs` (+125 行)

新增 `A2aConfig` 结构体：

```rust
pub struct A2aConfig {
    pub enabled: bool,
    pub agent_name: Option<String>,
    pub description: Option<String>,
    pub public_url: Option<String>,
    pub bearer_token: Option<String>,
    pub version: Option<String>,
    pub capabilities: Vec<String>,
    pub notify_chat_id: Option<i64>,
    pub strands_agent_url: Option<String>,
    pub client_token: Option<String>,
}
```

- `Debug` 实现中 `bearer_token` 和 `client_token` 用 `"***"` 脱敏
- `Config` 结构体新增 `a2a: A2aConfig` 字段
- `Config::apply_env_overrides()` 新增环境变量覆盖：

| 环境变量 | 对应字段 |
|---------|---------|
| `A2A_ENABLED` | `enabled` |
| `A2A_AGENT_NAME` | `agent_name` |
| `A2A_DESCRIPTION` | `description` |
| `A2A_PUBLIC_URL` | `public_url` |
| `A2A_BEARER_TOKEN` | `bearer_token` |
| `A2A_VERSION` | `version` |
| `A2A_CAPABILITIES` | `capabilities`（逗号分隔） |
| `A2A_NOTIFY_CHAT_ID` | `notify_chat_id` |
| `A2A_STRANDS_AGENT_URL` | `strands_agent_url` |
| `A2A_CLIENT_TOKEN` | `client_token` |

### 2.2 `src/config/mod.rs`

导出 `A2aConfig`。

### 2.3 `src/gateway/api.rs`

- `mask_sensitive_fields()`: 脱敏 `a2a.bearer_token`
- `restore_masked_sensitive_fields()`: 恢复 `a2a.bearer_token`

---

## 三、Gateway 集成

### 3.1 `src/gateway/mod.rs` (+74 行)

- `AppState` 新增字段：
  - `a2a_agent_card: Option<Arc<serde_json::Value>>` — 缓存的 Agent Card
  - `a2a_task_store: Option<Arc<a2a::TaskStore>>` — 内存任务存储
- 启用时安全警告：
  - 无认证警告：未设置 `bearer_token` 且未启用 `require_pairing`
  - 内网地址暴露警告：未设置 `public_url`
- 新增路由：
  - `GET /.well-known/agent-card.json` → `a2a::handle_agent_card`
  - `POST /a2a` → `a2a::handle_a2a_rpc`
- 新增 `CorsLayer`（CORS 全放通，用于跨框架 A2A 通信）：
  ```rust
  CorsLayer::new()
      .allow_origin(Any)
      .allow_methods(Any)
      .allow_headers(Any)
  ```
- 所有测试中 `AppState` 初始化补充 A2A 字段为 `None`

---

## 四、工具注册与 Agent Loop

### 4.1 `src/tools/mod.rs` (+17 行)

当 `a2a.enabled` 时注册 `A2aTool`：

```rust
if root_config.a2a.enabled {
    let client_token = root_config.a2a.client_token.clone()
        .or_else(|| std::env::var("A2A_CLIENT_TOKEN").ok());
    tool_arcs.push(Arc::new(a2a::A2aTool::with_config(
        security.clone(), 30,
        root_config.a2a.strands_agent_url.clone(),
        client_token,
    )));
}
```

### 4.2 `src/agent/loop_.rs` (+4 行)

A2A 启用时将 `"a2a"` 工具描述注入 prompt：

```rust
if config.a2a.enabled {
    tool_descs.push(("a2a", ""));
}
```

---

## 五、启动流程

### 5.1 `src/main.rs` (+12 行)

在日志初始化之前加载 `.env` 文件：

```rust
match dotenvy::dotenv() {
    Ok(path) => eprintln!("[zeroclaw] .env file loaded from: {}", path.display()),
    Err(_) => { /* .env is optional */ }
}
```

---

## 六、依赖变更 (`Cargo.toml`)

| 依赖 | 变更 |
|-----|------|
| `dotenvy` | 新增 `0.15`（.env 文件加载） |
| `futures-util` | 新增 `async-await` feature |
| `futures-core` | 新增 `0.3` |
| `tower-http` | 新增 `cors` feature |

---

## 七、配置文件与环境

| 文件 | 说明 |
|-----|------|
| `.env.example` | 新增 A2A 协议相关环境变量示例（+41 行） |
| `.envrc` | 新增 `.env` 文件自动加载 |
| `docs/reference/api/config-reference.md` | 新增 `[a2a]` 配置文档 |
| `docs/vi/config-reference.md` | 越南语配置文档 |
| `docs/zh-CN/reference/api/config-reference.zh-CN.md` | 中文配置文档 |

---

## 八、技能与前端

| 文件 | 说明 |
|-----|------|
| `skills/device-control/SKILL.md` | 设备控制技能定义（支持控制、查询、列表） |
| `skills/device-control/README.md` | 技能配置与使用说明 |
| `tool_descriptions/en.toml` | 英文工具描述 |
| `tool_descriptions/zh-CN.toml` | 中文工具描述 |
| `web/src/lib/i18n.ts` | Web 前端国际化（+81 行） |

---

## 九、其他变更

| 文件 | 说明 |
|-----|------|
| `src/onboard/wizard.rs` | 向导默认配置补充 `a2a` 字段 |
| `src/channels/mod.rs` | 测试中 `debouncer` 重复初始化 |

---
