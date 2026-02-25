import { useEffect, useState } from "react";
import type { SkillDetail, SkillMeta } from "../contracts/protocol";
import type { RuntimeAdapter } from "../runtime/adapter";

type SkillsPanelProps = {
  runtime: RuntimeAdapter;
  baseUrl: string;
};

export function SkillsPanel({ runtime, baseUrl }: SkillsPanelProps) {
  const [skills, setSkills] = useState<SkillMeta[]>([]);
  const [selected, setSelected] = useState<SkillDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError("");

    void runtime
      .listSkills(baseUrl)
      .then((list) => {
        if (!cancelled) setSkills(list);
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(String(err));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
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

  return (
    <div className="panel skills-panel">
      <h3 className="panel-title">Skills</h3>

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
