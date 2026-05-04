import { useStore, type View } from "../lib/store";
import "./Sidebar.css";

interface NavItem {
  id:    View;
  label: string;
  icon:  string;
}

const NAV: NavItem[] = [
  { id: "home",     label: "Home",     icon: "H" },
  { id: "scan",     label: "Scan",     icon: "S" },
  { id: "findings", label: "Findings", icon: "F" },
  { id: "history",  label: "History",  icon: "Y" },
  { id: "tools",    label: "Tools",    icon: "T" },
  { id: "settings", label: "Settings", icon: "G" },
];

export default function Sidebar() {
  const { view, setView, scan, tools } = useStore();
  const installedCount = tools.filter((t) => t.installed).length;

  return (
    <aside className="sidebar">
      <nav className="sidebar-nav">
        {NAV.map((item) => (
          <button
            key={item.id}
            className={`nav-item ${view === item.id ? "active" : ""}`}
            onClick={() => setView(item.id)}
            title={item.label}
          >
            <span className="nav-icon mono">{item.icon}</span>
            <span className="nav-label">{item.label}</span>
            {item.id === "scan" && scan.running && (
              <span className="nav-badge running" />
            )}
            {item.id === "findings" && scan.findings.length > 0 && (
              <span className="nav-count">{scan.findings.length}</span>
            )}
            {item.id === "tools" && (
              <span className="nav-count dim">{installedCount}</span>
            )}
          </button>
        ))}
      </nav>

      <div className="sidebar-footer">
        {scan.running && (
          <div className="sidebar-scan-indicator">
            <div className="scan-pulse" />
            <span className="scan-label mono">scanning</span>
          </div>
        )}
        <div className="sidebar-mode mono">
          {scan.mode === "loop"   && "LOOP"}
          {scan.mode === "daemon" && "DAEMON"}
          {scan.mode === "single" && "SINGLE"}
        </div>
      </div>
    </aside>
  );
}
