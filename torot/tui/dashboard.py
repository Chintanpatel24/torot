"""
Torot Terminal Dashboard.
Classic, clean Rich Live TUI — no emojis in UI chrome.
"""

from __future__ import annotations
import time
from typing import Callable

from rich.text   import Text
from rich.table  import Table
from rich.panel  import Panel
from rich.live   import Live
from rich.layout import Layout
from rich.align  import Align
from rich.console import Console
from rich        import box

from torot.core.models import ScanSession, ToolStatus, Severity, ToolResult


BANNER = (
    " ████████╗ ██████╗ ██████╗  ██████╗ ████████╗\n"
    "    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝\n"
    "    ██║   ██║   ██║██████╔╝██║   ██║   ██║   \n"
    "    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   \n"
    "    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   \n"
    "    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   \n"
)

SEV_COLORS = {
    "CRITICAL": "bold red",
    "HIGH":     "red",
    "MEDIUM":   "yellow",
    "LOW":      "cyan",
    "INFO":     "dim white",
}

STATUS_STYLE = {
    ToolStatus.PENDING:       ("o", "dim"),
    ToolStatus.CHECKING:      ("~", "blue"),
    ToolStatus.NOT_INSTALLED: ("x", "dim red"),
    ToolStatus.RUNNING:       ("*", "bold yellow"),
    ToolStatus.COMPLETED:     ("+", "bold green"),
    ToolStatus.FAILED:        ("!", "bold red"),
    ToolStatus.SKIPPED:       ("-", "dim yellow"),
}


