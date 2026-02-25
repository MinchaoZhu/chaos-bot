# task-14: React Shell 增加 Config Tab

## Task
- Description: 在 `frontend-react` 增加一个可见可用的 `Config` Tab，支持查看当前配置并执行 `apply/reset/restart`，复用现有后端 `/api/config*` 能力。
- Scope: `frontend-react/src/{App.tsx,components/*,runtime/*,contracts/*}`，`src-tauri/src/lib.rs`（如需补齐 tauri invoke 命令），`e2e/tests/*`。
- Risk: `restart` 会触发进程退出，联调与 e2e 容易抖动；配置 JSON 编辑/校验失败会影响可用性；desktop/mobile 双形态下 tab 行为需要保持一致。
- Status: done

## Phase 1: 需求与交互约束冻结
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 冻结 Config Tab 在 desktop/mobile 的入口与导航位置。 | done | desktop 采用右侧 `Events/Config` 切换 tab；mobile 在 `MobilePaneTabs` 新增 `config`。 |
| 1.2 | 冻结配置操作边界。 | done | 支持 `get/apply/reset/restart`，统一 loading/错误/状态反馈。 |
| 1.3 | 冻结数据展示形态。 | done | `ConfigPanel` 展示 provider/model/format 摘要 + raw JSON 编辑区。 |
| 1.v1 | Verify: 任务文档完成交互与边界定义。 | done | 按 Phase 1 约束完成落地，未破坏现有 sessions/chat/events 流程。 |

## Phase 2: Runtime 适配与协议对接
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 扩展前端 `RuntimeAdapter` 配置接口。 | done | `runtime/adapter.ts` 与 `contracts/protocol.ts` 新增 config 类型与方法。 |
| 2.2 | 实现 HTTP adapter 配置调用链。 | done | `http-adapter.ts` 已接入 `/api/config*` 四个端点。 |
| 2.3 | 实现/补齐 Tauri adapter 与 `src-tauri` invoke 命令。 | done | `tauri-adapter.ts` + `src-tauri/src/lib.rs` 新增 `get/apply/reset/restart_config` 命令。 |
| 2.v1 | Verify: adapter 层调用与错误码映射验证。 | done | `npm --prefix frontend-react run build` passed；`cargo check --manifest-path src-tauri/Cargo.toml` passed。 |

## Phase 3: Config Tab UI 实现
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 新增 Config 视图组件与状态管理。 | done | 新增 `components/ConfigPanel.tsx`，含 `Reload/Apply/Reset/Restart` 与状态展示。 |
| 3.2 | 将 Config Tab 接入现有布局。 | done | `App.tsx` desktop 右侧 tab 切换；mobile 新增 `config` pane。 |
| 3.3 | 增加可读错误提示。 | done | ConfigPanel 与 App 均统一输出结构化错误，避免 `[object Object]`。 |
| 3.v1 | Verify: 手工联调 `get/apply/reset/restart`。 | done | 通过代理链路验证 `GET/POST /api/config*` 可达且响应正常。 |

## Phase 4: E2E 与完成门禁
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 新增 Config Tab 专项 e2e。 | done | `e2e/tests/react-shell.spec.ts` 新增 desktop/mobile config tab 流程断言。 |
| 4.2 | 补充 restart 场景 e2e 或回归说明。 | done | e2e 已覆盖 `Restart Runtime` 按钮路径（`CHAOS_BOT_DISABLE_SELF_RESTART=1` 下验证响应）。 |
| 4.v1 | Verify: `make test-e2e` 通过。 | done | `make test-e2e` passed（2 passed, 2 skipped）。 |
| 4.v2 | Verify: `make test-all` 通过（任务完成门禁）。 | done | `make test-all` passed（unit + integration + e2e）。 |

## Exit Criteria
- UI 中存在可用的 `Config` Tab（desktop/mobile 均可访问）。
- 前端 runtime 在 `http/tauri` 两种 transport 下均可调用配置 API。
- `get/apply/reset/restart` 有明确成功/失败反馈，并可读错误信息。
- 新增 Config Tab e2e 用例纳入 `make test-e2e`，且 `make test-all` 通过。
