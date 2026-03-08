export type PrimaryPane =
  | "conversation"
  | "sessions"
  | "events"
  | "config"
  | "skills"
  | "about";

type PrimaryTabsProps = {
  activePane: PrimaryPane;
  onChange: (pane: PrimaryPane) => void;
};

const PANES: PrimaryPane[] = [
  "conversation",
  "sessions",
  "events",
  "config",
  "skills",
  "about",
];

function labelForPane(pane: PrimaryPane): string {
  switch (pane) {
    case "conversation":
      return "Conversation";
    case "sessions":
      return "Sessions";
    case "events":
      return "Events";
    case "config":
      return "Config";
    case "skills":
      return "Skills";
    case "about":
      return "About";
  }
}

export function PrimaryTabs({ activePane, onChange }: PrimaryTabsProps) {
  return (
    <nav className="primary-tabs" aria-label="Primary panes">
      {PANES.map((pane) => (
        <button
          key={pane}
          type="button"
          className={activePane === pane ? "active" : ""}
          onClick={() => onChange(pane)}
        >
          {labelForPane(pane)}
        </button>
      ))}
    </nav>
  );
}
