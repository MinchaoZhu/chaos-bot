# task-10: Tauri v2 + React Multi-Platform Frontend Refactor

## Task
- Description: 引入 Tauri v2 与 React 前端框架，形成桌面横屏与移动竖屏双形态运行能力，并保持与现有 agent 后端协议兼容。
- Scope: `frontend/`, `frontend-react/`, `src-tauri/`, `e2e/`, `Makefile`, `README.md`。
- Risk: Tauri v2 桌面/移动构建链路差异较大；UI 需要同时满足横屏/竖屏；前后端通信协议若变更可能导致回归。
- Status: done

## Phase 1: Foundation and Runtime Contract
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 1.1 | 确认 Tauri v2 + React 工程布局与构建入口（桌面/移动）。 | done | 新增 `frontend-react/` 与 `src-tauri/`，并在 `Makefile` 增加 `frontend-dev`、`tauri-dev`、`tauri-android-dev`、`tauri-ios-dev`。 |
| 1.2 | 建立 React 应用骨架与路由/状态管理基线。 | done | 落地 `frontend-react/src` 基础壳层（session rail + chat panel + event panel），并提供 HTTP/Tauri 双运行时适配层。 |
| 1.3 | 定义前后端通信契约（命令、事件、错误码）。 | done | 新增 `frontend-react/RUNTIME_CONTRACT.md` 与 `frontend-react/src/contracts/protocol.ts`，固定命令名、SSE 事件与错误码。 |
| 1.v1 | Verify: 桌面与移动两类 target 的最小可运行骨架可启动。 | done | desktop/mobile 双布局 e2e 已验证；Tauri 侧已提供 `tauri-preflight` / `tauri-android-init` 可执行链路用于环境 smoke。 |

## Phase 2: Adaptive UI for Desktop and Mobile
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 2.1 | 实现桌面横屏布局（信息密度高、侧栏工具区）。 | done | Desktop 下采用 `SessionRail + ConversationPanel + EventTimeline` 多面板布局并支持 resize。 |
| 2.2 | 实现移动竖屏布局（单列任务流 + 关键操作悬浮）。 | done | Mobile 下引入 `chat/sessions/events` 分页切换，单列竖屏流程可完成会话与发送操作。 |
| 2.3 | 抽象共享组件层，减少平台分叉代码。 | done | 新增 `layout/adapter.ts` 与 `components/*`，以 layout adapter 统一桌面/移动渲染。 |
| 2.v1 | Verify: 同一业务流程在桌面/移动布局均可完成。 | done | 已通过新增多端 e2e（desktop + mobile emulation）验证会话创建、消息发送与事件展示流程。 |

## Phase 3: Packaging and Delivery
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 3.1 | 打通桌面平台打包链路（Linux/macOS/Windows）。 | done | 新增 `tauri-preflight`/`tauri-build-desktop` 命令并修复 workspace/CLI 路径问题，当前 Linux 主机已可完成 preflight 与 desktop debug 构建。 |
| 3.2 | 打通移动平台打包链路（Android/iOS，按环境能力分阶段）。 | done | 新增 `tauri-android-init`/`tauri-android-build` 入口；落地 Android/JDK 与 iOS(macOS) 前置条件与签名发布约束文档。 |
| 3.v1 | Verify: CI/本地可执行至少一条桌面与一条移动构建链路。 | done | 已执行并通过 desktop(`make tauri-build-desktop`) 与 mobile(`make tauri-android-init`) 链路，矩阵见 `docs/tauri-packaging.md`。 |

## Phase 4: E2E and Full Gate
| # | Description | Status | Detail |
|---|-------------|--------|--------|
| 4.1 | 新增多端 UI 回归 e2e 场景（桌面横屏 + 移动竖屏）。 | done | 新增 `e2e/tests/react-shell.spec.ts`，并在 Playwright 中加入 `react-shell-desktop` 与 `react-shell-mobile` 项目。 |
| 4.v1 | Verify: `make test-e2e` 通过并覆盖新增关键路径。 | done | `make test-e2e` 通过（legacy + react-shell desktop/mobile emulation）。 |
| 4.v2 | Verify: `make test-all` 通过（任务完成门禁）。 | done | `make test-all` 通过（unit + integration + e2e）。 |

## Progress Record
- 新增 `frontend-react/`（Vite + React + TypeScript）基础壳层与双运行时适配（HTTP/Tauri）。
- 新增 `src-tauri/`（Tauri v2）命令桥接骨架，命令覆盖 health/session/chat-stream。
- 新增通信契约文档 `frontend-react/RUNTIME_CONTRACT.md`。
- 更新 `Makefile` 与 `README.md` 的多端构建/运行入口。
- 完成 Phase 2 自适配重构：`App.tsx` 拆分为 layout adapter + 共享组件，支持 desktop/mobile 双形态。
- 完成 Phase 4 e2e 扩展：Playwright 新增 React 壳 desktop/mobile 项目与测试用例，并接入现有 `make test-e2e`。
- 完成 Phase 3 打包矩阵落地：新增 `docs/tauri-packaging.md` 与 `tauri-preflight/tauri-build-desktop/tauri-android-init/tauri-android-build` 命令链。

## Verification Record
- `make test-all`: passed (unit + integration + e2e)
- `make test-all`: passed (unit + integration + e2e, phase-2 update)
- `make test-e2e`: passed (Playwright legacy + react-shell desktop/mobile)
- `make test-all`: passed (unit + integration + e2e, phase-4 update)
- `make tauri-preflight`: passed (`webkit2gtk-4.1`/`rsvg2`/Rust toolchain detected)
- `make tauri-build-desktop`: passed (debug desktop binary at `src-tauri/target/debug/chaos-bot-app`)
- `make tauri-android-init`: passed (generated Android project at `src-tauri/gen/android`)
- `make tauri-android-build`: passed (debug universal APK at `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`)

## Completion Record
- Task status: done
- Gate: `make test-all` passed
- Packaging matrix and troubleshooting: `docs/tauri-packaging.md`
