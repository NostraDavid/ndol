from __future__ import annotations

import argparse
import os
from pathlib import Path
from typing import TYPE_CHECKING

from ndol._native import apply_text, check_text

if TYPE_CHECKING:
    from collections.abc import Iterable

__all__ = ["apply_text", "check_text", "main"]

SCRIPT_SHEBANG = "#!/usr/bin/env -S uv run --script"


def main() -> None:
    parser = argparse.ArgumentParser(prog="ndol")
    parser.add_argument("command", choices=["check", "apply"])
    parser.add_argument("paths", nargs="*", type=Path, default=[Path()])
    args = parser.parse_args()

    paths = list(_python_files(args.paths))
    diagnostics_found = False

    for path in paths:
        source = path.read_text(encoding="utf-8")
        diagnostics = check_text(source)
        executable_needed = "/// script" in source
        executable_missing = executable_needed and not os.access(path, os.X_OK)

        if args.command == "apply":
            fixed = apply_text(source)
            if fixed != source:
                path.write_text(fixed, encoding="utf-8")
            if executable_needed:
                path.chmod(path.stat().st_mode | 0o755)

        for line_number, code, message in diagnostics:
            diagnostics_found = True
            print(f"{path}:{line_number}:1 {code} {message}")
        if executable_missing:
            diagnostics_found = True
            print(f"{path}:1:1 ND004 files containing `/// script` must be executable")

    if args.command == "check" and diagnostics_found:
        raise SystemExit(1)


def _python_files(paths: Iterable[Path]) -> Iterable[Path]:
    for path in paths:
        if path.is_file() and path.suffix == ".py":
            yield path
            continue
        if not path.is_dir():
            continue
        for candidate in path.rglob("*.py"):
            parts = set(candidate.parts)
            if {".git", ".venv", "__pycache__", "target"} & parts:
                continue
            yield candidate
