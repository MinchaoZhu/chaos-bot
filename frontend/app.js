let currentSessionId = null;
let activeMainTab = "chat";
let activeConfigTab = "raw";
let latestConfigState = null;

const sessionList = document.getElementById("sessionList");
const messagesEl = document.getElementById("messages");
const chatForm = document.getElementById("chatForm");
const messageInput = document.getElementById("messageInput");
const newSessionBtn = document.getElementById("newSessionBtn");

const chatTabBtn = document.getElementById("chatTabBtn");
const configTabBtn = document.getElementById("configTabBtn");
const chatView = document.getElementById("chatView");
const configView = document.getElementById("configView");

const configRawTabBtn = document.getElementById("configRawTabBtn");
const configFormTabBtn = document.getElementById("configFormTabBtn");
const configRawView = document.getElementById("configRawView");
const configFormView = document.getElementById("configFormView");
const configRawInput = document.getElementById("configRawInput");
const configMeta = document.getElementById("configMeta");
const configStatus = document.getElementById("configStatus");
const configResetBtn = document.getElementById("configResetBtn");
const configApplyBtn = document.getElementById("configApplyBtn");
const configRestartBtn = document.getElementById("configRestartBtn");

const cfgWorkspace = document.getElementById("cfgWorkspace");
const cfgHost = document.getElementById("cfgHost");
const cfgPort = document.getElementById("cfgPort");
const cfgProvider = document.getElementById("cfgProvider");
const cfgModel = document.getElementById("cfgModel");
const cfgTemperature = document.getElementById("cfgTemperature");
const cfgMaxTokens = document.getElementById("cfgMaxTokens");
const cfgMaxIterations = document.getElementById("cfgMaxIterations");
const cfgTokenBudget = document.getElementById("cfgTokenBudget");
const cfgLogLevel = document.getElementById("cfgLogLevel");
const cfgRetentionDays = document.getElementById("cfgRetentionDays");
const cfgLogDir = document.getElementById("cfgLogDir");
const cfgOpenAiKey = document.getElementById("cfgOpenAiKey");
const cfgAnthropicKey = document.getElementById("cfgAnthropicKey");
const cfgGeminiKey = document.getElementById("cfgGeminiKey");

function addMessage(role, content) {
  const el = document.createElement("div");
  el.className = `message ${role}`;
  el.textContent = content;
  messagesEl.appendChild(el);
  messagesEl.scrollTop = messagesEl.scrollHeight;
  return el;
}

async function refreshSessions() {
  const res = await fetch("/api/sessions");
  const sessions = await res.json();
  sessionList.innerHTML = "";
  sessions.forEach((s) => {
    const li = document.createElement("li");
    li.dataset.id = s.id;
    li.textContent = `${s.id.slice(0, 8)} · ${s.messages.length} 条`;
    if (s.id === currentSessionId) li.classList.add("active");
    li.addEventListener("click", () => loadSession(s.id));
    sessionList.appendChild(li);
  });
}

async function createSession() {
  const res = await fetch("/api/sessions", { method: "POST" });
  const session = await res.json();
  currentSessionId = session.id;
  await refreshSessions();
  await loadSession(currentSessionId);
}

async function loadSession(id) {
  const res = await fetch(`/api/sessions/${id}`);
  if (!res.ok) return;
  const session = await res.json();
  currentSessionId = session.id;
  messagesEl.innerHTML = "";
  session.messages
    .filter((m) => m.role !== "system")
    .forEach((m) => addMessage(m.role, m.content || ""));
  await refreshSessions();
}

function parseSseBlock(block) {
  const lines = block.split("\n");
  const out = { event: "message", data: "" };
  for (const line of lines) {
    if (line.startsWith("event:")) out.event = line.slice(6).trim();
    if (line.startsWith("data:")) out.data += line.slice(5).trim();
  }
  return out;
}

chatForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  const text = messageInput.value.trim();
  if (!text) return;

  messageInput.value = "";
  addMessage("user", text);
  const assistant = addMessage("assistant", "");

  const res = await fetch("/api/chat", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ session_id: currentSessionId, message: text })
  });

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });

    const blocks = buffer.split("\n\n");
    buffer = blocks.pop() || "";

    for (const block of blocks) {
      if (!block.trim()) continue;
      const event = parseSseBlock(block);
      if (event.event === "session") {
        const payload = JSON.parse(event.data);
        currentSessionId = payload.session_id;
      }
      if (event.event === "tool_call") {
        const payload = JSON.parse(event.data);
        addMessage("tool", `[tool] ${payload.name}: ${payload.output}`);
      }
      if (event.event === "delta") {
        assistant.textContent += event.data;
      }
      if (event.event === "done") {
        await refreshSessions();
      }
      messagesEl.scrollTop = messagesEl.scrollHeight;
    }
  }
});

function switchMainTab(tab) {
  activeMainTab = tab;
  const chatActive = tab === "chat";

  chatTabBtn.classList.toggle("active", chatActive);
  configTabBtn.classList.toggle("active", !chatActive);

  chatView.hidden = !chatActive;
  configView.hidden = chatActive;
  chatView.classList.toggle("active", chatActive);
  configView.classList.toggle("active", !chatActive);

  if (!chatActive && !latestConfigState) {
    refreshConfig().catch(() => {
      setConfigStatus("error", "配置加载失败，请稍后重试。");
    });
  }
}

function switchConfigTab(tab) {
  activeConfigTab = tab;
  const rawActive = tab === "raw";

  configRawTabBtn.classList.toggle("active", rawActive);
  configFormTabBtn.classList.toggle("active", !rawActive);

  configRawView.hidden = !rawActive;
  configFormView.hidden = rawActive;
  configRawView.classList.toggle("active", rawActive);
  configFormView.classList.toggle("active", !rawActive);
}

function setConfigStatus(kind, text) {
  configStatus.textContent = text;
  configStatus.className = `config-status ${kind}`;
}

function valueOrUndefined(value) {
  const trimmed = String(value ?? "").trim();
  return trimmed ? trimmed : undefined;
}

function numberOrUndefined(value) {
  const raw = String(value ?? "").trim();
  if (!raw) return undefined;
  const num = Number(raw);
  return Number.isFinite(num) ? num : undefined;
}

function compactObject(input) {
  if (Array.isArray(input)) {
    const mapped = input.map(compactObject).filter((v) => v !== undefined);
    return mapped.length ? mapped : undefined;
  }
  if (input && typeof input === "object") {
    const out = {};
    for (const [k, v] of Object.entries(input)) {
      const compacted = compactObject(v);
      if (compacted !== undefined) out[k] = compacted;
    }
    return Object.keys(out).length ? out : undefined;
  }
  if (input === null || input === undefined || input === "") return undefined;
  return input;
}

function buildConfigFromForm() {
  const payload = {
    workspace: valueOrUndefined(cfgWorkspace.value),
    server: {
      host: valueOrUndefined(cfgHost.value),
      port: numberOrUndefined(cfgPort.value)
    },
    llm: {
      provider: valueOrUndefined(cfgProvider.value),
      model: valueOrUndefined(cfgModel.value),
      temperature: numberOrUndefined(cfgTemperature.value),
      max_tokens: numberOrUndefined(cfgMaxTokens.value),
      max_iterations: numberOrUndefined(cfgMaxIterations.value),
      token_budget: numberOrUndefined(cfgTokenBudget.value)
    },
    logging: {
      level: valueOrUndefined(cfgLogLevel.value),
      retention_days: numberOrUndefined(cfgRetentionDays.value),
      directory: valueOrUndefined(cfgLogDir.value)
    },
    secrets: {
      openai_api_key: valueOrUndefined(cfgOpenAiKey.value),
      anthropic_api_key: valueOrUndefined(cfgAnthropicKey.value),
      gemini_api_key: valueOrUndefined(cfgGeminiKey.value)
    }
  };
  return compactObject(payload) || {};
}

