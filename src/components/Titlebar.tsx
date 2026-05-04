import { useStore } from "../lib/store";
import "./Titlebar.css";

export default function Titlebar() {
  const { scan, dbStats } = useStore();

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-left" data-tauri-drag-region>
        {/* Logo mark — bull icon rendered as SVG inline, no image tag as "logo" */}
        <div className="titlebar-logo">
          <svg width="22" height="22" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg">
            {/* Shield body */}
            <path d="M50 8 L88 24 L88 52 C88 72 70 88 50 95 C30 88 12 72 12 52 L12 24 Z"
              fill="#1a1e28" stroke="#f07a1a" strokeWidth="3"/>
            {/* Bull head */}
            <ellipse cx="50" cy="52" rx="20" ry="18" fill="#f07a1a"/>
            {/* Horns */}
            <path d="M30 44 Q22 30 26 24 Q32 30 35 40" fill="#c85a00"/>
            <path d="M70 44 Q78 30 74 24 Q68 30 65 40" fill="#8a8a8a"/>
            {/* Mechanical right half overlay */}
            <path d="M50 34 L50 70 C58 70 70 62 70 52 C70 42 62 34 50 34 Z"
              fill="#2a2e3a" stroke="#555" strokeWidth="1"/>
            {/* Gear teeth */}
            <circle cx="62" cy="46" r="4" fill="none" stroke="#888" strokeWidth="1.5"/>
            <circle cx="62" cy="58" r="3" fill="none" stroke="#888" strokeWidth="1.5"/>
            {/* Eye left (organic) */}
            <circle cx="42" cy="50" r="4" fill="#1a0800"/>
            <circle cx="42" cy="50" r="2" fill="#ff6a00"/>
            {/* Eye right (mechanical) */}
            <rect x="57" y="47" width="7" height="5" rx="1" fill="#4a90d9" opacity="0.9"/>
            <line x1="57" y1="49.5" x2="64" y2="49.5" stroke="#1a1e28" strokeWidth="1"/>
            {/* Nose */}
            <ellipse cx="50" cy="62" rx="8" ry="5" fill="#c85a00"/>
            <circle cx="47" cy="62" r="2" fill="#1a0800"/>
            <circle cx="53" cy="62" r="2" fill="#1a0800"/>
          </svg>
        </div>
        <span className="titlebar-name">TOROT</span>
        <span className="titlebar-version">v3.0</span>
      </div>

      <div className="titlebar-center" data-tauri-drag-region>
        {scan.running && (
          <div className="titlebar-status">
            <span className="status-dot running" />
            <span className="status-text mono">
              scanning {scan.target.length > 30 ? "..." + scan.target.slice(-28) : scan.target}
            </span>
            <span className="status-count">{scan.findings.length} findings</span>
          </div>
        )}
      </div>

      <div className="titlebar-right">
        <div className="titlebar-stat">
          <span className="stat-label">sessions</span>
          <span className="stat-value">{dbStats.sessions}</span>
        </div>
        <div className="titlebar-stat critical">
          <span className="stat-label">critical</span>
          <span className="stat-value">{dbStats.critical}</span>
        </div>
        <div className="titlebar-stat high">
          <span className="stat-label">high</span>
          <span className="stat-value">{dbStats.high}</span>
        </div>
      </div>
    </div>
  );
}
