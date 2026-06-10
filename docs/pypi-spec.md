# PyPI Packaging Specification

Status: draft specification, not yet implemented  
Primary goal: make `pip install weppcloud-wbt` install the Python wrapper and a platform-specific `whitebox_tools` executable built from this repository.

## Purpose

`weppcloud-wbt` is currently distributed to WEPPcloud by building the Rust CLI and committing the runtime artifact under `WBT/`. That works for the controlled WEPPcloud deployment path, but it is not the right default install story for Python, and GIS. Those users should not need a Rust toolchain for normal installation.

The PyPI package must therefore ship prebuilt wheels:

```bash
pip install weppcloud-wbt
```

The package must still keep Cargo as the source build mechanism for maintainers and CI:

```text
Cargo builds:      whitebox_tools executable
PyPI wheel ships:  Python wrapper + platform-specific whitebox_tools executable
Python users call: WhiteboxTools wrapper, which invokes the packaged executable
```

## External Packaging References

- Python packaging metadata belongs in `pyproject.toml`; the `[project]` table carries standard project metadata.
  See: https://packaging.python.org/en/latest/guides/writing-pyproject-toml/
- The current `pyproject.toml` license field is an SPDX license expression string such as `MIT`.
  See: https://packaging.python.org/en/latest/specifications/pyproject-toml/
- `cibuildwheel` supports Linux, macOS, and Windows wheel builds in CI and can run tests against installed wheels.
  See: https://cibuildwheel.pypa.io/
- PyPI trusted publishers can publish from GitHub Actions without storing long-lived API tokens; pending publishers can create the project on first publish.
  See: https://docs.pypi.org/trusted-publishers/using-a-publisher/

## Current PyPI Publisher State

A pending PyPI publisher has been configured with:

```text
Pending project name: weppcloud-wbt
Publisher:            GitHub
Repository:           rogerlew/weppcloud-wbt
Workflow:             pypi-publish.yml
Environment name:     any
```

Implications:

- The publishing workflow filename must be `.github/workflows/pypi-publish.yml`.
- Because the PyPI publisher was configured with environment `(Any)`, the workflow does not need a matching GitHub environment name for the pending publisher to match. If a GitHub environment is later added for approval controls, verify PyPI publisher settings before publishing.
- The first successful trusted-publisher upload can create the PyPI project. Do not delay the first publish after announcing the package name, because another user registering the name first invalidates the pending publisher.

## Package Contract

Use this package split:

```text
PyPI distribution name:  weppcloud-wbt
Import package:          weppcloud_wbt
Packaged executable:     weppcloud_wbt/bin/whitebox_tools(.exe)
Compatibility shim:      whitebox_tools.py
```

Supported imports:

```python
from weppcloud_wbt.whitebox_tools import WhiteboxTools
from whitebox_tools import WhiteboxTools
```

The second import is required for compatibility with existing WhiteboxTools and WEPPcloud-style code. The top-level `whitebox_tools.py` shim should re-export the package wrapper:

```python
from weppcloud_wbt.whitebox_tools import *
```

## Non-Goals

- Do not require users to compile Rust during `pip install`.
- Do not publish a source-only package as the primary installation path.
- Do not replace Cargo as the maintainer/developer build system.
- Do not remove the existing `WBT/` deployment artifact path until WEPPpy deployment has moved to the PyPI package or has an explicit transition plan.
- Do not silently fall back to an unrelated system WhiteboxTools binary unless the user explicitly sets `WHITEBOX_TOOLS_EXE`.

## Initial Platform Targets

Publish wheels for:

```text
Linux x86_64
Windows amd64
macOS arm64
```

Later candidates:

```text
macOS x86_64
Linux aarch64
musllinux x86_64
```

Each wheel must contain exactly one executable for its platform.

## Critical Wheel Tag Requirement

The wheel contains a platform-specific executable but no Python extension module. A default setuptools build can accidentally produce a universal wheel such as:

```text
weppcloud_wbt-...-py3-none-any.whl
```

That is wrong. The wheel must be platform-tagged, for example:

```text
weppcloud_wbt-...-py3-none-manylinux_2_28_x86_64.whl
weppcloud_wbt-...-py3-none-win_amd64.whl
weppcloud_wbt-...-py3-none-macosx_14_0_arm64.whl
```

