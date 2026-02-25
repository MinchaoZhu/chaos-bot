# task-15: Skills 系统 — workspace skills 扫描、agent 注入、前端展示

## Task
- Description: 为 chaos-bot 增加 Skills 系统。Skills 存储在 `~/.chaos-bot/skills/`，每个 skill 为一个子目录（含 `SKILL.md`，YAML frontmatter 包含 name/description）。agent system prompt 始终注入所有 skill 摘要，用户可通过 `/activate <skill-id>` 手动激活完整指令。前端新增 Skills tab 展示已安装 skill 列表。内置 skill-creator skill 指导 agent 创建新 skill。
- Scope: `backend/src/{domain,infrastructure,application,interface,runtime}/*`，`frontend-react/src/*`，`src-tauri/src/lib.rs`，`e2e/tests/*`。
- Risk: frontmatter 解析需引入 `serde_yaml` 依赖；skill 目录为空或格式异常需优雅降级；desktop/mobile 5-tab 布局需要验证响应式适配。
- Status: done

## Phase 1: Domain — Skill 类型与端口
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 新建 `domain/skills.rs`，定义 `SkillMeta`（id/name/description/path）和 `SkillDetail`（meta + body）。 | done | |
| 1.2 | 在 `domain/ports.rs` 新增 `SkillPort` trait：`list() -> Vec<SkillMeta>`、`get(id) -> SkillDetail`、`ensure_layout()`。 | done | |
| 1.3 | 在 `domain/mod.rs` 注册 `pub mod skills`。 | done | |
| 1.v1 | Verify: `cargo check --workspace` 通过。 | done | |

## Phase 2: Infrastructure — SkillStore 适配器
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 新建 `infrastructure/skills.rs`，实现 `SkillStore`：扫描 `skills_dir` 子目录，找到含 `SKILL.md` 的目录并解析 YAML frontmatter（`---` 分隔），提取 name/description。实现 `SkillPort`。 | done | 用简单字符串解析代替 serde_yaml，无需额外依赖。EmptySkillStore 供测试用。 |
| 2.2 | 在 `infrastructure/mod.rs` 注册 `pub mod skills`。 | done | |
| 2.3 | 在 `infrastructure/runtime_assets.rs` 新增 `DEFAULT_SKILL_CREATOR_MD` 常量，内嵌 skill-creator 的 SKILL.md 内容，指导 agent 用 file tools 在 `{workspace}/skills/{name}/` 下创建新 skill。 | done | |
| 2.4 | 在 `infrastructure/config.rs` 的 `AppConfig` 新增 `skills_dir: PathBuf`，默认值 `{workspace}/skills`，在 `defaults_for_workspace_base()` 中设置。 | done | |
| 2.5 | `Cargo.toml` 新增 `serde_yaml` 依赖（frontmatter 解析）。 | done | 改用轻量字符串解析，无需 serde_yaml。 |
| 2.v1 | Verify: `cargo check --workspace` 通过。 | done | |

## Phase 3: Agent 集成 — System Prompt 注入与 `/activate` 指令
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | `AgentLoop` 构造函数增加 `skills: Arc<dyn SkillPort>` 字段。 | done | |
| 3.2 | 修改 `build_system_prompt()` 追加 `# Available Skills` 块：列出每个 skill 的 `name: description`，并说明用户可发送 `/activate <skill-id>` 激活。 | done | |
| 3.3 | 在 `run_stream()` 中检测用户消息是否以 `/activate ` 开头，若匹配则调用 `skills.get(id)` 并将 skill body 作为 system-level context 注入到 messages 列表。 | done | |
| 3.4 | `runtime/mod.rs` 的 `build_agent_loop()` 创建 `SkillStore` 实例，调用 `ensure_layout()`，传入 `AgentLoop::new()`。 | done | |
| 3.v1 | Verify: `cargo test --workspace` 通过。 | done | |

