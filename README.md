# chaos-bot

Personal AI agent assistant with Rust backend, React shell, and Tauri v2 runtime.

## Part 1: 功能、架构、技术栈

### 1.1 核心功能

- 会话管理：创建/读取/删除会话，维护历史消息。
- 聊天流式输出：`/api/chat` 基于 SSE 输出 `session/delta/tool_call/done/error` 事件。
- Agent 工具链：内置 `read/write/edit/bash/grep/find/ls/memory_get/memory_search`。
- 配置中心：支持 `get/apply/reset/restart`，并带 `config.json.bak1/.bak2` 轮转备份。
- 多端壳：同一份 runtime contract 同时服务 Web React Shell 与 Tauri Shell。

### 1.2 架构分层（Backend DDD Frozen）

```text
backend/src
  application/      # use cases (agent/chat/config/session)
  domain/           # core models, errors, ports contracts
  infrastructure/   # adapters
    model/          # ModelPort implementations
    tooling/        # ToolExecutorPort implementations
  interface/        # HTTP/SSE router + handlers
  runtime/          # bootstrap + DI + binary composition
  lib.rs
```

依赖方向（必须保持）：

1. `interface -> application -> domain`
2. `runtime -> {application, interface, infrastructure}`
3. `application` 只能依赖 `domain::ports`，不能直接依赖具体 adapter
4. 反向依赖、跨层倒挂禁止

### 1.3 前后端/Tauri 关系

- `frontend-react/`：UI 与交互层。
- `src-tauri/`：Tauri invoke 桥接层。
- `backend/`：业务与能力中心。
- Web 模式链路：`frontend-react -> HTTP/SSE -> backend`
- Tauri 模式链路：`frontend-react -> invoke -> src-tauri -> HTTP/SSE -> backend`

### 1.4 技术栈

- Backend: Rust, Axum, Tokio
- Frontend: React 18, TypeScript, Vite
- Desktop/Mobile Shell: Tauri v2
- Testing: Rust tests + Playwright e2e

---

## Part 2: 开发指南（含 Agent 约束）

### 2.1 环境准备

- Rust toolchain
- Node.js 20+
- Linux desktop (Tauri):

```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev
```

### 2.2 启动命令（后端 / 前端 / Tauri）

```bash
# 安装前端依赖
make frontend-install

# 启动后端（默认 3000）
make run

# 启动前端开发服务器（默认 1420）
# 建议开发联调时让前端代理到后端
VITE_BACKEND_PROXY_TARGET=http://127.0.0.1:3000 make frontend-dev

# 启动 Tauri Desktop Dev Shell
make tauri-dev

# Tauri 环境检查
make tauri-preflight

# Tauri Desktop Debug Build（不打包原生 bundle）
make tauri-build-desktop
```

### 2.3 测试命令

```bash
# 单元 + 集成
make test

# 仅单元
make test-unit

# 仅集成
make test-integration

# 仅 e2e (Playwright)
make test-e2e

# 全量门禁（必过）
make test-all
```

### 2.4 Agent 开发指南

新增 agent 能力时，必须遵守：

- 模型 provider 只放在 `backend/src/infrastructure/model`。
- 工具注册与实现只放在 `backend/src/infrastructure/tooling`。
- `application` 只通过 `domain::ports::{ModelPort, ToolExecutorPort, MemoryPort}` 调用能力。
- 具体 adapter 注入必须在 `runtime` 完成。
- `README.md` 是架构/运行/测试单一文档入口，不新增根级 `docs/` 主文档。

### 2.5 架构与交付约束（必须）

- Backend 根目录只允许五层 + `lib.rs`：
  `application/ domain/ infrastructure/ interface/ runtime/ lib.rs`
- 禁止新增 `backend/src` 根级业务目录。
- 新功能必须覆盖多端一致性：
  - Backend API/行为完成
  - Frontend React Shell 完成
  - Tauri invoke/桥接完成
  - 对应测试（至少 e2e 主路径）完成
- 所有任务完成前必须通过 `make test-all`。

### 2.6 Runtime / Config 规则

- 默认配置路径：`~/.chaos-bot/config.json`
- 兼容回退：若无 `config.json` 且存在 `~/.chaos-bot/agent.json`，读取 `agent.json`
- 启动自动物化默认配置
- Secret 合并顺序：先环境变量，再配置文件覆盖
- 每次写配置都旋转：`config.json.bak1`、`config.json.bak2`

---

## Part 3: 使用说明（怎么用、有哪些功能）

### 3.1 快速开始

1. 启动后端：`make run`
2. 启动前端：`VITE_BACKEND_PROXY_TARGET=http://127.0.0.1:3000 make frontend-dev`
3. 浏览器打开：`http://127.0.0.1:1420`
4. `Backend URL` 建议填写：`http://127.0.0.1:1420`（通过 dev proxy 访问）

### 3.2 主要功能面板

- Sessions
  - `Refresh`：刷新健康状态和会话列表
  - `New`：创建新会话
- Conversation
  - 输入消息后 `Send`，实时流式显示 assistant 回复
- Stream Events
  - 查看 SSE 事件与 tool call 轨迹，便于排障
- Config（新增）
  - `Reload Config`：获取运行态/磁盘配置
  - `Apply Config`：应用编辑后的 raw JSON 配置
  - `Reset Config`：将磁盘配置重置到运行快照
  - `Restart Runtime`：请求进程重启（可被运行模式禁用）

### 3.3 API 入口（常用）

- `GET /api/health`
- `POST /api/chat` (SSE)
- `GET/POST /api/sessions`
- `GET/DELETE /api/sessions/:id`
- `GET /api/config`
- `POST /api/config/apply`
- `POST /api/config/reset`
- `POST /api/config/restart`

### 3.4 日志与排障

```bash
# 查看当日日志
tail -f ~/.chaos-bot/logs/$(date +%F).log

# 清理运行时产物和 .tmp
make clean-runtime
```

### 3.5 相关文件

- Runtime contract: `frontend-react/RUNTIME_CONTRACT.md`
- PM runtime status: `AGENTS.md`
- Tauri config: `src-tauri/tauri.conf.json`

