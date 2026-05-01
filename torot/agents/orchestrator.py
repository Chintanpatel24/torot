"""
Torot Orchestrator
Runs security tools, parses their output into Finding objects,
streams output to the UI, and feeds results to the agent brain.
"""
from __future__ import annotations
import asyncio
import glob
import json
import re
import time
from typing import Callable, Optional
from torot.core.models import Finding, Severity, Domain, Session, InputMode
from torot.tools.registry import ToolDef, run_tool, get_installed_for_domain, ALL_TOOLS


OnLine     = Callable[[str], None]
OnFinding  = Callable[[Finding], None]


# ─────────────────────────────────────────────────────────────────────────────
# Output parsers per tool
# ─────────────────────────────────────────────────────────────────────────────

def _sev(text: str, default: Severity = Severity.MEDIUM) -> Severity:
    t = text.lower()
    if "critical" in t: return Severity.CRITICAL
    if "high"     in t: return Severity.HIGH
    if "medium"   in t: return Severity.MEDIUM
    if "low"      in t: return Severity.LOW
    if "info"     in t or "informational" in t: return Severity.INFO
    return default


def parse_slither(stdout: str, stderr: str, domain: Domain) -> list[Finding]:
    findings: list[Finding] = []
    start = stdout.find("{")
    if start == -1:
        for line in (stdout + stderr).splitlines():
            if any(k in line.lower() for k in ["reentrancy", "overflow", "error", "warning"]):
                findings.append(Finding(
                    tool="slither", title=f"[slither] Issue",
                    severity=_sev(line), domain=domain,
                    description=line.strip(), raw=line,
                    fix_suggestion="Review the flagged code.",
                ))
        return findings
    try:
        data = json.loads(stdout[start:])
        for det in data.get("results", {}).get("detectors", []):
            sev = _sev(det.get("impact", "Low"))
            els = det.get("elements", [])
            src = els[0].get("source_mapping", {}) if els else {}
            findings.append(Finding(
                tool="slither",
                title=f"[slither] {det.get('check','').replace('-',' ').title()}",
                severity=sev, domain=domain,
                description=det.get("description", "").strip(),
                file=src.get("filename_relative", ""),
                line=(src.get("lines", [0]) or [0])[0],
                code_snippet=els[0].get("name", "") if els else "",
                fix_suggestion="Apply best practices for this vulnerability class.",
                bug_type=det.get("check", ""),
                references=[f"https://github.com/crytic/slither/wiki/Detector-Documentation#{det.get('check','')}"],
                raw=str(det),
            ))
    except Exception:
        pass
    return findings


def parse_mythril(stdout: str, stderr: str, domain: Domain) -> list[Finding]:
    findings: list[Finding] = []
    try:
        start = stdout.find("{")
        if start == -1:
            return findings
        data = json.loads(stdout[start:])
        for issue in data.get("issues", []):
            sev = {"High": Severity.HIGH, "Medium": Severity.MEDIUM,
                   "Low": Severity.LOW}.get(issue.get("severity", "Low"), Severity.INFO)
            findings.append(Finding(
                tool="mythril",
                title=f"[mythril] {issue.get('title','')}",
                severity=sev, domain=domain,
                description=issue.get("description", ""),
                file=issue.get("filename", ""),
                line=issue.get("lineno", 0),
                code_snippet=issue.get("code", ""),
                bug_type=issue.get("swc-id", ""),
                fix_suggestion="Apply symbolic execution analysis recommendations.",
                raw=str(issue),
            ))
    except Exception:
        pass
    return findings


def parse_nuclei(stdout: str, stderr: str, domain: Domain) -> list[Finding]:
    findings: list[Finding] = []
    for line in stdout.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            data = json.loads(line)
            sev  = _sev(data.get("info", {}).get("severity", "info"))
            findings.append(Finding(
                tool="nuclei",
                title=f"[nuclei] {data.get('info',{}).get('name','')}",
                severity=sev, domain=domain,
                description=data.get("info", {}).get("description", ""),
                file=data.get("matched-at", ""),
                fix_suggestion=data.get("info", {}).get("remediation", ""),
                bug_type=data.get("template-id", ""),
                references=data.get("info", {}).get("reference", []),
                raw=line,
            ))
        except Exception:
            if "[" in line and "]" in line:
                findings.append(Finding(
                    tool="nuclei", title="[nuclei] Finding",
                    severity=_sev(line), domain=domain,
                    description=line, raw=line,
                    fix_suggestion="Review the nuclei template finding.",
                ))
    return findings


