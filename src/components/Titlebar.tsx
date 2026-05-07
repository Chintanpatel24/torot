import { useStore } from "../lib/store";

export default function Titlebar() {
  const { appInfo, dbStats } = useStore();

  return (
    <div className="titlebar">
      <div className="titlebar-left">
        <span className="titlebar-logo">TOROT</span>
        <span className="titlebar-version">v{appInfo?.version ?? "4.0.0"}</span>
      </div>
      <div className="titlebar-right">
        {dbStats && (
          <>
            <span className="titlebar-stat">
              sessions<span>{dbStats.sessions}</span>
            </span>
            <span className="titlebar-stat">
              findings<span>{dbStats.findings}</span>
            </span>
            {dbStats.critical > 0 && (
              <span className="titlebar-stat" style={{ color: "var(--sev-critical)" }}>
                critical<span>{dbStats.critical}</span>
              </span>
            )}
          </>
        )}
      </div>
    </div>
  );
}
