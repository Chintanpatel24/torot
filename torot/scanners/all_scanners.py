"""
All security tool scanner integrations for Torot.
Each scanner gracefully handles the tool not being installed.
Adding a new tool: subclass BaseScanner, implement _run_tool + _parse_output,
then append it to ALL_SCANNERS at the bottom of this file.
"""

from __future__ import annotations
import json
import re
import glob

from torot.core.models import Bug, Severity
from torot.scanners.base import BaseScanner
from torot.scanners.slither_scanner import SlitherScanner


# ─────────────────────────────────────────────────────────────────────────────
# Shared helpers
# ─────────────────────────────────────────────────────────────────────────────

def _sol_files(path: str) -> list[str]:
    return glob.glob(f"{path}/**/*.sol", recursive=True)

def _rs_files(path: str) -> list[str]:
    return glob.glob(f"{path}/**/*.rs", recursive=True)

def _first_sol(path: str) -> str | None:
    files = _sol_files(path)
    return files[0] if files else None

SEV_WORD = {
    "critical": Severity.CRITICAL,
    "high": Severity.HIGH,
    "medium": Severity.MEDIUM,
    "low": Severity.LOW,
    "info": Severity.INFO,
    "informational": Severity.INFO,
    "optimization": Severity.INFO,
    "note": Severity.INFO,
    "warning": Severity.LOW,
    "error": Severity.HIGH,
}

def _word_sev(text: str, default: Severity = Severity.MEDIUM) -> Severity:
    t = text.lower()
    for k, v in SEV_WORD.items():
        if k in t:
            return v
    return default


