import { useState } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";
import type { ToolProfile } from "../lib/store";

export default function ToolsView() {
  const { tools, setTools } = useStore();
  const [search, setSearch] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const filtered = tools.filter(
    (t) =>
      t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.description.toLowerCase().includes(search.toLowerCase())
  );

  async function toggleTool(name: string, enabled: boolean) {
    const tool = tools.find((t) => t.name === name);
    if (!tool) return;
    setSaving(true);
    try {
      const updated = await api.saveToolProfile({ ...tool, enabled } as Partial<ToolProfile>);
      setTools(updated);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 20 }}>
        <h2 style={{ color: "var(--text-primary)", fontSize: 15, fontWeight: 700 }}>
          Tool Registry
        </h2>
        <span style={{ color: "var(--text-muted)", fontSize: 12, fontFamily: "var(--font-mono)" }}>
          {tools.filter((t) => t.installed).length}/{tools.length} installed
        </span>
        <input
          type="search"
          placeholder="Filter tools…"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          style={{ marginLeft: "auto", width: 200 }}
        />
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
        {filtered.map((tool) => (
          <div
            key={tool.name}
            style={{
              background: "var(--bg-elevated)",
              border: `1px solid ${editing === tool.name ? "var(--accent)" : "var(--border-default)"}`,
              borderRadius: "var(--radius-md)",
              padding: "12px 14px",
              display: "flex",
              alignItems: "center",
              gap: 12,
            }}
          >
            {/* Status dot */}
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: tool.installed ? "var(--accent)" : "var(--text-muted)",
                flexShrink: 0,
              }}
            />

            {/* Name + desc */}
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontWeight: 700,
                    fontSize: 12,
                    color: "var(--text-primary)",
                  }}
                >
                  {tool.name}
                </span>
                <span
                  style={{
                    padding: "1px 6px",
                    borderRadius: 4,
                    background: "var(--bg-hover)",
                    fontSize: 10,
                    color: "var(--text-muted)",
                    fontFamily: "var(--font-mono)",
                  }}
                >
                  {tool.domain}
                </span>
                {tool.source === "custom" && (
                  <span style={{ fontSize: 10, color: "var(--blue)" }}>custom</span>
                )}
              </div>
              <p style={{ fontSize: 11, color: "var(--text-muted)", marginTop: 2 }}>
                {tool.description}
              </p>
              {tool.installed && tool.version && (
                <span
                  style={{
                    fontSize: 10,
                    color: "var(--text-secondary)",
                    fontFamily: "var(--font-mono)",
                  }}
                >
                  {tool.version}
                </span>
              )}
              {!tool.installed && (
                <span style={{ fontSize: 10, color: "var(--text-muted)" }}>
                  {tool.install_hint}
                </span>
              )}
            </div>

            {/* Toggle */}
            <button
              className={`btn ${tool.enabled ? "btn-secondary" : "btn-secondary"}`}
              style={{
                padding: "4px 10px",
                fontSize: 11,
                opacity: saving ? 0.5 : 1,
                color: tool.enabled ? "var(--accent)" : "var(--text-muted)",
              }}
              onClick={() => toggleTool(tool.name, !tool.enabled)}
              disabled={saving}
            >
              {tool.enabled ? "enabled" : "disabled"}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
