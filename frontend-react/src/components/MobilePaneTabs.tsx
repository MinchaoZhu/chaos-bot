export type MobilePane = "chat" | "sessions" | "events" | "config" | "skills";

type MobilePaneTabsProps = {
  activePane: MobilePane;
  onChange: (pane: MobilePane) => void;
};

const PANES: MobilePane[] = ["chat", "sessions", "events", "config", "skills"];

export function MobilePaneTabs({ activePane, onChange }: MobilePaneTabsProps) {
  return (
    <nav className="mobile-tabs" aria-label="Mobile panes">
      {PANES.map((pane) => (
        <button
          key={pane}
          type="button"
          className={activePane === pane ? "active" : ""}
          onClick={() => onChange(pane)}
        >
          {pane}
        </button>
      ))}
    </nav>
  );
}
