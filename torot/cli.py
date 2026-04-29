#!/usr/bin/env python3
"""
Torot — Blockchain & Smart Contract Bug Hunter
CLI Entry Point

Usage:
    torot <path>                     Scan a folder
    torot <path> --report out.md     Save report to custom path
    torot <path> --no-dashboard      Skip TUI, just print results
    torot --version                  Show version
"""

from __future__ import annotations
import asyncio
import sys
import os
import argparse
import time
import signal

from rich.console import Console
from rich.panel import Panel
from rich.text import Text

console = Console()


def print_banner():
    banner = Text()
    banner.append("\n")
    banner.append(" ████████╗ ██████╗ ██████╗  ██████╗ ████████╗\n", style="bold cyan")
    banner.append("    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝\n", style="bold cyan")
    banner.append("    ██║   ██║   ██║██████╔╝██║   ██║   ██║   \n", style="bold cyan")
    banner.append("    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   \n", style="bold cyan")
    banner.append("    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   \n", style="bold cyan")
    banner.append("    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   \n", style="bold cyan")
    banner.append("\n")
    banner.append("  Blockchain & Smart Contract Bug Hunter  ", style="bold white on dark_blue")
    banner.append("  v1.0.0\n", style="dim")
    console.print(Panel(banner, border_style="cyan", padding=(0, 2)))


def parse_args():
    parser = argparse.ArgumentParser(
        prog="torot",
        description="Torot — Blockchain & Smart Contract Bug Hunter",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  torot ./my-contracts/
  torot ./my-contracts/ --report security_audit.md
  torot ./my-contracts/ --no-dashboard
  torot ./my-contracts/ --concurrent 3
        """,
    )
    parser.add_argument(
        "path",
        nargs="?",
        help="Path to the code folder to analyze",
    )
    parser.add_argument(
        "--report", "-r",
        metavar="FILE",
        help="Output path for the Markdown report (default: torot_report_<timestamp>.md)",
        default=None,
    )
    parser.add_argument(
        "--no-dashboard",
        action="store_true",
        help="Disable the live TUI dashboard (plain output only)",
    )
    parser.add_argument(
        "--concurrent", "-c",
        type=int,
        default=4,
        help="Max concurrent tools (default: 4)",
    )
    parser.add_argument(
        "--version", "-v",
        action="store_true",
        help="Show Torot version and exit",
    )
    return parser.parse_args()


async def run_scan(path: str, report_path: str | None, no_dashboard: bool, concurrent: int):
    from torot.core.engine import ScanEngine
    from torot.tui.dashboard import TorotDashboard
    from torot.report.generator import generate_report
    from torot.core.models import ToolStatus

    engine = ScanEngine(target_path=path, max_concurrent=concurrent)
    tool_names = engine.all_tool_names

    dashboard: TorotDashboard | None = None

    def on_status_change(tool_name, status, message):
        if dashboard:
            dashboard.update_tool(tool_name, status, message)
        elif not no_dashboard:
            pass
        else:
            icon = status.icon
            color_map = {
                ToolStatus.RUNNING: "yellow",
                ToolStatus.COMPLETED: "green",
                ToolStatus.FAILED: "red",
                ToolStatus.NOT_INSTALLED: "dim",
            }
            color = color_map.get(status, "white")
            console.print(f"  [{color}]{icon} {tool_name}[/{color}] → {message}")

    engine.on_status_change = on_status_change

    if not no_dashboard:
        dashboard = TorotDashboard(session=engine.session, tool_names=tool_names)
        dashboard.start()

    try:
        session = await engine.run()
    except KeyboardInterrupt:
        console.print("\n[yellow]Scan interrupted by user.[/yellow]")
        session = engine.session
    finally:
        if dashboard:
            dashboard.stop()

    # Print final summary
    console.print()
    _print_final_summary(session)

    # Generate report
    report_file = generate_report(session, report_path)
    console.print(f"\n[bold green]✔ Report saved to:[/bold green] [underline]{report_file}[/underline]\n")

    return session


def _print_final_summary(session):
    from torot.core.models import Severity, ToolStatus

    summary = session.bug_summary
    total = session.total_bugs

    lines = []
    for sev in Severity:
        count = summary.get(sev.value, 0)
        if count > 0:
            lines.append(f"  {sev.emoji} {sev.value:<10} {count}")

    installed = sum(1 for r in session.tool_results.values() if r.status != ToolStatus.NOT_INSTALLED)
    not_installed = sum(1 for r in session.tool_results.values() if r.status == ToolStatus.NOT_INSTALLED)

    status_line = (
        f"[bold white]Scan Complete[/bold white] — "
        f"[cyan]{installed} tools ran[/cyan], "
        f"[dim]{not_installed} not installed[/dim], "
        f"[bold red]{total} issues found[/bold red]"
    )

    content = status_line + "\n\n" + "\n".join(lines) if lines else status_line
    console.print(Panel(content, title="[bold]Torot Results[/bold]", border_style="cyan"))


def main():
    args = parse_args()

    if args.version:
        console.print("[bold cyan]Torot[/bold cyan] v1.0.0 — Blockchain & Smart Contract Bug Hunter")
        sys.exit(0)

    if not args.path:
        print_banner()
        console.print("[red]Error:[/red] Please provide a path to scan.\n")
        console.print("Usage: [bold]torot <path>[/bold]")
        console.print("       [bold]torot --help[/bold] for more options")
        sys.exit(1)

    path = os.path.abspath(args.path)
    if not os.path.exists(path):
        console.print(f"[red]Error:[/red] Path does not exist: {path}")
        sys.exit(1)

    if args.no_dashboard:
        print_banner()
        console.print(f"[bold]Scanning:[/bold] {path}\n")

    try:
        asyncio.run(run_scan(
            path=path,
            report_path=args.report,
            no_dashboard=args.no_dashboard,
            concurrent=args.concurrent,
        ))
    except FileNotFoundError as e:
        console.print(f"[red]Error:[/red] {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        console.print("\n[yellow]Torot stopped.[/yellow]")
        sys.exit(0)


if __name__ == "__main__":
    main()
