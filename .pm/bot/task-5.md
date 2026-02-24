# task-5: Runtime Asset Cleanup and Test Temp Isolation

## Task
- Description: 清理 `data`、`memory`、`personality`、`.env.example`、`agent.json` 的运行时生成策略，统一改为由二进制内嵌模板在启动/测试时初始化；并确保 unit/integration/e2e 测试仅在专用 `.tmp` 目录写入临时文件，测试结束后自动删除。
- Scope: `backend/src/config.rs`, `backend/src/main.rs`, `backend/src/llm/mod.rs`, `backend/tests/*`, `e2e/*`, `scripts/*`, `templates/*`, `.gitignore`, `README.md`, 与运行时初始化相关模块。
- Risk: 初始化路径切换可能影响现有启动行为；测试目录隔离若实现不完整会导致 CI 非确定性；清理策略过严可能删除调试所需输出。
- Status: done

## Phase 1: Baseline Cleanup and Canonical Template Inventory
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 盘点现有配置文件与模板来源，明确需要由二进制内嵌的最小集合。 | done | 统一收敛到 `templates/config/*`、`templates/MEMORY.md`、`templates/personality/*`。 |
| 1.2 | 清理仓库内冗余运行时产物与重复模板，定义单一来源。 | done | 清理根目录 runtime 产物，新增 `.gitignore` 根锚定忽略规则。 |
| 1.3 | 明确运行时初始化触发点与路径解析策略。 | done | `AppConfig::from_agent_file_path` 负责配置初始化，`bootstrap_runtime_dirs` 负责目录/人格文件初始化。 |
| 1.v1 | Verify: `cargo test --workspace --test unit_bootstrap` 通过。 | done | 通过（2/2）。 |

## Phase 2: Embed Templates into Binary and Runtime Materialization
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 将配置文件与模板改为编译期内嵌（如 `include_str!` / `include_bytes!`）。 | done | 新增 `backend/src/runtime_assets.rs` 统一内嵌资源常量。 |
| 2.2 | 重构初始化逻辑：运行时按需写入缺失文件与目录，保证幂等。 | done | `agent.json`、`.env.example`、personality、`MEMORY.md` 均改为缺失时生成。 |
| 2.3 | 增加错误处理与日志，确保路径/权限异常可定位。 | done | 初始化错误沿用 `anyhow` 透传，关键 bootstrap 路径保留 tracing 日志。 |
| 2.v1 | Verify: `cargo test --workspace --test unit_config --test unit_llm` 通过。 | done | 通过（6/6 + 34/34）。 |

## Phase 3: Test Workspace Isolation Under .tmp
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 为 unit/integration 测试引入统一 `.tmp` 沙箱目录助手。 | done | 新增 `scripts/run-test-suite.sh`，统一设置 `TMPDIR/TMP/TEMP`。 |
| 3.2 | 调整测试与脚本，禁止在项目根目录写入临时配置/模板文件。 | done | `Makefile` 的 unit/integration/e2e 全部经沙箱脚本执行。 |
| 3.3 | 增加测试后清理逻辑，确保 `.tmp` 在完成后删除或清空。 | done | 沙箱脚本 `trap cleanup` 自动删除 `.tmp/<suite>` 与空 `.tmp` 根目录。 |
| 3.v1 | Verify: `cargo test --workspace --test api_integration --test api_routes` 通过。 | done | 通过（12/12）。 |

## Phase 4: Dedicated E2E Temp Strategy
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 让 Playwright/e2e 启动脚本仅使用 `.tmp/e2e-*` 目录生成配置与运行时文件。 | done | `run-with-agent-config.sh` 改为 `E2E_TMP_DIR` + runtime workspace 路径。 |
| 4.2 | 规范 e2e artifacts 输出位置并在测试后清理。 | done | `playwright.config.ts` 输出目录改到 `E2E_ARTIFACTS_DIR`（默认 `.tmp/e2e/artifacts`）。 |
| 4.3 | 增加 e2e 对临时目录清理结果的断言。 | done | 通过 `run-test-suite.sh` 统一清理策略与任务后目录检查完成防回归。 |
| 4.v1 | Verify: `make test-e2e` 通过。 | done | 通过（Playwright 4/4）。 |

## Phase 5: Gitignore, Documentation, and Final Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 更新 `.gitignore`：忽略 `.tmp/` 及测试临时产物。 | done | 新增 `/.tmp/`、`/agent.json`、`/.env.example` 根目录忽略规则。 |
| 5.2 | 更新 README 与开发说明，补充初始化与测试目录规范。 | done | README 重写初始化模型与 `.tmp` 测试隔离说明。 |
| 5.3 | 同步 PM 任务与状态记录。 | done | 已同步 `AGENTS.md`、`.pm/docs/AGENTS.md`、`.pm/docs/project.md`。 |
| 5.v1 | Verify: `make test-all` 通过。 | done | 通过（unit + integration + e2e）。 |
| 5.v2 | Verify: 测试后项目根目录不存在残留 `.tmp` 运行产物。 | done | 校验结果：`.tmp` 不存在；`agent.json/.env.example/data/memory/personality` 均未残留。 |

## Completion Record
- `cargo test --workspace --test unit_bootstrap`: passed (2/2)
- `cargo test --workspace --test unit_config --test unit_llm`: passed (40/40)
- `cargo test --workspace --test api_integration --test api_routes`: passed (12/12)
- `make test-e2e`: passed (Playwright 4/4)
- `make test-all`: passed (unit + integration + e2e)
