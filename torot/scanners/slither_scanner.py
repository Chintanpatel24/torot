"""
Slither — Static analysis for Solidity smart contracts.
https://github.com/crytic/slither
"""

from __future__ import annotations
import json
import re
from typing import Callable, Optional

from torot.core.models import Bug, Severity
from torot.scanners.base import BaseScanner

SEVERITY_MAP = {
    "High": Severity.HIGH,
    "Medium": Severity.MEDIUM,
    "Low": Severity.LOW,
    "Informational": Severity.INFO,
    "Optimization": Severity.INFO,
}

IMPACT_NOTES = {
    "reentrancy": "Attacker can drain funds by re-entering the function before state is updated.",
    "suicidal": "Contract can be destroyed by any caller, losing all funds permanently.",
    "uninitialized": "Uninitialized storage pointer may corrupt contract storage.",
    "tx-origin": "Using tx.origin for auth allows phishing attacks.",
    "arbitrary-send": "Ether can be sent to an arbitrary address, enabling theft.",
    "controlled-delegatecall": "Delegatecall to attacker-controlled address enables code injection.",
}

FIX_TEMPLATES = {
    "reentrancy": "Apply the Checks-Effects-Interactions pattern. Update state BEFORE external calls. Consider using OpenZeppelin's ReentrancyGuard.",
    "suicidal": "Remove or restrict selfdestruct() with a multisig or timelock guard.",
    "uninitialized": "Explicitly initialize all storage variables before use.",
    "tx-origin": "Replace tx.origin with msg.sender for authentication checks.",
    "arbitrary-send": "Restrict ether transfer targets to known, trusted addresses.",
    "controlled-delegatecall": "Never delegatecall to user-supplied addresses. Use a fixed implementation address.",
}


class SlitherScanner(BaseScanner):
    tool_name = "slither"
    display_name = "Slither"
    description = "Static analysis for Solidity — reentrancy, overflow, access control"
    supported_languages = ["solidity"]
    binary_names = ["slither"]

    async def _run_tool(self, binary: str) -> tuple[str, str]:
        cmd = [
            binary,
            self.target_path,
            "--json", "-",
            "--no-fail-pedantic",
            "--exclude-optimization",
        ]
        return await self._run_command(cmd, timeout=300)

    def _parse_output(self, output: str) -> list[Bug]:
        bugs: list[Bug] = []
        if not output.strip():
            return bugs

        # Slither may emit text before JSON — find the JSON blob
        json_start = output.find("{")
        if json_start == -1:
            return self._parse_text(output)

        try:
            data = json.loads(output[json_start:])
        except json.JSONDecodeError:
            return self._parse_text(output)

        detectors = data.get("results", {}).get("detectors", [])
        for det in detectors:
            check = det.get("check", "")
            severity = SEVERITY_MAP.get(det.get("impact", "Low"), Severity.LOW)
            description = det.get("description", "").strip()

            # Extract file/line from first element
            elements = det.get("elements", [])
            file_path = ""
            line_no = 0
            code_snippet = ""
            if elements:
                src_map = elements[0].get("source_mapping", {})
                file_path = src_map.get("filename_relative", "")
                lines = src_map.get("lines", [])
                line_no = lines[0] if lines else 0
                code_snippet = elements[0].get("name", "")

            key = check.lower()
            impact = next((v for k, v in IMPACT_NOTES.items() if k in key), "")
            fix = next((v for k, v in FIX_TEMPLATES.items() if k in key), "Review and apply best practices for this pattern.")

            bugs.append(Bug(
                tool=self.tool_name,
                title=f"[Slither] {check.replace('-', ' ').title()}",
                severity=severity,
                description=description,
                file=file_path,
                line=line_no,
                code_snippet=code_snippet,
                fix_suggestion=fix,
                impact=impact,
                bug_type=check,
                references=[f"https://github.com/crytic/slither/wiki/Detector-Documentation#{check}"],
                raw=str(det),
            ))

        return bugs

    def _parse_text(self, output: str) -> list[Bug]:
        """Fallback plain-text parser."""
        bugs: list[Bug] = []
        for line in output.splitlines():
            if any(k in line.lower() for k in ["error", "warning", "reentrancy", "overflow"]):
                bugs.append(Bug(
                    tool=self.tool_name,
                    title="[Slither] Issue Detected",
                    severity=Severity.MEDIUM,
                    description=line.strip(),
                    fix_suggestion="Review the flagged code manually.",
                    raw=line,
                ))
        return bugs
