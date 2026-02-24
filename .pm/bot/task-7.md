# task-7: Workspace Logging Queue, Retention, and Observability Standards

## Task
- Description: 在 workspace 下新增 `logs/` 体系，建设队列式日志写入与按天保留策略（默认 7 天，可配置）；支持标准日志级别 `debug/info/warn/error`；在关键执行链路补齐排障日志；将日志规范同步到 `AGENTS`。
- Scope: `backend/src/main.rs`, `backend/src/config.rs`, `backend/src/api.rs`, `backend/src/agent.rs`, `backend/src/llm/mod.rs`, `backend/src/tools/*`, `backend/src/sessions.rs`, `backend/src/memory.rs`, `backend/tests/*`, `README.md`, `AGENTS.md`。
- Risk: 日志量过大可能引入 I/O 开销与噪声；队列刷新/退出时机处理不当可能丢日志；保留策略若误删会影响问题追溯。
- Status: done

## Phase 1: Logging Config and Directory Layout
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 在配置中新增日志配置段：级别、保留天数（默认 7）、输出目录（workspace/logs）。 | done | `agent.json` 新增 `logging.level/retention_days/directory`，并映射到 `AppConfig`。 |
| 1.2 | 启动时自动创建 `workspace/logs`，并统一日志文件命名规则。 | done | 启动时自动创建目录，按 `YYYY-MM-DD.log` 命名。 |
| 1.3 | 明确 stdout 与文件日志并存策略，兼顾本地调试与持久化排障。 | done | tracing subscriber 同时输出 stdout 与 file layer。 |
| 1.v1 | Verify: `cargo test --workspace --test unit_config` 覆盖日志配置解析与默认值。 | done | 通过（8/8）。 |

## Phase 2: Queue-Based Writer and Retention Cleanup
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 引入异步队列写入通道，解耦业务线程与磁盘 I/O。 | done | 采用 `tracing-appender` non-blocking writer（内部队列）写入文件。 |
| 2.2 | 实现日志落盘刷新与优雅退出 flush。 | done | 使用 `WorkerGuard` 保证进程退出前 flush。 |
| 2.3 | 实现按配置天数清理历史日志（默认仅保留近 7 天）。 | done | 启动时执行 retention 清理（按 `YYYY-MM-DD.log` 日期窗口）。 |
| 2.v1 | Verify: 新增单测覆盖队列写入、flush 与 retention 行为。 | done | 新增 `unit_logging`（2/2）覆盖 retention 与 flush。 |

## Phase 3: Critical Path Instrumentation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 在启动与配置加载阶段增加关键日志（workspace 解析、配置来源、初始化结果）。 | done | 启动日志包含 workspace/log_dir/log_file/log_level/retention。 |
| 3.2 | 在 API 入口、会话生命周期、LLM 请求/响应、工具调用链路增加结构化日志。 | done | 已在 `api/sessions/agent/llm/tools/memory/bootstrap` 增加关键结构化日志。 |
| 3.3 | 在错误路径补充统一错误事件日志，便于关联请求上下文。 | done | chat/llm/tool/memory 错误路径统一记录 `warn` 级别日志。 |
| 3.v1 | Verify: `cargo test --workspace --test unit_agent --test unit_llm --test unit_tools` 通过。 | done | 通过（80/80）。 |

## Phase 4: Dedicated E2E Log Validation
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 在 e2e 场景中验证日志文件在目标 workspace/logs 产生。 | done | Playwright 用例新增 workspace/logs 文件存在性校验。 |
| 4.2 | 增加 e2e 对日志级别与关键字段存在性的断言。 | done | 校验日志包含 `chaos-bot logging initialized` 与 `INFO`。 |
| 4.v1 | Verify: `make test-e2e` 通过。 | done | 通过（Playwright 5/5）。 |

## Phase 5: Logging Spec Documentation and Final Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 将日志规范写入 `AGENTS.md`（级别语义、字段规范、保留策略、排障流程）。 | done | `AGENTS` 新增日志规范章节并同步 `.pm/docs/AGENTS.md`。 |
| 5.2 | 更新 README 日志章节，补充配置示例与常见排错命令。 | done | README 新增 logging 配置、行为与排查命令。 |
| 5.v1 | Verify: `make test-all` 通过（任务完成门禁）。 | done | 通过（unit + integration + e2e）。 |

## Completion Record
- `cargo test --workspace --test unit_config --test unit_logging --test unit_llm`: passed (44/44)
- `cargo test --workspace --test unit_agent --test unit_tools`: passed (46/46)
- `cargo test --workspace --test api_integration --test api_routes`: passed (12/12)
- `make test-e2e`: passed (Playwright 5/5)
- `make test-all`: passed (unit + integration + e2e)