def parse_semgrep(stdout: str, stderr: str, domain: Domain) -> list[Finding]:
    findings: list[Finding] = []
    try:
        start = stdout.find("{")
        if start == -1:
            return findings
        data = json.loads(stdout[start:])
        for r in data.get("results", []):
            meta = r.get("extra", {}).get("metadata", {})
            findings.append(Finding(
                tool="semgrep",
                title=f"[semgrep] {r.get('check_id','').split('.')[-1]}",
                severity=_sev(r.get("extra", {}).get("severity", "WARNING")),
                domain=domain,
                description=r.get("extra", {}).get("message", ""),
                file=r.get("path", ""),
                line=r.get("start", {}).get("line", 0),
                code_snippet=r.get("extra", {}).get("lines", ""),
                fix_suggestion=meta.get("fix", ""),
                bug_type=r.get("check_id", ""),
                references=meta.get("references", []),
                raw=str(r),
            ))
    except Exception:
        pass
    return findings


def parse_generic(tool_name: str, stdout: str, stderr: str, domain: Domain) -> list[Finding]:
    """Generic parser that extracts anything that looks like a finding."""
    findings: list[Finding] = []
    keywords = ["error", "warning", "vulnerability", "vuln", "bug",
                "critical", "high", "medium", "low", "issue", "fail"]
    for line in (stdout + "\n" + stderr).splitlines():
        line = line.strip()
        if not line:
            continue
        lw = line.lower()
        if any(k in lw for k in keywords):
            findings.append(Finding(
                tool=tool_name,
                title=f"[{tool_name}] Issue",
                severity=_sev(line),
                domain=domain,
                description=line,
                fix_suggestion=f"Review the {tool_name} finding.",
                raw=line,
            ))
    return findings


PARSERS = {
    "slither":  parse_slither,
    "mythril":  parse_mythril,
    "nuclei":   parse_nuclei,
    "semgrep":  parse_semgrep,
}


def parse_output(tool_name: str, stdout: str, stderr: str, domain: Domain) -> list[Finding]:
    parser = PARSERS.get(tool_name, None)
    if parser:
        return parser(stdout, stderr, domain)
    return parse_generic(tool_name, stdout, stderr, domain)


# ─────────────────────────────────────────────────────────────────────────────
# Orchestrator
# ─────────────────────────────────────────────────────────────────────────────

