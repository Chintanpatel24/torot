import { useStore } from "../lib/store";
import type { View } from "../lib/store";
import "./Sidebar.css";

const NAV: { view: View; icon: string; label: string }[] = [
  { view: "home",     icon: "⌂",  label: "Home"     },
  { view: "scan",     icon: "◉",  label: "Scan"     },
  { view: "findings", icon: "◈",  label: "Findings" },
  { view: "history",  icon: "⊟",  label: "History"  },
  { view: "tools",    icon: "⚙",  label: "Tools"    },
  { view: "settings", icon: "≡",  label: "Settings" },
];

export default function Sidebar() {
  const { view, setView, scan, tools } = useStore();
  const critCount     = scan.findings.filter((f) => f.severity === "CRITICAL").length;
  const installedCount = tools.filter((t) => t.installed).length;

  return (
    <nav className="sidebar">
      <div className="sidebar-nav">
        {NAV.map(({ view: v, icon, label }) => (
          <button
            key={v}
            className={`nav-item ${view === v ? "active" : ""}`}
            onClick={() => setView(v)}
          >
            <span className="nav-icon">{icon}</span>
            <span className="nav-label">{label}</span>
            {v === "scan" && scan.running && <span className="nav-badge running" />}
            {v === "findings" && critCount > 0 && (
              <span className="nav-count">{critCount}</span>
            )}
          </button>
        ))}
      </div>

      <div className="sidebar-footer">
        {scan.running && (
          <div className="sidebar-scan-indicator">
            <span className="scan-pulse" />
            <span className="scan-label">scanning</span>
          </div>
        )}
        <span className="sidebar-tool-count">
          {installedCount}/{tools.length} tools
        </span>
      </div>
    </nav>
  );
}
