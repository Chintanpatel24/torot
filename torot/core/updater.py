"""
Torot built-in updater.
Called by: torot --update  or  torot --update --check
"""
from __future__ import annotations
import shutil
import subprocess
import sys
import os
from dataclasses import dataclass

from rich.console import Console
from rich.table   import Table
from rich.text    import Text
from rich         import box

console = Console()


@dataclass
class ToolUpdateResult:
    name:    str
    binary:  str
    cmd:     str
    status:  str   # "updated" | "skipped" | "not_installed" | "failed"
    note:    str   = ""


# ── pip tools ────────────────────────────────────────────────────────────────
PIP_TOOLS = [
    ("slither",    "slither",    "pip install --upgrade slither-analyzer"),
    ("mythril",    "myth",       "pip install --upgrade mythril"),
    ("manticore",  "manticore",  "pip install --upgrade manticore[native]"),
    ("halmos",     "halmos",     "pip install --upgrade halmos"),
    ("semgrep",    "semgrep",    "pip install --upgrade semgrep"),
    ("sqlmap",     "sqlmap",     "pip install --upgrade sqlmap"),
    ("wfuzz",      "wfuzz",      "pip install --upgrade wfuzz"),
    ("arjun",      "arjun",      "pip install --upgrade arjun"),
    ("binwalk",    "binwalk",    "pip install --upgrade binwalk"),
    ("checksec",   "checksec",   "pip install --upgrade checksec"),
    ("eth-wake",   "wake",       "pip install --upgrade eth-wake"),
]

# ── cargo tools ──────────────────────────────────────────────────────────────
CARGO_TOOLS = [
    ("aderyn",      "aderyn",      "cargo install aderyn"),
    ("cargo-audit", "cargo-audit", "cargo install cargo-audit"),
    ("pyrometer",   "pyrometer",   "cargo install pyrometer"),
    ("heimdall",    "heimdall",    "cargo install heimdall-rs"),
]

# ── npm tools ────────────────────────────────────────────────────────────────
NPM_TOOLS = [
    ("solhint",    "solhint",    "npm update -g solhint"),
    ("smartcheck", "smartcheck", "npm update -g smartcheck"),
]

# ── go tools ─────────────────────────────────────────────────────────────────
GO_TOOLS = [
    ("nuclei",      "nuclei",   "go install github.com/projectdiscovery/nuclei/v3/cmd/nuclei@latest"),
    ("ffuf",        "ffuf",     "go install github.com/ffuf/ffuf/v2@latest"),
    ("gobuster",    "gobuster", "go install github.com/OJ/gobuster/v3@latest"),
    ("dalfox",      "dalfox",   "go install github.com/hahwul/dalfox/v2@latest"),
    ("gitleaks",    "gitleaks", "go install github.com/zricethezav/gitleaks/v8@latest"),
    ("kiterunner",  "kr",       "go install github.com/assetnote/kiterunner/cmd/kr@latest"),
]


def _run_cmd(cmd: str) -> tuple[bool, str]:
    """Run a shell command, return (success, error_message)."""
    try:
        result = subprocess.run(
            cmd, shell=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            timeout=300,
        )
        if result.returncode == 0:
            return True, ""
        return False, result.stderr.decode("utf-8", errors="replace")[:200]
    except subprocess.TimeoutExpired:
        return False, "timed out"
    except Exception as e:
        return False, str(e)


def _update_group(
    group:      list[tuple[str, str, str]],
    check_only: bool,
    prereq:     str = "",
) -> list[ToolUpdateResult]:
    """Update a group of tools. If prereq binary is missing, skip all."""
    results: list[ToolUpdateResult] = []

    if prereq and not shutil.which(prereq):
        for name, binary, cmd in group:
            results.append(ToolUpdateResult(
                name=name, binary=binary, cmd=cmd,
                status="skipped",
                note=f"{prereq} not installed",
            ))
        return results

    for name, binary, cmd in group:
        if not shutil.which(binary):
            results.append(ToolUpdateResult(
                name=name, binary=binary, cmd=cmd,
                status="not_installed",
                note="not in PATH",
            ))
            continue

        if check_only:
            results.append(ToolUpdateResult(
                name=name, binary=binary, cmd=cmd,
                status="installed",
                note=f"would run: {cmd}",
            ))
            continue

        ok, err = _run_cmd(cmd)
        results.append(ToolUpdateResult(
            name=name, binary=binary, cmd=cmd,
            status="updated" if ok else "failed",
            note=err if not ok else "",
        ))

    return results


