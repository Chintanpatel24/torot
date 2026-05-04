import { useEffect } from "react";
import { useStore } from "./lib/store";
import { api, onStreamLine, onNewFinding, onScanComplete } from "./lib/api";
import Titlebar    from "./components/Titlebar";
import Sidebar     from "./components/Sidebar";
import HomeView    from "./components/HomeView";
import ScanView    from "./components/ScanView";
import FindingsView from "./components/FindingsView";
import HistoryView from "./components/HistoryView";
import ToolsView   from "./components/ToolsView";
import SettingsView from "./components/SettingsView";
import "./styles/app.css";

export default function App() {
  const { view, addStreamLine, addFinding, stopScanState, setTools, setDbStats } = useStore();

  useEffect(() => {
    // Load tools on boot
    api.getTools().then(setTools).catch(() => {});
    api.getDbStats().then(setDbStats).catch(() => {});

    // Subscribe to backend events
    const unsubs: (() => void)[] = [];

    onStreamLine((line) => addStreamLine(line)).then((u) => unsubs.push(u));
    onNewFinding((f)    => addFinding(f)).then((u)    => unsubs.push(u));
    onScanComplete(()   => stopScanState()).then((u)   => unsubs.push(u));

    return () => unsubs.forEach((u) => u());
  }, []);

  return (
    <div className="app-root">
      <Titlebar />
      <div className="app-body">
        <Sidebar />
        <main className="app-content">
          {view === "home"     && <HomeView />}
          {view === "scan"     && <ScanView />}
          {view === "findings" && <FindingsView />}
          {view === "history"  && <HistoryView />}
          {view === "tools"    && <ToolsView />}
          {view === "settings" && <SettingsView />}
        </main>
      </div>
    </div>
  );
}
