"""
Torot Scan Engine.
Orchestrates all security tool scanners concurrently,
feeds updates into the TUI dashboard, and produces a final report.
"""

from __future__ import annotations
import asyncio
import time
from typing import Callable, Optional

from torot.core.models import ScanSession, ToolStatus, ToolResult
from torot.core.detector import detect_project
from torot.scanners.all_scanners import (
    SlitherScanner,
    AderynScanner, MythrilScanner, ManticoreScanner,
    EchidnaScanner, SecurifyScanner, SolhintScanner,
    OyenteScanner, SmartCheckScanner, HalmosScanner,
)

ALL_SCANNER_CLASSES = [
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


class ScanEngine:
    def __init__(
        self,
        target_path: str,
        on_status_change: Optional[Callable] = None,
        max_concurrent: int = 4,
    ):
        self.target_path = target_path
        self.on_status_change = on_status_change
        self.max_concurrent = max_concurrent
        self.session = ScanSession(target_path=target_path)

    async def run(self) -> ScanSession:
        """
        Full scan pipeline:
        1. Detect project languages & files
        2. Run all scanners concurrently (with concurrency limit)
        3. Collect results into session
        4. Return completed session
        """
        # Step 1: Detect project
        try:
            languages, files = detect_project(self.target_path)
            self.session.detected_languages = languages
            self.session.detected_files = files
        except FileNotFoundError as e:
            raise

        # Step 2: Initialize all scanner instances
        scanners = []
        for cls in ALL_SCANNER_CLASSES:
            def make_callback(tool_name):
                def callback(tn, status, message):
                    # Update session result
                    if tn not in self.session.tool_results:
                        self.session.tool_results[tn] = ToolResult(
                            tool_name=tn, status=status
                        )
                    self.session.tool_results[tn].status = status
                    # Forward to dashboard
                    if self.on_status_change:
                        self.on_status_change(tn, status, message)
                return callback

            scanner = cls(
                target_path=self.target_path,
                on_status_change=make_callback(cls.tool_name),
            )
            # Pre-register each tool in session
            self.session.tool_results[cls.tool_name] = ToolResult(
                tool_name=cls.tool_name,
                status=ToolStatus.PENDING,
            )
            scanners.append(scanner)

        # Step 3: Run with semaphore to limit concurrent tools
        semaphore = asyncio.Semaphore(self.max_concurrent)

        async def bounded_scan(scanner):
            async with semaphore:
                result = await scanner.scan()
                # Merge result into session
                self.session.tool_results[scanner.tool_name] = result
                return result

        tasks = [bounded_scan(s) for s in scanners]
        await asyncio.gather(*tasks, return_exceptions=True)

        self.session.end_time = time.time()
        return self.session

    @property
    def all_tool_names(self) -> list[str]:
        return [cls.tool_name for cls in ALL_SCANNER_CLASSES]
