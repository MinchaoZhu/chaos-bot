import type { SessionState } from "../contracts/protocol";

export type SlashCommandName = "help" | "model" | "models" | "new" | "compact" | "clear" | "sessions";

export type SlashCommandSpec = {
  name: SlashCommandName;
  usage: string;
  description: string;
  completion: string;
};

export type ParsedSlashCommand = {
  name: SlashCommandName;
  args: string[];
  raw: string;
};

export type ParseSlashCommandResult =
  | { kind: "not_command"; text: string }
  | { kind: "escaped"; text: string }
  | { kind: "error"; message: string }
  | { kind: "command"; command: ParsedSlashCommand };

export const SLASH_COMMANDS: SlashCommandSpec[] = [
  {
    name: "help",
    usage: "/help",
    description: "Show available slash commands",
    completion: "/help",
  },
  {
    name: "model",
    usage: "/model",
    description: "Show current provider/model",
    completion: "/model",
  },
  {
    name: "models",
    usage: "/models set <model_id>",
    description: "List models or switch current model",
    completion: "/models set ",
  },
  {
    name: "new",
    usage: "/new",
    description: "Create and switch to a new session",
    completion: "/new",
  },
  {
    name: "compact",
    usage: "/compact",
    description: "Summarize active session into a fresh session",
    completion: "/compact",
  },
  {
    name: "clear",
    usage: "/clear",
    description: "Clear current input draft",
    completion: "/clear",
  },
  {
    name: "sessions",
    usage: "/sessions",
    description: "Open sessions pane on mobile",
    completion: "/sessions",
  },
];

const MODELS_BY_PROVIDER: Record<string, string[]> = {
  mock: ["mock", "mock-fast", "mock-large"],
  openai: ["gpt-5", "gpt-5-mini", "gpt-4.1", "gpt-4o", "gpt-4o-mini"],
  anthropic: ["claude-3-7-sonnet", "claude-3-5-sonnet", "claude-3-5-haiku"],
  gemini: ["gemini-2.0-flash", "gemini-2.0-pro"],
};

function isSlashCommandName(value: string): value is SlashCommandName {
  return SLASH_COMMANDS.some((command) => command.name === value);
}

export function parseSlashCommand(input: string): ParseSlashCommandResult {
  const trimmed = input.trim();
  if (!trimmed.startsWith("/")) {
    return { kind: "not_command", text: trimmed };
  }

  if (trimmed.startsWith("//")) {
    return { kind: "escaped", text: trimmed.slice(1) };
  }

  const payload = trimmed.slice(1).trim();
  if (!payload) {
    return { kind: "error", message: "empty command; use /help" };
  }

  const tokens = payload.split(/\s+/);
  const name = tokens[0]?.toLowerCase() ?? "";
  if (!isSlashCommandName(name)) {
    return { kind: "error", message: `unknown command: /${name}` };
  }

  return {
    kind: "command",
    command: {
      name,
      args: tokens.slice(1),
      raw: trimmed,
    },
  };
}

export function getSlashCommandHints(draft: string): SlashCommandSpec[] {
  const trimmed = draft.trimStart();
  if (!trimmed.startsWith("/") || trimmed.startsWith("//")) {
    return [];
  }

  const payload = trimmed.slice(1);
  const token = payload.split(/\s+/)[0]?.toLowerCase() ?? "";

  if (!token) {
    return SLASH_COMMANDS;
  }

  if (payload.includes(" ")) {
    return [];
  }

  return SLASH_COMMANDS.filter((command) => command.name.startsWith(token));
}

export function formatHelpLines(): string[] {
  return SLASH_COMMANDS.map((command) => `${command.usage} - ${command.description}`);
}

export function modelsForProvider(provider: string): string[] {
  const normalized = provider.toLowerCase();
  if (MODELS_BY_PROVIDER[normalized]) {
    return MODELS_BY_PROVIDER[normalized];
  }
  const merged = new Set<string>();
  Object.values(MODELS_BY_PROVIDER).forEach((models) => {
    models.forEach((model) => merged.add(model));
  });
  return Array.from(merged.values());
}

function normalizeLine(content: string): string {
  return content.replace(/\s+/g, " ").trim();
}

function truncate(content: string, max = 180): string {
  if (content.length <= max) {
    return content;
  }
  return `${content.slice(0, max - 3)}...`;
}

export function buildCompactSummary(session: SessionState): string {
  const entries = session.messages
    .filter((message) => message.role !== "system" && typeof message.content === "string")
    .map((message) => ({
      role: message.role,
      content: normalizeLine(message.content ?? ""),
    }))
    .filter((message) => message.content.length > 0);

  const recent = entries.slice(-12);
  const lines = recent.map((message) => `- ${message.role}: ${truncate(message.content)}`);

  return [
    `Source session: ${session.id}`,
    `Captured turns: ${recent.length}`,
    "Recent dialogue:",
    ...lines,
    "Preserve this summary as context for future replies.",
  ].join("\n");
}
