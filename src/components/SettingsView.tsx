import { useState } from "react";

interface Setting { label: string; key: string; placeholder: string; secret?: boolean }

const SETTINGS: Setting[] = [
  { label: "Anthropic API Key",   key: "anthropic_key",   placeholder: "sk-ant-...", secret: true },
  { label: "OpenAI API Key",      key: "openai_key",      placeholder: "sk-...",     secret: true },
  { label: "Ollama URL",          key: "ollama_url",      placeholder: "http://localhost:11434" },
  { label: "Ollama Model",        key: "ollama_model",    placeholder: "llama3" },
  { label: "Etherscan API Key",   key: "etherscan_key",   placeholder: "ABCDEF123...", secret: true },
  { label: "GitHub Token",        key: "github_token",    placeholder: "ghp_...",    secret: true },
  { label: "GitHub Repo",         key: "github_repo",     placeholder: "owner/repo" },
];

export default function SettingsView() {
  const [values, setValues] = useState<Record<string, string>>(() => {
    const stored: Record<string, string> = {};
    SETTINGS.forEach((s) => {
      stored[s.key] = localStorage.getItem(`torot:${s.key}`) || "";
    });
    return stored;
  });
  const [saved, setSaved] = useState(false);

  function set(key: string, val: string) {
    setValues((v) => ({ ...v, [key]: val }));
    setSaved(false);
  }

  function save() {
    SETTINGS.forEach((s) => {
      if (values[s.key]) {
        localStorage.setItem(`torot:${s.key}`, values[s.key]);
      } else {
        localStorage.removeItem(`torot:${s.key}`);
      }
    });
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  }

  return (
    <div style={{ padding: 24, overflow: "auto", height: "100%", maxWidth: 600 }}>
      <h2 style={{ color: "var(--text-primary)", fontSize: 16, fontWeight: 600, marginBottom: 20 }}>
        Settings
      </h2>

      <div style={{ display: "flex", flexDirection: "column", gap: 14 }}>
        {SETTINGS.map((s) => (
          <div key={s.key} style={{ display: "flex", flexDirection: "column", gap: 5 }}>
            <label style={{ fontSize: 11, fontWeight: 600, color: "var(--text-muted)", textTransform: "uppercase", letterSpacing: "0.06em" }}>
              {s.label}
            </label>
            <input
              type={s.secret ? "password" : "text"}
              value={values[s.key] || ""}
              onChange={(e) => set(s.key, e.target.value)}
              placeholder={s.placeholder}
              style={{
                background: "var(--bg-input)",
                border: "1px solid var(--border-default)",
                borderRadius: "var(--radius-md)",
                padding: "8px 11px",
                color: "var(--text-primary)",
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                outline: "none",
              }}
            />
          </div>
        ))}
      </div>

      <div style={{ marginTop: 24, display: "flex", alignItems: "center", gap: 12 }}>
        <button
          onClick={save}
          style={{
            padding: "9px 24px",
            background: "var(--orange)",
            color: "#0d0f14",
            border: "none",
            borderRadius: "var(--radius-md)",
            fontWeight: 700,
            fontSize: 13,
            cursor: "pointer",
          }}
        >
          Save Settings
        </button>
        {saved && (
          <span style={{ fontSize: 12, color: "var(--green)", fontFamily: "var(--font-mono)" }}>
            Saved.
          </span>
        )}
      </div>

      <div style={{ marginTop: 32, padding: "14px 16px", background: "var(--bg-surface)", border: "1px solid var(--border-subtle)", borderRadius: "var(--radius-md)" }}>
        <p style={{ fontSize: 12, color: "var(--text-muted)", lineHeight: 1.7 }}>
          API keys are stored locally in your browser's localStorage and are never transmitted except directly to their respective services (Anthropic, OpenAI, Etherscan, GitHub).
        </p>
      </div>
    </div>
  );
}
