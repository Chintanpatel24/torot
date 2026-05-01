"""
Torot Agent Brain
The central intelligence of Torot — thinks like an elite security researcher.
Supports Claude, OpenAI GPT-4, and Ollama (local LLMs).
"""
from __future__ import annotations
import json
import urllib.request
import urllib.error
import time
from typing import Optional, AsyncGenerator
from torot.core.models import (
    AIConfig, AIProvider, Session, Finding, AgentPlan,
    PlanStep, StepStatus, Domain, InputMode, Severity
)


# ─────────────────────────────────────────────────────────────────────────────
# Hacker Memory — elite security researcher knowledge injected into every prompt
# ─────────────────────────────────────────────────────────────────────────────

HACKER_SYSTEM_PROMPT = """\
You are Torot, an elite security research agent with the mindset and skills of a \
top-tier bug bounty hunter, smart contract auditor, and offensive security researcher. \
You have deep expertise in:

BLOCKCHAIN & SMART CONTRACTS:
- All EVM vulnerability classes: reentrancy (single/cross-function/cross-contract), \
  integer overflow/underflow, tx.origin auth, selfdestruct, delegatecall injection, \
  flash loan attacks, price oracle manipulation, front-running/MEV, access control bypass, \
  timestamp dependence, gas griefing, storage collision, proxy upgrade vulnerabilities, \
  signature replay attacks, unchecked return values
- Rust/Substrate security: unsafe blocks, integer overflow, race conditions, \
  memory safety, dependency vulnerabilities, improper error handling
- DeFi protocol patterns: AMM invariant breaks, liquidation exploits, \
  governance attacks, token standard edge cases (ERC20/ERC721/ERC1155)

WEB APPLICATION SECURITY:
- OWASP Top 10, SQL injection, XSS, CSRF, SSRF, XXE, path traversal
- Authentication bypass, JWT attacks, OAuth flaws, session fixation
- Business logic flaws, rate limiting, mass assignment

BINARY & NATIVE CODE:
- Buffer overflows, format string bugs, use-after-free, heap exploitation
- Return-oriented programming (ROP), ASLR/NX bypass techniques
- Reverse engineering, anti-debug techniques

API SECURITY:
- REST/GraphQL injection, BOLA/IDOR, mass assignment, rate limit bypass
- API key exposure, endpoint enumeration, versioning attacks

OPERATIONAL MINDSET:
- You think like a hacker first, defender second
- You always look for the highest-impact, most exploitable paths
- You prioritize findings by real-world exploitability, not just theoretical risk
- You write clear, actionable reproduction steps and PoC code
- You know Immunefi, Code4rena, Sherlock, HackerOne, Bugcrowd bug report formats
- You never give up on a target — if one tool misses it, another catches it

When proposing a plan, be methodical: reconnaissance → static analysis → \
dynamic analysis → manual review → exploitation → reporting.

Always be direct, technical, and precise. No fluff."""


# ─────────────────────────────────────────────────────────────────────────────
# LLM call dispatchers
# ─────────────────────────────────────────────────────────────────────────────

