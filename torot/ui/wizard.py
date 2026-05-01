"""
Torot Startup Wizard
Shown on first launch or when no arguments provided.
Handles: AI provider selection, API key entry, Ollama model selection.
"""
from __future__ import annotations
import shutil
import urllib.request
import json
from typing import Optional

from rich.console import Console
from rich.panel   import Panel
from rich.table   import Table
from rich.text    import Text
from rich.prompt  import Prompt, Confirm
from rich         import box

from torot.core.models import AIConfig, AIProvider

console = Console()

BANNER = """\
 ████████╗ ██████╗ ██████╗  ██████╗ ████████╗
    ██╔══╝██╔═══██╗██╔══██╗██╔═══██╗╚══██╔══╝
    ██║   ██║   ██║██████╔╝██║   ██║   ██║   
    ██║   ██║   ██║██╔══██╗██║   ██║   ██║   
    ██║   ╚██████╔╝██║  ██║╚██████╔╝   ██║   
    ╚═╝    ╚═════╝ ╚═╝  ╚═╝ ╚═════╝    ╚═╝   
"""

VERSION = "2.0.0"


def print_banner():
    console.print(Text(BANNER, style="bold cyan", justify="center"))
    console.print(Text(
        f"  Universal Security Agent  |  v{VERSION}\n",
        style="dim", justify="center"
    ))


def _get_ollama_models(url: str) -> list[str]:
    try:
        with urllib.request.urlopen(f"{url.rstrip('/')}/api/tags", timeout=5) as resp:
            data = json.loads(resp.read())
        return [m["name"] for m in data.get("models", [])]
    except Exception:
        return []


def run_startup_wizard() -> AIConfig:
    """
    Interactive startup wizard.
    Returns a configured AIConfig.
    """
    print_banner()

    console.print(Panel(
        "[bold white]Select your AI provider.[/bold white]\n"
        "[dim]Torot works fully offline without AI — tools still run.\n"
        "With AI, you get planning, analysis, PoC generation, and smart summaries.[/dim]",
        border_style="cyan",
    ))
    console.print()

    # ── Provider selection ──────────────────────────────────────────────
    tbl = Table(box=box.SIMPLE, show_header=False, padding=(0, 2))
    tbl.add_column("No.", style="bold cyan", width=4)
    tbl.add_column("Provider", style="bold white")
    tbl.add_column("Notes", style="dim")

    tbl.add_row("1", "Anthropic Claude", "claude-opus-4-6  (recommended)")
    tbl.add_row("2", "OpenAI GPT-4",     "gpt-4o")
    tbl.add_row("3", "Ollama (local)",   "runs 100% offline — requires Ollama installed")
    tbl.add_row("4", "None / offline",   "tools run, no AI reasoning")

    console.print(tbl)
    console.print()

    choice = Prompt.ask(
        "  Choose provider",
        choices=["1", "2", "3", "4"],
        default="4",
    )

    cfg = AIConfig()

    if choice == "1":
        cfg.provider = AIProvider.CLAUDE
        cfg.api_key  = Prompt.ask("  Anthropic API key", password=True)
        cfg.model    = "claude-opus-4-6"
        console.print("[dim]  Using claude-opus-4-6[/dim]")

    elif choice == "2":
        cfg.provider = AIProvider.OPENAI
        cfg.api_key  = Prompt.ask("  OpenAI API key", password=True)
        cfg.model    = "gpt-4o"
        console.print("[dim]  Using gpt-4o[/dim]")

    elif choice == "3":
        cfg.provider   = AIProvider.OLLAMA
        ollama_url     = Prompt.ask("  Ollama URL", default="http://localhost:11434")
        cfg.ollama_url = ollama_url

        console.print("[dim]  Checking for available Ollama models...[/dim]")
        models = _get_ollama_models(ollama_url)

        if models:
            console.print(f"  Found models: {', '.join(models)}")
            cfg.ollama_model = Prompt.ask("  Select model", default=models[0])
        else:
            console.print("[yellow]  Could not reach Ollama. Make sure it's running: ollama serve[/yellow]")
            cfg.ollama_model = Prompt.ask("  Model name", default="llama3")

    else:
        cfg.provider = AIProvider.NONE
        console.print("[dim]  Running in offline mode. Tools still execute.[/dim]")

    console.print()

    # ── Optional API keys ───────────────────────────────────────────────
    if Confirm.ask("  Add optional API keys (Etherscan, GitHub)?", default=False):
        console.print()
        etherscan = Prompt.ask("  Etherscan API key (for on-chain analysis)", default="")
        github    = Prompt.ask("  GitHub token (for auto-issue creation)", default="")
        repo      = ""
        if github:
            repo = Prompt.ask("  GitHub repo (owner/repo)", default="")
        cfg.etherscan_key = etherscan
        cfg.github_token  = github
        cfg.github_repo   = repo

    console.print()
    return cfg


def print_tool_table():
    """Print all tools and their install status."""
    from torot.tools.registry import ALL_TOOLS, Domain

    console.print()
    for domain in Domain:
        domain_tools = [t for t in ALL_TOOLS if t.domain == domain]
        if not domain_tools:
            continue
        tbl = Table(
            title=f"{domain.value.upper()} Tools",
            box=box.SIMPLE_HEAVY,
            header_style="bold magenta",
            show_edge=True,
        )
        tbl.add_column("Tool",        style="bold white", width=14)
        tbl.add_column("Installed",   width=10)
        tbl.add_column("Description")
        tbl.add_column("Install", style="dim")

        for t in domain_tools:
            installed = t.is_installed()
            status    = Text("yes", style="bold green") if installed else Text("no", style="dim red")
            tbl.add_row(t.name, status, t.description, t.install_hint if not installed else "")
        console.print(tbl)
        console.print()

    installed = sum(1 for t in ALL_TOOLS if t.is_installed())
    console.print(
        f"  [bold]{installed}[/bold] of [bold]{len(ALL_TOOLS)}[/bold] tools installed. "
        f"Torot works with any subset.\n"
    )


def print_memory_stats(memory):
    """Print persistent memory statistics."""
    stats = memory.stats()
    console.print(Panel(
        Text.assemble(
            Text("  Memory Database\n\n", style="bold white"),
            Text(f"  Sessions:         {stats['total_sessions']}\n", style="white"),
            Text(f"  Total findings:   {stats['total_findings']}\n", style="white"),
            Text(f"  Critical:         {stats['critical']}\n", style="bold red"),
            Text(f"  High:             {stats['high']}\n", style="red"),
        ),
        title="Persistent Memory",
        border_style="blue",
    ))
    console.print()
