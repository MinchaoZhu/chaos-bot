# task-2: Restructure config UX and global navigation

## Task
- Description: Reorganize the React frontend so sessions become a first-class top-level pane, config moves into domain-specific sub-tabs, and release/about information gets its own top-level page.
- Scope: `frontend-react` navigation, config forms, about page, session browsing flow, and related styling.
- Risk: Medium.
- Status: done

## Phase 1: Plan the new IA
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | Confirm the target top-level navigation and config sub-tab structure. | done | Locked to `Conversation`, `Sessions`, `Events`, `Config`, `Skills`, `About`, with config sub-tabs `LLM`, `Search`, `IM Connectors`, `System`, `Raw`. |
| 1.2 | Define where version and upgrade information should live. | done | `About` is a top-level pane; version / upgrade UI moved out of config. |
| 1.v1 | Verify: plan is decision-complete for implementation. | done | Layout, field grouping, and session flow decisions were all fixed before implementation. |

## Phase 2: Implement the frontend restructure
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | Replace the split desktop/mobile pane model with a unified top-level tab model. | done | `App.tsx` and `MobilePaneTabs.tsx` were reworked around a shared primary pane state. |
| 2.2 | Move the session rail into a dedicated Sessions pane and load selected history back into Conversation. | done | Sessions now live in their own panel, and selecting one fetches its state then switches to the conversation pane. |
| 2.3 | Rebuild config editing into domain-specific sub-tabs and fold Telegram into config. | done | `ConfigPanel.tsx` now drives `LLM`, `Search`, `IM Connectors`, `System`, and `Raw` from a shared raw-config source. |
| 2.4 | Add an About pane for project/repository/version/upgrade information. | done | New `AboutPanel.tsx` exposes repository facts, app version, and upgrade actions. |
| 2.5 | Refresh styling so the new pane model and config sections render correctly on desktop and mobile. | done | `styles.css` now supports primary tabs, config tabs, field grids, and about cards. |

## Phase 3: Verification
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | Build the frontend after the navigation/config refactor. | done | `npm --prefix frontend-react run build` passed. |
| 3.2 | Run existing frontend unit coverage. | done | `npm --prefix frontend-react run test:unit` passed. |
| 3.v1 | Verify: sessions/config/about flows compile and existing slash command tests still pass. | done | Build and unit test verification completed on 2026-03-08. |
