"""
Torot Scan Engine.
Orchestrates all security tool scanners concurrently.
Works with any subset of installed tools — minimum 1 required.
"""

from __future__ import annotations
import asyncio
import time
from typing import Callable, Optional

from torot.core.models import ScanSession, ToolStatus, ToolResult, ApiConfig
from torot.core.detector import detect_project
from torot.scanners.all_scanners import ALL_SCANNERS


class ScanEngine:
    def __init__(
        self,
        target_path:      str,
        on_status_change: Optional[Callable] = None,
        max_concurrent:   int  = 5,
        api_config:       Optional[ApiConfig] = None,
    ):
        self.target_path      = target_path
        self.on_status_change = on_status_change
        self.max_concurrent   = max_concurrent
        self.session          = ScanSession(
            target_path=target_path,
            api_config=api_config,
        )

    # ------------------------------------------------------------------ #
    #  Public API                                                          #
    # ------------------------------------------------------------------ #

    async def run(self) -> ScanSession:
        # 1. Detect project files
        try:
            languages, files = detect_project(self.target_path)
            self.session.detected_languages = languages
            self.session.detected_files     = files
        except FileNotFoundError:
            raise

        # 2. Build scanner instances
        scanners = []
        for cls in ALL_SCANNERS:
            scanner = cls(
                target_path=self.target_path,
                on_status_change=self._make_callback(cls.tool_name),
            )
            self.session.tool_results[cls.tool_name] = ToolResult(
                tool_name=cls.tool_name,
                status=ToolStatus.PENDING,
            )
            scanners.append(scanner)

        # 3. Run concurrently with semaphore
        sem = asyncio.Semaphore(self.max_concurrent)

        async def bounded(scanner):
            async with sem:
                result = await scanner.scan()
                self.session.tool_results[scanner.tool_name] = result
                return result

        await asyncio.gather(*[bounded(s) for s in scanners], return_exceptions=True)

        # 4. Enrich bugs: reproduction guides + production path
        from torot.core.reproduction import enrich_bugs_with_reproduction
        all_bugs = []
        for result in self.session.tool_results.values():
            if result.bugs:
                result.bugs = enrich_bugs_with_reproduction(result.bugs)
                all_bugs.extend(result.bugs)

        # 5. Optional API enrichment
        if self.session.api_config and self.session.api_config.has_ai():
            try:
                from torot.core.api_enricher import run_api_enrichment
                run_api_enrichment(
                    self.session,
                    log=lambda m: self.on_status_change("api", ToolStatus.RUNNING, m)
                    if self.on_status_change else None,
                )
            except Exception:
                pass

        self.session.end_time = time.time()
        return self.session

    # ------------------------------------------------------------------ #
    #  Helpers                                                             #
    # ------------------------------------------------------------------ #

    def _make_callback(self, tool_name: str) -> Callable:
        def callback(tn: str, status: ToolStatus, message: str):
            if tn in self.session.tool_results:
                self.session.tool_results[tn].status = status
            if self.on_status_change:
                self.on_status_change(tn, status, message)
        return callback

    @property
    def all_tool_names(self) -> list[str]:
        return [cls.tool_name for cls in ALL_SCANNERS]

    def installed_tool_count(self) -> int:
        """Count tools actually available in PATH."""
        import shutil
        count = 0
        for cls in ALL_SCANNERS:
            for b in cls.binary_names:
                if shutil.which(b):
                    count += 1
                    break
        return count
