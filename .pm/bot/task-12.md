# task-12: LLM/Tools Port 化下沉重构

## Task
- Description: 将 `llm` 与 `tools` 从“应用层直接依赖的具体实现”重构为“由应用层端口驱动、基础设施层适配”的架构，消除 `application::agent` 对实现细节的编译时耦合。
- Scope: `backend/src/{application,domain,infrastructure,llm,tools,runtime}`，`backend/tests/`，`e2e/tests/`，`docs/backend-modular-architecture.md`，`README.md`，`AGENTS.md`。
- Risk: 端口抽象过度可能造成类型复杂度上升；事件流/SSE 输出契约需保持不变；迁移期兼容层处理不当会导致测试回归。
- Status: done

## Phase 1: Port Contract Freeze and Dependency Boundary
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 定义模型调用端口（Model Port）与工具执行端口（Tool Executor Port）接口契约。 | done | 新增 `domain/ports.rs`：`ModelPort`、`ToolExecutorPort`、`MemoryPort` 与对应请求/事件模型。 |
| 1.2 | 明确依赖方向：`application -> domain(port) -> infrastructure(adapter)`。 | done | `application::agent` 已仅依赖 `domain::ports`，不再直接依赖 `llm/tools`。 |
| 1.3 | 制定迁移策略与兼容窗口。 | done | 采用别名兼容：`llm` 对外继续暴露 `Llm*` 名称，底层切换到 domain ports。 |
| 1.v1 | Verify: 端口契约文档冻结。 | done | `docs/backend-modular-architecture.md` 与 `README.md` 已更新端口/适配器说明。 |

## Phase 2: LLM Port 化与 Adapter 注入
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 引入 LLM 端口 trait 与领域级请求/响应模型。 | done | `ModelRequest/ModelResponse/ModelStreamEvent` 已在 domain 定义并被 agent 使用。 |
| 2.2 | 将 OpenAI/Anthropic/Gemini/Mock provider 归入 infrastructure adapter 角色。 | done | `llm/mod.rs` 通过 `pub use domain::ports as Llm*` 兼容旧接口并承载 provider 实现。 |
| 2.3 | 在 `runtime` 完成 provider adapter 注入与生命周期管理。 | done | `runtime::build_agent_loop` 注入 `build_provider(config)` 到 `AgentLoop` 的 `ModelPort`。 |
| 2.v1 | Verify: LLM 迁移相关单测通过。 | done | `cargo test --workspace --test unit_llm --test unit_agent` 通过。 |

## Phase 3: Tools Port 化与 Adapter 注入
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 定义工具执行端口与统一执行结果模型。 | done | `ToolExecutionContext` + `ToolExecutorPort::execute` 已在 domain ports 固化。 |
| 3.2 | 将工具注册表与具体工具实现归入 infrastructure adapter。 | done | `ToolRegistry` 已实现 `ToolExecutorPort`；`application::agent` 不再调用 `dispatch` 细节。 |
| 3.3 | 迁移工具审计日志链路，保证字段不回退。 | done | tool audit 字段保持不变：`tool_call_id/tool_name/is_error`。 |
| 3.v1 | Verify: Tools 迁移相关单测通过。 | done | `cargo test --workspace --test unit_tools --test unit_agent` 通过。 |

## Phase 4: Cleanup and Boundary Enforcement
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 移除 `application` 到 `llm/tools` 实现层的直接 import。 | done | 静态扫描确认 `backend/src/application` 无 `crate::llm` / `crate::tools` 直接依赖。 |
| 4.2 | 收敛兼容导出与模块入口。 | done | `llm` 保持 `Llm*` 公共名，内部已切到 `domain::ports`；工具上下文改为端口上下文。 |
| 4.3 | 更新架构文档与 README。 | done | `docs/backend-modular-architecture.md` 与 `README.md` 已补充 ports/adapters 与启动链路。 |
| 4.v1 | Verify: API/integration 回归通过。 | done | `cargo test --workspace --test api_routes --test api_integration` 通过。 |

## Phase 5: E2E Regression and Release Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 增加“端口化后 provider/tool 链路”专门 e2e 场景。 | done | 新增 `chat.spec.ts` 用例：`llm/tools port adapters: apply config then tool + model chain stays healthy`。 |
| 5.v1 | Verify: `make test-e2e` 通过并保留失败证据链。 | done | 通过（Playwright 11 passed, 2 skipped）。 |
| 5.v2 | Verify: `make test-all` 通过（任务完成门禁）。 | done | 通过（unit + integration + e2e，全量门禁）。 |

## Exit Criteria
- `application::agent` 不再直接依赖 `llm/tools` 具体实现。
- 模型调用与工具执行均通过端口注入完成。
- 架构文档与 README 已更新端口化后的依赖结构。
- `make test-all` 通过后任务方可标记 `done`。

## Completion Notes
- 新增 `backend/src/domain/ports.rs` 作为 ports 契约中心。
- `llm` 模块改为对外兼容别名（`Llm*`），内部基于 `domain::ports`。
- `tools::ToolRegistry` 实现 `ToolExecutorPort`，`application::agent` 通过端口执行工具。
