"""
Top half: live tool output stream (scrollable)
Bottom half: chat input + agent responses

Built with Textual — a modern Python TUI framework.
"""
from __future__ import annotations
import asyncio
import time
from typing import Optional, Callable

from textual.app         import App, ComposeResult
from textual.widgets     import (
    Header, Footer, Static, Input, RichLog,
    Label, ProgressBar, Button, SelectionList
)
from textual.containers  import Vertical, Horizontal, ScrollableContainer
from textual.binding     import Binding
from textual             import on, work
from textual.reactive    import reactive
from rich.text           import Text
from rich.panel          import Panel
from rich.table          import Table
from rich                import box

from torot.core.models   import (
    Session, Finding, ChatMessage, AgentPlan,
    PlanStep, StepStatus, Severity, AIProvider
)


# ─────────────────────────────────────────────────────────────────────────────
# Severity colors for Rich
# ─────────────────────────────────────────────────────────────────────────────
SEV_STYLE = {
    "CRITICAL": "bold red",
    "HIGH":     "red",
    "MEDIUM":   "yellow",
    "LOW":      "cyan",
    "INFO":     "dim white",
}

STATUS_STYLE = {
    "pending":          ("o", "dim"),
    "running":          ("*", "bold yellow"),
    "done":             ("+", "bold green"),
    "skipped":          ("-", "dim"),
    "failed":           ("!", "bold red"),
    "waiting_approval": ("?", "bold magenta"),
}


# ─────────────────────────────────────────────────────────────────────────────
# Custom Widgets
# ─────────────────────────────────────────────────────────────────────────────

class StreamPane(RichLog):
    """Top pane — live streaming tool output."""

    DEFAULT_CSS = """
    StreamPane {
        height: 1fr;
        border: solid $primary-darken-3;
        border-title-color: $primary;
        background: $surface;
        scrollbar-gutter: stable;
    }
    """

    def on_mount(self):
        self.border_title = "Live Output"
        self.auto_scroll  = True
        self.markup       = True
        self.highlight    = True

    def stream_line(self, line: str, style: str = ""):
        ts = time.strftime("%H:%M:%S")
        if style:
            self.write(Text.assemble(
                Text(f" {ts} ", style="dim"),
                Text(line, style=style),
            ))
        else:
            self.write(Text.assemble(
                Text(f" {ts} ", style="dim"),
                Text(line),
            ))

    def stream_separator(self, label: str = ""):
        self.write(Text(f" {'─' * 60} {label}", style="dim"))

    def stream_finding(self, f: Finding):
        sev_style = SEV_STYLE.get(f.severity.value, "white")
        self.write(Text.assemble(
            Text(f"  [{f.severity.value}]", style=sev_style + " bold"),
            Text(f" {f.title}", style="bold white"),
            Text(f"  {f.location}", style="dim"),
        ))

    def stream_agent(self, msg: str):
        for line in msg.splitlines():
            self.write(Text.assemble(
                Text(" agent ", style="bold green on dark_green"),
                Text(f" {line}", style="white"),
            ))

    def stream_system(self, msg: str):
        self.write(Text(f"  {msg}", style="bold cyan"))


class ChatPane(Static):
    """Bottom pane — chat history display."""

    DEFAULT_CSS = """
    ChatPane {
        height: auto;
        min-height: 6;
        max-height: 12;
        background: $surface-darken-1;
        border: solid $primary-darken-3;
        border-title-color: $accent;
        overflow-y: auto;
        padding: 0 1;
    }
    """

    messages: reactive[list[dict]] = reactive([], recompose=True)

    def on_mount(self):
        self.border_title = "Conversation"

    def compose(self) -> ComposeResult:
        for msg in self.messages[-6:]:
            role    = msg.get("role", "user")
            content = msg.get("content", "")
            if role == "user":
                yield Label(Text.assemble(
                    Text(" you  ", style="bold blue on dark_blue"),
                    Text(f" {content}", style="white"),
                ))
            elif role == "agent":
                yield Label(Text.assemble(
                    Text(" torot", style="bold green on dark_green"),
                    Text(f" {content[:180]}{'...' if len(content)>180 else ''}", style="white"),
                ))
            elif role == "system":
                yield Label(Text(f"  {content}", style="dim italic"))

    def add_message(self, role: str, content: str):
        self.messages = self.messages + [{"role": role, "content": content}]