Implementation must explicitly force non-pure wheels. With setuptools, use a small `setup.py` override for `bdist_wheel`:

```python
from setuptools import setup
from wheel.bdist_wheel import bdist_wheel as _bdist_wheel


class bdist_wheel(_bdist_wheel):
    def finalize_options(self):
        super().finalize_options()
        self.root_is_pure = False


setup(cmdclass={"bdist_wheel": bdist_wheel})
```

The final implementation may choose another backend, but the acceptance gate is the same: built wheels must not be `*-none-any.whl`.

## Proposed Repository Layout

```text
pyproject.toml
setup.py                         # only if needed to force platform wheel tags
python/
  whitebox_tools.py              # compatibility shim
  weppcloud_wbt/
    __init__.py
    _paths.py
    whitebox_tools.py            # wrapper copied/adapted from current whitebox_tools.py
    bin/
      .gitkeep
```

The existing top-level `whitebox_tools.py` and `WBT/whitebox_tools.py` remain source/deployment artifacts until the migration is complete. The PyPI package should not import from `WBT/`.

## Executable Resolution

The package wrapper must resolve the executable deterministically:

1. If `WHITEBOX_TOOLS_EXE` is set, use that exact path.
2. Otherwise use `weppcloud_wbt/bin/whitebox_tools(.exe)` inside the installed package.
3. If neither exists, fail with a clear `FileNotFoundError`.

Do not implicitly use `shutil.which("whitebox_tools")` in the default path. An implicit system fallback can call the wrong upstream binary and hide packaging defects.

Suggested `_paths.py`:

```python
from __future__ import annotations

import os
import platform
from pathlib import Path


def whitebox_tools_exe() -> str:
    exe_name = "whitebox_tools.exe" if platform.system() == "Windows" else "whitebox_tools"

    env_path = os.environ.get("WHITEBOX_TOOLS_EXE")
    if env_path:
        return env_path

    packaged = Path(__file__).resolve().parent / "bin" / exe_name
    if packaged.exists():
        return str(packaged)

    raise FileNotFoundError(
        f"Could not find packaged {exe_name}. Set WHITEBOX_TOOLS_EXE "
        "or install a platform wheel for weppcloud-wbt."
    )
```

The wrapper constructor should derive `exe_path` and `exe_name` from this function:

```python
from os import path

from ._paths import whitebox_tools_exe

exe = whitebox_tools_exe()
self.exe_path = path.dirname(exe)
self.exe_name = path.basename(exe)
```

## Minimal `pyproject.toml`

Use setuptools first. Keep the first implementation boring and transparent.

```toml
[build-system]
requires = ["setuptools>=77", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "weppcloud-wbt"
version = "2.4.0.post1"
description = "WEPPcloud WhiteboxTools fork with TOPAZ-style watershed preprocessing tools"
readme = "README.md"
requires-python = ">=3.9"
license = "MIT"
authors = [
  { name = "Roger Lew", email = "rogerlew@gmail.com" }
]
keywords = [
  "WEPP",
  "WEPPcloud",
  "WhiteboxTools",
  "TOPAZ",
  "hydrology",
  "watershed delineation",
  "geomorphometry"
]
classifiers = [
  "Programming Language :: Python :: 3",
  "Programming Language :: Rust",
  "Operating System :: Microsoft :: Windows",
  "Operating System :: POSIX :: Linux",
  "Operating System :: MacOS",
  "Topic :: Scientific/Engineering :: GIS",
]

[project.urls]
Homepage = "https://github.com/rogerlew/weppcloud-wbt"
Repository = "https://github.com/rogerlew/weppcloud-wbt"
Documentation = "https://github.com/rogerlew/weppcloud-wbt/tree/master/docs"

[tool.setuptools]
package-dir = {"" = "python"}

[tool.setuptools.packages.find]
where = ["python"]

[tool.setuptools.package-data]
weppcloud_wbt = [
  "bin/whitebox_tools",
  "bin/whitebox_tools.exe"
]
```

Version policy:

- Align the public package version with the WhiteboxTools application version where possible.
- Use `.postN` for packaging-only rebuilds that do not change Rust behavior.
- Use an ordinary patch/minor version bump when Rust tools, wrapper contracts, or packaged behavior change.

## Build Workflow Shape

Initial workflow name:

```text
.github/workflows/pypi-publish.yml
```

