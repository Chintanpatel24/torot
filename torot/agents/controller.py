"""
Torot Controller
The conductor that connects:
  - TorotApp (TUI)         — what the user sees and types
  - AgentBrain             — AI reasoning
  - Orchestrator           — tool execution
  - MemoryStore            — persistence
  - Session                — state
"""
from __future__ import annotations
import asyncio
import time
import os
from typing import Optional

from torot.core.models import (
    Session, Finding, ChatMessage, AgentPlan, PlanStep,
    StepStatus, InputMode, Domain, AIConfig, AIProvider
)
from torot.agents.brain        import AgentBrain
from torot.agents.orchestrator import Orchestrator
from torot.memory.store        import MemoryStore
from torot.tools.registry      import get_installed_tools, get_installed_for_domain
from torot.ui.app              import TorotApp


class TorotController:
    """
    Drives the full agent loop:
      1. User sends input
      2. Detect mode (folder / address / question)
      3. Brain builds a plan
      4. Present plan to user for approval
      5. Execute approved steps, streaming output to TUI
      6. Brain analyses findings
      7. Persist to memory, offer export
    """

    def __init__(self, session: Session, memory: MemoryStore):
        self.session       = session
        self.memory        = memory
        self.brain         = AgentBrain(config=session.ai_config)
        self._approval_evt = asyncio.Event()
        self._approval_ok  = False
        self._app:   Optional[TorotApp] = None
        self._orch:  Optional[Orchestrator] = None
        self._queue: asyncio.Queue = asyncio.Queue()   # user input queue

    # ── Setup ───────────────────────────────────────────────────────────────

    def create_app(self) -> TorotApp:
        self._app = TorotApp(
            session=self.session,
            on_user_input=self._on_user_input,
            on_approve=self._on_approve,
        )
        return self._app

    def _on_user_input(self, text: str):
        """Called by TUI when user submits text."""
        self._queue.put_nowait(text)

    def _on_approve(self, approved: bool):
        """Called by TUI when user clicks Approve/Skip."""
        self._approval_ok  = approved
        self._approval_evt.set()

    # ── Main agent loop (runs in background) ────────────────────────────────

    async def run_loop(self):
        """Background task: reads user inputs and drives the agent."""
        self._emit_system("Ready. Type a target path, contract address, or security question.")
        self._emit_system("Examples:  ./contracts/   |   0x1234...   |   is tx.origin safe?")
        self._emit_separator()

        while self.session.active:
            try:
                text = await asyncio.wait_for(self._queue.get(), timeout=0.5)
            except asyncio.TimeoutError:
                continue

            msg = ChatMessage(role="user", content=text)
            self.session.messages.append(msg)
            self.memory.save_message(self.session.id, msg)

            await self._process_input(text)

    async def _process_input(self, text: str):
        """Detect input type and route to the correct handler."""
        mode = self._detect_mode(text)
        self.session.input_mode = mode
        self.session.target     = text

        if mode == InputMode.FOLDER:
            if not os.path.isdir(text):
                self._emit_agent(f"Path not found: {text}")
                return
            self.session.domain = self._detect_domain_from_path(text)
            await self._run_scan_flow(text)

        elif mode == InputMode.ADDRESS:
            self.session.domain = Domain.BLOCKCHAIN
            await self._run_address_flow(text)

        else:
            await self._run_question_flow(text)

    def _detect_mode(self, text: str) -> InputMode:
        """Classify user input as folder path, contract address, or question."""
        t = text.strip()
        # Ethereum address: 0x followed by 40 hex chars
        if t.startswith("0x") and len(t) == 42 and all(c in "0123456789abcdefABCDEF" for c in t[2:]):
            return InputMode.ADDRESS
        # Looks like a path
        if t.startswith("/") or t.startswith("./") or t.startswith("../") or os.path.isdir(t):
            return InputMode.FOLDER
        # Otherwise treat as a question
        return InputMode.QUESTION

    def _detect_domain_from_path(self, path: str) -> Domain:
        """Guess the security domain from file extensions in the target."""
        import glob
        sol = glob.glob(f"{path}/**/*.sol", recursive=True)
        rs  = glob.glob(f"{path}/**/*.rs",  recursive=True)
        if sol or rs:
            return Domain.BLOCKCHAIN
        return Domain.GENERAL

    # ── Scan flow ────────────────────────────────────────────────────────────

    async def _run_scan_flow(self, path: str):
        installed = get_installed_tools()
        tool_names = [t.name for t in installed]

        self._emit_system(f"Target: {path}")
        self._emit_system(f"Domain: {self.session.domain.value}")
        self._emit_system(f"Installed tools ({len(installed)}): {', '.join(tool_names) or 'none'}")

        if not installed:
            self._emit_agent(
                "No security tools found in PATH. Install at least one tool to proceed.\n"
                "Run: torot --list-tools  to see what to install."
            )
            return

        # Build plan
        self._emit_separator("Planning")
        self._emit_system("Building attack plan...")
        plan = self.brain.build_plan(self.session, tool_names)
        self.session.plan = plan

        if self._app:
            self._app.show_plan(plan)

        # Present plan summary
        plan_summary = "\n".join(
            f"  Step {i}: {s.title}  ({s.tool})"
            for i, s in enumerate(plan.steps, 1)
        )
        self._emit_agent(f"Proposed plan for {path}:\n\n{plan_summary}\n\nApprove to start?")

        # Execute each step with approval
        self._orch = Orchestrator(
            session=self.session,
            on_line=self._on_tool_line,
            on_finding=self._on_finding,
        )

        for step in plan.steps:
            approved = await self._request_approval(step)
            if not approved:
                step.status = StepStatus.SKIPPED
                self._emit_system(f"Skipped: {step.title}")
                if self._app:
                    self._app.update_plan(plan)
                continue

            step.status = StepStatus.RUNNING
            if self._app:
                self._app.update_plan(plan)
            self._emit_separator(step.title)

            t0 = time.time()
            if step.tool in ("internal", "all"):
                await self._orch.run_all_for_target(path)
            elif step.tool == "ai-analysis":
                await self._run_ai_review()
            else:
                # Run specific tool
                from torot.tools.registry import TOOL_MAP
                if step.tool in TOOL_MAP:
                    tool_def = TOOL_MAP[step.tool]
                    args, cwd = self._orch._build_args(tool_def, path)
                    if args is not None:
                        await self._orch.run_tool_and_parse(tool_def, args, cwd)

            step.duration = time.time() - t0
            step.status   = StepStatus.DONE
            if self._app:
                self._app.update_plan(plan)
                self._app.refresh_stats()

        await self._finish_scan()

    async def _run_address_flow(self, address: str):
        self._emit_system(f"On-chain target: {address}")
        if self.session.ai_config and self.session.ai_config.etherscan_key:
            self._emit_system("Fetching contract from Etherscan...")
            # Etherscan lookup
            info = await self._fetch_etherscan(address)
            if info.get("source"):
                self._emit_agent(f"Contract: {info.get('name','unknown')}  |  Compiler: {info.get('compiler','')}")
                self._emit_system("Source code retrieved. Saving for analysis...")
                # Save to temp file and scan
                import tempfile
                tmp = tempfile.mkdtemp()
                sol_path = os.path.join(tmp, f"{info.get('name','contract')}.sol")
                with open(sol_path, "w") as f:
                    f.write(info["source"])
                self.session.target = tmp
                await self._run_scan_flow(tmp)
            else:
                self._emit_agent(
                    "Source code not verified on Etherscan. "
                    "I can analyze the bytecode or answer questions about this address."
                )
        else:
            self._emit_agent(
                f"On-chain analysis requested for {address}.\n"
                "To fetch and decompile the contract, add your Etherscan key:\n"
                "  --api etherscan=YOUR_KEY\n\n"
                "Without a key, I can still answer questions about the address."
            )

    async def _run_question_flow(self, question: str):
        self._emit_system("Processing security question...")
        # Search memory for relevant context
        knowledge = self.memory.search_knowledge(question)
        context = ""
        if knowledge:
            context = "Relevant past findings:\n" + "\n".join(
                f"- {k['topic']}: {k['content'][:200]}" for k in knowledge[:3]
            )

        answer = self.brain.answer_question(question, context)
        self._emit_agent(answer)

        # Save to memory
        msg = ChatMessage(role="agent", content=answer)
        self.session.messages.append(msg)
        self.memory.save_message(self.session.id, msg)
        self.memory.add_knowledge(topic=question[:80], content=answer[:500], source="agent-response")

    # ── Step approval ────────────────────────────────────────────────────────

    async def _request_approval(self, step: PlanStep) -> bool:
        """Show approval UI and wait for user response."""
        step.status = StepStatus.WAITING
        self._emit_system(f"Waiting for approval: {step.title}  [{step.tool}]")

        if self._app:
            self._app.show_approval_bar(step)
            self._approval_evt.clear()
            try:
                await asyncio.wait_for(self._approval_evt.wait(), timeout=120)
            except asyncio.TimeoutError:
                self._emit_system("Approval timed out — skipping step")
                return False
            return self._approval_ok
        else:
            # Non-interactive mode: auto-approve
            return True

    # ── Post-scan ────────────────────────────────────────────────────────────

    async def _run_ai_review(self):
        if not self.session.findings:
            self._emit_agent("No findings to review yet.")
            return
        self._emit_system(f"Reviewing {len(self.session.findings)} findings with AI...")
        for f in self.session.findings[:10]:   # limit to top 10 for token cost
            analysis = self.brain.analyze_finding(f)
            f.ai_analysis = analysis
            self._emit_agent(f"[{f.severity.value}] {f.title}: {analysis[:200]}")
            await asyncio.sleep(0.1)

    async def _finish_scan(self):
        self.session.end_time = time.time()
        self._emit_separator("Complete")
        self._emit_system(
            f"Scan done in {self.session.duration:.1f}s  |  "
            f"{len(self.session.findings)} total findings"
        )

        summary = self.session.finding_summary
        for sev, count in summary.items():
            if count:
                self._emit_line(f"  {sev}: {count}", style=Severity[sev].color)

        # Save session to persistent memory
        self.memory.save_session(self.session)
        for f in self.session.findings:
            self.memory.save_finding(self.session.id, f)
            self.memory.add_knowledge(
                topic=f.title, content=f.description[:400], source=f.tool
            )

        # AI executive summary
        if self.session.ai_config and self.session.ai_config.is_ready():
            self._emit_system("Generating executive summary...")
            summary_text = self.brain.summarize_session(self.session)
            self._emit_agent(summary_text)

        self._emit_separator()
        self._emit_system("Type a new target or question to continue. Ctrl+E to export report.")
        if self._app:
            self._app.refresh_stats()

    # ── Etherscan helper ─────────────────────────────────────────────────────

    async def _fetch_etherscan(self, address: str) -> dict:
        import urllib.request, json
        key = self.session.ai_config.etherscan_key if self.session.ai_config else ""
        url = (
            f"https://api.etherscan.io/api?module=contract"
            f"&action=getsourcecode&address={address}&apikey={key}"
        )
        try:
            with urllib.request.urlopen(url, timeout=15) as resp:
                data = json.loads(resp.read())
            r = data.get("result", [{}])[0]
            return {
                "source":   r.get("SourceCode", ""),
                "name":     r.get("ContractName", ""),
                "compiler": r.get("CompilerVersion", ""),
            }
        except Exception as e:
            return {"error": str(e)}

    # ── Event emitters ───────────────────────────────────────────────────────

    def _emit_line(self, line: str, style: str = ""):
        if self._app:
            self._app.write_line(line, style)

    def _emit_agent(self, msg: str):
        if self._app:
            self._app.write_agent(msg)

    def _emit_system(self, msg: str):
        if self._app:
            self._app.write_system(msg)

    def _emit_separator(self, label: str = ""):
        if self._app:
            self._app.write_separator(label)

    def _on_tool_line(self, line: str):
        if self._app:
            self._app.write_line(line)

    def _on_finding(self, f: Finding):
        if self._app:
            self._app.write_finding(f)
            self._app.refresh_stats()