class TorotDashboard:
    """
    Live Rich TUI dashboard. Four panels:
      Top-left   — tool pipeline status table
      Top-right  — bug severity counters + scan stats
      Bottom     — scrolling activity log
      Footer     — target info + controls
    """

    def __init__(self, session: ScanSession, tool_names: list[str]):
        self.session      = session
        self.tool_names   = tool_names
        self.tool_msgs:   dict[str, str] = {t: "waiting" for t in tool_names}
        self.console      = Console()
        self._live:       Live | None = None
        self._log:        list[str] = []
        self._t0          = time.time()

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

    def update_tool(self, tool_name: str, status: ToolStatus, message: str):
        self.tool_msgs[tool_name] = message
        if tool_name not in self.session.tool_results:
            self.session.tool_results[tool_name] = ToolResult(
                tool_name=tool_name, status=status
            )
        self.session.tool_results[tool_name].status = status
        ts    = time.strftime("%H:%M:%S")
        icon, _ = STATUS_STYLE.get(status, ("?", "white"))
        self._append_log(f"[dim]{ts}[/dim] [{status.color}]{icon}[/{status.color}] {tool_name} — {message}")
        if self._live:
            self._live.update(self._render())

    def _append_log(self, line: str):
        self._log.append(line)
        if len(self._log) > 40:
            self._log = self._log[-40:]

    # ------------------------------------------------------------------ #
    #  Rendering                                                           #
    # ------------------------------------------------------------------ #

    def _render(self) -> Panel:
        elapsed = time.time() - self._t0
        layout  = Layout()
        layout.split_column(
            Layout(name="banner", size=8),
            Layout(name="body"),
            Layout(name="footer", size=3),
        )
        layout["body"].split_row(
            Layout(name="pipeline", ratio=2),
            Layout(name="right",    ratio=3),
        )
        layout["right"].split_column(
            Layout(name="stats", size=12),
            Layout(name="log"),
        )

        layout["banner"].update(self._render_banner(elapsed))
        layout["pipeline"].update(self._render_pipeline())
        layout["stats"].update(self._render_stats())
        layout["log"].update(self._render_log())
        layout["footer"].update(self._render_footer())

        return Panel(layout, border_style="bright_black", padding=0)

    # ── Banner ─────────────────────────────────────────────────────────

    def _render_banner(self, elapsed: float) -> Panel:
        t = Text(BANNER, style="bold cyan", justify="center")
        sub = Text(
            f"  Blockchain & Smart Contract Security Scanner  |  "
            f"Target: [bold white]{self.session.target_path}[/bold white]  |  "
            f"Elapsed: [yellow]{elapsed:.1f}s[/yellow]",
            justify="center",
        )
        combined = Text.assemble(t, sub)
        return Panel(Align.center(combined), border_style="cyan", padding=(0, 1))

    # ── Pipeline ───────────────────────────────────────────────────────

    def _render_pipeline(self) -> Panel:
        tbl = Table(
            show_header=True,
            header_style="bold magenta",
            box=box.SIMPLE_HEAVY,
            expand=True,
            show_edge=False,
        )
        tbl.add_column("Tool",    style="bold white", width=14)
        tbl.add_column("Status",  width=14)
        tbl.add_column("Bugs",    justify="right", width=6)
        tbl.add_column("Message", style="dim")

        for name in self.tool_names:
            result = self.session.tool_results.get(name)
            status = result.status if result else ToolStatus.PENDING
            count  = len(result.bugs) if result else 0

            icon, style = STATUS_STYLE.get(status, ("?", "white"))
            tbl.add_row(
                Text(name, style="bold white"),
                Text(f"{icon} {status.value}", style=style),
                Text(str(count) if count else "—", style="yellow bold" if count else "dim"),
                Text(self.tool_msgs.get(name, "")[:44], style="dim"),
            )

        completed    = sum(1 for r in self.session.tool_results.values() if r.status == ToolStatus.COMPLETED)
        not_inst     = sum(1 for r in self.session.tool_results.values() if r.status == ToolStatus.NOT_INSTALLED)
        running      = sum(1 for r in self.session.tool_results.values() if r.status == ToolStatus.RUNNING)

        footer_line = Text.assemble(
            Text(f"  {completed} done  ", style="green"),
            Text(f"{running} running  ", style="yellow"),
            Text(f"{not_inst} not installed", style="dim red"),
        )

        content = Text.assemble(Text.from_markup(str(tbl)), "\n", footer_line)
        return Panel(tbl, title="[bold magenta]Tool Pipeline[/bold magenta]", border_style="magenta")

    # ── Stats ──────────────────────────────────────────────────────────

    def _render_stats(self) -> Panel:
        summary = self.session.bug_summary
        total   = self.session.total_bugs
        rows    = []

        for sev in Severity:
            count = summary.get(sev.value, 0)
            bar   = "#" * min(count, 18)
            style = SEV_COLORS[sev.value]
            rows.append(Text.assemble(
                Text(f"  {sev.value:<10}", style=style),
                Text(f" {bar:<18} ", style=style),
                Text(f"{count:>4}", style="bold " + style.replace("bold ", "")),
            ))

        rows.append(Text(""))
        rows.append(Text.assemble(
            Text("  Total findings: ", style="dim"),
            Text(str(total), style="bold red" if total else "bold green"),
            Text("   |   Files: ", style="dim"),
            Text(str(len(self.session.detected_files)), style="white"),
        ))

        combined = Text("\n").join(rows)
        return Panel(combined, title="[bold red]Bug Summary[/bold red]", border_style="red")

    # ── Log ────────────────────────────────────────────────────────────

    def _render_log(self) -> Panel:
        lines   = self._log[-16:] if self._log else ["[dim]Waiting for tools...[/dim]"]
        content = "\n".join(lines)
        try:
            rendered = Text.from_markup(content)
        except Exception:
            rendered = Text(content)
        return Panel(rendered, title="[bold blue]Activity Log[/bold blue]", border_style="blue")

    # ── Footer ─────────────────────────────────────────────────────────

    def _render_footer(self) -> Panel:
        langs = ", ".join(self.session.detected_languages) if self.session.detected_languages else "detecting..."
        footer = Text.assemble(
            Text("  Languages: ", style="dim"),
            Text(langs, style="bold cyan"),
            Text("    Press ", style="dim"),
            Text("Ctrl+C", style="bold yellow"),
            Text(" to stop gracefully", style="dim"),
        )
        return Panel(footer, border_style="bright_black")