function fillFormFromConfig(config) {
  cfgWorkspace.value = config.workspace || "";
  cfgHost.value = config.server?.host || "";
  cfgPort.value = config.server?.port ?? "";
  cfgProvider.value = config.llm?.provider || "";
  cfgModel.value = config.llm?.model || "";
  cfgTemperature.value = config.llm?.temperature ?? "";
  cfgMaxTokens.value = config.llm?.max_tokens ?? "";
  cfgMaxIterations.value = config.llm?.max_iterations ?? "";
  cfgTokenBudget.value = config.llm?.token_budget ?? "";
  cfgLogLevel.value = config.logging?.level || "info";
  cfgRetentionDays.value = config.logging?.retention_days ?? "";
  cfgLogDir.value = config.logging?.directory || "";
  cfgOpenAiKey.value = config.secrets?.openai_api_key || "";
  cfgAnthropicKey.value = config.secrets?.anthropic_api_key || "";
  cfgGeminiKey.value = config.secrets?.gemini_api_key || "";
}

function renderConfigState(state) {
  latestConfigState = state;
  configRawInput.value = state.raw || "";
  fillFormFromConfig(state.running || {});
  configMeta.textContent = `${state.config_format} · ${state.config_path}`;
  if (state.disk_parse_error) {
    setConfigStatus("warning", `配置文件解析异常：${state.disk_parse_error}`);
  }
}

async function refreshConfig() {
  const res = await fetch("/api/config");
  if (!res.ok) {
    setConfigStatus("error", "配置接口暂不可用。");
    return;
  }
  const state = await res.json();
  renderConfigState(state);
  setConfigStatus("ok", "配置已加载。");
}

async function mutateConfig(action) {
  const endpointByAction = {
    reset: "/api/config/reset",
    apply: "/api/config/apply",
    restart: "/api/config/restart"
  };

  const endpoint = endpointByAction[action];
  if (!endpoint) return;

  let body = {};
  if (action !== "reset") {
    body = activeConfigTab === "raw"
      ? { raw: configRawInput.value }
      : { config: buildConfigFromForm() };
  }

  const res = await fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body)
  });

  if (!res.ok) {
    const message = action === "apply" ? "应用配置失败。" : action === "restart" ? "重启请求失败。" : "重置配置失败。";
    setConfigStatus("error", message);
    return;
  }

  const payload = await res.json();
  renderConfigState(payload.state);

  if (payload.action === "restart") {
    setConfigStatus(
      payload.restart_scheduled ? "warning" : "ok",
      payload.restart_scheduled
        ? "重启请求已提交，服务将很快退出并由外部进程拉起。"
        : "当前运行模式禁用了进程重启，仅完成配置更新。"
    );
    return;
  }

  if (payload.action === "apply") {
    setConfigStatus("ok", "配置已动态应用到运行时。");
    return;
  }

  setConfigStatus("ok", "配置已重置到当前运行快照。");
}

chatTabBtn.addEventListener("click", () => switchMainTab("chat"));
configTabBtn.addEventListener("click", () => switchMainTab("config"));

configRawTabBtn.addEventListener("click", () => switchConfigTab("raw"));
configFormTabBtn.addEventListener("click", () => switchConfigTab("form"));

configResetBtn.addEventListener("click", () => mutateConfig("reset"));
configApplyBtn.addEventListener("click", () => mutateConfig("apply"));
configRestartBtn.addEventListener("click", () => mutateConfig("restart"));

newSessionBtn.addEventListener("click", createSession);

(async function init() {
  await refreshSessions();
  if (!currentSessionId) {
    const res = await fetch("/api/sessions", { method: "POST" });
    const session = await res.json();
    currentSessionId = session.id;
    await refreshSessions();
    await loadSession(currentSessionId);
  }
  await refreshConfig();
})();
