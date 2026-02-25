# task-13: LLM/Tools 下沉 + 文档收敛到 README

## Task
- Description: 将 `backend/src/llm` 与 `backend/src/tools` 迁移到 `backend/src/infrastructure` 下，同时把 `docs/` 中现有架构与打包文档合并到 `README.md`，并在 `AGENTS.md` 固化 DDD 分层与模块化治理约束（禁止新增根级业务目录）。
- Scope: `backend/src/{llm,tools,infrastructure,runtime,application,lib.rs}`，`backend/tests/`，`e2e/tests/`，`README.md`，`AGENTS.md`，`docs/{backend-modular-architecture.md,tauri-packaging.md}`。
- Risk: 大规模路径迁移可能导致 import 断链、测试夹具失效；文档收敛可能产生信息遗漏；约束不清会导致后续再次出现根级业务目录漂移。
- Status: done

## Phase 1: Target Structure and Constraints Freeze
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 冻结目标目录树。 | done | 目录已收敛为 `application/domain/infrastructure/interface/runtime + lib.rs`，其中 `llm/tools` 落位至 `infrastructure/{model,tooling}`。 |
| 1.2 | 冻结兼容策略。 | done | 迁移期通过 `lib.rs` 兼容导出过渡，后续在调用方全部切换后已移除兼容壳。 |
| 1.3 | 冻结依赖方向和模块化约束。 | done | 约束已写入 README/AGENTS：`application` 仅依赖 `domain::ports`，注入由 `runtime` 负责。 |
| 1.v1 | Verify: 结构与约束草案写入任务文档。 | done | 本任务文档、README、AGENTS 三处已同步冻结规则。 |

## Phase 2: LLM Directory Migration
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 将 `backend/src/llm` 迁移至 `backend/src/infrastructure/model`。 | done | provider 实现与 `build_provider` 行为保持不变。 |
| 2.2 | 修正 `runtime` 与测试中的模块路径。 | done | `runtime` 与 tests 统一改用 `infrastructure::model` 路径。 |
| 2.3 | 清理旧入口并保留必要兼容导出。 | done | 移除根级 `llm` 目录，兼容导出收敛至 `lib.rs`。 |
| 2.v1 | Verify: LLM 相关测试通过。 | done | `cargo test --workspace --test unit_llm --test unit_tools --test unit_agent` passed（含 llm/agent 断链回归）。 |

## Phase 3: Tools Directory Migration
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 将 `backend/src/tools` 迁移至 `backend/src/infrastructure/tooling`。 | done | `ToolRegistry` 与工具实现完整迁移。 |
| 3.2 | 修正工具执行上下文与端口适配路径。 | done | 注入链路维持 `runtime -> ToolRegistry -> ToolExecutorPort`。 |
| 3.3 | 清理旧路径引用并补齐导出。 | done | 移除根级 `tools` 目录，调用方与 tests 改为 `infrastructure::tooling`。 |
| 3.v1 | Verify: Tools 相关测试通过。 | done | `cargo test --workspace --test unit_llm --test unit_tools --test unit_agent` passed。 |

## Phase 4: Documentation Merge and Governance
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 合并 `docs/backend-modular-architecture.md` 到 `README.md`。 | done | README 已内联架构分层、依赖方向、端口适配链路与启动流。 |
| 4.2 | 合并 `docs/tauri-packaging.md` 到 `README.md`。 | done | README 已覆盖 packaging matrix、依赖前置、排障与 CI 顺序。 |
| 4.3 | 更新 `AGENTS.md` 的文档与架构治理约束。 | done | AGENTS 已新增文档单一来源与 DDD/模块化治理硬约束。 |
| 4.4 | 清理文档引用与过期说明。 | done | README/AGENTS 不再依赖 `docs/*`；根级 `docs/` 已删除。 |
| 4.v1 | Verify: README 文档完整性检查。 | done | README 已覆盖架构、启动、打包、测试与 CI artifact 说明。 |
| 4.v2 | Verify: AGENTS 约束检查。 | done | AGENTS 已明确“文档单一来源 + DDD 分层 + 模块化治理”。 |

## Phase 5: Boundary Cleanup and Release Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 清理遗留 `llm/tools` 顶层目录与引用。 | done | 根级目录已移除，`backend/src` 达到目标形态。 |
| 5.v1 | Verify: API/integration 测试通过。 | done | `cargo test --workspace --test api_routes --test api_integration` passed (10/10)。 |
| 5.v2 | Verify: `make test-e2e` 通过并保留失败证据链。 | done | `make test-e2e` passed (Playwright 2 passed, 2 skipped; react-shell desktop/mobile)。 |
| 5.v3 | Verify: `make test-all` 通过（任务完成门禁）。 | done | `make test-all` passed（unit + integration + e2e）。 |

## Exit Criteria
- `backend/src/llm` 与 `backend/src/tools` 顶层目录被移除或仅保留受控兼容壳。
- `backend/src/infrastructure/model` 与 `backend/src/infrastructure/tooling` 成为唯一实现位置。
- `README.md` 成为架构/打包说明的单一入口；`docs/` 不再作为主维护面。
- `AGENTS.md` 明确禁止新增根级业务目录并固化 DDD 模块化约束。
- `make test-all` 通过后任务才可标记 `done`。
