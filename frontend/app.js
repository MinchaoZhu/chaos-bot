let currentSessionId = null;

const sessionList = document.getElementById("sessionList");
const messagesEl = document.getElementById("messages");
const chatForm = document.getElementById("chatForm");
const messageInput = document.getElementById("messageInput");
const newSessionBtn = document.getElementById("newSessionBtn");

function addMessage(role, content) {
  const el = document.createElement("div");
  el.className = `message ${role}`;
  el.textContent = content;
  messagesEl.appendChild(el);
  messagesEl.scrollTop = messagesEl.scrollHeight;
  return el;
}

function upsertSession(session) {
  let item = sessionList.querySelector(`[data-id="${session.id}"]`);
  if (!item) {
    item = document.createElement("li");
    item.dataset.id = session.id;
    item.addEventListener("click", () => loadSession(session.id));
    sessionList.prepend(item);
  }
  item.textContent = `${session.id.slice(0, 8)} · ${session.messages.length} 条`;
  [...sessionList.children].forEach((li) => li.classList.toggle("active", li.dataset.id === currentSessionId));
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
})();
