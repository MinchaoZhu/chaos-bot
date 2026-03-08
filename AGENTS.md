# PM Runtime AGENTS

## Current Status
- Project: bot
- Main Repository: /home/debian/projects/chaos-bot
- Branch: feat/bot
- Active Task: none
- Last Updated: 2026-03-08T19:21:27+08:00

## Task Index
- task-1: done
- task-2: done
- task-3: done
- task-4: done
- task-5: done
- task-6: done
- task-7: done
- task-8: done
- task-9: done
- task-10: done
- task-11: done
- task-12: done
- task-13: done
- task-14: done
- task-15: done

## Verification
- 当前工作区为 `/home/debian/projects/chaos-bot/.projects/bot`（branch `feat/bot`）；`master` 由主 worktree `/home/debian/projects/chaos-bot` 持有。
- 新增 `google gemini` provider：通过 OpenAI-compatible 协议复用 chat/stream 实现，默认端点 `https://generativelanguage.googleapis.com/v1beta/openai`，支持 `GEMINI_BASE_URL` 覆盖。
- 密钥优先级保持为 `defaults < env secrets < config.json secrets`，其中 `GEMINI_API_KEY` 先从环境变量读取，再由 `config.json` 覆盖。
- 新增单测 `build_provider_gemini_with_key`，并完成格式化与回归：`cargo fmt`、`cargo test -p chaos-bot-backend --test unit_llm --test unit_config`（通过）。

## Commit Constraints
- 仓库启用 `core.hooksPath=.githooks`，推送前会执行 `.githooks/pre-push`。
- 所有待推送提交标题必须符合：
  - `Feature: <summary>`
  - `Fix: <summary>`
  - `Refactor: <summary>`
- 不符合格式会被 hook 拒绝推送。

## PM File Map
- `.pm/docs/project.md`: 当前项目上下文文档（历史记录仍保留 `cicd` 轨迹）。
- `.pm/docs/AGENTS.md`: Runtime 状态镜像。
- `.pm/bot/`: bot 项目任务目录（task-1 到 task-15）。
- `.pm/cicd/`: cicd 项目历史任务目录（task-1 到 task-3）。
- `AGENTS.md`: 共享 runtime 状态源。
- `CLAUDE.md`: 指向 `AGENTS.md` 的符号链接。

## Next Actions
1. 如需生产可用的 Anthropic provider，可按与 Gemini 一致的 provider 抽象继续落地真实实现。
2. 为 OpenAI-compatible provider 增加 HTTP mock 级集成测试，覆盖非 2xx、SSE 中断、tool call 分片拼装等边界。
3. 若后续以 bot 作为主 PM 项目，更新 `.pm/docs/project.md` 的项目元信息以消除与 cicd 的历史偏差。
