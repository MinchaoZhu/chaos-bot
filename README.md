# chaos-bot

Personal AI agent assistant with Rust backend, React shell, and Tauri v2 runtime.

## Part 1: 用户使用指南

### 1.1 从 GitHub 安装

推荐直接使用一行命令安装最新的 Linux `x86_64` 发布版本：

```bash
curl -fsSL https://raw.githubusercontent.com/MinchaoZhu/chaos-bot/master/scripts/install-from-github.sh | bash
```

如需自定义安装前缀：

```bash
curl -fsSL https://raw.githubusercontent.com/MinchaoZhu/chaos-bot/master/scripts/install-from-github.sh | \
  bash -s -- --prefix /opt/chaos-bot
```

安装脚本会自动：

- 读取最新 GitHub Release
- 下载最新 `linux-x86_64` 安装包
- 解压并执行 bundle 内的 `install.sh`

如果你想手动下载，发布版本仍然会产出一个 Linux `x86_64` 安装包，可直接从 GitHub Releases 获取：

- Releases 页面：`https://github.com/MinchaoZhu/chaos-bot/releases`
- 资产命名：`<release-version>-linux-x86_64.tar.gz`
- 版本标签：`v<release-version>`

示例：

```bash
curl -fL -o chaos-bot.tar.gz \
  https://github.com/MinchaoZhu/chaos-bot/releases/download/v0.1.1/0.1.1-linux-x86_64.tar.gz

mkdir -p /tmp/chaos-bot-install
tar -xzf chaos-bot.tar.gz -C /tmp/chaos-bot-install
/tmp/chaos-bot-install/0.1.1-linux-x86_64/install.sh
```

默认会安装到：

- `~/.local/share/chaos-bot/releases/<release-version>`
- `~/.local/bin/chaos-bot`

### 1.2 首次启动前配置

API Key 通过环境变量注入，不写入配置文件：

```bash
export OPENAI_API_KEY=sk-...
# 或
export ANTHROPIC_API_KEY=sk-ant-...
```

首次启动会自动生成 `~/.chaos-bot/config.json`。最小配置示例：

```json
{
  "llm": {
    "provider": "openai",
    "model": "gpt-5.2"
  }
}
```

支持的 provider：`openai` / `anthropic` / `gemini` / `mock`。

### 1.3 使用方式

安装完成后直接启动：

```bash
~/.local/bin/chaos-bot
```

默认行为：

- 后端 API 监听 `127.0.0.1:3000`
- 根路径 `/` 直接返回内置前端页面
- 浏览器访问 `http://127.0.0.1:3000` 即可使用

常用 API：

- `GET /api/health`
- `POST /api/chat`
- `GET/POST /api/sessions`
- `GET /api/config`
- `POST /api/config/apply`
- `GET /api/upgrade`
- `POST /api/upgrade/apply`

### 1.4 升级方式

已安装的 Linux bundle 支持直接在 Web UI 中检查并安装最新 GitHub Release：

1. 打开 `http://127.0.0.1:3000`
2. 进入右侧 `Config`
3. 在 `Web Upgrade` 卡片里点击 `Refresh Upgrade`
4. 如果检测到新版本，点击 `Install Latest Release`
5. 安装完成后，按提示重新启动 `~/.local/bin/chaos-bot`

底层仍然使用以下 API，因此也可以用 `curl` 手动触发：

```bash
curl -fsS http://127.0.0.1:3000/api/upgrade
curl -fsS -H 'content-type: application/json' -d '{}' http://127.0.0.1:3000/api/upgrade/apply
```

升级行为：

- 新版本会安装到新的 `~/.local/share/chaos-bot/releases/<release-version>` 目录
- `~/.local/bin/chaos-bot` 启动器会切换到新版本
- 升级完成后需要重新启动 `~/.local/bin/chaos-bot`

也可以重新执行一行安装命令，或手动下载新版本安装包后重复执行 `install.sh` 完成覆盖升级。

### 1.5 日志与排障

```bash
tail -f ~/.chaos-bot/logs/$(date +%F).log
```

相关文件：

- Runtime contract: `frontend-react/RUNTIME_CONTRACT.md`
- PM runtime status: `AGENTS.md`
- Tauri config: `src-tauri/tauri.conf.json`

---

## Part 2: 架构说明

### 2.1 核心功能

- 会话管理：创建/读取/删除会话，维护历史消息。
- 聊天流式输出：`/api/chat` 基于 SSE 输出 `session/delta/tool_call/done/error` 事件。
- Agent 工具链：内置 `read/write/edit/bash/grep/find/ls/memory_get/memory_search`。
- 配置中心：支持 `get/apply/reset/restart`，并带 `config.json.bak1/.bak2` 轮转备份。
- 多端壳：同一份 runtime contract 同时服务 Web React Shell 与 Tauri Shell。