class Orchestrator:
    def __init__(
        self,
        session:       Session,
        on_line:       Optional[OnLine]    = None,
        on_finding:    Optional[OnFinding] = None,
        max_concurrent: int = 5,
    ):
        self.session         = session
        self.on_line         = on_line
        self.on_finding      = on_finding
        self.max_concurrent  = max_concurrent
        self._sem            = asyncio.Semaphore(max_concurrent)

    def _emit(self, line: str):
        if self.on_line:
            self.on_line(line)

    def _emit_finding(self, f: Finding):
        self.session.findings.append(f)
        if self.on_finding:
            self.on_finding(f)

    async def run_tool_and_parse(self, tool: ToolDef, args: list[str], cwd: str = ".") -> list[Finding]:
        self._emit(f"  > Starting: {tool.name}")
        async with self._sem:
            stdout, stderr, duration = await run_tool(
                tool, args, cwd=cwd, on_line=self._emit
            )
        self._emit(f"  > Finished: {tool.name} ({duration:.1f}s)")
        findings = parse_output(tool.name, stdout, stderr, self.session.domain)
        for f in findings:
            self._emit_finding(f)
        return findings

    async def run_all_for_target(self, target: str) -> list[Finding]:
        """
        Detect which tools apply to this target and run them all.
        Builds command args per tool based on target type.
        """
        domain  = self.session.domain
        tools   = get_installed_for_domain(domain)

        if not tools:
            self._emit(f"  No tools installed for domain: {domain.value}")
            self._emit("  Run: torot --list-tools  to see what to install")
            return []

        installed_names = [t.name for t in tools]
        self._emit(f"  Installed tools for {domain.value}: {', '.join(installed_names)}")
        self._emit(f"  Running {len(tools)} tool(s) in parallel (max {self.max_concurrent} at a time)")
        self._emit("")

        # Build (tool, args, cwd) triples per tool
        tasks = []
        for tool in tools:
            args, cwd = self._build_args(tool, target)
            if args is None:
                self._emit(f"  Skipping {tool.name} — no applicable target found")
                continue
            tasks.append(self.run_tool_and_parse(tool, args, cwd))

        results = await asyncio.gather(*tasks, return_exceptions=True)
        all_findings: list[Finding] = []
        for r in results:
            if isinstance(r, list):
                all_findings.extend(r)
        return all_findings

    def _build_args(self, tool: ToolDef, target: str) -> tuple[Optional[list[str]], str]:
        """Build command-line args for a tool given a target path."""
        import os
        cwd = target if os.path.isdir(target) else "."

        # Find first sol/rs file
        sol_files = glob.glob(f"{target}/**/*.sol", recursive=True) if os.path.isdir(target) else []
        rs_files  = glob.glob(f"{target}/**/*.rs",  recursive=True) if os.path.isdir(target) else []
        first_sol = sol_files[0] if sol_files else None
        first_rs  = rs_files[0]  if rs_files  else None

        args_map = {
            # Blockchain
            "slither":     ([target, "--json", "-", "--no-fail-pedantic"],                cwd),
            "aderyn":      ([target, "--output", "json"],                                 cwd),
            "mythril":     (["analyze", first_sol, "-o", "json", "--execution-timeout", "60"] if first_sol else None, cwd),
            "echidna":     ([first_sol, "--format", "text", "--test-limit", "1000"] if first_sol else None,            cwd),
            "solhint":     ([f"{target}/**/*.sol", "--formatter", "json"],                cwd),
            "halmos":      (["--root", target, "--json"],                                 cwd),
            "semgrep":     (["--config", "auto", "--json", target, "--no-git-ignore", "--quiet"], cwd),
            "solc":        ([first_sol, "--combined-json", "abi", "--no-color"] if first_sol else None,                cwd),
            "wake":        (["detect", "--json", target],                                 cwd),
            "cargo-audit": (["audit", "--json"],                                          cwd),
            "clippy":      (["clippy", "--message-format=json", "--", "-D", "warnings"] if rs_files else None,        cwd),
            # Web App
            "nuclei":      (["-target", target, "-json", "-silent"],                      cwd),
            "nikto":       (["-h", target, "-Format", "txt"],                             cwd),
            "sqlmap":      (["-u", target, "--batch", "--level=2"],                       cwd),
            "wfuzz":       (["-z", "file,/usr/share/wordlists/dirb/common.txt", f"{target}/FUZZ"], cwd),
            "ffuf":        (["-u", f"{target}/FUZZ", "-w", "/usr/share/seclists/Discovery/Web-Content/common.txt", "-o", "json"], cwd),
            "whatweb":     ([target, "--log-json=-"],                                     cwd),
            "dalfox":      (["url", target, "--silence"],                                 cwd),
            "trufflehog":  (["filesystem", target, "--json"],                             cwd),
            "gitleaks":    (["detect", "--source", target, "--report-format", "json"],    cwd),
            # API
            "arjun":       (["-u", target, "--json"],                                     cwd),
            # Binary
            "checksec":    (["--file", target, "--output", "json"],                       cwd),
            "binwalk":     (["-e", target],                                               cwd),
            "strings":     ([target],                                                     cwd),
            "objdump":     (["-d", target],                                               cwd),
        }

        entry = args_map.get(tool.name)
        if entry is None:
            return ([], cwd)
        if entry[0] is None:
            return (None, cwd)
        return entry
