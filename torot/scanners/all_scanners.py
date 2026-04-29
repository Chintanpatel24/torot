"""
All secondary scanner integrations for Torot.
Each scanner follows the BaseScanner interface.
"""

from __future__ import annotations
import json
import re
from typing import Callable, Optional

from torot.core.models import Bug, Severity
from torot.scanners.base import BaseScanner


# ═══════════════════════════════════════════════════════════════════════════
#  ADERYN  (Rust-based multi-contract Solidity analyzer)
# ═══════════════════════════════════════════════════════════════════════════
class AderynScanner(BaseScanner):
    tool_name = "aderyn"
    display_name = "Aderyn"
    description = "Rust-based static analyzer for multi-contract Solidity systems"
    supported_languages = ["solidity"]
    binary_names = ["aderyn"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        return await self._run_command(
            [binary, self.target_path, "--output", "json"],
            timeout=300
        )

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        try:
            start = output.find("{")
            if start == -1:
                return self._parse_markdown(output)
            data = json.loads(output[start:])
            for severity_key, sev in [
                ("high_issues", Severity.HIGH),
                ("medium_issues", Severity.MEDIUM),
                ("low_issues", Severity.LOW),
                ("nc_issues", Severity.INFO),
            ]:
                for issue in data.get(severity_key, {}).get("issues", []):
                    for inst in issue.get("instances", [{}]):
                        bugs.append(Bug(
                            tool=self.tool_name,
                            title=f"[Aderyn] {issue.get('title', 'Issue')}",
                            severity=sev,
                            description=issue.get("description", ""),
                            file=inst.get("contract_path", ""),
                            line=inst.get("line_no", 0),
                            code_snippet=inst.get("src", ""),
                            fix_suggestion=issue.get("recommendation", "Review the flagged code."),
                            impact=issue.get("description", ""),
                            bug_type=issue.get("title", ""),
                            raw=str(issue),
                        ))
        except Exception:
            return self._parse_markdown(output)
        return bugs

    def _parse_markdown(self, output: str) -> list[Bug]:
        bugs = []
        current_sev = Severity.INFO
        for line in output.splitlines():
            if "## High" in line:
                current_sev = Severity.HIGH
            elif "## Medium" in line:
                current_sev = Severity.MEDIUM
            elif "## Low" in line:
                current_sev = Severity.LOW
            elif line.startswith("### "):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[Aderyn] {line.strip('# ').strip()}",
                    severity=current_sev,
                    description=line.strip(),
                    fix_suggestion="See Aderyn documentation for remediation.",
                    raw=line,
                ))
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  MYTHRIL  (Symbolic execution for EVM bytecode)
# ═══════════════════════════════════════════════════════════════════════════
class MythrilScanner(BaseScanner):
    tool_name = "mythril"
    display_name = "Mythril"
    description = "Symbolic execution tool for EVM bytecode — reentrancy, tx.origin bugs"
    supported_languages = ["solidity"]
    binary_names = ["myth"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        import os, glob
        sol_files = glob.glob(f"{self.target_path}/**/*.sol", recursive=True)
        if not sol_files:
            return "", "No .sol files found"
        # Analyze the first contract file found (myth analyze per-file)
        cmd = [binary, "analyze", sol_files[0], "-o", "json", "--execution-timeout", "60"]
        return await self._run_command(cmd, timeout=120)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        try:
            start = output.find("{")
            if start == -1:
                return bugs
            data = json.loads(output[start:])
            issues = data.get("issues", [])
            for issue in issues:
                sev_str = issue.get("severity", "Low")
                sev = {"High": Severity.HIGH, "Medium": Severity.MEDIUM, "Low": Severity.LOW}.get(sev_str, Severity.INFO)
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[Mythril] {issue.get('title', 'Issue')}",
                    severity=sev,
                    description=issue.get("description", ""),
                    file=issue.get("filename", ""),
                    line=issue.get("lineno", 0),
                    code_snippet=issue.get("code", ""),
                    fix_suggestion=issue.get("extra", {}).get("recommendation", "Apply symbolic execution analysis recommendations."),
                    impact=issue.get("description", ""),
                    bug_type=issue.get("swc-id", ""),
                    references=[issue.get("swc-title", "")],
                    raw=str(issue),
                ))
        except Exception:
            pass
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  MANTICORE  (Binary analysis + symbolic execution)
# ═══════════════════════════════════════════════════════════════════════════
class ManticoreScanner(BaseScanner):
    tool_name = "manticore"
    display_name = "Manticore"
    description = "Binary analysis using symbolic execution; supports custom security properties"
    supported_languages = ["solidity"]
    binary_names = ["manticore"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        import glob
        sol_files = glob.glob(f"{self.target_path}/**/*.sol", recursive=True)
        if not sol_files:
            return "", "No .sol files found"
        cmd = [binary, "--contract", sol_files[0], "--config", "{}"]
        return await self._run_command(cmd, timeout=300)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        for line in output.splitlines():
            if any(k in line.lower() for k in ["bug", "vulnerability", "error", "assertion"]):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title="[Manticore] Property Violation",
                    severity=Severity.HIGH,
                    description=line.strip(),
                    fix_suggestion="Review symbolic execution traces in Manticore output directory.",
                    raw=line,
                ))
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  ECHIDNA  (Property-based fuzzer for Solidity)
# ═══════════════════════════════════════════════════════════════════════════
class EchidnaScanner(BaseScanner):
    tool_name = "echidna"
    display_name = "Echidna"
    description = "Property-based fuzzer for Solidity — tests invariants via automated inputs"
    supported_languages = ["solidity"]
    binary_names = ["echidna", "echidna-test"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        import glob
        sol_files = glob.glob(f"{self.target_path}/**/*.sol", recursive=True)
        if not sol_files:
            return "", "No .sol files found"
        cmd = [binary, sol_files[0], "--format", "text", "--test-limit", "1000"]
        return await self._run_command(cmd, timeout=180)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        for line in output.splitlines():
            if "FAILED" in line or "failed" in line.lower():
                bugs.append(Bug(
                    tool=self.tool_name,
                    title="[Echidna] Invariant Broken",
                    severity=Severity.HIGH,
                    description=line.strip(),
                    fix_suggestion="Fix the failing property / invariant. Review fuzzing corpus.",
                    impact="An invariant that should always hold was violated during fuzzing.",
                    raw=line,
                ))
            elif "assertion" in line.lower() and "fail" in line.lower():
                bugs.append(Bug(
                    tool=self.tool_name,
                    title="[Echidna] Assertion Failure",
                    severity=Severity.MEDIUM,
                    description=line.strip(),
                    fix_suggestion="Review contract assertions and ensure they hold for all inputs.",
                    raw=line,
                ))
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  SECURIFY2  (Static analyzer for Ethereum contracts)
# ═══════════════════════════════════════════════════════════════════════════
class SecurifyScanner(BaseScanner):
    tool_name = "securify"
    display_name = "Securify2"
    description = "Static analyzer checking compliance with Ethereum security patterns"
    supported_languages = ["solidity"]
    binary_names = ["securify", "securify2"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        import glob
        sol_files = glob.glob(f"{self.target_path}/**/*.sol", recursive=True)
        if not sol_files:
            return "", "No .sol files found"
        cmd = [binary, "-fj", sol_files[0]]
        return await self._run_command(cmd, timeout=300)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        try:
            data = json.loads(output)
            for contract, results in data.items():
                for pattern, result in results.items():
                    if result.get("violations"):
                        for v in result["violations"]:
                            bugs.append(Bug(
                                tool=self.tool_name,
                                title=f"[Securify] {pattern}",
                                severity=Severity.MEDIUM,
                                description=f"Pattern violation in contract {contract}: {pattern}",
                                file=str(v),
                                fix_suggestion=f"Ensure {pattern} compliance. Consult Securify documentation.",
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


# ═══════════════════════════════════════════════════════════════════════════
#  SOLHINT  (Solidity linter)
# ═══════════════════════════════════════════════════════════════════════════
class SolhintScanner(BaseScanner):
    tool_name = "solhint"
    display_name = "solhint"
    description = "Solidity linter — enforces coding standards and detects common issues"
    supported_languages = ["solidity"]
    binary_names = ["solhint"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        cmd = [binary, f"{self.target_path}/**/*.sol", "--formatter", "json"]
        return await self._run_command(cmd, timeout=120)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        try:
            start = output.find("[")
            if start == -1:
                return bugs
            data = json.loads(output[start:])
            for file_result in data:
                fpath = file_result.get("filePath", "")
                for msg in file_result.get("messages", []):
                    sev_num = msg.get("severity", 1)
                    sev = Severity.HIGH if sev_num == 2 else Severity.LOW
                    bugs.append(Bug(
                        tool=self.tool_name,
                        title=f"[solhint] {msg.get('ruleId', 'lint-issue')}",
                        severity=sev,
                        description=msg.get("message", ""),
                        file=fpath,
                        line=msg.get("line", 0),
                        fix_suggestion=f"Fix rule: {msg.get('ruleId', '')}. See solhint docs.",
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
                        fix_suggestion="Fix the linting issue as per solhint rules.",
                        raw=line,
                    ))
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  OYENTE  (Early static analyzer for Solidity)
# ═══════════════════════════════════════════════════════════════════════════
class OyenteScanner(BaseScanner):
    tool_name = "oyente"
    display_name = "Oyente"
    description = "Static analyzer — detects timestamp dependence, reentrancy"
    supported_languages = ["solidity"]
    binary_names = ["oyente"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        import glob
        sol_files = glob.glob(f"{self.target_path}/**/*.sol", recursive=True)
        if not sol_files:
            return "", "No .sol files found"
        cmd = [binary, "-s", sol_files[0], "-j"]
        return await self._run_command(cmd, timeout=300)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        VULN_MAP = {
            "callstack_bug": ("Callstack Bug", Severity.HIGH, "Stack depth attacks can cause unexpected reverts."),
            "money_concurrency_bug": ("Money Concurrency", Severity.HIGH, "Race conditions on ETH transfers."),
            "time_dependency_bug": ("Timestamp Dependence", Severity.MEDIUM, "Block timestamp can be manipulated by miners."),
            "reentrancy_bug": ("Reentrancy", Severity.CRITICAL, "Contract state updated after external call."),
        }
        try:
            data = json.loads(output)
            for contract, vulns in data.items():
                for key, (title, sev, impact) in VULN_MAP.items():
                    if vulns.get(key, False):
                        bugs.append(Bug(
                            tool=self.tool_name,
                            title=f"[Oyente] {title}",
                            severity=sev,
                            description=f"{title} detected in {contract}",
                            fix_suggestion=f"Mitigate {title}: review contract logic carefully.",
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
                        fix_suggestion="Consult Oyente documentation.",
                        raw=line,
                    ))
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  SMARTCHECK  (XPath-based static analysis)
# ═══════════════════════════════════════════════════════════════════════════
class SmartCheckScanner(BaseScanner):
    tool_name = "smartcheck"
    display_name = "SmartCheck"
    description = "Static analysis using XPath patterns — identifies known vulnerability patterns"
    supported_languages = ["solidity"]
    binary_names = ["smartcheck"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        cmd = [binary, "-p", self.target_path]
        return await self._run_command(cmd, timeout=180)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        current_sev = Severity.MEDIUM
        current_title = ""
        for line in output.splitlines():
            line = line.strip()
            if line.startswith("ruleId:"):
                current_title = line.replace("ruleId:", "").strip()
            elif line.startswith("severity:"):
                sev_str = line.replace("severity:", "").strip().lower()
                current_sev = {"error": Severity.HIGH, "warning": Severity.MEDIUM, "info": Severity.INFO}.get(sev_str, Severity.LOW)
            elif line.startswith("description:"):
                desc = line.replace("description:", "").strip()
                bugs.append(Bug(
                    tool=self.tool_name,
                    title=f"[SmartCheck] {current_title}",
                    severity=current_sev,
                    description=desc,
                    fix_suggestion="Review SmartCheck rule documentation for remediation.",
                    raw=line,
                ))
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  HALMOS  (Bounded model checker using SMT)
# ═══════════════════════════════════════════════════════════════════════════
class HalmosScanner(BaseScanner):
    tool_name = "halmos"
    display_name = "Halmos"
    description = "Bounded model checker for Solidity — verifies correctness using SMT solving"
    supported_languages = ["solidity"]
    binary_names = ["halmos"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        cmd = [binary, "--root", self.target_path, "--json"]
        return await self._run_command(cmd, timeout=600)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs = []
        try:
            start = output.find("{")
            if start != -1:
                data = json.loads(output[start:])
                for result in data.get("results", []):
                    if result.get("result") == "fail":
                        bugs.append(Bug(
                            tool=self.tool_name,
                            title=f"[Halmos] Verification Failure: {result.get('name', '')}",
                            severity=Severity.HIGH,
                            description=f"Property {result.get('name')} could not be verified (SMT counterexample found).",
                            fix_suggestion="Inspect the counterexample trace and fix the logical error in the contract.",
                            impact="Contract logic does not satisfy its formal specification.",
                            raw=str(result),
                        ))
        except Exception:
            for line in output.splitlines():
                if "FAIL" in line or "counterexample" in line.lower():
                    bugs.append(Bug(
                        tool=self.tool_name,
                        title="[Halmos] Verification Failure",
                        severity=Severity.HIGH,
                        description=line.strip(),
                        fix_suggestion="Inspect the SMT counterexample and fix contract logic.",
                        raw=line,
                    ))
        return bugs


# ═══════════════════════════════════════════════════════════════════════════
#  Registry: all scanners available in Torot
# ═══════════════════════════════════════════════════════════════════════════
from torot.scanners.slither_scanner import SlitherScanner

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
]


def get_all_scanner_names() -> list[str]:
    return [s.tool_name for s in ALL_SCANNERS]