class PlanWidget(Static):
    """Displays the agent plan with step statuses."""

    DEFAULT_CSS = """
    PlanWidget {
        height: auto;
        border: solid $warning-darken-2;
        border-title-color: $warning;
        background: $surface;
        padding: 0 1;
    }
    """

    def __init__(self, plan: AgentPlan, **kwargs):
        super().__init__(**kwargs)
        self._plan = plan

    def on_mount(self):
        self.border_title = "Agent Plan"
        self._render_plan()

    def _render_plan(self):
        lines = [Text(f"  Goal: {self._plan.goal}", style="bold white"), Text("")]
        for i, step in enumerate(self._plan.steps, 1):
            icon, style = STATUS_STYLE.get(step.status.value, ("?", "white"))
            lines.append(Text.assemble(
                Text(f"  {i:2}. [{icon}] ", style=style),
                Text(step.title, style="bold white" if step.status == StepStatus.RUNNING else "white"),
                Text(f"  ({step.tool})", style="dim"),
            ))
        self.update(Text("\n").join(lines))

    def update_plan(self, plan: AgentPlan):
        self._plan = plan
        self._render_plan()


class StatsBar(Static):
    """Compact status bar showing finding counts."""

    DEFAULT_CSS = """
    StatsBar {
        height: 1;
        background: $primary-darken-3;
        color: $text;
        padding: 0 1;
    }
    """

    def render_stats(self, session: Session):
        s = session.finding_summary
        parts = [
            Text(" TOROT ", style="bold white on dark_blue"),
            Text(f"  target: {session.target[:40]}", style="dim"),
            Text("   findings: ", style="dim"),
            Text(f"C:{s.get('CRITICAL',0)} ", style="bold red"),
            Text(f"H:{s.get('HIGH',0)} ", style="red"),
            Text(f"M:{s.get('MEDIUM',0)} ", style="yellow"),
            Text(f"L:{s.get('LOW',0)} ", style="cyan"),
            Text(f"  total:{sum(s.values())}", style="bold white"),
        ]
        if session.ai_config:
            provider = session.ai_config.provider.value
            parts += [Text(f"  ai:{provider}", style="dim green")]
        self.update(Text.assemble(*parts))


# ─────────────────────────────────────────────────────────────────────────────
# Main TUI App
# ─────────────────────────────────────────────────────────────────────────────

