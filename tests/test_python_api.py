from __future__ import annotations

from ndol import apply_text, check_text


def test_python_api_applies_text() -> None:
    assert apply_text("from structlog import get_logger\n") == (
        "from structlog.stdlib import get_logger\n"
    )


def test_python_api_checks_text() -> None:
    diagnostics = check_text("import requests\n")
    assert diagnostics == [(1, "ND003", "import HTTP client libraries with the `http` alias")]
