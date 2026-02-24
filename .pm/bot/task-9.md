# task-9: CI Failure Artifact Retention and Download Strategy

## Task
- Description: 在 CI 中为失败场景保留并上传运行时证据（日志、配置与备份、Playwright 产物），保证可下载复盘。
- Scope: `.github/workflows/ci.yml`, `scripts/run-test-suite.sh`, `README.md`, `AGENTS.md`。
- Risk: 若失败后仍清理 `.tmp`，artifact 将丢失；若上传路径过宽，可能引入无关大文件。
- Status: done

## Phase 1: CI Workflow Baseline
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 新增 GitHub Actions CI workflow，统一执行 `make test-all`。 | done | 新增 `.github/workflows/ci.yml`。 |
| 1.2 | 在 CI 准备 Rust/Node/Playwright 运行环境。 | done | 配置 rust toolchain、node 20、`npm ci`、`playwright install --with-deps`。 |
| 1.v1 | Verify: workflow 配置可静态通过并覆盖 unit/integration/e2e 入口。 | done | 已验证流程与命令链路一致。 |

## Phase 2: Failure Artifact Preservation Switch
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 修改测试套件运行脚本，支持失败时保留 `.tmp`。 | done | `scripts/run-test-suite.sh` 增加 `CHAOS_BOT_KEEP_TMP_ON_FAIL=1` 开关。 |
| 2.2 | 成功场景保持原有自动清理，避免本地污染。 | done | 默认行为不变，失败且开关开启才保留。 |
| 2.v1 | Verify: 脚本在成功/失败两类路径符合预期。 | done | 本地 smoke 验证：成功清理、失败保留。 |

## Phase 3: Artifact Upload Strategy
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | CI 失败时上传 `.tmp` 中关键目录。 | done | 上传 `.tmp/unit`、`.tmp/integration`、`.tmp/e2e/runtime`、`.tmp/e2e/artifacts`。 |
| 3.2 | 设定 artifact 命名与 retention，便于下载定位。 | done | 名称包含 run id/attempt，保留 14 天。 |
| 3.v1 | Verify: 上传策略覆盖日志、配置备份与 Playwright 产物。 | done | 路径覆盖 `logs`、`config.json*`、`playwright-report/test-results`。 |

## Phase 4: Documentation and Final Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 在 README 增加 CI 失败 artifact 策略与路径说明。 | done | README 新增 `CI Failure Artifacts` 章节。 |
| 4.v1 | Verify: `make test-all` 通过（任务完成门禁）。 | done | 通过（unit + integration + e2e）。 |

## Completion Record
- `make test-all`: passed (unit + integration + e2e)
- `scripts/run-test-suite.sh` failure-preserve smoke: passed
