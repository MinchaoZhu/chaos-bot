# task-4: Agent Config Refactor (Remove Env-Driven Runtime Config)

## Task
- Description: 删除 `CHAOS_*` 等 env 业务配置入口，统一改为基于 `agent.json` 的配置加载；支持配置注入，并实现“通用 API key 可从环境变量读取，再由 `agent.json` 覆盖”的优先级。
- Scope: `backend/src/config.rs`, `backend/src/main.rs`, `backend/src/llm/mod.rs`, `backend/tests/unit_config.rs`, `backend/tests/unit_llm.rs`, `e2e/playwright.config.ts`, `README.md`, `agent.json`（新增模板/默认文件）。
- Risk: 配置优先级切换可能导致启动行为变化；e2e 启动配置来源切换可能导致不稳定；多 provider key 映射遗漏会造成运行时错误。
- Status: done

## Phase 1: Agent JSON Schema & Loader

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 设计并定义 `agent.json` 的配置结构（server/llm/paths/secrets）。 | done | 已实现 `AgentFileConfig` 及嵌套 schema。 |
| 1.2 | 新增基于 `agent.json` 的配置读取入口（替代 `from_env` 作为主入口）。 | done | `AppConfig::load()` 已切换为 `agent.json` 驱动。 |
| 1.3 | 实现缺失 `agent.json` 时的默认文件生成逻辑。 | done | 启动时若缺失将自动生成默认模板。 |
| 1.4 | 支持配置注入能力（调用侧可注入配置对象/配置来源，不强耦合全局 env）。 | done | 新增 `from_inputs` / `from_agent_file_path` 支持注入。 |
| 1.v1 | Verify: `cargo test --workspace --test unit_config` 通过。 | done | 通过（6/6）。 |

## Phase 2: Env to JSON Priority & Secret Overlay

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 移除 `CHAOS_*` 业务配置读取逻辑。 | done | host/port/provider/model/runtime 参数不再读取 `CHAOS_*`。 |
| 2.2 | 实现通用密钥 env 读取层（OpenAI/Anthropic/Gemini 等）。 | done | 已支持 `OPENAI_API_KEY`/`ANTHROPIC_API_KEY`/`GEMINI_API_KEY`。 |
| 2.3 | 实现优先级：默认值 < env secrets < `agent.json`。 | done | 单测覆盖 env 值被 `agent.json` secrets 覆盖场景。 |
| 2.4 | 更新 provider 构建错误信息，指向新的配置来源。 | done | 报错文案已指向 env secrets 与 `agent.json` secrets。 |
| 2.v1 | Verify: `cargo test --workspace --test unit_llm` 通过。 | done | 通过（34/34）。 |
| 2.v2 | Verify: 新增单测验证 env 与 `agent.json` 覆盖关系。 | done | 已在 `unit_config` 新增覆盖优先级验证。 |

## Phase 3: App Wiring & Backward Compatibility Cleanup

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | `main.rs` 切换到新配置加载入口。 | done | 已改为 `AppConfig::load()`。 |
| 3.2 | 清理代码与测试中对 `CHAOS_*` 的依赖。 | done | 运行时逻辑已清理；保留测试仅用于验证 legacy env 被忽略。 |
| 3.3 | 保证 bootstrap/memory/personality 路径行为与现有语义一致。 | done | 相对路径按配置根解析，默认路径语义保持一致。 |
| 3.v1 | Verify: `cargo check -p chaos-bot-backend` 通过。 | done | 通过。 |
| 3.v2 | Verify: `cargo test --workspace --test api_integration --test api_routes` 通过。 | done | 通过（12/12）。 |

## Phase 4: Dedicated E2E Config Migration

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 将 e2e 启动配置从 `CHAOS_*` 注入切换为 `agent.json` 驱动。 | done | Playwright webServer 改为 `run-with-agent-config.sh` + `AGENT_CONFIG_PATH`。 |
| 4.2 | 为 e2e 增加临时配置覆盖与恢复机制。 | done | e2e 使用 `/tmp` 临时 config 文件，进程退出自动清理。 |
| 4.3 | 验证 mock provider 场景在新配置体系下行为一致。 | done | 4/4 用例全部通过。 |
| 4.v1 | Verify: `make test-e2e` 通过。 | done | 通过（Playwright 4/4）。 |

## Phase 5: Docs, Final Gate & PM Sync

| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | 更新 README 配置说明：从 env 驱动迁移到 `agent.json`。 | done | 已新增 `agent.json` 配置章节与优先级说明。 |
| 5.2 | 增加 `agent.json` 示例，标注密钥配置推荐方式。 | done | 已新增根目录 `agent.json` 模板。 |
| 5.3 | 同步任务完成记录与回归结果。 | done | 已记录验证结果并同步 AGENTS。 |
| 5.v1 | Verify: `make test-all` 通过（unit + integration + e2e）。 | done | 通过。 |
| 5.v2 | Verify: `cargo llvm-cov --workspace --summary-only --fail-under-lines 85` 通过。 | done | 通过（TOTAL Lines 85.19%）。 |

## Completion Record
- `cargo test --workspace --test unit_config`: passed (6/6)
- `cargo test --workspace --test unit_llm`: passed (34/34)
- `cargo check -p chaos-bot-backend`: passed
- `cargo test --workspace --test api_integration --test api_routes`: passed (12/12)
- `make test-e2e`: passed (Playwright 4/4)
- `make test-all`: passed (unit + integration + e2e)
- `cargo llvm-cov --workspace --summary-only --fail-under-lines 85`: passed (TOTAL Lines 85.19%)