class TorotApp(App):
    """
    Torot v2 — Universal Security Agent
    Warp-style split terminal UI:
      - Top: live streaming tool output
      - Middle: agent plan (when active)
      - Bottom: chat input
    """

    CSS = """
    Screen {
        layers: base overlay;
        background: $background;
    }

    #root-layout {
        height: 100%;
        layout: vertical;
    }

    #stats-bar {
        height: 1;
        dock: top;
    }

    #stream-pane {
        height: 1fr;
        min-height: 10;
    }

    #plan-section {
        height: auto;
        max-height: 16;
    }

    #chat-history {
        height: auto;
        max-height: 10;
    }

    #input-row {
        height: 3;
        layout: horizontal;
        background: $surface-darken-2;
        border: solid $primary-darken-3;
        border-title-color: $accent;
        padding: 0 1;
    }

    #user-input {
        width: 1fr;
        border: none;
        background: transparent;
    }

    #send-btn {
        width: 10;
        margin: 0 0 0 1;
        background: $primary-darken-2;
    }

    .approve-bar {
        height: 3;
        layout: horizontal;
        background: $warning-darken-3;
        padding: 0 1;
        border: solid $warning;
    }

    .approve-btn {
        width: 16;
        margin: 0 1 0 0;
        background: $success-darken-1;
    }

    .skip-btn {
        width: 12;
        background: $error-darken-1;
    }
    """

    BINDINGS = [
        Binding("ctrl+c",  "quit",       "Quit",         priority=True),
        Binding("ctrl+l",  "clear_log",  "Clear log"),
        Binding("ctrl+e",  "export",     "Export report"),
        Binding("ctrl+p",  "show_plan",  "Show plan"),
        Binding("escape",  "cancel",     "Cancel step"),
    ]

    def __init__(
        self,
        session:        Session,
        on_user_input:  Optional[Callable[[str], None]] = None,
        on_approve:     Optional[Callable[[bool], None]] = None,
        **kwargs,
    ):
        super().__init__(**kwargs)
        self.session       = session
        self.on_user_input = on_user_input
        self.on_approve    = on_approve
        self._plan_widget: Optional[PlanWidget] = None
        self._stream:      Optional[StreamPane] = None
        self._chat:        Optional[ChatPane]   = None
        self._stats:       Optional[StatsBar]   = None
        self._approval_pending = False

    # ── Layout ─────────────────────────────────────────────────────────────

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True, name="Torot")

        with Vertical(id="root-layout"):
            stats = StatsBar(id="stats-bar")
            yield stats

            stream = StreamPane(id="stream-pane")
            yield stream

            chat = ChatPane(id="chat-history")
            yield chat

            with Horizontal(id="input-row"):
                yield Input(
                    placeholder="Ask Torot anything, or type a target path / contract address...",
                    id="user-input",
                )
                yield Button("Send", id="send-btn", variant="primary")

        yield Footer()

    def on_mount(self):
        self._stream = self.query_one("#stream-pane", StreamPane)
        self._chat   = self.query_one("#chat-history", ChatPane)
        self._stats  = self.query_one("#stats-bar", StatsBar)
        inp          = self.query_one("#user-input", Input)
        inp.focus()

        self._stream.stream_system("Torot v2 — Universal Security Agent")
        self._stream.stream_system(f"Session: {self.session.id}")
        self._stream.stream_system(
            f"AI: {self.session.ai_config.provider.value if self.session.ai_config else 'offline'}"
        )
        self._stream.stream_separator()
        self._stats.render_stats(self.session)

    # ── Input handling ─────────────────────────────────────────────────────

    @on(Input.Submitted, "#user-input")
    def handle_input(self, event: Input.Submitted):
        text = event.value.strip()
        if not text:
            return
        event.input.clear()
        self._handle_user_message(text)

    @on(Button.Pressed, "#send-btn")
    def handle_send(self):
        inp  = self.query_one("#user-input", Input)
        text = inp.value.strip()
        if not text:
            return
        inp.clear()
        self._handle_user_message(text)

    def _handle_user_message(self, text: str):
        self._chat.add_message("user", text)
        self._stream.stream_line(f"you: {text}", style="bold blue")
        if self.on_user_input:
            self.on_user_input(text)

    # ── Approval bar ───────────────────────────────────────────────────────

    def show_approval_bar(self, step: PlanStep):
        """Show approve/skip buttons for a plan step."""
        self._approval_pending = True
        bar = Horizontal(classes="approve-bar")

        async def mount_bar():
            await self.mount(bar, before="#input-row")
            await bar.mount(
                Label(
                    Text(f"  Step: {step.title}  |  Tool: {step.tool}  |  Approve?",
                         style="bold white"),
                )
            )
            await bar.mount(Button("Approve [Enter]", id="approve-btn", classes="approve-btn"))
            await bar.mount(Button("Skip", id="skip-btn", classes="skip-btn", variant="error"))

        self.call_after_refresh(mount_bar)

    def hide_approval_bar(self):
        self._approval_pending = False
        try:
            bars = self.query(".approve-bar")
            for bar in bars:
                bar.remove()
        except Exception:
            pass

    @on(Button.Pressed, "#approve-btn")
    def handle_approve(self):
        self.hide_approval_bar()
        if self.on_approve:
            self.on_approve(True)

    @on(Button.Pressed, "#skip-btn")
    def handle_skip(self):
        self.hide_approval_bar()
        if self.on_approve:
            self.on_approve(False)

    # ── Public API (called by controller) ─────────────────────────────────

    def write_line(self, line: str, style: str = ""):
        if self._stream:
            self._stream.stream_line(line, style)

    def write_agent(self, msg: str):
        if self._stream:
            self._stream.stream_agent(msg)
        if self._chat:
            self._chat.add_message("agent", msg)

    def write_system(self, msg: str):
        if self._stream:
            self._stream.stream_system(msg)
        if self._chat:
            self._chat.add_message("system", msg)

    def write_separator(self, label: str = ""):
        if self._stream:
            self._stream.stream_separator(label)

    def write_finding(self, f: Finding):
        if self._stream:
            self._stream.stream_finding(f)

    def show_plan(self, plan: AgentPlan):
        async def _mount():
            try:
                old = self.query_one("#plan-section")
                old.remove()
            except Exception:
                pass
            pw = PlanWidget(plan, id="plan-section")
            await self.mount(pw, before="#chat-history")
            self._plan_widget = pw
        self.call_after_refresh(_mount)

    def update_plan(self, plan: AgentPlan):
        if self._plan_widget:
            self._plan_widget.update_plan(plan)

    def refresh_stats(self):
        if self._stats:
            self._stats.render_stats(self.session)

    # ── Actions ────────────────────────────────────────────────────────────

    def action_clear_log(self):
        if self._stream:
            self._stream.clear()

    def action_export(self):
        self.write_system("Exporting report... (use Ctrl+E)")

    def action_show_plan(self):
        if self.session.plan and self._plan_widget:
            self._plan_widget.update_plan(self.session.plan)

    def action_cancel(self):
        if self._approval_pending:
            self.hide_approval_bar()
            if self.on_approve:
                self.on_approve(False)
