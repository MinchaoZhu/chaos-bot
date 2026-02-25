# task-2: Chat Slash Commands（/ 命令体系）

## Task
- Description: 在聊天输入框新增 `/` 命令体系，支持模型查看/切换、会话管理与上下文压缩等常见快捷操作。
- Scope: `frontend-react` 命令解析与执行链路、runtime adapter 配置接口扩展、相关单元/集成/e2e 测试。
- Risk: Medium. `/compact` 需要在现有 API 下实现“可用且可预期”的压缩体验，需控制失败回退和状态一致性。
- Status: done

## Command Set (MVP)
- `/help`: 显示可用命令和示例。
- `/model`: 显示当前 provider/model（读取运行中配置）。
- `/models`: 显示可选模型列表；支持 `/models set <model_id>` 切换当前模型。
- `/new`: 创建并切换到新会话。
- `/compact`: 压缩当前上下文（生成摘要并迁移到新会话，减少后续上下文负担）。
- `/clear`: 清空输入框草稿。
- `/sessions`: 在移动端切换到 sessions pane（桌面端高亮提示会话区域）。

## Phase 1: Command UX + Parser Foundation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 定义 slash command 语法、参数约定与错误提示规范。 | done | 新增 `frontend-react/src/commands/slash.ts`，约定命令语法、错误消息、提示过滤策略和 `//` 转义。 |
| 1.2 | 在前端新增命令注册表与解析器（`/cmd args`），接入现有发送入口。 | done | `App.tsx` 已接入 parser + executor，普通消息与命令路径完全分离。 |
| 1.3 | 为输入框增加命令提示（最少支持 `/` 触发列表 + 命令说明）。 | done | `ConversationPanel` 新增命令提示列表与点击补全，`styles.css` 增加样式。 |
| 1.v1 | Verify: 命令解析器单元测试通过。 | done | `npm --prefix frontend-react run test:unit` passed (`tests/slash-parser.test.mjs` 1 passed)。 |

## Phase 2: Model Commands (/model, /models)
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 扩展 runtime contract/adapter，接入 `/api/config` 与 `/api/config/apply`。 | done | `protocol.ts`、`runtime/adapter.ts`、`http-adapter.ts`、`tauri-adapter.ts` 已支持 `getConfig/applyConfig`。 |
| 2.2 | 实现 `/model`：展示当前 provider/model 与来源（running config）。 | done | `/model` 从 running config 读取 provider/model，并写入事件面板日志。 |
| 2.3 | 实现 `/models` 与 `/models set <model_id>`。 | done | 已实现 provider 白名单、模型列表展示、`set` 参数校验和热更新 apply。 |
| 2.v1 | Verify: model 命令执行链路测试通过。 | done | `e2e/tests/slash-commands-desktop.spec.ts` 覆盖 `/model` 与 `/models set` 主链路并通过。 |

## Phase 3: Session Commands (/new, /compact, /sessions, /clear, /help)
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 实现 `/new` 与 `/clear`。 | done | `/new` 复用创建会话流程；`/clear` 仅清空输入草稿。 |
| 3.2 | 实现 `/help` 与 `/sessions`。 | done | `/help` 输出命令目录；`/sessions` 在 mobile 切 pane，desktop 给出提示。 |
| 3.3 | 实现 `/compact`（摘要压缩 + 新会话迁移）。 | done | 已实现摘要生成 -> 新会话 -> 注入 compact prompt -> 刷新会话与日志。 |
| 3.v1 | Verify: 会话类命令组件行为测试通过。 | done | `slash-commands-desktop/mobile` e2e 覆盖 `/new`、`/compact`、`/sessions`、`/clear` 并通过。 |

## Phase 4: E2E + Completion Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 新增 desktop e2e：验证 `/model`、`/models set`、`/new`、`/compact` 主链路。 | done | 新增 `e2e/tests/slash-commands-desktop.spec.ts` 并通过。 |
| 4.2 | 新增 mobile e2e：验证 `/sessions` pane 切换与命令执行反馈。 | done | 新增 `e2e/tests/slash-commands-mobile.spec.ts`，并拆分 `react-shell` desktop/mobile spec 去除跨项目 skip。 |
| 4.v1 | Verify: `make test-e2e` 通过。 | done | passed: 4/4（desktop 2 + mobile 2，0 skipped）。 |
| 4.v2 | Verify: `make test-all` 通过（任务门禁）。 | done | passed: unit/integration/e2e 全量通过（task gate satisfied）。 |
