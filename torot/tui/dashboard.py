"""
Torot Terminal TUI Dashboard.
Built with Textual — a rich, live dashboard that shows real-time progress
of all security tool scans, bug counts, and workflow status.
"""

from __future__ import annotations
import asyncio
import time
from typing import Callable

from rich.text import Text
from rich.table import Table
from rich.panel import Panel
from rich.progress import Progress, SpinnerColumn, BarColumn, TextColumn, TimeElapsedColumn
from rich.console import Console
from rich.layout import Layout
from rich.live import Live
from rich.columns import Columns
from rich.align import Align
from rich import box

from torot.core.models import ScanSession, ToolStatus, Severity, Bug, ToolResult


TOROT_BANNER = r"""
 ████████╗ ██████╗ ██████╗  ██████╗ ████████╗
    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝
    ██║   ██║   ██║██████╔╝██║   ██║   ██║   
    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   
    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   
"""

SEVERITY_COLORS = {
    "CRITICAL": "bold red",
    "HIGH": "red",
    "MEDIUM": "yellow",
    "LOW": "cyan",
    "INFO": "dim white",
}

STATUS_ICONS = {
    ToolStatus.PENDING: ("○", "dim white"),
    ToolStatus.CHECKING: ("⟳", "bold blue"),
    ToolStatus.NOT_INSTALLED: ("✗", "dim red"),
    ToolStatus.RUNNING: ("◉", "bold yellow"),
    ToolStatus.COMPLETED: ("✔", "bold green"),
    ToolStatus.FAILED: ("✘", "bold red"),
    ToolStatus.SKIPPED: ("⊘", "dim yellow"),
}


