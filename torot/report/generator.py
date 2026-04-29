"""
Torot Report Generator.
Produces a detailed Markdown report from a completed ScanSession.
"""

from __future__ import annotations
import os
import time
from pathlib import Path

from torot.core.models import ScanSession, Severity, Bug, ToolStatus


SEVERITY_ORDER = [Severity.CRITICAL, Severity.HIGH, Severity.MEDIUM, Severity.LOW, Severity.INFO]


def generate_report(session: ScanSession, output_path: str | None = None) -> str:
    """
    Generate a Markdown report and write it to disk.
    Returns the path to the saved file.
    """
    if output_path is None:
        ts = time.strftime("%Y%m%d_%H%M%S")
        output_path = f"torot_report_{ts}.md"

    content = _build_markdown(session)
    Path(output_path).write_text(content, encoding="utf-8")
    return output_path


def _build_markdown(session: ScanSession) -> str:
    lines: list[str] = []
    ts = time.strftime("%Y-%m-%d %H:%M:%S")
    summary = session.bug_summary
    all_bugs = session.all_bugs

    # ── Header ──────────────────────────────────────────────────────────────
    lines += [
        "# 🔍 Torot Security Analysis Report",
        "",
        f"> **Generated:** {ts}  ",
        f"> **Target:** `{session.target_path}`  ",
        f"> **Duration:** {session.duration:.1f}s  ",
        f"> **Languages Detected:** {', '.join(session.detected_languages) or 'N/A'}  ",
        f"> **Files Scanned:** {len(session.detected_files)}  ",
        "",
        "---",
        "",
    ]

    # ── Executive Summary ───────────────────────────────────────────────────
    lines += [
        "## 📊 Executive Summary",
        "",
        f"| Severity | Count |",
        f"|----------|-------|",
    ]
    total = 0
    for sev in SEVERITY_ORDER:
        count = summary.get(sev.value, 0)
        total += count
        emoji = sev.emoji
        lines.append(f"| {emoji} **{sev.value}** | {count} |")
    lines += [
        f"| **TOTAL** | **{total}** |",
        "",
    ]

    # ── Tool Results Overview ────────────────────────────────────────────────
    lines += [
        "## ⚙️ Tool Pipeline Results",
        "",
        "| Tool | Status | Bugs Found | Duration |",
        "|------|--------|-----------|----------|",
    ]
    for tool_name, result in session.tool_results.items():
        status_icon = result.status.icon
        status_label = result.status.value
        bug_count = len(result.bugs)
        duration = f"{result.duration:.1f}s" if result.duration else "—"
        lines.append(f"| **{tool_name}** | {status_icon} {status_label} | {bug_count} | {duration} |")
    lines += ["", "---", ""]

    # ── Bugs by Severity ────────────────────────────────────────────────────
    lines += ["## 🐛 Detailed Findings", ""]

    bugs_by_severity: dict[str, list[Bug]] = {s.value: [] for s in SEVERITY_ORDER}
    for bug in all_bugs:
        bugs_by_severity[bug.severity.value].append(bug)

    for sev in SEVERITY_ORDER:
        bugs = bugs_by_severity[sev.value]
        if not bugs:
            continue

        lines += [
            f"### {sev.emoji} {sev.value} Severity — {len(bugs)} Issue(s)",
            "",
        ]

        for i, bug in enumerate(bugs, 1):
            lines += [
                f"#### {i}. {bug.title}",
                "",
                f"- **Tool:** `{bug.tool}`",
                f"- **Severity:** {bug.severity.emoji} `{bug.severity.value}`",
                f"- **Type:** `{bug.bug_type or 'general'}`",
                f"- **Location:** `{bug.location}`" if bug.location != "unknown" else "- **Location:** N/A",
                "",
                "**Description:**",
                "",
                f"{bug.description}",
                "",
            ]

            if bug.code_snippet:
                lines += [
                    "**Buggy Code:**",
                    "",
                    "```solidity" if bug.tool not in ("aderyn",) else "```rust",
                    bug.code_snippet,
                    "```",
                    "",
                ]

            if bug.impact:
                lines += [
                    "**Potential Impact:**",
                    "",
                    f"> ⚠️ {bug.impact}",
                    "",
                ]

            if bug.fix_suggestion:
                lines += [
                    "**Fix / Recommendation:**",
                    "",
                    f"```",
                    bug.fix_suggestion,
                    "```",
                    "",
                ]

            if bug.references:
                lines += ["**References:**", ""]
                for ref in bug.references:
                    if ref.strip():
                        lines.append(f"- {ref}")
                lines.append("")

            lines.append("---")
            lines.append("")

    # ── Not Installed Tools ──────────────────────────────────────────────────
    not_installed = [
        name for name, result in session.tool_results.items()
        if result.status == ToolStatus.NOT_INSTALLED
    ]
    if not_installed:
        lines += [
            "## 🔧 Tools Not Installed",
            "",
            "The following tools were not found in PATH and were skipped:",
            "",
        ]
        for tool in not_installed:
            lines.append(f"- `{tool}`")
        lines += [
            "",
            "> Install missing tools to get more comprehensive coverage.",
            "",
        ]

    # ── Footer ───────────────────────────────────────────────────────────────
    lines += [
        "---",
        "",
        "## ℹ️ About Torot",
        "",
        "**Torot** is an open-source blockchain & smart contract bug hunting tool.",
        "It orchestrates industry-standard security tools and produces unified reports.",
        "",
        "- GitHub: [github.com/your-org/torot](https://github.com/your-org/torot)",
        "- Powered by: Slither, Aderyn, Mythril, Manticore, Echidna, Securify2, solhint, Oyente, SmartCheck, Halmos",
        "",
        f"*Report generated by Torot v1.0.0 at {ts}*",
    ]

    return "\n".join(lines)