Triggers:

```yaml
on:
  workflow_dispatch:
  push:
    tags:
      - "v*"
```

High-level job flow:

```text
checkout
set up Rust
set up Python
build Rust CLI in release mode
copy executable into python/weppcloud_wbt/bin/
build a platform-tagged wheel
test the installed wheel
upload wheel artifact
publish from a final job through PyPI trusted publishing
```

The trusted-publishing job must request an OIDC token:

```yaml
permissions:
  id-token: write
  contents: read
```

Use `pypa/gh-action-pypi-publish` without username/password when publishing through the trusted publisher.

## Suggested Manual Matrix

Start with a direct matrix rather than a complicated cross-compilation setup:

```yaml
strategy:
  fail-fast: false
  matrix:
    include:
      - os: ubuntu-22.04
        exe: whitebox_tools
        target_dir: target/release
      - os: windows-2022
        exe: whitebox_tools.exe
        target_dir: target/release
      - os: macos-14
        exe: whitebox_tools
        target_dir: target/release
```

Linux should be checked especially carefully because the current local release binary links dynamically to system libraries:

```text
libproj.so.25
libsqlite3.so.0
libtiff.so.6
libcurl-gnutls.so.4
libstdc++.so.6
```

That was observed locally with:

```bash
ldd target/release/whitebox_tools
```

This is a release risk. A wheel built on GitHub's Ubuntu runner may fail on user machines if required shared libraries are missing or incompatible. The Linux wheel plan must either:

1. produce a self-contained binary,
2. rely on `auditwheel` repair through `cibuildwheel` where possible, or
3. document and enforce system package prerequisites, accepting that the wheel is not fully self-contained.

Preference: make the Linux wheel self-contained enough that ordinary Python users do not need to install PROJ manually.

## Test Requirements

Each wheel build must install the produced wheel into a clean environment and verify:

```bash
python -c "from weppcloud_wbt.whitebox_tools import WhiteboxTools; print(WhiteboxTools().version())"
python -c "from whitebox_tools import WhiteboxTools; print(WhiteboxTools().version())"
python -m pytest -q tests/test_ifolp_wrapper_smoke.py
```

Also verify executable tool metadata from the installed package:

```bash
python - <<'PY'
from weppcloud_wbt.whitebox_tools import WhiteboxTools

wbt = WhiteboxTools()
tools = wbt.list_tools()
assert "IterativeFirstOrderLinkPrune" in tools
assert "RemoveShortStreams" in tools
PY
```

If the wrapper API does not currently return tool lists in a test-friendly form, add a minimal test helper rather than parsing user-facing console output.

## Acceptance Criteria

The PyPI packaging implementation is complete when:

- `pip install weppcloud-wbt` installs a working wrapper and executable on each supported platform.
- Built wheels are platform-tagged, not `py3-none-any`.
- The top-level compatibility import works: `from whitebox_tools import WhiteboxTools`.
- The namespace import works: `from weppcloud_wbt.whitebox_tools import WhiteboxTools`.
- `WHITEBOX_TOOLS_EXE` can override the packaged executable for developer/debug workflows.
- Without `WHITEBOX_TOOLS_EXE`, the wrapper uses the packaged executable.
- Installed-wheel tests confirm the fork-specific tools are present, including `HillslopesTopaz`, `FVSlope`, `RaiseRoads`, `IterativeFirstOrderLinkPrune`, and `RemoveShortStreams`.
- Linux dependency handling is explicitly validated with `ldd` and installed-wheel smoke tests.
- Publishing uses PyPI trusted publishing through `.github/workflows/pypi-publish.yml`.
- A release tag, GitHub release, and PyPI version all refer to the same source commit.

## Migration Notes for WEPPpy

WEPPpy currently executes a repository-local binary path:

```text
/workdir/weppcloud-wbt/WBT/whitebox_tools
```

Do not remove or stop updating that artifact until WEPPpy has an approved migration path. A future WEPPpy migration can choose one of:

```text
Use PyPI package executable via weppcloud_wbt._paths.whitebox_tools_exe()
Set WHITEBOX_TOOLS_EXE explicitly in deployment config
Continue using /workdir/weppcloud-wbt/WBT/whitebox_tools for production only
```

The PyPI work should be additive first. Operational deployment can migrate after wheel builds have been validated on production-like Linux hosts.
