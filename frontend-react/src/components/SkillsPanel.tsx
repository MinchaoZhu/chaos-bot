import { useEffect, useState } from "react";
import type { SkillDetail, SkillMeta } from "../contracts/protocol";
import type { RuntimeAdapter } from "../runtime/adapter";

type SkillsPanelProps = {
  runtime: RuntimeAdapter;
  baseUrl: string;
  onLog?: (line: string) => void;
};

export function SkillsPanel({ runtime, baseUrl, onLog }: SkillsPanelProps) {
  const [skills, setSkills] = useState<SkillMeta[]>([]);
  const [selected, setSelected] = useState<SkillDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [installSource, setInstallSource] = useState("");
  const [installBusy, setInstallBusy] = useState(false);
  const [installNotice, setInstallNotice] = useState("");

  async function reloadSkills(cancelledRef?: { cancelled: boolean }) {
    setLoading(true);
    setError("");
    try {
      const list = await runtime.listSkills(baseUrl);
      if (!cancelledRef?.cancelled) {
        setSkills(list);
      }
    } catch (err) {
      if (!cancelledRef?.cancelled) {
        setError(String(err));
      }
    } finally {
      if (!cancelledRef?.cancelled) {
        setLoading(false);
      }
    }
  }

  useEffect(() => {
    const cancelledRef = { cancelled: false };
    void reloadSkills(cancelledRef);

    return () => {
      cancelledRef.cancelled = true;
    };
  }, [runtime, baseUrl]);

  async function handleSelect(id: string) {
    if (selected?.meta.id === id) {
      setSelected(null);
      return;
    }
    try {
      const detail = await runtime.getSkill(baseUrl, id);
      setSelected(detail);
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleInstall() {
    const source = installSource.trim();
    if (!source || installBusy) {
      return;
    }

    setInstallBusy(true);
    setInstallNotice("");
    setError("");
    try {
      const response = await runtime.installSkill(baseUrl, { source });
      const ids = response.installed.map((item) => item.id);
      if (ids.length === 0) {
        setInstallNotice("No SKILL.md found.");
      } else {
        setInstallNotice(`Installed: ${ids.join(", ")}`);
        onLog?.(`[skills.install] source=${source} installed=${ids.join(",")}`);
        const first = ids[0];
        if (first) {
          try {
            const detail = await runtime.getSkill(baseUrl, first);
            setSelected(detail);
          } catch {
            // Keep install success even if detail loading fails.
          }
        }
      }
      await reloadSkills();
    } catch (err) {
      setError(String(err));
      onLog?.(`[skills.install.error] ${String(err)}`);
    } finally {
      setInstallBusy(false);
    }
  }

  return (
    <div className="panel skills-panel">
      <h3 className="panel-title">Skills</h3>
      <div className="skill-install">
        <label htmlFor="skill-install-source">Install from git URL</label>
        <input
          id="skill-install-source"
          type="text"
          placeholder="https://github.com/org/repo.git or .../tree/main/path"
          value={installSource}
          onChange={(event) => setInstallSource(event.target.value)}
        />
        <button type="button" onClick={() => void handleInstall()} disabled={installBusy || !installSource.trim()}>
          {installBusy ? "Installing…" : "Install Skill"}
        </button>
        {installNotice ? <p className="skills-hint">{installNotice}</p> : null}
      </div>

      {loading && <p className="skills-hint">Loading skills…</p>}
      {error && <p className="skills-error">{error}</p>}
      {!loading && skills.length === 0 && !error && (
        <p className="skills-hint">
          No skills found. Place a <code>SKILL.md</code> inside{" "}
          <code>~/.chaos-bot/skills/&lt;name&gt;/</code>.
        </p>
      )}

      <ul className="skill-list">
        {skills.map((skill) => (
          <li key={skill.id}>
            <button
              type="button"
              className={`skill-card ${selected?.meta.id === skill.id ? "active" : ""}`}
              onClick={() => void handleSelect(skill.id)}
            >
              <span className="skill-name">{skill.name || skill.id}</span>
              <span className="skill-desc">{skill.description}</span>
            </button>
          </li>
        ))}
      </ul>

      {selected && (
        <div className="skill-detail">
          <h4>{selected.meta.name || selected.meta.id}</h4>
          <pre className="skill-body">{selected.body}</pre>
        </div>
      )}
    </div>
  );
}
