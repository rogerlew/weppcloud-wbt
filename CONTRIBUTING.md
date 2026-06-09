# Contributing to weppcloud-wbt

Thanks for your interest in weppcloud-wbt.

## How to contribute

Contributions are welcome from the community, including bug reports, documentation fixes, and code changes.
This project is currently maintained primarily by a single maintainer.

1. Open an issue describing the bug, regression, or feature request.
2. Describe the expected and observed behavior, with logs or command examples when possible.
3. Submit changes as a PR with:
   - a clear summary of the change
   - rationale tied to WEPPcloud workflow requirements
   - test or validation notes

## Development setup

The project is a Rust workspace with a Python helper workflow. At minimum:

- Rust toolchain (stable)
- Python 3.11+

The existing CI uses this repository’s existing build and release scripts (`build.py`).

## Style and review expectations

- Keep existing CLI/Python interfaces stable unless there is a clear operational need.
- Preserve existing conventions in command naming, tool registration, and diagnostics.
- Prefer small, targeted changes with comments/tests when behavior changes.
- Provide a short description in commit messages.

## Testing guidance

- Prefer targeted tests for any behavior change.
- If adding/altering terrain, hydrology, or I/O behavior, include a test or reproduction case in the PR.

## Communication

Bug triage and acceptance decisions are made by the maintainer.
