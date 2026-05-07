import { useStore } from "../lib/store";

export default function Titlebar() {
  const { appInfo, dbStats } = useStore();

  return (
    <div className="titlebar">
      <span className="titlebar-logo">TOROT</span>
      <div className="titlebar-sep" />
      <span className="titlebar-version">v{appInfo?.version ?? "4.0.0"}</span>
      <div className="titlebar-spacer" />
      <div className="titlebar-stats">
        {dbStats && (
          <>
            <span className="titlebar-stat">
              <span className="titlebar-stat-label">sessions</span>
              <span className="titlebar-stat-val">{dbStats.sessions}</span>
            </span>
            <span className="titlebar-stat">
              <span className="titlebar-stat-label">findings</span>
              <span className="titlebar-stat-val">{dbStats.findings}</span>
            </span>
            {dbStats.critical > 0 && (
              <span className="titlebar-stat">
                <span className="titlebar-stat-label">critical</span>
                <span className="titlebar-stat-val hot">{dbStats.critical}</span>
              </span>
            )}
          </>
        )}
      </div>
    </div>
  );
}
