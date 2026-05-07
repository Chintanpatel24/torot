import { useStore } from "../lib/store";
import type { View } from "../lib/store";
import "./Sidebar.css";

const NAV_ITEMS: { view: View; icon: string; label: string }[] = [
  { view: "home",     icon: "⌂", label: "Home"     },
  { view: "scan",     icon: "◎", label: "Scan"     },
  { view: "findings", icon: "◈", label: "Findings" },
  { view: "history",  icon: "◷", label: "History"  },
  { view: "tools",    icon: "⚙", label: "Tools"    },
  { view: "settings", icon: "≡", label: "Settings" },
];

export default function Sidebar() {
  const { view, setView, scan, tools } = useStore();
  const installedCount = tools.filter((t) => t.installed).length;
  const criticalCount  = scan.findings.filter((f) => f.severity === "CRITICAL").length;

  return (
    <nav className="sidebar">
      <span className="sidebar-section">Navigate</span>
      {NAV_ITEMS.map(({ view: v, icon, label }) => (
        <button
          key={v}
          className={`sidebar-nav-btn ${view === v ? "active" : ""}`}
          onClick={() => setView(v)}
        >
          <span className="nav-icon">{icon}</span>
          {label}
          {v === "findings" && criticalCount > 0 && (
            <span className="badge badge-critical" style={{ marginLeft: "auto", fontSize: 9 }}>
              {criticalCount}
            </span>
          )}
        </button>
      ))}
      <div className="sidebar-divider" />
      <div className="sidebar-bottom">
        <div>{installedCount} / {tools.length} tools</div>
        {scan.running && (
          <div style={{ color: "var(--accent)", marginTop: 4 }}>⦿ scanning</div>
        )}
      </div>
    </nav>
  );
}
