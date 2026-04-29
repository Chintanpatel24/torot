"""
Base class for all tool scanners in Torot.
"""

from __future__ import annotations
import asyncio
import shutil
import time
from abc import ABC, abstractmethod
from typing import Callable, Optional

from torot.core.models import Bug, ToolResult, ToolStatus


class BaseScanner(ABC):
    """
    Abstract base for every security tool integration.
    Subclasses implement `_run_tool()` and `_parse_output()`.
    """

    # Override in subclass
    tool_name: str = "unknown"
    display_name: str = "Unknown Tool"
    description: str = ""
    supported_languages: list[str] = []
    binary_names: list[str] = []   # possible binary names to search for

    def __init__(self, target_path: str, on_status_change: Optional[Callable] = None):
        self.target_path = target_path
        self.on_status_change = on_status_change  # callback(tool_name, status, message)
        self.result = ToolResult(
            tool_name=self.tool_name,
            status=ToolStatus.PENDING,
        )

    # ------------------------------------------------------------------ #
    #  Public API                                                          #
    # ------------------------------------------------------------------ #

    async def scan(self) -> ToolResult:
        """Full scan lifecycle: check → run → parse → return."""
        self._update_status(ToolStatus.CHECKING, "Checking installation…")

        binary = self._find_binary()
        if binary is None:
            self._update_status(ToolStatus.NOT_INSTALLED, f"{self.display_name} not found in PATH")
            self.result.status = ToolStatus.NOT_INSTALLED
            return self.result

        self._update_status(ToolStatus.RUNNING, "Running analysis…")
        t0 = time.time()
        try:
            raw_output, error = await self._run_tool(binary)
            self.result.raw_output = raw_output
            self.result.error = error
            self.result.duration = time.time() - t0

            if error and not raw_output:
                self._update_status(ToolStatus.FAILED, f"Tool error: {error[:120]}")
                self.result.status = ToolStatus.FAILED
            else:
                bugs = self._parse_output(raw_output)
                self.result.bugs = bugs
                self._update_status(
                    ToolStatus.COMPLETED,
                    f"Found {len(bugs)} issue(s) in {self.result.duration:.1f}s"
                )
                self.result.status = ToolStatus.COMPLETED
        except Exception as exc:
            self.result.duration = time.time() - t0
            self.result.error = str(exc)
            self._update_status(ToolStatus.FAILED, f"Exception: {exc}")
            self.result.status = ToolStatus.FAILED

        return self.result

    # ------------------------------------------------------------------ #
    #  Internal helpers                                                    #
    # ------------------------------------------------------------------ #

    def _find_binary(self) -> Optional[str]:
        for name in self.binary_names:
            found = shutil.which(name)
            if found:
                return found
        return None

    def _update_status(self, status: ToolStatus, message: str = ""):
        self.result.status = status
        if self.on_status_change:
            self.on_status_change(self.tool_name, status, message)

    async def _run_command(
        self,
        cmd: list[str],
        timeout: int = 300,
        cwd: Optional[str] = None,
    ) -> tuple[str, str]:
        """Run a subprocess and return (stdout, stderr)."""
        proc = await asyncio.create_subprocess_exec(
            *cmd,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=cwd or self.target_path,
        )
        try:
            stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
            return stdout.decode("utf-8", errors="replace"), stderr.decode("utf-8", errors="replace")
        except asyncio.TimeoutError:
            proc.kill()
            return "", f"{self.display_name} timed out after {timeout}s"

    # ------------------------------------------------------------------ #
    #  Abstract methods                                                    #
    # ------------------------------------------------------------------ #

    @abstractmethod
    async def _run_tool(self, binary: str) -> tuple[str, str]:
        """Execute the tool and return (stdout, stderr)."""
        ...

    @abstractmethod
    def _parse_output(self, output: str) -> list[Bug]:
        """Parse raw tool output into a list of Bug objects."""
        ...
