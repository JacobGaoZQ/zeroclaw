# CodeSpaces 快速开始指南

在 GitHub CodeSpaces 中编译、启动 ZeroClaw 并在 Web Dashboard 中与 Agent 对话的完整步骤。

## 一、项目概览

ZeroClaw 是一个纯 Rust 编写的 AI Agent 运行时，核心二进制为 `zeroclaw`。它内置了基于 React + Vite 的 Web Dashboard，通过 `rust-embed` 在编译时将 `web/dist/` 前端资源打包进二进制，无需额外部署前端。

Gateway（网关）默认监听 `127.0.0.1:42617`，提供：
- Web Dashboard (`/`)
- WebSocket 实时对话 (`/ws/chat`)
- REST API (`/api/*`)
- Webhook 接入 (`/webhook`)

## 二、前置依赖

CodeSpaces 默认环境通常已包含 Rust，如未安装或版本过低：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env
rustc --version   # 要求 >= 1.87
```

## 三、编译项目

```bash
cd /workspaces/zeroclaw
cargo build --release --locked
```

编译产物位于 `target/release/zeroclaw`。

> 若不想全局安装，后续所有 `zeroclaw` 命令可替换为 `cargo run --release --`。

## 四、初始化配置（Onboard）

复制.env.example，命名.env，注释其他所有配置，仅配置以下几项：

A2A_ENABLED=true
ZEROCLAW_PROVIDER=qwen
ZEROCLAW_MODEL=qwen-plus
DASHSCOPE_API_KEY=xxxxxx
A2A_STRANDS_AGENT_URL=http://120.26.206.98:8000/a2a
A2A_CLIENT_TOKEN=your-a2a-api-key-here

配置将写入 `~/.zeroclaw/config.toml`。

## 五、安装模拟测试的SKILL

./target/release/zeroclaw skills list — 查看当前已安装的 skills
mkdir -p /home/codespace/.zeroclaw/workspace/skills && cp -r /workspaces/zeroclaw/skills/device-control
/home/codespace/.zeroclaw/workspace/skills/ — 创建 skills 目录并将 device-control 复制过去
./target/release/zeroclaw skills list — 验证 skill 加载成功（显示 device-control v0.3.0）

## 六、启动 Gateway

设置环境变量

```bash
echo 'alias zeroclaw="/workspaces/zeroclaw/target/release/zeroclaw"' >> ~/.bashrc 
source ~/.bashrc 
```

启动命令

```bash
zeroclaw gateway 或 cargo run --release -- gateway
```

启动成功后，终端将输出类似：

```
🦀 ZeroClaw Gateway listening on http://0.0.0.0:42617
  🌐 Web Dashboard: http://0.0.0.0:42617/

  🔐 PAIRING REQUIRED — use this one-time code:
     ┌──────────────┐
     │  ABCD1234    │
     └──────────────┘
```

## 七、CodeSpaces 端口转发

1. 切换到 VS Code 底部 **Ports** 面板
2. CodeSpaces 会自动分配一个公网可访问的 URL，例如：
   ```
   https://your-codespace-name-42617.github.dev
   ```
3. 右键该端口 → **Open in Browser**

> 或者可点击端口行上的 **globe 图标** 打开。

## 八、在 Dashboard 中与 Agent 对话

1. **访问 Dashboard**：浏览器打开 CodeSpaces 转发的 URL
2. **配对认证**：首次访问会要求输入 Pairing Code。从 Gateway 终端日志中找到一次性配对码（如 `ABCD1234`），输入即可获取 Bearer Token
   - 或运行：`cargo run --release -- gateway get-paircode --new`
3. **进入 Agent Chat**：Dashboard 左侧导航选择 **Agent Chat**
4. **开始对话**：在输入框中发送消息，Agent 会通过 WebSocket (`/ws/chat`) 实时返回流式响应

## 九、常用命令速查

| 操作 | 命令 |
|------|------|
| 编译 | `cargo build --release --locked` |
| 快速初始化 | `cargo run --release -- onboard --quick --api-key ... --provider ...` |
| 启动 Gateway | `cargo run --release -- gateway` |
| 指定 Host/Port | `cargo run --release -- gateway --host 0.0.0.0 --port 42617` |
| 获取新配对码 | `cargo run --release -- gateway get-paircode --new` |
| 查看状态 | `cargo run --release -- status` |
| 运行诊断 | `cargo run --release -- doctor` |
| 交互式 CLI 对话 | `cargo run --release -- agent` |

## 十、故障排查

- **Dashboard 显示 "Web dashboard not available"**：说明 `web/dist/` 缺失。需要在前端目录执行 `cd web && npm ci && npm run build`，然后重新编译 Rust 项目。
- **端口转发后页面无法访问**：确认 `gateway.host` 已设为 `0.0.0.0`，且 CodeSpaces 端口转发状态为 **Forwarded**（绿色）。
- **配对码失效/找不到**：运行 `cargo run --release -- gateway get-paircode --new` 生成新的配对码，或重启 Gateway。
- **API Key 未生效**：确认 `~/.zeroclaw/config.toml` 中 `api_key` 字段已填写，或通过环境变量 `ZEROCLAW_API_KEY` 传入。
