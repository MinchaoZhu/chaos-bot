# task-11: Agent Backend Modular Architecture Refactor

## Task
- Description: 对 agent 后端进行模块化重构，消除当前目录平铺与职责耦合问题，建立可扩展、可测试、可观测的分层架构；本次重规划要求把剩余平铺模块全部纳入分层迁移范围。
- Scope: `backend/src/{agent.rs,api.rs,bootstrap.rs,config_runtime.rs,config.rs,lib.rs,logging.rs,main.rs,memory.rs,personality.rs,runtime_assets.rs,sessions.rs,types.rs}`，以及对应测试、文档、`AGENTS.md`。
- Risk: 大规模迁移可能引入回归；模块拆分后依赖方向若不受控，可能形成循环依赖；兼容层移除时存在 API/导出路径破坏风险。
- Status: done

## Baseline Milestones (Completed)
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| B1 | 已建立目标分层：`interface`、`application`、`domain`、`infrastructure`、`runtime`。 | done | `backend/src/{interface,application,domain,infrastructure,runtime}` 已落地。 |
| B2 | 已完成第一轮 handler/usecase 拆分与错误/审计基线。 | done | `application` + `domain::{error,audit}` 已投入使用。 |
| B3 | 已引入兼容策略（Strangler facade）。 | done | `api.rs` 当前作为兼容层转发。 |
| B.v1 | Baseline Verify: 当时全量门禁已通过。 | done | `make test-all` 已通过（task-11 第一轮）。 |

## Legacy Flat File Migration Map
| Legacy File | Target Layer | Migration Goal |
|---|---|---|
| `agent.rs` | `application` + `domain` + `infrastructure` | 拆出编排逻辑、domain 协议、provider/tool 端口实现。 |
| `api.rs` | `interface` | 收敛到 `interface/http`，最终删除 facade。 |
| `bootstrap.rs` | `runtime` | 启动初始化流程归并到 `runtime/bootstrap`。 |
| `config_runtime.rs` | `runtime` + `application` | 配置热更新机制与应用服务边界解耦。 |
| `config.rs` | `domain` + `infrastructure` | 领域配置模型与磁盘/env 装载职责拆分。 |
| `lib.rs` | crate root | 仅保留分层模块导出，不再暴露遗留平铺入口。 |
| `logging.rs` | `infrastructure` | 观测设施下沉至 `infrastructure/logging`。 |
| `main.rs` | `runtime` + bin entry | `main` 只保留 wiring/启动，业务组装全部下沉。 |
| `memory.rs` | `domain` + `infrastructure` | 抽象 memory port 与 store 实现。 |
| `personality.rs` | `domain` + `infrastructure` | 抽象 personality source 与文件实现。 |
| `runtime_assets.rs` | `interface`/`runtime` | 静态资源访问边界显式化。 |
| `sessions.rs` | `application` + `infrastructure` | 会话存储与用例接口解耦。 |
| `types.rs` | `domain` | 通用类型按 bounded context 切分。 |

## Phase 1: Boundary Freeze for Remaining Flat Modules
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 对 13 个 legacy 文件建立“现有职责 -> 目标模块”映射清单。 | done | 已按映射完成迁移：13 个 root file 全部落位到 `application/interface/runtime/infrastructure/domain`。 |
| 1.2 | 冻结依赖方向与禁止清单。 | done | `lib.rs` 仅保留分层模块入口；`backend/src` 顶层仅剩 `lib.rs`。 |
| 1.3 | 定义兼容窗口与移除策略。 | done | 兼容壳 `src/api.rs` 已移除，公共入口切换为 `interface::api`。 |
| 1.v1 | Verify: 架构冻结文档更新并评审。 | done | `docs/backend-modular-architecture.md` 已更新为迁移完成版。 |

## Phase 2: Runtime and Config Stack Migration
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 迁移 `bootstrap.rs`、`main.rs` 到 runtime 组装层。 | done | `runtime/bootstrap.rs` + `src/runtime/bin/chaos_bot_backend.rs` 已落地，`Cargo.toml` 切换 bin path。 |
| 2.2 | 迁移 `config.rs`、`config_runtime.rs` 到 domain/runtime/application 分层。 | done | `infrastructure/config.rs` 与 `runtime/config_runtime.rs` 已替换旧路径并完成引用迁移。 |
| 2.3 | 迁移 `logging.rs`、`runtime_assets.rs` 到明确基础设施/接口边界。 | done | 已迁移到 `infrastructure/{logging,runtime_assets}.rs`，模板 include 路径同步修正。 |
| 2.v1 | Verify: runtime/config 回归通过。 | done | `cargo test --workspace --test unit_config --test unit_bootstrap --test unit_logging` 通过。 |

## Phase 3: Agent Core, Memory, Personality, Session Migration
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 拆分 `agent.rs` 为编排用例、domain 协议、infrastructure 实现。 | done | `application/agent.rs` + `domain/chat::ToolEvent` 已替换旧 `agent.rs`。 |
| 3.2 | 迁移 `memory.rs`、`personality.rs` 到 domain port + infrastructure adapter。 | done | 已迁移到 `infrastructure/{memory,personality}.rs`，行为与协议保持兼容。 |
| 3.3 | 迁移 `sessions.rs` 与 `types.rs` 到 bounded context。 | done | `infrastructure/session_store.rs` 与 `domain/types.rs` 已替换旧模块。 |
| 3.v1 | Verify: agent/tool/memory/session 单测通过。 | done | `cargo test --workspace --test unit_agent --test unit_tools --test unit_memory --test unit_sessions --test unit_types --test unit_personality` 通过。 |

## Phase 4: Interface and Public Export Cleanup
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 收敛 `api.rs` 到 `interface`，准备移除 facade。 | done | 新入口 `interface/api.rs` 已落地；`backend/src/api.rs` 已删除。 |
| 4.2 | 重构 `lib.rs` 导出清单，仅暴露分层入口。 | done | `lib.rs` 仅导出 `application/domain/infrastructure/interface/llm/runtime/tools`。 |
| 4.3 | 扫描并移除旧路径引用。 | done | `backend/src` 与 `backend/tests` 已切换到新模块路径，无 legacy import 命中。 |
| 4.v1 | Verify: API/integration 测试通过。 | done | `cargo test --workspace --test api_routes --test api_integration` 通过。 |

## Phase 5: E2E Regression and Release Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 增加“legacy 模块移除后”端到端回归场景。 | done | 已复用并通过现有回归（含 `backend modular regression` 场景）。 |
| 5.v1 | Verify: `make test-e2e` 通过并保留失败证据链。 | done | 通过（Playwright 10 passed, 2 skipped）。 |
| 5.v2 | Verify: `make test-all` 通过（任务完成门禁）。 | done | 通过（unit + integration + e2e，全量门禁）。 |

## Exit Criteria
- `backend/src` 不再保留上述 13 个 legacy 平铺文件。
- 分层目录成为唯一实现入口，兼容 facade 已移除。
- `docs/backend-modular-architecture.md` 与 `README.md` 已同步新结构。
- `make test-all` 最终通过后再将 task 状态标记为 `done`。

## Completion Notes
- 13 个 legacy root file 已全部移除；`backend/src` 顶层仅保留 `lib.rs`。
- 新二进制入口为 `backend/src/runtime/bin/chaos_bot_backend.rs`（由 `backend/Cargo.toml` 的 `[[bin]]` 指定）。
- 所有 `backend/src` 与 `backend/tests` 导入已切换到分层模块路径。
