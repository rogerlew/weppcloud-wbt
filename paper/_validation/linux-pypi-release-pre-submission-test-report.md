# Validation Report: Linux PyPI Release Pre-Submission Test

**Date:** June 9, 2026  
**Package:** `weppcloud-wbt`  
**Version:** `2.3.0.post2`  
**Environment:** Linux (`forest`), Ubuntu kernel `6.8.0-111-generic`, x86_64, `uv` package manager

## 1. Executive Summary

The `weppcloud-wbt` package release `2.3.0.post2` was validated on Linux from a fresh PyPI install in `/tmp`. The wheel installs successfully, the bundled `whitebox_tools` executable launches, repaired shared-library loading works, and both supported Python import surfaces pass wrapper smoke checks.

## 2. PyPI Release Verification

A clean installation was performed using `uv` and CPython 3.11.14.

### 2.1 Environment Setup

```bash
root=/tmp/weppcloud-wbt-pypi-linux-20260609-235447
uv venv --python 3.11 "$root/.venv"
uv pip install --python "$root/.venv/bin/python" --upgrade --refresh weppcloud-wbt
uv pip show --python "$root/.venv/bin/python" weppcloud-wbt
```

Observed result:

```text
Name: weppcloud-wbt
Version: 2.3.0.post2
Location: /tmp/weppcloud-wbt-pypi-linux-20260609-235447/.venv/lib/python3.11/site-packages
```

## 3. Smoke Test Results

- **Installation:** PASS
- **Direct executable startup:** PASS
- **Version verification:** `WhiteboxTools v2.4.0`
- **Tool enumeration:** PASS, 468 tools registered
- **Compatibility import:** PASS, `from whitebox_tools import WhiteboxTools`
- **Namespace import:** PASS, `from weppcloud_wbt import WhiteboxTools`
- **Bundled executable path:** PASS, `weppcloud_wbt/bin/whitebox_tools`
- **Executable permission:** PASS
- **Repaired dependency payload:** PASS, `weppcloud_wbt.libs/libsqlite3-4afd0a63.so.0.8.6` present and used by `ldd`

### 3.1 Direct Executable Checks

```bash
/tmp/weppcloud-wbt-pypi-linux-20260609-235447/.venv/lib/python3.11/site-packages/weppcloud_wbt/bin/whitebox_tools --version
/tmp/weppcloud-wbt-pypi-linux-20260609-235447/.venv/lib/python3.11/site-packages/weppcloud_wbt/bin/whitebox_tools --listtools
```

Observed result:

```text
WhiteboxTools v2.4.0 (c) Dr. John Lindsay 2017-2025
LISTTOOLS_EXIT=0
LISTTOOLS_FIRST_LINE=All 468 Available Tools:
LISTTOOLS_LINES=938
```

### 3.2 Shared Dependency Inspection

`ldd` confirmed that the executable resolves the repaired SQLite dependency from the wheel payload:

```text
libsqlite3-4afd0a63.so.0.8.6 => /tmp/weppcloud-wbt-pypi-linux-20260609-235447/.venv/lib/python3.11/site-packages/weppcloud_wbt/bin/../../weppcloud_wbt.libs/libsqlite3-4afd0a63.so.0.8.6
```

Other dependencies resolved from the base Linux system included `libstdc++.so.6`, `libgcc_s.so.1`, `libm.so.6`, and `libc.so.6`.

### 3.3 Python Wrapper Checks

The Python smoke test was run from `/tmp/weppcloud-wbt-pypi-linux-20260609-235447` so imports could not resolve to checkout files.

```python
from whitebox_tools import WhiteboxTools as CompatWhiteboxTools
from weppcloud_wbt import WhiteboxTools as NamespaceWhiteboxTools
```

Observed result:

```text
PACKAGE_VERSION=2.3.0.post2
BINARY_EXISTS=True
BINARY_EXECUTABLE=True
REPAIRED_LIBS_EXISTS=True
REPAIRED_LIBS=libsqlite3-4afd0a63.so.0.8.6
COMPAT_TOOL_COUNT=468
COMPAT_MISSING_REQUIRED=[]
NAMESPACE_TOOL_COUNT=468
NAMESPACE_MISSING_REQUIRED=[]
FIND_OUTLET_HELP_NONEMPTY=True
FIND_OUTLET_HELP_HAS_D8=True
PYTHON_SMOKE_EXIT=0
```

### 3.4 Custom Tool Availability

The smoke test confirmed the following WEPPcloud-specific tools are registered through both import surfaces:

- `HillslopesTopaz`
- `FVSlope`
- `RaiseRoads`
- `IterativeFirstOrderLinkPrune`
- `RemoveShortStreams`
- `FindOutlet`

The `FindOutlet` help output was non-empty and included the expected `--d8_pntr` argument.

## 4. Note on Test Isolation

An initial wrapper smoke attempt was launched from the repository checkout and picked up a local `settings.json` file. The final reported smoke was rerun from `/tmp/weppcloud-wbt-pypi-linux-20260609-235447`, matching the installed-wheel isolation requirement and avoiding checkout-local configuration.

## 5. Conclusion

The `weppcloud-wbt` v2.3.0.post2 Linux PyPI wheel is ready for submission and production use on Linux. The wheel installs cleanly from PyPI, launches the packaged executable, resolves its repaired shared dependency payload, and exposes the required WEPPcloud tools through both Python wrapper surfaces.
