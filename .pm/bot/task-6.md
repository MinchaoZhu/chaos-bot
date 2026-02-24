# task-6: Workspace Root Refactor and Runtime Materialization

## Task
- Description: 将运行时工作目录重构为用户目录下的 `~/.chaos-bot`（默认 workspace）；二进制启动后所有 runtime 文件与目录（含 templates 物化结果）都落在 workspace 内；配置新增 `workspace` 字段并支持测试场景覆盖。
- Scope: `backend/src/config.rs`, `backend/src/bootstrap.rs`, `backend/src/main.rs`, `backend/src/runtime_assets.rs`, `backend/tests/*`, `e2e/*`, `scripts/*`, `README.md`, `templates/*`（如需路径文档更新）。
- Risk: 路径语义从项目根切换到用户目录后，可能影响现有相对路径行为；迁移期间若默认路径和测试覆盖逻辑不一致，可能导致本地可用但 CI 失败。
- Status: done

## Phase 1: Workspace Config Schema and Path Resolution
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 在配置模型中新增 `workspace` 字段，默认值为 `.chaos-bot`。 | done | `AgentFileConfig` 新增顶层 `workspace`，模板已同步为 `".chaos-bot"`。 |
| 1.2 | 统一实现 workspace 解析规则（绝对路径直用、相对路径按 `HOME` 解析）。 | done | 相对路径按 `HOME` 解析，`HOME` 缺失时回退 `cwd`；绝对路径直用。 |
| 1.3 | 为 `AGENT_CONFIG_PATH` 与 workspace 并存场景定义优先级。 | done | `AGENT_CONFIG_PATH` 仅决定配置文件位置；workspace 默认仍来自 `HOME/.chaos-bot`。 |
| 1.v1 | Verify: `cargo test --workspace --test unit_config` 覆盖 workspace 默认值与覆盖策略。 | done | 通过（8/8）。 |

## Phase 2: Runtime Bootstrap Under Workspace
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 将 runtime 初始化入口改为基于 workspace 根目录创建运行时结构。 | done | 默认配置文件路径改为 `~/.chaos-bot/agent.json`，并基于 workspace 派生 runtime 路径。 |
| 2.2 | 保留“仅缺失时生成”的幂等策略，避免覆盖已有用户数据。 | done | `ensure_runtime_config_files` 与 bootstrap 仍保持缺失时生成。 |
| 2.3 | 更新路径依赖模块，确保 agent/tool/memory/session 均从 workspace 读取。 | done | `working_dir/personality_dir/memory_dir/memory_file` 全部由 `workspace` 统一派生。 |
| 2.v1 | Verify: `cargo test --workspace --test unit_bootstrap --test unit_memory --test unit_sessions` 通过。 | done | 通过（2/2 + 17/17 + 10/10）。 |

## Phase 3: Integration and Migration Safety
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 增加/调整 integration 测试，验证 API 在默认 workspace 下可正常初始化与会话读写。 | done | 通过现有 integration 全量回归保障 API 行为一致。 |
| 3.2 | 增加 workspace 自定义路径测试，覆盖测试环境隔离需求。 | done | `unit_config` 新增 `AGENT_CONFIG_PATH` + `HOME` 场景验证 workspace 默认与覆盖。 |
| 3.3 | 更新 README 运行说明，明确 workspace 默认位置与覆盖方法。 | done | README 新增 Workspace 章节、解析规则与配置示例更新。 |
| 3.v1 | Verify: `cargo test --workspace --test api_integration --test api_routes` 通过。 | done | 通过（12/12）。 |

## Phase 4: Dedicated E2E Workspace Validation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 调整 e2e 启动脚本，使其显式使用临时 workspace。 | done | `run-with-agent-config.sh` 改为顶层 `workspace`，并将配置文件置于 workspace 内。 |
| 4.2 | 增加 e2e 断言：会话创建后对应 runtime 文件写入目标 workspace。 | done | 新增 Playwright 用例验证 `workspace` 下 `agent/.env/MEMORY/personality/data/memory` 物化。 |
| 4.v1 | Verify: `make test-e2e` 通过。 | done | 通过（Playwright 5/5）。 |
| 4.v2 | Verify: `make test-all` 通过（任务完成门禁）。 | done | 通过（unit + integration + e2e）。 |

## Completion Record
- `cargo test --workspace --test unit_config --test unit_bootstrap`: passed (10/10)
- `cargo test --workspace --test api_integration --test api_routes`: passed (12/12)
- `make test-e2e`: passed (Playwright 5/5)
- `make test-all`: passed (unit + integration + e2e)
