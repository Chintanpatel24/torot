import { useState, useEffect } from "react";
import { useStore } from "../lib/store";
import { api } from "../lib/api";
import type { AppConfig } from "../lib/store";

export default function SettingsView() {
  const { config, setConfig } = useStore();
  const [draft, setDraft]     = useState<AppConfig | null>(null);
  const [saving, setSaving]   = useState(false);
  const [saved,  setSaved]    = useState(false);

  useEffect(() => {
    if (config) setDraft(JSON.parse(JSON.stringify(config)));
  }, [config]);

  if (!draft) return (
    <div style={{ padding: 24, color: "var(--text-muted)" }}>Loading settings…</div>
  );

  async function save() {
    if (!draft) return;
    setSaving(true);
    try {
      const updated = await api.saveSettings(draft);
      setConfig(updated);
      setDraft(JSON.parse(JSON.stringify(updated)));
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%", maxWidth: 680 }}>
      <h2 style={{ color: "var(--text-primary)", fontSize: 15, fontWeight: 700, marginBottom: 20 }}>
        Settings
      </h2>

      <Section title="General">
        <FormRow label="Install mode">
          <select
            value={draft.install_mode}
            onChange={(e) => setDraft({ ...draft, install_mode: e.target.value })}
          >
            <option value="both">Desktop + CLI</option>
            <option value="desktop">Desktop only</option>
            <option value="cli">CLI only</option>
          </select>
        </FormRow>
      </Section>

      <Section title="Sandbox">
        <FormRow label="Profile">
          <select
            value={draft.sandbox.profile}
            onChange={(e) =>
              setDraft({ ...draft, sandbox: { ...draft.sandbox, profile: e.target.value } })
            }
          >
            <option value="strong">Strong</option>
            <option value="moderate">Moderate</option>
            <option value="off">Off</option>
          </select>
        </FormRow>
        <FormRow label="Max runtime (seconds)">
          <input
            type="text"
            value={draft.sandbox.max_runtime_seconds}
            onChange={(e) =>
              setDraft({
                ...draft,
                sandbox: { ...draft.sandbox, max_runtime_seconds: Number(e.target.value) || 900 },
              })
            }
          />
        </FormRow>
      </Section>

      <Section title="Default Report Template">
        <textarea
          rows={10}
          value={draft.default_report_template}
          onChange={(e) => setDraft({ ...draft, default_report_template: e.target.value })}
          style={{ fontFamily: "var(--font-mono)", fontSize: 12, resize: "vertical" }}
        />
      </Section>

      <div style={{ display: "flex", gap: 10, marginTop: 10 }}>
        <button className="btn btn-primary" onClick={save} disabled={saving}>
          {saving ? "Saving…" : saved ? "Saved ✓" : "Save Settings"}
        </button>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={{ marginBottom: 24 }}>
      <div
        style={{
          fontSize: 10,
          fontWeight: 700,
          letterSpacing: "0.08em",
          textTransform: "uppercase",
          color: "var(--text-muted)",
          marginBottom: 12,
        }}
      >
        {title}
      </div>
      <div
        style={{
          background: "var(--bg-elevated)",
          border: "1px solid var(--border-default)",
          borderRadius: "var(--radius-md)",
          overflow: "hidden",
        }}
      >
        {children}
      </div>
    </div>
  );
}

function FormRow({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 16,
        padding: "10px 14px",
        borderBottom: "1px solid var(--border-subtle)",
      }}
    >
      <span style={{ width: 180, fontSize: 12, color: "var(--text-secondary)", flexShrink: 0 }}>
        {label}
      </span>
      <div style={{ flex: 1 }}>{children}</div>
    </div>
  );
}
