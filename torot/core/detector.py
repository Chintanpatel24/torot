"""
Detects code files and languages in a target directory.
"""

from __future__ import annotations
import os
from pathlib import Path


SOLIDITY_EXTENSIONS = {".sol"}
RUST_EXTENSIONS = {".rs"}
EVM_EXTENSIONS = {".abi", ".json", ".bin"}

IGNORE_DIRS = {
    ".git", "node_modules", "target", "__pycache__", ".venv",
    "venv", "dist", "build", ".cache", "artifacts", "cache"
}


def detect_project(path: str) -> tuple[list[str], list[str]]:
    """
    Walk the target path and return (languages_detected, file_paths).
    """
    root = Path(path).resolve()
    if not root.exists():
        raise FileNotFoundError(f"Path does not exist: {path}")

    files: list[str] = []
    languages: set[str] = set()

    for dirpath, dirnames, filenames in os.walk(root):
        # Prune ignored directories in-place
        dirnames[:] = [d for d in dirnames if d not in IGNORE_DIRS]

        for fname in filenames:
            ext = Path(fname).suffix.lower()
            full = os.path.join(dirpath, fname)

            if ext in SOLIDITY_EXTENSIONS:
                files.append(full)
                languages.add("solidity")
            elif ext in RUST_EXTENSIONS:
                files.append(full)
                languages.add("rust")
            elif ext in EVM_EXTENSIONS and fname != "package.json":
                files.append(full)

    return sorted(languages), files


def summarize_project(path: str) -> dict:
    """Return a summary dict of detected files grouped by language."""
    languages, files = detect_project(path)
    summary = {
        "path": str(Path(path).resolve()),
        "languages": languages,
        "total_files": len(files),
        "solidity_files": [f for f in files if f.endswith(".sol")],
        "rust_files": [f for f in files if f.endswith(".rs")],
        "other_files": [
            f for f in files
            if not f.endswith(".sol") and not f.endswith(".rs")
        ],
    }
    return summary
