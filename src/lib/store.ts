import { create } from "zustand";

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

export type Severity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "INFO";
export type Domain   = "blockchain" | "webapp" | "api" | "binary" | "general";
export type Mode     = "single" | "loop" | "daemon";
export type View     = "home" | "scan" | "findings" | "history" | "tools" | "settings";

