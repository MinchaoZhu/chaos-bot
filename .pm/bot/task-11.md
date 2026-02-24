# task-11: Agent Backend Modular Architecture Refactor

## Task
- Description: 对 agent 后端进行模块化重构，消除当前目录平铺与职责耦合问题，建立可扩展、可测试、可观测的分层架构。
- Scope: `backend/`（目录重组与模块边界）, `tests/` 或 `backend/tests/`, `README.md`, `AGENTS.md`。
- Risk: 大规模迁移可能引入回归；模块拆分后依赖方向若不受控，可能形成新的循环依赖；重构期间需保证 API 对外行为稳定。
- Status: todo

## Phase 1: Architecture Definition and Boundary Agreement
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 定义目标分层：`interface`、`application`、`domain`、`infrastructure`、`runtime`。 | todo | 明确每层职责与依赖方向。 |
| 1.2 | 划分核心 bounded context：会话编排、模型调用、工具链、配置中心、日志观测。 | todo | 形成模块与 owner 映射。 |
| 1.3 | 制定迁移顺序与兼容策略（Strangler 模式）。 | todo | 先抽稳定接口，再逐步搬迁实现。 |
| 1.v1 | Verify: 架构文档评审通过并冻结模块边界。 | todo | 输出目录树、依赖图、迁移里程碑。 |

## Phase 2: Incremental Refactor and Module Extraction
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 拆分 API 路由层与用例层，移除 handler 内业务逻辑。 | todo | 路由只做协议转换与错误映射。 |
| 2.2 | 抽离 domain 模型与策略，隔离 LLM provider 与工具执行端口。 | todo | 通过 trait/interface 注入实现。 |
| 2.3 | 重组基础设施模块（配置、存储、日志、外部客户端）。 | todo | 避免横向工具函数散落。 |
| 2.v1 | Verify: unit/integration 测试在每个迁移里程碑保持通过。 | todo | 阶段性执行相关测试集。 |

## Phase 3: Reliability, Security, and Observability Hardening
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 统一错误模型与错误码，补充跨层错误传播策略。 | todo | 便于 API 与日志消费。 |
| 3.2 | 建立统一脱敏与审计日志中间层（配置、工具参数、模型请求）。 | todo | 符合敏感信息不可落盘要求。 |
| 3.v1 | Verify: 关键日志字段完整且无 secret 泄漏。 | todo | 抽样检查 startup/api/tool 链路日志。 |

## Phase 4: E2E Regression and Release Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 增加后端重构后的端到端回归场景。 | todo | 覆盖会话、配置热更新、工具调用、失败恢复。 |
| 4.v1 | Verify: `make test-e2e` 通过并保留失败证据链。 | todo | 对齐 CI artifact 规范。 |
| 4.v2 | Verify: `make test-all` 通过（任务完成门禁）。 | todo | 满足 Mandatory Rules。 |