# ─────────────────────────────────────────────────────────────────────────────
# Aderyn
# ─────────────────────────────────────────────────────────────────────────────
class AderynScanner(BaseScanner):
    tool_name          = "aderyn"
    display_name       = "Aderyn"
    description        = "Rust-based static analyzer for multi-contract Solidity systems"
    supported_languages = ["solidity"]
    binary_names       = ["aderyn"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command(
            [binary, self.target_path, "--output", "json"], timeout=300
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("{")
            if start == -1:
                return self._parse_md(output)
            data = json.loads(output[start:])
            for sev_key, sev in [
                ("high_issues",   Severity.HIGH),
                ("medium_issues", Severity.MEDIUM),
                ("low_issues",    Severity.LOW),
                ("nc_issues",     Severity.INFO),
            ]:
                for issue in data.get(sev_key, {}).get("issues", []):
                    for inst in issue.get("instances", [{}]):
                        bugs.append(Bug(
                            tool=self.tool_name,
                            title=f"[Aderyn] {issue.get('title', 'Issue')}",
                            severity=sev,
                            description=issue.get("description", ""),
                            file=inst.get("contract_path", ""),
                            line=inst.get("line_no", 0),
                            code_snippet=inst.get("src", ""),
                            fix_suggestion=issue.get("recommendation", ""),
                            impact=issue.get("description", ""),
                            bug_type=issue.get("title", ""),
                            raw=str(issue),
                        ))
        except Exception:
            return self._parse_md(output)
        return bugs

    def _parse_md(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        sev = Severity.INFO
        for line in output.splitlines():
            if "## High"   in line: sev = Severity.HIGH
            elif "## Medium" in line: sev = Severity.MEDIUM
            elif "## Low"    in line: sev = Severity.LOW
            elif line.startswith("### "):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[Aderyn] {line.strip('# ').strip()}",
                    severity=sev,
                    description=line.strip(),
                    fix_suggestion="See Aderyn documentation.",
                    raw=line,
                ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Mythril
# ─────────────────────────────────────────────────────────────────────────────
class MythrilScanner(BaseScanner):
    tool_name          = "mythril"
    display_name       = "Mythril"
    description        = "Symbolic execution for EVM bytecode — reentrancy, tx.origin"
    supported_languages = ["solidity"]
    binary_names       = ["myth"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        sol = _first_sol(self.target_path)
        if not sol:
            return "", "No .sol files found"
        return await self._run_command(
            [binary, "analyze", sol, "-o", "json", "--execution-timeout", "60"],
            timeout=120,
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("{")
            if start == -1:
                return bugs
            data  = json.loads(output[start:])
            for issue in data.get("issues", []):
                sev = {"High": Severity.HIGH, "Medium": Severity.MEDIUM,
                       "Low":  Severity.LOW}.get(issue.get("severity","Low"), Severity.INFO)
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[Mythril] {issue.get('title','Issue')}",
                    severity=sev,
                    description=issue.get("description", ""),
                    file=issue.get("filename", ""),
                    line=issue.get("lineno", 0),
                    code_snippet=issue.get("code", ""),
                    fix_suggestion=issue.get("extra", {}).get("recommendation", ""),
                    impact=issue.get("description", ""),
                    bug_type=issue.get("swc-id", ""),
                    references=[issue.get("swc-title", "")],
                    raw=str(issue),
                ))
        except Exception:
            pass
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Manticore
# ─────────────────────────────────────────────────────────────────────────────
class ManticoreScanner(BaseScanner):
    tool_name          = "manticore"
    display_name       = "Manticore"
    description        = "Binary analysis via symbolic execution with custom properties"
    supported_languages = ["solidity"]
    binary_names       = ["manticore"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        sol = _first_sol(self.target_path)
        if not sol:
            return "", "No .sol files found"
        return await self._run_command([binary, sol], timeout=300)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        for line in output.splitlines():
            if any(k in line.lower() for k in ["bug", "vulnerability", "assertion", "error"]):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title="[Manticore] Property Violation",
                    severity=Severity.HIGH,
                    description=line.strip(),
                    fix_suggestion="Review Manticore output traces for counterexample.",
                    raw=line,
                ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Echidna
# ─────────────────────────────────────────────────────────────────────────────
class EchidnaScanner(BaseScanner):
    tool_name          = "echidna"
    display_name       = "Echidna"
    description        = "Property-based fuzzer for Solidity — breaks invariants"
    supported_languages = ["solidity"]
    binary_names       = ["echidna", "echidna-test"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        sol = _first_sol(self.target_path)
        if not sol:
            return "", "No .sol files found"
        return await self._run_command(
            [binary, sol, "--format", "text", "--test-limit", "1000"], timeout=180
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        for line in output.splitlines():
            if "FAILED" in line or "failed" in line.lower():
                bugs.append(Bug(
                    tool=self.tool_name,
                    title="[Echidna] Invariant Broken",
                    severity=Severity.HIGH,
                    description=line.strip(),
                    fix_suggestion="Fix the failing invariant. Review fuzzing corpus.",
                    impact="A property that should always hold was violated by the fuzzer.",
                    raw=line,
                ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Securify2
# ─────────────────────────────────────────────────────────────────────────────
class SecurifyScanner(BaseScanner):
    tool_name          = "securify"
    display_name       = "Securify2"
    description        = "Static analysis for Ethereum security pattern compliance"
    supported_languages = ["solidity"]
    binary_names       = ["securify", "securify2"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        sol = _first_sol(self.target_path)
        if not sol:
            return "", "No .sol files found"
        return await self._run_command([binary, "-fj", sol], timeout=300)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            data = json.loads(output)
            for contract, results in data.items():
                for pattern, result in results.items():
                    for v in result.get("violations", []):
                        bugs.append(Bug(
                            tool=self.tool_name,
                            title=f"[Securify] {pattern}",
                            severity=Severity.MEDIUM,
                            description=f"Pattern violation in {contract}: {pattern}",
                            file=str(v),
                            fix_suggestion=f"Ensure {pattern} compliance.",
                            raw=str(v),
                        ))
        except Exception:
            for line in output.splitlines():
                if "violation" in line.lower() or "unsafe" in line.lower():
                    bugs.append(Bug(
                        tool=self.tool_name,
                        title="[Securify] Violation",
                        severity=Severity.MEDIUM,
                        description=line.strip(),
                        fix_suggestion="Review Securify pattern documentation.",
                        raw=line,
                    ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# solhint
# ─────────────────────────────────────────────────────────────────────────────
class SolhintScanner(BaseScanner):
    tool_name          = "solhint"
    display_name       = "solhint"
    description        = "Solidity linter — coding standards and common issues"
    supported_languages = ["solidity"]
    binary_names       = ["solhint"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command(
            [binary, f"{self.target_path}/**/*.sol", "--formatter", "json"], timeout=120
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("[")
            if start == -1:
                return bugs
            data = json.loads(output[start:])
            for file_result in data:
                fpath = file_result.get("filePath", "")
                for msg in file_result.get("messages", []):
                    sev = Severity.HIGH if msg.get("severity", 1) == 2 else Severity.LOW
                    bugs.append(Bug(
                        tool=self.tool_name,
                        title=f"[solhint] {msg.get('ruleId', 'lint-issue')}",
                        severity=sev,
                        description=msg.get("message", ""),
                        file=fpath,
                        line=msg.get("line", 0),
                        fix_suggestion=f"Fix rule: {msg.get('ruleId','')}",
                        raw=str(msg),
                    ))
        except Exception:
            for line in output.splitlines():
                if "error" in line.lower() or "warning" in line.lower():
                    bugs.append(Bug(
                        tool=self.tool_name,
                        title="[solhint] Lint Issue",
                        severity=Severity.LOW,
                        description=line.strip(),
                        fix_suggestion="Fix the linting issue per solhint rules.",
                        raw=line,
                    ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Oyente
# ─────────────────────────────────────────────────────────────────────────────
class OyenteScanner(BaseScanner):
    tool_name          = "oyente"
    display_name       = "Oyente"
    description        = "Static analyzer — timestamp dependence, reentrancy"
    supported_languages = ["solidity"]
    binary_names       = ["oyente"]

    VULN_MAP = {
        "callstack_bug":        ("Callstack Bug",       Severity.HIGH,     "Stack depth attacks cause unexpected reverts."),
        "money_concurrency_bug":("Money Concurrency",   Severity.HIGH,     "Race conditions on ETH transfers."),
        "time_dependency_bug":  ("Timestamp Dependence",Severity.MEDIUM,   "Block timestamp can be manipulated by miners."),
        "reentrancy_bug":       ("Reentrancy",          Severity.CRITICAL, "State updated after external call."),
    }

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        sol = _first_sol(self.target_path)
        if not sol:
            return "", "No .sol files found"
        return await self._run_command([binary, "-s", sol, "-j"], timeout=300)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            data = json.loads(output)
            for contract, vulns in data.items():
                for key, (title, sev, impact) in self.VULN_MAP.items():
                    if vulns.get(key, False):
                        bugs.append(Bug(
                            tool=self.tool_name,
                            title=f"[Oyente] {title}",
                            severity=sev,
                            description=f"{title} detected in {contract}",
                            fix_suggestion=f"Mitigate {title} — review contract logic.",
                            impact=impact,
                            raw=str(vulns),
                        ))
        except Exception:
            for line in output.splitlines():
                if "vulnerability" in line.lower() or "bug" in line.lower():
                    bugs.append(Bug(
                        tool=self.tool_name,
                        title="[Oyente] Vulnerability",
                        severity=Severity.MEDIUM,
                        description=line.strip(),
                        raw=line,
                    ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# SmartCheck
# ─────────────────────────────────────────────────────────────────────────────
class SmartCheckScanner(BaseScanner):
    tool_name          = "smartcheck"
    display_name       = "SmartCheck"
    description        = "XPath-based static analysis — known vulnerability patterns"
    supported_languages = ["solidity"]
    binary_names       = ["smartcheck"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command([binary, "-p", self.target_path], timeout=180)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        title, sev = "", Severity.MEDIUM
        for line in output.splitlines():
            line = line.strip()
            if line.startswith("ruleId:"):
                title = line.replace("ruleId:", "").strip()
            elif line.startswith("severity:"):
                sev = _word_sev(line)
            elif line.startswith("description:"):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[SmartCheck] {title}",
                    severity=sev,
                    description=line.replace("description:", "").strip(),
                    fix_suggestion="Review SmartCheck rule documentation.",
                    raw=line,
                ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Halmos
# ─────────────────────────────────────────────────────────────────────────────
class HalmosScanner(BaseScanner):
    tool_name          = "halmos"
    display_name       = "Halmos"
    description        = "Bounded model checker for Solidity — SMT-based proofs"
    supported_languages = ["solidity"]
    binary_names       = ["halmos"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command(
            [binary, "--root", self.target_path, "--json"], timeout=600
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("{")
            if start != -1:
                data = json.loads(output[start:])
                for r in data.get("results", []):
                    if r.get("result") == "fail":
                        bugs.append(Bug(
                            tool=self.tool_name,
                            title=f"[Halmos] Verification Failure: {r.get('name','')}",
                            severity=Severity.HIGH,
                            description=f"Property {r.get('name')} has an SMT counterexample.",
                            fix_suggestion="Inspect the counterexample trace and fix contract logic.",
                            impact="Contract does not satisfy its formal specification.",
                            raw=str(r),
                        ))
        except Exception:
            for line in output.splitlines():
                if "FAIL" in line or "counterexample" in line.lower():
                    bugs.append(Bug(
                        tool=self.tool_name,
                        title="[Halmos] Verification Failure",
                        severity=Severity.HIGH,
                        description=line.strip(),
                        fix_suggestion="Inspect the SMT counterexample.",
                        raw=line,
                    ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Semgrep (with Solidity rules)
# ─────────────────────────────────────────────────────────────────────────────
class SemgrepScanner(BaseScanner):
    tool_name          = "semgrep"
    display_name       = "Semgrep"
    description        = "Pattern-based static analysis — custom and community Solidity rules"
    supported_languages = ["solidity", "rust"]
    binary_names       = ["semgrep"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command(
            [binary, "--config", "auto", "--json", self.target_path,
             "--no-git-ignore", "--quiet"],
            timeout=300,
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("{")
            if start == -1:
                return bugs
            data = json.loads(output[start:])
            for r in data.get("results", []):
                meta    = r.get("extra", {}).get("metadata", {})
                sev_str = r.get("extra", {}).get("severity", "WARNING")
                sev     = _word_sev(sev_str)
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[Semgrep] {r.get('check_id','').split('.')[-1]}",
                    severity=sev,
                    description=r.get("extra", {}).get("message", ""),
                    file=r.get("path", ""),
                    line=r.get("start", {}).get("line", 0),
                    code_snippet=r.get("extra", {}).get("lines", ""),
                    fix_suggestion=meta.get("fix", "Apply the recommended Semgrep rule fix."),
                    bug_type=r.get("check_id", ""),
                    references=meta.get("references", []),
                    raw=str(r),
                ))
        except Exception:
            pass
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# cargo-audit (Rust dependency auditing)
# ─────────────────────────────────────────────────────────────────────────────
class CargoAuditScanner(BaseScanner):
    tool_name          = "cargo-audit"
    display_name       = "cargo-audit"
    description        = "Rust dependency vulnerability scanner — checks against RustSec advisory DB"
    supported_languages = ["rust"]
    binary_names       = ["cargo-audit", "cargo"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        cmd = [binary, "audit", "--json"] if "cargo-audit" in binary else \
              [binary, "audit", "--json"]
        return await self._run_command(cmd, timeout=120, cwd=self.target_path)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("{")
            if start == -1:
                return bugs
            data = json.loads(output[start:])
            for vuln in data.get("vulnerabilities", {}).get("list", []):
                adv = vuln.get("advisory", {})
                sev_str = adv.get("cvss", "")
                sev = Severity.HIGH if "9" in sev_str or "8" in sev_str else Severity.MEDIUM
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[cargo-audit] {adv.get('title', 'Vulnerable Dependency')}",
                    severity=sev,
                    description=adv.get("description", ""),
                    file=vuln.get("package", {}).get("name", ""),
                    fix_suggestion=(
                        f"Upgrade {vuln.get('package',{}).get('name','')} to "
                        f"{vuln.get('versions',{}).get('patched',['latest'])[0] if vuln.get('versions',{}).get('patched') else 'a patched version'}."
                    ),
                    impact=adv.get("description", ""),
                    bug_type=adv.get("id", ""),
                    references=[adv.get("url", "")],
                    raw=str(vuln),
                ))
        except Exception:
            pass
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Clippy (Rust linter via cargo clippy)
# ─────────────────────────────────────────────────────────────────────────────
class ClippyScanner(BaseScanner):
    tool_name          = "clippy"
    display_name       = "Clippy"
    description        = "Rust linter — catches common mistakes and unsafe patterns"
    supported_languages = ["rust"]
    binary_names       = ["cargo"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command(
            [binary, "clippy", "--message-format=json", "--", "-D", "warnings"],
            timeout=300,
            cwd=self.target_path,
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        for line in output.splitlines():
            try:
                msg = json.loads(line)
                if msg.get("reason") != "compiler-message":
                    continue
                inner = msg.get("message", {})
                level = inner.get("level", "warning")
                if level not in ("error", "warning"):
                    continue
                sev = Severity.HIGH if level == "error" else Severity.LOW
                spans = inner.get("spans", [{}])
                primary = next((s for s in spans if s.get("is_primary")), spans[0] if spans else {})
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[Clippy] {inner.get('code',{}).get('code','lint')}",
                    severity=sev,
                    description=inner.get("message", ""),
                    file=primary.get("file_name", ""),
                    line=primary.get("line_start", 0),
                    code_snippet=primary.get("text", [{}])[0].get("text", "") if primary.get("text") else "",
                    fix_suggestion=inner.get("rendered", ""),
                    bug_type=inner.get("code", {}).get("code", ""),
                    raw=line,
                ))
            except Exception:
                continue
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# solc (Solidity compiler warnings)
# ─────────────────────────────────────────────────────────────────────────────
class SolcScanner(BaseScanner):
    tool_name          = "solc"
    display_name       = "solc"
    description        = "Solidity compiler — surfaces warnings and errors as findings"
    supported_languages = ["solidity"]
    binary_names       = ["solc"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        sol = _first_sol(self.target_path)
        if not sol:
            return "", "No .sol files found"
        return await self._run_command(
            [binary, "--combined-json", "abi,bin", "--no-color", sol], timeout=60
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        for line in output.splitlines():
            lw = line.lower()
            if "warning:" in lw or "error:" in lw:
                sev = Severity.HIGH if "error:" in lw else Severity.LOW
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[solc] {'Error' if sev == Severity.HIGH else 'Warning'}",
                    severity=sev,
                    description=line.strip(),
                    fix_suggestion="Fix the compiler diagnostic before deploying.",
                    raw=line,
                ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Pyrometer (Rust-based Solidity analyzer)
# ─────────────────────────────────────────────────────────────────────────────
class PyrometerScanner(BaseScanner):
    tool_name          = "pyrometer"
    display_name       = "Pyrometer"
    description        = "Rust-based range analysis for Solidity — detects bounds errors"
    supported_languages = ["solidity"]
    binary_names       = ["pyrometer"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        sol = _first_sol(self.target_path)
        if not sol:
            return "", "No .sol files found"
        return await self._run_command([binary, sol], timeout=180)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        for line in output.splitlines():
            lw = line.lower()
            if any(k in lw for k in ["unreachable", "overflow", "underflow", "error", "warning"]):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title="[Pyrometer] Range Analysis Issue",
                    severity=_word_sev(line),
                    description=line.strip(),
                    fix_suggestion="Review the range analysis output and constrain inputs.",
                    raw=line,
                ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Wake (Python-based Solidity testing/analysis framework)
# ─────────────────────────────────────────────────────────────────────────────
class WakeScanner(BaseScanner):
    tool_name          = "wake"
    display_name       = "Wake"
    description        = "Python-based Solidity analysis framework with detector plugins"
    supported_languages = ["solidity"]
    binary_names       = ["wake"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command(
            [binary, "detect", "--json", self.target_path], timeout=300
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("[")
            if start == -1:
                return bugs
            data = json.loads(output[start:])
            for item in data:
                sev = _word_sev(item.get("severity", ""))
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[Wake] {item.get('detector_name', 'Issue')}",
                    severity=sev,
                    description=item.get("description", ""),
                    file=item.get("filename", ""),
                    line=item.get("line", 0),
                    code_snippet=item.get("source_code", ""),
                    fix_suggestion=item.get("recommendation", ""),
                    bug_type=item.get("detector_name", ""),
                    raw=str(item),
                ))
        except Exception:
            pass
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# 4naly3er (report-style static analyzer)
# ─────────────────────────────────────────────────────────────────────────────
class FourNaly3erScanner(BaseScanner):
    tool_name          = "4naly3er"
    display_name       = "4naly3er"
    description        = "Automated report-style static analyzer for audit contests"
    supported_languages = ["solidity"]
    binary_names       = ["4naly3er", "analyzer"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command([binary, self.target_path], timeout=120)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        sev = Severity.INFO
        for line in output.splitlines():
            if line.startswith("## [H]") or "High" in line:
                sev = Severity.HIGH
            elif line.startswith("## [M]") or "Medium" in line:
                sev = Severity.MEDIUM
            elif line.startswith("## [L]") or "Low" in line:
                sev = Severity.LOW
            elif line.startswith("### "):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[4naly3er] {line.strip('# ').strip()}",
                    severity=sev,
                    description=line.strip(),
                    fix_suggestion="See 4naly3er output for details.",
                    raw=line,
                ))
        return bugs


# ─────────────────────────────────────────────────────────────────────────────
# Registry — append new scanners here to auto-include them
# ─────────────────────────────────────────────────────────────────────────────
ALL_SCANNERS = [
    SlitherScanner,
    AderynScanner,
    MythrilScanner,
    ManticoreScanner,
    EchidnaScanner,
    SecurifyScanner,
    SolhintScanner,
    OyenteScanner,
    SmartCheckScanner,
    HalmosScanner,
    SemgrepScanner,
    CargoAuditScanner,
    ClippyScanner,
    SolcScanner,
    PyrometerScanner,
    WakeScanner,
    FourNaly3erScanner,
]


def get_all_scanner_names() -> list[str]:
    return [s.tool_name for s in ALL_SCANNERS]
