# task-7: Workspace Logging Queue, Retention, and Observability Standards

## Task
- Description: 在 workspace 下新增 `logs/` 体系，建设队列式日志写入与按天保留策略（默认 7 天，可配置）；支持标准日志级别 `debug/info/warn/error`；在关键执行链路补齐排障日志；将日志规范同步到 `AGENTS`。
- Scope: `backend/src/main.rs`, `backend/src/config.rs`, `backend/src/api.rs`, `backend/src/agent.rs`, `backend/src/llm/mod.rs`, `backend/src/tools/*`, `backend/src/sessions.rs`, `backend/src/memory.rs`, `backend/tests/*`, `README.md`, `AGENTS.md`。
- Risk: 日志量过大可能引入 I/O 开销与噪声；队列刷新/退出时机处理不当可能丢日志；保留策略若误删会影响问题追溯。
- Status: todo

## Phase 1: Logging Config and Directory Layout
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 在配置中新增日志配置段：级别、保留天数（默认 7）、输出目录（workspace/logs）。 | todo | 级别仅支持 `debug/info/warn/error`。 |
| 1.2 | 启动时自动创建 `workspace/logs`，并统一日志文件命名规则。 | todo | 按日期或滚动文件方案确定最小可行格式。 |
| 1.3 | 明确 stdout 与文件日志并存策略，兼顾本地调试与持久化排障。 | todo | 保持开发体验不退化。 |
| 1.v1 | Verify: `cargo test --workspace --test unit_config` 覆盖日志配置解析与默认值。 | todo | |

## Phase 2: Queue-Based Writer and Retention Cleanup
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 引入异步队列写入通道，解耦业务线程与磁盘 I/O。 | todo | 明确背压策略与错误处理。 |
| 2.2 | 实现日志落盘刷新与优雅退出 flush。 | todo | 防止进程退出导致尾部日志丢失。 |
| 2.3 | 实现按配置天数清理历史日志（默认仅保留近 7 天）。 | todo | 以文件时间戳进行清理并记录清理摘要。 |
| 2.v1 | Verify: 新增单测覆盖队列写入、flush 与 retention 行为。 | todo | |

## Phase 3: Critical Path Instrumentation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 在启动与配置加载阶段增加关键日志（workspace 解析、配置来源、初始化结果）。 | todo | 覆盖成功与失败分支。 |
| 3.2 | 在 API 入口、会话生命周期、LLM 请求/响应、工具调用链路增加结构化日志。 | todo | 对敏感字段做脱敏/截断。 |
| 3.3 | 在错误路径补充统一错误事件日志，便于关联请求上下文。 | todo | 统一字段键名与错误分类。 |
| 3.v1 | Verify: `cargo test --workspace --test unit_agent --test unit_llm --test unit_tools` 通过。 | todo | |

## Phase 4: Dedicated E2E Log Validation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 在 e2e 场景中验证日志文件在目标 workspace/logs 产生。 | todo | 校验至少一条 API/会话/工具日志。 |
| 4.2 | 增加 e2e 对日志级别与关键字段存在性的断言。 | todo | 确认排障关键信息可见。 |
| 4.v1 | Verify: `make test-e2e` 通过。 | todo | |

## Phase 5: Logging Spec Documentation and Final Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 将日志规范写入 `AGENTS.md`（级别语义、字段规范、保留策略、排障流程）。 | todo | 与 PM 文档保持一致。 |
| 5.2 | 更新 README 日志章节，补充配置示例与常见排错命令。 | todo | 对使用者可直接执行。 |
| 5.v1 | Verify: `make test-all` 通过（任务完成门禁）。 | todo | |