### 2.2 架构分层（Backend DDD Frozen）

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

### 2.3 前后端/Tauri 关系

- `frontend-react/`：UI 与交互层。
- `src-tauri/`：Tauri invoke 桥接层。
- `backend/`：业务与能力中心。
- Web 模式链路：`frontend-react -> HTTP/SSE -> backend`
- Tauri 模式链路：`frontend-react -> invoke -> src-tauri -> HTTP/SSE -> backend`

### 2.4 技术栈

- Backend: Rust, Axum, Tokio
- Frontend: React 18, TypeScript, Vite
- Desktop/Mobile Shell: Tauri v2
- Testing: Rust tests + Playwright e2e

---

## Part 3: 开发与发布指南

### 3.1 环境准备

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

### 3.2 启动命令（后端 / 前端 / Tauri）

```bash
make frontend-install
make run
VITE_BACKEND_PROXY_TARGET=http://127.0.0.1:3000 make frontend-dev
make tauri-dev
make tauri-preflight
make tauri-build-desktop
```

### 3.3 测试命令

```bash
make test
make test-unit
make test-integration
make test-e2e
make test-all
```

### 3.4 CI/CD 与版本发布

- 仓库基础版本由根目录 `VERSION` 文件维护，并且必须与 `backend/Cargo.toml`、`frontend-react/package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 保持一致。
- 所有 `push` 都必须更新当前提交中的 `VERSION`；如果与 push 之前的分支版本相同，CI 会直接失败。
- 仓库只允许三种 commit 类型前缀：`Feature:`、`Fix:`、`Refactor:`。
- 每个 commit message 的首行都必须严格以 `Feature:`、`Fix:` 或 `Refactor:` 开头，后面直接跟单行摘要；不允许其他类型、merge 标题或临时标题。
- push 时会由 [`.githooks/pre-push`](/home/debian/projects/chaos-bot/.projects/cicd/.githooks/pre-push) 校验待推送提交，不符合格式的提交会被拒绝。
- 新 clone 或新 worktree 启用时需要执行一次：`git config core.hooksPath .githooks`。
- 本地发布前校验命令：`make release-check`
- 本地 Linux 安装包构建命令：`make package-linux-x86_64`
- 本地安装包验证命令：`make package-verify`
- 本地自升级验证命令：`make upgrade-verify`
- 本地一行安装脚本验证命令：`make install-verify`
- GitHub Actions 在 `master` push 时执行完整门禁并发布 GitHub Release。
- 发布标签格式为 `v<base-version>`，例如 `v0.1.1`。
- GitHub Release 标题使用纯版本号：`<base-version>`。
- 当前发布资产包含 `release-metadata.json`、`release-metadata.sha256`、`<release-version>-linux-x86_64.tar.gz`、对应 `.sha256` 与 bundle manifest。

### 3.5 Agent 开发指南

新增 agent 能力时，必须遵守：

- 模型 provider 只放在 `backend/src/infrastructure/model`。
- 工具注册与实现只放在 `backend/src/infrastructure/tooling`。
- `application` 只通过 `domain::ports::{ModelPort, ToolExecutorPort, MemoryPort}` 调用能力。
- 具体 adapter 注入必须在 `runtime` 完成。
- `README.md` 是架构/运行/测试单一文档入口，不新增根级 `docs/` 主文档。
- 如果变更存在约束、兼容性影响、升级动作或发布注意事项，commit body 必须补充清楚，保证基于 commit 生成的 changelog 有足够上下文。

### 3.6 架构与交付约束（必须）

- Backend 根目录只允许五层 + `lib.rs`：
  `application/ domain/ infrastructure/ interface/ runtime/ lib.rs`
- 禁止新增 `backend/src` 根级业务目录。
- 新功能必须覆盖多端一致性：
  - Backend API/行为完成
  - Frontend React Shell 完成
  - Tauri invoke/桥接完成
  - 对应测试（至少 e2e 主路径）完成
- 所有任务完成前必须通过 `make test-all`。
- 准备发布前必须检查待发布提交历史可用于生成 changelog：待发布提交必须全部通过 `Feature:` / `Fix:` / `Refactor:` 的 pre-push 校验，必要时先整理提交历史再推送。

### 3.7 Runtime / Config 规则

- 默认配置路径：`~/.chaos-bot/config.json`
- 兼容回退：若无 `config.json` 且存在 `~/.chaos-bot/agent.json`，读取 `agent.json`
- 启动自动物化默认配置
- Secret 合并顺序：先环境变量，再配置文件覆盖
- 每次写配置都旋转：`config.json.bak1`、`config.json.bak2`
