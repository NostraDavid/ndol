# ndol

`ndol` is a Rust-first Python linter/fixer for NostraDavid's preferred Python
style. It ships as both a Rust crate/CLI and a Python package backed by the same
Rust engine.

## Current Rules

| Code    | Rule                                                                                                 |
| ------- | ---------------------------------------------------------------------------------------------------- |
| `ND001` | Replace `from datetime import ...` with `import datetime as dt`.                                     |
| `ND002` | Replace `from structlog import get_logger` with `from structlog.stdlib import get_logger`.           |
| `ND003` | Import HTTP client libraries as `http`, for example `import niquests as http`.                       |
| `ND004` | Files containing `/// script` must start with `#!/usr/bin/env -S uv run --script` and be executable. |

## Usage

```bash
ndol check src tests
ndol apply src tests
```

Inline suppression:

```python
from datetime import datetime  # noqa: ND001
```

File-wide suppression in the first comments of a file:

```python
# ndol: noqa: ND001, ND003
```

Project-wide suppression:

```toml
[tool.ndol]
ignore = ["ND003"]
```

## Development

```bash
just allow
just shell
just sync
just test
just check
```

Release builds are prepared for both the Rust crate and Python wheels. The repo
is intended to work with trunk-based development plus release branches such as
`release/1.x` for later minor releases after a newer major line exists.