class TorotDashboard:
    """
    Live terminal dashboard powered by Rich's Live rendering.
    Displays: header, tool pipeline status, live bug feed, summary stats.
    """

    def __init__(self, session: ScanSession, tool_names: list[str]):
        self.session = session
        self.tool_names = tool_names
        self.tool_messages: dict[str, str] = {t: "Waiting…" for t in tool_names}
        self.console = Console()
        self._live: Live | None = None
        self._log_lines: list[str] = []
        self._start = time.time()

    # ------------------------------------------------------------------ #
    #  Lifecycle                                                           #
    # ------------------------------------------------------------------ #

    def start(self):
        self._live = Live(
            self._render(),
            console=self.console,
            refresh_per_second=4,
            screen=True,
        )
        self._live.start()

    def stop(self):
        if self._live:
            self._live.stop()

    def refresh(self):
        if self._live:
            self._live.update(self._render())

    def update_tool(self, tool_name: str, status: ToolStatus, message: str):
        self.tool_messages[tool_name] = message
        if tool_name not in self.session.tool_results:
            self.session.tool_results[tool_name] = ToolResult(
                tool_name=tool_name, status=status
            )
        self.session.tool_results[tool_name].status = status
        ts = time.strftime("%H:%M:%S")
        icon, _ = STATUS_ICONS.get(status, ("?", "white"))
        self._log(f"[dim]{ts}[/dim] {icon} [bold]{tool_name}[/bold] → {message}")
        self.refresh()

    def _log(self, line: str):
        self._log_lines.append(line)
        if len(self._log_lines) > 30:
            self._log_lines = self._log_lines[-30:]

    # ------------------------------------------------------------------ #
    #  Rendering                                                           #
    # ------------------------------------------------------------------ #

    def _render(self) -> Panel:
        elapsed = time.time() - self._start
        layout = Layout()
        layout.split_column(
            Layout(name="header", size=9),
            Layout(name="body"),
            Layout(name="footer", size=3),
        )
        layout["body"].split_row(
            Layout(name="pipeline", ratio=2),
            Layout(name="right", ratio=3),
        )
        layout["right"].split_column(
            Layout(name="stats", size=10),
            Layout(name="log"),
        )

        layout["header"].update(self._render_header(elapsed))
        layout["pipeline"].update(self._render_pipeline())
        layout["stats"].update(self._render_stats())
        layout["log"].update(self._render_log())
        layout["footer"].update(self._render_footer())

        return Panel(layout, border_style="bright_black", padding=0)

    def _render_header(self, elapsed: float) -> Panel:
        banner = Text(TOROT_BANNER, style="bold cyan", justify="center")
        subtitle = Text(
            f"  Blockchain & Smart Contract Bug Hunter  |  "
            f"Target: [bold white]{self.session.target_path}[/bold white]  |  "
            f"Elapsed: [yellow]{elapsed:.1f}s[/yellow]",
            justify="center",
        )
        combined = Text.assemble(banner, "\n", subtitle)
        return Panel(Align.center(combined), border_style="cyan", padding=(0, 1))

    def _render_pipeline(self) -> Panel:
        table = Table(
            show_header=True,
            header_style="bold magenta",
            box=box.SIMPLE_HEAVY,
            expand=True,
            show_edge=False,
        )
        table.add_column("Tool", style="bold white", width=14)
        table.add_column("Status", width=14)
        table.add_column("Bugs", justify="right", width=6)
        table.add_column("Message", style="dim")

        for tool_name in self.tool_names:
            result = self.session.tool_results.get(tool_name)
            if result:
                status = result.status
                bug_count = len(result.bugs)
            else:
                status = ToolStatus.PENDING
                bug_count = 0

            icon, color = STATUS_ICONS.get(status, ("?", "white"))
            status_text = Text(f"{icon} {status.value}", style=color)

            bugs_text = Text(str(bug_count) if bug_count else "—", style="yellow bold" if bug_count else "dim")
            msg = self.tool_messages.get(tool_name, "")

            table.add_row(
                Text(tool_name, style="bold white"),
                status_text,
                bugs_text,
                Text(msg[:45], style="dim"),
            )

        return Panel(table, title="[bold magenta]⚙  Tool Pipeline[/bold magenta]", border_style="magenta")

    def _render_stats(self) -> Panel:
        summary = self.session.bug_summary
        total = self.session.total_bugs

        rows = []
        for sev in Severity:
            count = summary.get(sev.value, 0)
            bar = "█" * min(count, 20)
            style = SEVERITY_COLORS[sev.value]
            rows.append(
                Text.assemble(
                    Text(f"  {sev.emoji} {sev.value:<9}", style=style),
                    Text(f" {bar:<20} ", style=style),
                    Text(f"{count:>4}", style="bold " + style.replace("bold ", "")),
                )
            )

        completed = sum(
            1 for r in self.session.tool_results.values()
            if r.status == ToolStatus.COMPLETED
        )
        running = sum(
            1 for r in self.session.tool_results.values()
            if r.status == ToolStatus.RUNNING
        )
        not_installed = sum(
            1 for r in self.session.tool_results.values()
            if r.status == ToolStatus.NOT_INSTALLED
        )

        progress_line = Text.assemble(
            Text(f"  Tools: ", style="dim"),
            Text(f"✔ {completed} done  ", style="green"),
            Text(f"◉ {running} running  ", style="yellow"),
            Text(f"✗ {not_installed} missing  ", style="dim red"),
            Text(f"│  Total Bugs: ", style="dim"),
            Text(f"{total}", style="bold red" if total > 0 else "bold green"),
        )

        content = Text("\n").join(rows) + Text("\n\n") + progress_line
        return Panel(content, title="[bold red]🐛  Bug Summary[/bold red]", border_style="red")

    def _render_log(self) -> Panel:
        lines = self._log_lines[-15:] if self._log_lines else [Text("Waiting for tools to start…", style="dim")]
        content = "\n".join(str(l) for l in lines)
        from rich.markup import render
        try:
            rendered = Text.from_markup(content)
        except Exception:
            rendered = Text(content)
        return Panel(rendered, title="[bold blue]📋  Live Log[/bold blue]", border_style="blue")

    def _render_footer(self) -> Panel:
        langs = ", ".join(self.session.detected_languages) if self.session.detected_languages else "detecting…"
        files = len(self.session.detected_files)
        footer = Text.assemble(
            Text("  Languages: ", style="dim"),
            Text(langs, style="bold cyan"),
            Text("   Files Scanned: ", style="dim"),
            Text(str(files), style="bold white"),
            Text("   Press ", style="dim"),
            Text("Ctrl+C", style="bold yellow"),
            Text(" to stop scan gracefully", style="dim"),
        )
        return Panel(footer, border_style="bright_black")
