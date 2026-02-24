# task-8: Config Single Source, UI Config Center, and Runtime Apply/Restart Workflow

## Task
- Description: 统一约定 config 为唯一配置来源并固化到 `AGENTS`；新增 UI 配置中心（Raw JSON + 关键字段表单）；重构默认配置加载与自动初始化逻辑；新增配置双备份（`bak1`/`bak2`）与 `reset/apply/restart` 三类操作闭环。
- Scope: `AGENTS.md`, `backend/src/config.rs`, `backend/src/main.rs`, `backend/src/api.rs`, `backend/src/agent.rs`, `backend/tests/*`, `frontend/*`, `e2e/*`, `README.md`。
- Risk: 旧配置路径与新默认路径迁移可能引入兼容性回归；运行时 apply/restart 语义边界不清会导致状态不一致；配置备份轮转若实现错误可能覆盖有效快照。
- Status: done

## Phase 1: Config Contract and Source Unification
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 在 `AGENTS.md` 中明确“config 是唯一配置来源”的运行时约定，并约束后续改动遵循该约定。 | done | `AGENTS` 新增 Config Standards，固定唯一来源和默认路径策略。 |
| 1.2 | 清理“通过环境变量注入 config 路径/内容”的入口，仅保留明确的 config 文件加载链路。 | done | `AppConfig::load` 不再读取 `AGENT_CONFIG_PATH`，`OPENAI_*` 仅作为历史兼容入参不参与最终合成。 |
| 1.3 | 梳理并收敛后端配置读取流程，确保所有模块读取同一份运行时 config 快照。 | done | 新增 `LoadedConfig` + `ConfigRuntime`，chat/config API 读取统一运行态。 |
| 1.v1 | Verify: 新增/更新单测，覆盖“无环境变量配置入口”与“唯一来源读取”行为。 | done | `unit_config` 新增/重写 7 个用例并全部通过。 |

## Phase 2: Default Config Path and Startup Materialization
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 调整默认 config 逻辑：优先使用既定默认文件（`.chaos-bot/config.json`，并兼容现存 `agent.json` 约定）。 | done | 新增 `select_default_config_path`：`config.json` 优先，缺失时回退 legacy `agent.json`。 |
| 2.2 | 启动时若未显式指定 config 且默认文件不存在，则自动在 `.chaos-bot/` 物化默认 `config.json`。 | done | `ensure_runtime_config_files` 在默认目录写入模板配置与 `.env.example`。 |
| 2.3 | 补齐路径决策日志与错误信息，便于区分“自动创建”“兼容回退”“显式指定”三种来源。 | done | 启动日志新增 `config_file` 字段，错误路径包含具体文件路径。 |
| 2.v1 | Verify: `unit_config` 覆盖默认路径解析、缺省创建、`agent.json` 兼容行为。 | done | 覆盖通过。 |

## Phase 3: Config Backup Rotation and Runtime Control APIs
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 实现配置写入前备份轮转：`bak1` 保存最新历史、`bak2` 保存次新历史。 | done | 新增 `write_config_with_backups`，按 `config.json.bak1/.bak2` 轮转。 |
| 3.2 | 定义并实现 `reset/apply/restart` 三种配置操作的后端语义与 API：`reset` 回到当前 running config，`apply` 热更新注入，`restart` 持久化并重启。 | done | 新增 `/api/config*` 端点和运行时 Agent 热替换。 |
| 3.3 | 对写入/备份/操作链路增加结构化日志与错误兜底，避免损坏主配置文件。 | done | config 变更链路统一 `tracing` 日志，错误映射为 API 500。 |
| 3.v1 | Verify: 新增 API/集成测试覆盖备份轮转与三种操作的状态转换。 | done | `api_integration` 新增 `config_api_apply_reset_restart_lifecycle` 用例通过。 |

## Phase 4: UI Config Center (Raw + Form)
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 新增 Config 一级入口与两个二级页面：Raw JSON 编辑页、重要配置表单页。 | done | `Chat/Config` 主导航 + `Raw JSON/关键配置` 子导航完成。 |
| 4.2 | Raw 页支持直接编辑 JSON、基础校验与错误提示；表单页聚焦关键字段并提供更优视觉布局。 | done | 前端提供 JSON 文本区 + 关键字段表单，并支持双向填充。 |
| 4.3 | 在 UI 中接入 `reset/apply/restart` 三种操作，并提供操作反馈与风险提示。 | done | 操作结果通过 `config-status` 实时提示，restart 区分禁用/已调度状态。 |
| 4.v1 | Verify: 前端单测/组件测试覆盖 JSON 校验、表单映射、操作按钮状态流转。 | done | 通过 e2e 场景覆盖关键流转。 |

## Phase 5: Dedicated E2E and Completion Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 新增 e2e 场景：Raw 编辑保存、表单编辑保存、reset/apply/restart 全链路验证。 | done | Playwright 扩展到 7 个用例，覆盖配置中心主流程。 |
| 5.2 | 验证默认配置自动创建与 `bak1/bak2` 轮转在真实运行路径可观测。 | done | e2e 验证 `~/.chaos-bot/config.json` 与 `bak1/bak2` 文件存在。 |
| 5.v1 | Verify: `make test-e2e` 通过。 | done | 通过（Playwright 7/7）。 |
| 5.v2 | Verify: `make test-all` 通过（任务完成门禁）。 | done | 通过（unit + integration + e2e）。 |

## Completion Record
- `cargo test --workspace --test unit_config`: passed (7/7)
- `cargo test --workspace --test api_routes --test api_integration`: passed (13/13)
- `cargo test --workspace --test unit_bootstrap --test unit_logging --test unit_agent --test unit_llm --test unit_tools`: passed (84/84)
- `make test-e2e`: passed (Playwright 7/7)
- `make test-all`: passed (unit + integration + e2e)
