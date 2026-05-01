"""
Torot v2 Report Generator
Produces Markdown reports from Session objects.
"""
from __future__ import annotations
import time
from pathlib import Path
from torot.core.models import Session, Severity, Finding


SEVERITY_ORDER = [Severity.CRITICAL, Severity.HIGH, Severity.MEDIUM, Severity.LOW, Severity.INFO]


def generate_report(session: Session, output_path: str | None = None) -> str:
    if not output_path:
        ts = time.strftime("%Y%m%d_%H%M%S")
        output_path = f"torot_report_{ts}.md"
    Path(output_path).write_text(_build(session), encoding="utf-8")
    return output_path


def _build(session: Session) -> str:
    ts = time.strftime("%Y-%m-%d %H:%M:%S")
    L: list[str] = []

    L += [
        "# Torot Security Report",
        "",
        f"> **Generated:** {ts}",
        f"> **Session:** `{session.id}`",
        f"> **Target:** `{session.target}`",
        f"> **Domain:** {session.domain.value}",
        f"> **Duration:** {session.duration:.1f}s",
        f"> **AI Provider:** {session.ai_config.provider.value if session.ai_config else 'offline'}",
        "",
        "---", "",
        "## Executive Summary", "",
        "| Severity | Count |",
        "|----------|-------|",
    ]

    for sev in SEVERITY_ORDER:
        count = session.finding_summary.get(sev.value, 0)
        L.append(f"| **{sev.value}** | {count} |")
    L += [f"| **TOTAL** | **{len(session.findings)}** |", "", "---", ""]

    # Conversation log
    if session.messages:
        L += ["## Session Conversation", ""]
        for msg in session.messages:
            role = msg.role.upper()
            L.append(f"**{role}:** {msg.content[:400]}")
            L.append("")
        L += ["---", ""]

    # Findings
    L += ["## Findings", ""]
    bugs_by_sev: dict[str, list[Finding]] = {s.value: [] for s in SEVERITY_ORDER}
    for f in session.findings:
        bugs_by_sev[f.severity.value].append(f)

    for sev in SEVERITY_ORDER:
        findings = bugs_by_sev[sev.value]
        if not findings:
            continue
        L += [f"### {sev.value} — {len(findings)} finding(s)", ""]
        for i, f in enumerate(findings, 1):
            L += [
                f"#### {i}. {f.title}",
                "",
                f"| Field | Value |",
                f"|-------|-------|",
                f"| Tool | `{f.tool}` |",
                f"| Location | `{f.location}` |",
                f"| Type | `{f.bug_type or 'general'}` |",
                "",
                f"**Description:** {f.description}",
                "",
            ]
            if f.code_snippet:
                L += ["**Code:**", "", "```", f.code_snippet.strip(), "```", ""]
            if f.impact:
                L += [f"**Impact:** {f.impact}", ""]
            if f.ai_analysis:
                L += [f"**AI Analysis:** {f.ai_analysis}", ""]
            if f.fix_suggestion:
                L += ["**Fix:**", "", "```", f.fix_suggestion.strip(), "```", ""]
            L += ["---", ""]

    L += [
        "---", "",
        "## About Torot",
        "",
        "Torot v2 — Universal Security Agent",
        f"*Report generated at {ts}*",
    ]
    return "\n".join(L)
