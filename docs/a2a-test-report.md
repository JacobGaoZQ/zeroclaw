# ZeroClaw 多 Agent A2A 通信测试报告

> 环境：GitHub Codespaces
> 版本：v0.6.4
> 协议：A2A (Agent-to-Agent) Protocol MVP

---

## 一、目标

在 GitHub Codespaces 环境中启动两个独立的 ZeroClaw 实例（Agent-A、Agent-B），通过 A2A 协议实现跨 Agent 通信，并在 Agent-A 的 Web UI 页面中完成端对端测试。

---

## 二、配置修改

### Agent-A 配置（`/tmp/zeroclaw-a/config.toml`）

```toml
default_provider = "qwen"
default_model = "qwen-plus"
default_temperature = 0.7
api_key = "<your-api-key>"

[gateway]
port = 8080
host = "0.0.0.0"
allow_public_bind = true
require_pairing = false

[a2a]
enabled = true
agent_name = "Agent-A"
description = "A2A Test Agent A - Alpha node"
public_url = "https://<CODESPACE_NAME>-8080.app.github.dev"
bearer_token = "a2a-shared-secret"
allow_local = true

[cost]
enabled = false
```

关键配置项说明：

| 配置项 | 值 | 原因 |
|---|---|---|
| `gateway.host` | `0.0.0.0` | 绑定所有网卡，Codespaces 端口转发需要 |
| `gateway.allow_public_bind` | `true` | 允许绑定非 localhost，否则启动报错 |
| `gateway.require_pairing` | `false` | 测试阶段关闭配对验证，简化测试流程 |
| `a2a.enabled` | `true` | 开启 A2A 服务端和出站工具 |
| `a2a.public_url` | Codespaces 公网域名 | Agent Card 中对外声明的服务地址，必须是外部可达的 URL |
| `a2a.bearer_token` | `a2a-shared-secret` | A2A 请求认证 Token，两端保持一致 |
| `a2a.allow_local` | `true` | 允许出站 A2A 工具访问本地地址（开发环境用） |

---

### Agent-B 配置（`/tmp/zeroclaw-b/config.toml`）

```toml
default_provider = "qwen"
default_model = "qwen-plus"
default_temperature = 0.7
api_key = "<your-api-key>"

[gateway]
port = 8081
host = "0.0.0.0"
allow_public_bind = true
require_pairing = false

[a2a]
enabled = true
agent_name = "Agent-B"
description = "A2A Test Agent B - Beta node"
public_url = "https://<CODESPACE_NAME>-8081.app.github.dev"
bearer_token = "a2a-shared-secret"
allow_local = true

[cost]
enabled = false
```

与 Agent-A 的差异：

| 配置项 | Agent-A | Agent-B |
|---|---|---|
| `gateway.port` | `8080` | `8081` |
| `a2a.agent_name` | `Agent-A` | `Agent-B` |
| `a2a.description` | Alpha node | Beta node |
| `a2a.public_url` | `...-8080.app.github.dev` | `...-8081.app.github.dev` |

两个实例的 `bearer_token` 相同（`a2a-shared-secret`），使 Agent-A 可以直接向 Agent-B 发起认证请求。

---

## 三、Codespaces 启动步骤

### 1. 准备独立配置目录

每个实例需要独立的配置目录，避免互相干扰：

```bash
mkdir -p /tmp/zeroclaw-a /tmp/zeroclaw-b
```

将各自的 config.toml 写入对应目录（注意将 `<CODESPACE_NAME>` 替换为实际值）：

```bash
# 查看当前 Codespace 名称
echo $CODESPACE_NAME
```

`public_url` 格式为：

```
https://<CODESPACE_NAME>-<PORT>.app.github.dev
```

### 2. 启动两个实例

使用 `--config-dir` 指定配置目录（不能用环境变量，必须用此参数）：

```bash
# 启动 Agent-A（后台运行）
zeroclaw --config-dir /tmp/zeroclaw-a gateway start > /tmp/agent-a.log 2>&1 &

# 启动 Agent-B（后台运行）
zeroclaw --config-dir /tmp/zeroclaw-b gateway start > /tmp/agent-b.log 2>&1 &
```

确认启动成功（日志中应出现 `A2A protocol enabled`）：

```bash
cat /tmp/agent-a.log
cat /tmp/agent-b.log
```

### 3. 开放 Codespaces 端口可见性

Codespaces 端口默认为 **Private**，浏览器跨端口请求会被拦截，导致 UI 中的 Remote Agent 测试失败。

操作步骤：
1. 打开 VS Code 底部 **Ports** 面板（或 GitHub Codespaces 网页的 Ports 标签）
2. 找到端口 `8080` 和 `8081`
3. 右键 → **Port Visibility** → 改为 **Public**（或 Organization）

### 4. 验证端点可用性

```bash
# 验证 Agent-A
curl http://localhost:8080/.well-known/agent-card.json

# 验证 Agent-B
curl http://localhost:8081/.well-known/agent-card.json
```

两者均应返回包含各自 `agent_name` 和 Codespaces `public_url` 的 JSON。

---

## 四、在 Agent-A UI 中测试与 Agent-B 的通信

打开 Agent-A 的 Web Dashboard：

```
https://<CODESPACE_NAME>-8080.app.github.dev
```

进入左侧菜单 **A2A Test** 页面。

### 参数配置

点击 **Remote Agent** 切换到远端模式，填写：

| 字段 | 填写值 |
|---|---|
| Agent URL | `https://<CODESPACE_NAME>-8081.app.github.dev` |
| Bearer Token | `a2a-shared-secret` |

### 测试步骤

**Step 1 — 发现（discover）**

选择 `discover`，点击 Run。

调用 `GET <Agent-B URL>/.well-known/agent-card.json`，成功后返回 Agent-B 的能力声明，确认通信链路正常。

**Step 2 — 发送消息（send）**

选择 `send`，在 Message 框输入内容，点击 Run。

调用 `POST <Agent-B URL>/a2a`（JSON-RPC `message/send`），Agent-B 的 LLM 处理后返回结果。响应卡片中包含 `task_id`，点击 **Use Task ID** 可自动填入。

**Step 3 — 查询状态（status）**

选择 `status`，Task ID 已自动填入，点击 Run。

查询 Agent-B 上该任务的执行状态（`submitted` / `working` / `completed` / `failed`）。

**Step 4 — 获取结果（result）**

选择 `result`，点击 Run。

获取 Agent-B 返回的 artifacts（完整响应内容）。

---

## 五、常见问题

**启动时报 `Address already in use`**

两个实例读取了同一份配置（默认 `~/.zeroclaw/config.toml`），未使用 `--config-dir` 参数，或两份配置的端口相同。

**`discover` 在 UI 中报网络错误**

Codespaces 端口可见性为 Private，需按第三节第 3 步将 8080、8081 改为 Public。

**`send` 返回 HTTP 401**

`bearer_token` 填写错误，或 Agent-B 配置中 `bearer_token` 与请求中的值不一致。

**agent card 中 `url` 仍是 `localhost`**

`a2a.public_url` 未更新为 Codespaces 域名，重新修改配置后重启实例。