def update_torot_package(check_only: bool = False) -> str:
    """Upgrade the torot Python package itself."""
    if check_only:
        from torot.cli import parse_args   # noqa: just checking import
        return "check-only"

    ok, err = _run_cmd(f"{sys.executable} -m pip install --upgrade torot")
    if ok:
        return "updated"
    # Try upgrading from local source (editable install)
    torot_dir = os.path.dirname(os.path.dirname(os.path.dirname(__file__)))
    if os.path.exists(os.path.join(torot_dir, "pyproject.toml")):
        ok2, err2 = _run_cmd(f"{sys.executable} -m pip install -e {torot_dir} --quiet")
        return "updated" if ok2 else f"failed: {err2}"
    return f"failed: {err}"


def run_update(check_only: bool = False):
    """
    Main entry point for torot --update.
    Prints a live table of every tool's update status.
    """
    console.print()
    console.print(Text("  Torot Updater", style="bold cyan"))
    console.print()

    if check_only:
        console.print(Text("  Running in check-only mode — no changes will be made.", style="dim"))
        console.print()

    all_results: list[ToolUpdateResult] = []

    # Torot itself
    console.print(Text("  Updating Torot...", style="cyan"))
    status = update_torot_package(check_only)
    all_results.append(ToolUpdateResult(
        name="torot", binary="torot", cmd="pip install --upgrade torot",
        status=status,
    ))

    # Tool groups
    groups = [
        (PIP_TOOLS,   "pip3",  "Python tools"),
        (CARGO_TOOLS, "cargo", "Rust tools"),
        (NPM_TOOLS,   "npm",   "Node tools"),
        (GO_TOOLS,    "go",    "Go tools"),
    ]

    for tool_list, prereq, label in groups:
        console.print(Text(f"  Updating {label}...", style="cyan"))
        results = _update_group(tool_list, check_only, prereq=prereq)
        all_results.extend(results)

    # Print results table
    _print_results_table(all_results, check_only)


def _print_results_table(results: list[ToolUpdateResult], check_only: bool):
    action = "Status" if check_only else "Result"

    tbl = Table(
        title=f"Update {action}",
        box=box.SIMPLE_HEAVY,
        header_style="bold magenta",
        show_edge=True,
    )
    tbl.add_column("Tool",      style="bold white", width=14)
    tbl.add_column(action,      width=14)
    tbl.add_column("Note",      style="dim")

    STATUS_STYLE = {
        "updated":       ("updated",       "bold green"),
        "installed":     ("installed",     "green"),
        "failed":        ("failed",        "bold red"),
        "not_installed": ("not installed", "dim red"),
        "skipped":       ("skipped",       "dim yellow"),
        "check-only":    ("check only",    "cyan"),
    }

    updated_count = 0
    failed_count  = 0

    for r in results:
        label, style = STATUS_STYLE.get(r.status, (r.status, "white"))
        tbl.add_row(r.name, Text(label, style=style), r.note[:60])
        if r.status == "updated":
            updated_count += 1
        elif r.status == "failed":
            failed_count += 1

    console.print()
    console.print(tbl)
    console.print()

    if not check_only:
        console.print(f"  Updated: [bold green]{updated_count}[/bold green]   "
                      f"Failed: [bold red]{failed_count}[/bold red]")
        console.print()
        console.print("  Run [bold]torot --list-tools[/bold] to verify all tool statuses.")
    else:
        console.print("  Run [bold]torot --update[/bold] to apply updates.")
    console.print()
