#!/usr/bin/env python3
"""
Torot — Blockchain & Smart Contract Security Scanner
CLI entry point.

Usage:
  torot <path>
  torot <path> --report out.md
  torot <path> --api openai=sk-... --api etherscan=ABC
  torot <path> --api anthropic=sk-ant-... --api github=TOKEN --api github-repo=owner/repo
  torot <path> --no-dashboard
  torot <path> --concurrent 3
  torot --list-tools
  torot --version
"""

from __future__ import annotations
import asyncio
import sys
import os
import argparse
import time
import shutil

from rich.console import Console
from rich.panel   import Panel
from rich.table   import Table
from rich.text    import Text
from rich         import box

console = Console()

BANNER = (
    " ████████╗ ██████╗ ██████╗  ██████╗ ████████╗\n"
    "    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝\n"
    "    ██║   ██║   ██║██████╔╝██║   ██║   ██║   \n"
    "    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   \n"
    "    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   \n"
    "    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   \n"
)


def print_banner():
    console.print(Text(BANNER, style="bold cyan", justify="center"))
    console.print(Text(
        "  Blockchain & Smart Contract Security Scanner  |  v1.0.0\n",
        style="dim", justify="center"
    ))


def list_tools():
    """Print a table of all known tools and their install status."""
    from torot.scanners.all_scanners import ALL_SCANNERS
    print_banner()
    tbl = Table(
        title="Torot — Supported Security Tools",
        box=box.SIMPLE_HEAVY,
        show_edge=True,
        header_style="bold magenta",
    )
    tbl.add_column("Tool",         style="bold white", width=14)
    tbl.add_column("Language",     width=12)
    tbl.add_column("Installed",    width=10)
    tbl.add_column("Description")

    for cls in ALL_SCANNERS:
        installed = any(shutil.which(b) for b in cls.binary_names)
        status    = Text("yes", style="bold green") if installed else Text("no", style="dim red")
        langs     = ", ".join(cls.supported_languages)
        tbl.add_row(cls.tool_name, langs, status, cls.description)

    console.print(tbl)
    console.print()

    inst_count = sum(
        1 for cls in ALL_SCANNERS
        if any(shutil.which(b) for b in cls.binary_names)
    )
    console.print(
        f"  [bold]{inst_count}[/bold] of [bold]{len(ALL_SCANNERS)}[/bold] tools installed. "
        f"Torot works with any subset — even 1 tool.\n"
    )


