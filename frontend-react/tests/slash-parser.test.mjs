import assert from "node:assert/strict";
import test from "node:test";
import { getSlashCommandHints, modelsForProvider, parseSlashCommand } from "../.tmp/unit/commands/slash.js";

test("parseSlashCommand leaves plain chat text untouched", () => {
  const result = parseSlashCommand("hello world");
  assert.equal(result.kind, "not_command");
  if (result.kind === "not_command") {
    assert.equal(result.text, "hello world");
  }
});

test("parseSlashCommand supports escaped slash commands", () => {
  const result = parseSlashCommand("//not-a-command");
  assert.equal(result.kind, "escaped");
  if (result.kind === "escaped") {
    assert.equal(result.text, "/not-a-command");
  }
});

test("parseSlashCommand parses known commands and args", () => {
  const result = parseSlashCommand("/models set mock-large");
  assert.equal(result.kind, "command");
  if (result.kind === "command") {
    assert.equal(result.command.name, "models");
    assert.deepEqual(result.command.args, ["set", "mock-large"]);
  }
});

test("parseSlashCommand reports unknown command", () => {
  const result = parseSlashCommand("/unknown");
  assert.equal(result.kind, "error");
});

test("getSlashCommandHints filters by typed prefix", () => {
  const hints = getSlashCommandHints("/mo");
  assert.deepEqual(
    hints.map((hint) => hint.name),
    ["model", "models"],
  );
});

test("modelsForProvider returns provider-specific and fallback model sets", () => {
  assert.deepEqual(modelsForProvider("mock"), ["mock", "mock-fast", "mock-large"]);
  const fallback = modelsForProvider("custom-provider");
  assert.ok(fallback.includes("gpt-5"));
  assert.ok(fallback.includes("claude-3-7-sonnet"));
  assert.ok(fallback.includes("gemini-2.0-flash"));
});