def _call_anthropic(messages: list[dict], config: AIConfig, system: str = "") -> str:
    payload = json.dumps({
        "model":      config.model or "claude-opus-4-6",
        "max_tokens": 2000,
        "system":     system or HACKER_SYSTEM_PROMPT,
        "messages":   messages,
    }).encode()
    req = urllib.request.Request(
        "https://api.anthropic.com/v1/messages",
        data=payload,
        headers={
            "x-api-key":         config.api_key,
            "anthropic-version": "2023-06-01",
            "Content-Type":      "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.loads(resp.read())
    return data["content"][0]["text"].strip()


def _call_openai(messages: list[dict], config: AIConfig, system: str = "") -> str:
    full_msgs = [{"role": "system", "content": system or HACKER_SYSTEM_PROMPT}] + messages
    payload = json.dumps({
        "model":      config.model or "gpt-4o",
        "max_tokens": 2000,
        "messages":   full_msgs,
    }).encode()
    req = urllib.request.Request(
        "https://api.openai.com/v1/chat/completions",
        data=payload,
        headers={
            "Authorization": f"Bearer {config.api_key}",
            "Content-Type":  "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.loads(resp.read())
    return data["choices"][0]["message"]["content"].strip()


def _call_ollama(messages: list[dict], config: AIConfig, system: str = "") -> str:
    full_msgs = [{"role": "system", "content": system or HACKER_SYSTEM_PROMPT}] + messages
    payload = json.dumps({
        "model":    config.ollama_model,
        "messages": full_msgs,
        "stream":   False,
    }).encode()
    url = f"{config.ollama_url.rstrip('/')}/api/chat"
    req = urllib.request.Request(
        url, data=payload,
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read())
    return data.get("message", {}).get("content", "").strip()


def _llm_call(messages: list[dict], config: AIConfig, system: str = "") -> str:
    """Route to the correct LLM provider."""
    if not config or config.provider == AIProvider.NONE:
        return "[AI not configured — running in offline mode]"
    try:
        if config.provider == AIProvider.CLAUDE:
            return _call_anthropic(messages, config, system)
        elif config.provider == AIProvider.OPENAI:
            return _call_openai(messages, config, system)
        elif config.provider == AIProvider.OLLAMA:
            return _call_ollama(messages, config, system)
    except Exception as e:
        return f"[AI error: {e}]"
    return ""


# ─────────────────────────────────────────────────────────────────────────────
# Agent Brain — high-level reasoning
# ─────────────────────────────────────────────────────────────────────────────

class AgentBrain:
    def __init__(self, config: Optional[AIConfig] = None):
        self.config   = config
        self._history: list[dict] = []   # conversation history for this brain instance

    def _chat(self, user_msg: str, system: str = "") -> str:
        self._history.append({"role": "user", "content": user_msg})
        reply = _llm_call(self._history, self.config, system)
        self._history.append({"role": "assistant", "content": reply})
        return reply

    def reset_history(self):
        self._history = []

    # ── Planning ─────────────────────────────────────────────────────────

    def build_plan(self, session: "Session", available_tools: list[str]) -> AgentPlan:
        """
        Given a target and available tools, generate a structured attack plan.
        Returns an AgentPlan with steps for user approval.
        """
        tools_str = ", ".join(available_tools) if available_tools else "manual analysis only"

        prompt = f"""
You are planning a security assessment. Generate a step-by-step attack plan.

TARGET: {session.target}
MODE:   {session.input_mode.value}
DOMAIN: {session.domain.value}
TOOLS AVAILABLE: {tools_str}

Respond ONLY with valid JSON in this exact format:
{{
  "goal": "one sentence describing the overall mission",
  "steps": [
    {{
      "title": "short step name",
      "description": "what this step does and why",
      "tool": "tool name or 'manual' or 'ai-analysis'",
      "command": "exact shell command to run or empty string"
    }}
  ]
}}

Include reconnaissance, static analysis, dynamic testing, and reporting steps.
Maximum 10 steps. Be specific and actionable.
"""
        raw = self._chat(prompt)
        try:
            # Strip markdown fences if present
            raw = raw.strip()
            if raw.startswith("```"):
                raw = raw.split("```")[1]
                if raw.startswith("json"):
                    raw = raw[4:]
            data  = json.loads(raw.strip())
            steps = [
                PlanStep(
                    title=s.get("title", ""),
                    description=s.get("description", ""),
                    tool=s.get("tool", ""),
                    command=s.get("command", ""),
                    status=StepStatus.PENDING,
                )
                for s in data.get("steps", [])
            ]
            return AgentPlan(
                goal=data.get("goal", f"Assess {session.target}"),
                steps=steps,
            )
        except Exception:
            # Fallback plan
            return self._fallback_plan(session, available_tools)

    def _fallback_plan(self, session: "Session", tools: list[str]) -> AgentPlan:
        steps = []
        if session.input_mode == InputMode.FOLDER:
            steps = [
                PlanStep(title="Detect languages and files",
                         description="Walk the target directory and identify all code files",
                         tool="internal", command=""),
                PlanStep(title="Static analysis — all installed tools",
                         description="Run every available static analyzer in parallel",
                         tool="all", command=""),
                PlanStep(title="AI review of findings",
                         description="Have the AI brain review and prioritize all findings",
                         tool="ai-analysis", command=""),
                PlanStep(title="Generate report",
                         description="Produce a full Markdown report with reproduction guides",
                         tool="internal", command=""),
            ]
        elif session.input_mode == InputMode.ADDRESS:
            steps = [
                PlanStep(title="Fetch on-chain bytecode",
                         description="Download the contract bytecode and ABI from Etherscan",
                         tool="etherscan", command=""),
                PlanStep(title="Decompile and reconstruct source",
                         description="Attempt to decompile bytecode using available tools",
                         tool="heimdall", command=""),
                PlanStep(title="Analyze decompiled code",
                         description="Run static analyzers on reconstructed source",
                         tool="all", command=""),
                PlanStep(title="Generate report",
                         description="Produce findings report",
                         tool="internal", command=""),
            ]
        else:
            steps = [
                PlanStep(title="Process question",
                         description="Analyze the security question using the AI brain",
                         tool="ai-analysis", command=""),
                PlanStep(title="Search knowledge base",
                         description="Check persistent memory for relevant past findings",
                         tool="internal", command=""),
            ]
        return AgentPlan(goal=f"Assess {session.target}", steps=steps)

    # ── Analysis ─────────────────────────────────────────────────────────

    def analyze_finding(self, finding: Finding) -> str:
        """Deep-dive AI analysis of a single finding."""
        prompt = f"""
Analyze this security finding from a hacker's perspective:

Tool:        {finding.tool}
Title:       {finding.title}
Severity:    {finding.severity.value}
Location:    {finding.location}
Description: {finding.description}
Code:
{finding.code_snippet[:600] if finding.code_snippet else "N/A"}

Provide:
1. Confirm whether this is a real, exploitable vulnerability (not a false positive)
2. The exact exploit scenario in 2-3 sentences
3. A one-line code fix
4. Real-world impact (funds at risk? contract destroyed? governance hijacked?)

Be direct and technical. No padding.
"""
        return self._chat(prompt)

    def answer_question(self, question: str, context: str = "") -> str:
        """Answer a security question as a senior security researcher."""
        prompt = question
        if context:
            prompt = f"Context:\n{context}\n\nQuestion: {question}"
        return self._chat(prompt)

    def summarize_session(self, session: "Session") -> str:
        """Produce an executive summary of a completed session."""
        summary = session.finding_summary
        total   = len(session.findings)
        prompt  = f"""
Write a 3-paragraph executive summary of this security assessment:

Target:   {session.target}
Domain:   {session.domain.value}
Duration: {session.duration:.0f}s
Findings: {json.dumps(summary)}
Total:    {total}

Top findings:
{chr(10).join(f"- [{f.severity.value}] {f.title} ({f.tool})" for f in session.findings[:5])}

Write for a technical audience. Paragraph 1: what was tested and how.
Paragraph 2: key findings and risk. Paragraph 3: recommended next steps.
"""
        return self._chat(prompt)

    def suggest_next_step(self, session: "Session", completed_output: str) -> str:
        """After a step completes, suggest what to investigate next."""
        prompt = f"""
A scan step just completed on target: {session.target}

Output snippet:
{completed_output[:800]}

Current findings so far: {len(session.findings)} issues
Severities: {json.dumps(session.finding_summary)}

Suggest the single most impactful next action for the attacker.
Be specific — name the tool, the exact flag, or the manual check to perform.
One paragraph maximum.
"""
        return self._chat(prompt)

    def generate_poc(self, finding: Finding) -> str:
        """Generate a working PoC script for a finding."""
        prompt = f"""
Generate a Python proof-of-concept script for this vulnerability:

Title:    {finding.title}
Type:     {finding.bug_type}
Location: {finding.location}
Code:     {finding.code_snippet[:500] if finding.code_snippet else "N/A"}
Description: {finding.description}

Write a complete, runnable Python PoC using web3.py.
Include comments explaining each step.
Include the attacker contract in Solidity as a Python string if needed.
"""
        return self._chat(prompt)

    def generate_disclosure(self, finding: Finding) -> str:
        """Generate a bug bounty disclosure report."""
        prompt = f"""
Write a complete bug bounty disclosure report for this finding:

Title:    {finding.title}
Severity: {finding.severity.value}
Tool:     {finding.tool}
Location: {finding.location}
Description: {finding.description}
Code:     {finding.code_snippet[:400] if finding.code_snippet else "N/A"}
Impact:   {finding.impact}
Fix:      {finding.fix_suggestion}

Format it for submission to Immunefi or Code4rena.
Include: Summary, Vulnerability Details, Impact, Proof of Concept,
Recommended Fix, and References sections.
"""
        return self._chat(prompt)
