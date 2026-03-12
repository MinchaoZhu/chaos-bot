# task-8: 全仓测试扫描、覆盖率补齐与质量门禁固化

## Task
- Description: 全面扫描当前 CLI-first 仓库的测试与验证面，补齐单元测试、CLI 集成测试和关键场景覆盖，建立可执行的覆盖率门禁，并把“测试与覆盖率达标后才能完成迭代”的质量标准固化到团队运行规范中。
- Scope: `backend/src/**`, `backend/tests/**`, `Makefile`, `scripts/run-test-suite.sh`, `.github/workflows/ci.yml`, 以及根目录/`.pm/docs/` 下的 `AGENTS.md` 质量规范同步。
- Risk: High. 仓库范围大，补齐测试会触及多层模块与脚本；若门禁设计不当，可能造成 CI 时间显著增加、覆盖率统计失真，或对当前发布/升级校验流程产生误报。
- Status: done

## Phase 1: 建立测试与覆盖率基线
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 盘点现有单测、CLI 集成测试、发布校验脚本与覆盖率入口，形成完整测试面清单。 | done | 现状入口确认为 `make test-unit`、`make test-cli`、`make test-all`、`make package-verify`、`make install-verify`、`make upgrade-verify`、`cargo llvm-cov --workspace --summary-only`；测试面已覆盖 domain/application/infrastructure/runtime/CLI 与 release/install/upgrade 脚本链路。 |
| 1.2 | 对核心用户路径与高分支逻辑做缺口分析，整理必须补齐的场景矩阵。 | done | 缺口集中在 `runtime/cli/output.rs`、`runtime/cli/config.rs`、`runtime/cli/mod.rs`、`runtime/config_runtime.rs`、`application/config_service.rs`、`application/session_service.rs`、`application/upgrade_service.rs`，以及 `config --stdin`、`config restart`、`skills get` 等 CLI 场景。 |
| 1.3 | 明确覆盖率统计与门禁方案，要求后续实现能稳定衡量 line coverage、branch coverage 与关键场景覆盖。 | done | 方案拆分为：稳定的 line gate 使用稳定版 `cargo llvm-cov`；branch gate 使用 nightly `cargo llvm-cov --branch`；若 nightly reporter/export 崩溃则必须显式失败并记录 blocker，不能静默跳过。 |
| 1.v1 | Verify: 任务说明内记录清晰的测试清单、缺口矩阵、覆盖率度量口径与后续实施边界。 | done | 已在本任务文件中记录现有入口、主要缺口、line/branch 度量方案与当前 nightly blocker。 |

## Phase 2: 补齐自动化测试与场景覆盖
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 为 branch-heavy 模块补充单测，覆盖正常路径、异常路径、边界条件和关键分支。 | done | 已为 CLI output 渲染、CLI config 输入解析、CLI 错误分类、config runtime、config/session/upgrade service 补充单测，覆盖 happy/error/boundary 与输出契约路径。 |
| 2.2 | 扩展 CLI 集成测试，覆盖核心命令链路、失败场景、跨调用状态延续，以及脚本安全的 stdout/stderr/output-mode 契约。 | done | 已扩展 `cli_integration` 覆盖 `config apply --stdin`、`config restart`、`skills get`，并继续验证 `chat`、`sessions`、`skills install`、workspace override 与 clean stdout/stderr/logging 契约。 |
| 2.3 | 对现有发布、安装、自升级、打包等脚本化校验补齐必要断言，或明确说明哪些场景因环境限制只能通过受控验证脚本保障。 | done | 现有 `package/install/upgrade` 验证脚本保持为受控自动化入口；`task-8` 未新增 release 脚本逻辑，但已把这些脚本纳入基线清单与质量门禁说明，继续作为受控验证链路。 |
| 2.v1 | Verify: 补齐后的单测、CLI 集成测试与关键脚本验证全部通过，且每类测试都有新增或强化后的断言。 | done | 已验证 `cargo test -p chaos-bot-backend`、`cargo llvm-cov --workspace --summary-only`、`make quality-gate` 通过；line coverage 提升到 87.76%。 |

## Phase 3: 固化覆盖率门禁与长期规范
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 更新 Makefile/脚本/CI，使完整测试门禁默认执行，并强制 line coverage 与 branch coverage 均不低于 85%。 | done | 已新增 `scripts/check-coverage.sh`、`make quality-gate`、`make coverage-branch-check`；nightly `llvm-cov report/export` SIGSEGV 已通过 grcov 替代路径绕过——脚本先用 `cargo +nightly llvm-cov --no-report` 收集 profraw，再用 grcov 解析 LCOV branch 数据并检查阈值。当前 branch coverage 95.90% >= 85%。 |
| 3.2 | 将质量标准写入 `AGENTS.md`，要求后续每次迭代在结束前补齐相关单测、集成测试、场景覆盖与覆盖率检查。 | done | 根 `AGENTS.md` 与 `.pm/docs/AGENTS.md` 已同步保留 task-8 质量规范，并将当前 blocker 与下一步行动写入运行状态。 |
| 3.3 | 为“场景覆盖率”补充可执行约束或清单，避免只满足数值覆盖率而遗漏真实业务流。 | done | 质量规范已明确要求每次改动覆盖 happy path、error path、boundary path 与 text/JSON/JSONL output contract，并点名 `chat`、`sessions`、`config`、`skills`、`upgrade` 与 logging/tooling/runtime 链路。 |
| 3.v1 | Verify: 质量门禁在本地与 CI 中可执行，低于阈值时会失败；运行规范已明确写入 AGENTS 并可用于后续任务验收。 | done | `make quality-gate` 通过；`make coverage-branch-check` 通过（grcov workaround，branch 95.90% >= 85%）；运行规范已写入 AGENTS。 |

## Coverage Snapshot
- `cargo llvm-cov --workspace --summary-only` => total line coverage `87.76%`, region coverage `86.21%`.
- 低覆盖重点已从 `runtime/cli/output.rs`、`runtime/cli/config.rs`、`runtime/config_runtime.rs`、`application/config_service.rs` 等模块显著提升到可接受区间。
- 当前仍偏低但未阻断总线性覆盖的模块包括 `application/chat_service.rs`、`runtime/cli/upgrade.rs`、`runtime/cli/mod.rs`、`infrastructure/model/mod.rs`；后续如果这些模块继续变更，必须继续按质量规范补场景。

## Blockers
- ~~2026-03-13 nightly `llvm-cov report/export` SIGSEGV~~ — resolved by replacing the `llvm-cov report` step with `grcov` LCOV parsing in `scripts/check-coverage.sh`. The nightly toolchain is still used for instrumented test execution (`--no-report`), but branch summary is extracted via `grcov --branch -t lcov`.

## Verification
- `cargo test -p chaos-bot-backend`
- `cargo llvm-cov --workspace --summary-only`
- `make quality-gate`
- `rustup toolchain install nightly --profile minimal && cargo install grcov`
- `make coverage-branch-check` (passes; branch coverage 95.90% >= 85%)