## Phase 4: API — Skills 列表与详情端点
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | `interface/http.rs` 新增 `GET /api/skills` → `Vec<SkillMeta>` 和 `GET /api/skills/:id` → `SkillDetail`。 | done | |
| 4.2 | `application/skill_service.rs` — 直接在 http handler 中调用 AppState.skills，无需额外 service 层。 | done | 精简：handler 直接调用 `state.skills`。 |
| 4.3 | `AppState` 新增 `skills: Arc<dyn SkillPort>` 字段，`with_skills()` builder，默认 EmptySkillStore。 | done | |
| 4.4 | `src-tauri/src/lib.rs` 新增 `list_skills` / `get_skill` Tauri commands，注册到 `generate_handler![]`。 | done | |
| 4.v1 | Verify: `cargo test --workspace` 通过。 | done | |

## Phase 5: Frontend — Skills Tab
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 5.1 | `contracts/protocol.ts` 新增 `SkillMeta` / `SkillDetail` 类型定义。 | done | |
| 5.2 | `runtime/adapter.ts` 新增 `listSkills(baseUrl)` / `getSkill(baseUrl, skillId)` 方法签名。 | done | |
| 5.3 | `http-adapter.ts` + `tauri-adapter.ts` 实现 skills 方法。 | done | |
| 5.4 | 新建 `components/SkillsPanel.tsx`：mount 加载列表，展示 name+description 卡片，点击展开 body 详情。 | done | |
| 5.5 | `App.tsx`：`DesktopSidePane` 扩展为 3 tab（events/config/skills），`MobilePane` 扩展为 5 tab。 | done | |
| 5.6 | `MobilePaneTabs.tsx`：PANES 数组增加 `"skills"`。 | done | |
| 5.7 | `styles.css`：`.pane-tabs` grid 改为 `repeat(3, ...)`，`.mobile-tabs` 改为 `repeat(5, ...)`，新增 `.skills-panel` / `.skill-card` 样式。 | done | |
| 5.v1 | Verify: `npm --prefix frontend-react run build` 通过。 | done | |
| 5.v2 | Verify: `cargo check --manifest-path src-tauri/Cargo.toml` 通过。 | done | |

## Phase 6: E2E 测试与完成门禁
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 6.1 | 新增 e2e spec 覆盖 Skills tab（desktop 切换 + mobile pane）。 | done | `assertSkillsTabFlow` helper 覆盖 desktop/mobile，验证 `.skills-panel` 可见 + skill-creator card 存在。 |
| 6.2 | Verify: `make test-e2e` 通过。 | done | 2 passed, 2 skipped。 |
| 6.3 | Verify: `make test-all` 通过（任务完成门禁）。 | done | unit + integration + e2e all green。 |
| 6.4 | 更新 `README.md` 增加 Skills 功能说明。 | done | |
| 6.5 | 同步 `AGENTS.md`。 | done | |

## Exit Criteria — All Met
- `~/.chaos-bot/skills/` 目录启动时自动创建，内置 `skill-creator` skill。✓
- Agent system prompt 始终包含所有 skill 摘要。✓
- 用户发送 `/activate <skill-id>` 可注入 skill 完整指令到当前会话。✓
- `GET /api/skills` 返回已安装 skill 列表，`GET /api/skills/:id` 返回详情。✓
- 前端 Skills tab 在 desktop/mobile 均可访问，展示 skill 列表与详情。✓
- `make test-all` 通过。✓

## Completion Notes
- `serde_yaml` 依赖未引入，改用零依赖 frontmatter 解析（`strip_prefix("---\n")` + `find("\n---\n")`）
- `EmptySkillStore` 保持现有测试零改动（除新增 Arc::new(EmptySkillStore) 参数）
- Two SkillStore instances in production: one inside AgentLoop (prompt injection), one in AppState (REST API) — both read-only from same directory
- `AppState::new(agent)` still works without skills (defaults to EmptySkillStore), production uses `.with_skills(skills)` builder