def parse_args():
    parser = argparse.ArgumentParser(
        prog="torot",
        description="Torot — Blockchain & Smart Contract Security Scanner",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
API keys (--api):
  openai=<key>        GPT-4 powered bug analysis
  anthropic=<key>     Claude-powered fix suggestions
  etherscan=<key>     On-chain contract verification
  github=<token>      Auto-open GitHub issues
  github-repo=owner/repo   Target repo for issues

Examples:
  torot ./contracts/
  torot ./contracts/ --report audit.md
  torot ./contracts/ --api anthropic=sk-ant-... --api github=ghp_...
  torot ./contracts/ --no-dashboard --concurrent 3
  torot --list-tools
        """,
    )
    parser.add_argument("path",         nargs="?",      help="Code folder to scan")
    parser.add_argument("--report","-r",metavar="FILE", help="Markdown report output path")
    parser.add_argument("--api",        action="append",metavar="KEY=VALUE",
                        help="API key (repeatable). E.g. --api openai=sk-...")
    parser.add_argument("--no-dashboard", action="store_true",
                        help="Disable live TUI; use plain output")
    parser.add_argument("--concurrent","-c", type=int, default=5,
                        help="Max tools running in parallel (default: 5)")
    parser.add_argument("--list-tools", action="store_true",
                        help="List all supported tools and install status")
    parser.add_argument("--version","-v", action="store_true",
                        help="Show version and exit")
    return parser.parse_args()


def parse_api_config(api_args: list[str] | None):
    from torot.core.models import ApiConfig
    cfg = ApiConfig()
    if not api_args:
        return cfg
    for entry in api_args:
        if "=" not in entry:
            console.print(f"[yellow]Warning:[/yellow] Ignoring malformed --api entry: {entry!r}")
            continue
        key, _, value = entry.partition("=")
        key = key.strip().lower()
        value = value.strip()
        if   key == "openai":       cfg.openai_key    = value
        elif key == "anthropic":    cfg.anthropic_key = value
        elif key == "etherscan":    cfg.etherscan_key = value
        elif key == "github":       cfg.github_token  = value
        elif key == "github-repo":  cfg.github_repo   = value
        else:
            cfg.custom_apis[key] = value
            console.print(f"  [dim]Custom API registered:[/dim] {key}")
    return cfg


# ─────────────────────────────────────────────────────────────────────────────
# Main scan runner
# ─────────────────────────────────────────────────────────────────────────────

async def run_scan(
    path:         str,
    report_path:  str | None,
    no_dashboard: bool,
    concurrent:   int,
    api_config,
):
    from torot.core.engine    import ScanEngine
    from torot.tui.dashboard  import TorotDashboard
    from torot.report.generator import generate_report
    from torot.core.models    import ToolStatus

    engine     = ScanEngine(
        target_path=path,
        max_concurrent=concurrent,
        api_config=api_config,
    )
    tool_names = engine.all_tool_names
    dashboard: TorotDashboard | None = None

    def on_update(tool_name, status, message):
        if dashboard:
            dashboard.update_tool(tool_name, status, message)
        else:
            icon   = status.icon
            color  = status.color
            ts     = time.strftime("%H:%M:%S")
            console.print(f"  [dim]{ts}[/dim] [{color}]{icon}[/{color}] {tool_name} — {message}")

    engine.on_status_change = on_update

    if not no_dashboard:
        dashboard = TorotDashboard(session=engine.session, tool_names=tool_names)
        dashboard.start()

    try:
        session = await engine.run()
    except KeyboardInterrupt:
        if dashboard:
            dashboard.stop()
        console.print("\n[yellow]Scan interrupted.[/yellow]")
        session = engine.session
    else:
        if dashboard:
            dashboard.stop()

    _print_summary(session)

    report_file = generate_report(session, report_path)
    console.print(f"\n  Report saved: [underline]{report_file}[/underline]\n")

    return session


def _print_summary(session):
    from torot.core.models import Severity, ToolStatus

    total   = session.total_bugs
    summary = session.bug_summary
    ran     = session.tools_ran
    missing = sum(1 for r in session.tool_results.values() if r.status == ToolStatus.NOT_INSTALLED)

    lines = [
        f"  Scan complete — [bold]{ran}[/bold] tool(s) ran, "
        f"[dim]{missing}[/dim] not installed, "
        f"[bold red]{total}[/bold red] finding(s) total",
        "",
    ]
    for sev in Severity:
        count = summary.get(sev.value, 0)
        if count:
            bar = "#" * min(count, 24)
            lines.append(f"  [{sev.color}]{sev.value:<10}[/{sev.color}]  {bar}  {count}")

    console.print(Panel("\n".join(lines), title="Results", border_style="cyan"))


# ─────────────────────────────────────────────────────────────────────────────
# Entry point
# ─────────────────────────────────────────────────────────────────────────────

def main():
    args = parse_args()

    if args.version:
        console.print("[bold cyan]Torot[/bold cyan] v1.0.0 — Blockchain & Smart Contract Security Scanner")
        sys.exit(0)

    if args.list_tools:
        list_tools()
        sys.exit(0)

    if not args.path:
        print_banner()
        console.print("[red]Error:[/red] Provide a path to scan.\n")
        console.print("  Usage: [bold]torot <path>[/bold]")
        console.print("         [bold]torot --list-tools[/bold]  to see available tools")
        sys.exit(1)

    path = os.path.abspath(args.path)
    if not os.path.exists(path):
        console.print(f"[red]Error:[/red] Path does not exist: {path}")
        sys.exit(1)

    api_config = parse_api_config(args.api)

    # Show which APIs are active
    if args.no_dashboard:
        print_banner()
        console.print(f"  Target: [bold]{path}[/bold]")
        if api_config.has_ai():
            provider = "Anthropic Claude" if api_config.anthropic_key else "OpenAI GPT-4"
            console.print(f"  AI enrichment: [green]{provider}[/green]")
        if api_config.has_etherscan():
            console.print("  Etherscan verification: [green]enabled[/green]")
        if api_config.has_github():
            console.print(f"  GitHub issues: [green]{api_config.github_repo}[/green]")
        console.print()

    try:
        asyncio.run(run_scan(
            path=path,
            report_path=args.report,
            no_dashboard=args.no_dashboard,
            concurrent=args.concurrent,
            api_config=api_config,
        ))
    except FileNotFoundError as e:
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        console.print("\n[yellow]Torot stopped.[/yellow]")
        sys.exit(0)


if __name__ == "__main__":
    main()
