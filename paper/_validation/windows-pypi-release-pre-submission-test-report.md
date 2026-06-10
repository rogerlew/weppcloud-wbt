# Validation Report: Windows PyPI Release Pre-Submission Test

**Date:** June 9, 2026  
**Package:** `weppcloud-wbt`  
**Version:** `2.3.0.post2`  
**Environment:** Windows 11 Home (`BLARHG`), AMD64, OpenSSH, `uv` package manager

## 1. Executive Summary

The `weppcloud-wbt` package release `2.3.0.post2` was validated on Windows from a fresh PyPI install. The repaired Windows wheel installs successfully, launches the bundled `whitebox_tools.exe`, includes the required PROJ runtime payload, and passes wrapper smoke checks through both supported Python import surfaces.

This validation specifically confirms the Windows repair for the prior `2.3.0.post1` failure mode where the wheel installed but the executable exited before startup because `proj_9.dll` was missing.

## 2. PyPI Release Verification

A clean installation was performed on `blarhg` using `uv` and CPython 3.11.15.

### 2.1 Environment Setup

```powershell
uv venv --python 3.11 C:\Users\roger\AppData\Local\Temp\weppcloud-wbt-pypi-rerun-20260609-235036\.venv
uv pip install --python C:\Users\roger\AppData\Local\Temp\weppcloud-wbt-pypi-rerun-20260609-235036\.venv\Scripts\python.exe --upgrade --refresh weppcloud-wbt
uv pip show --python C:\Users\roger\AppData\Local\Temp\weppcloud-wbt-pypi-rerun-20260609-235036\.venv\Scripts\python.exe weppcloud-wbt
```

Observed result:

```text
Name: weppcloud-wbt
Version: 2.3.0.post2
Location: C:\Users\roger\AppData\Local\Temp\weppcloud-wbt-pypi-rerun-20260609-235036\.venv\Lib\site-packages
```

## 3. Smoke Test Results

- **Installation:** PASS
- **Direct executable startup:** PASS
- **Version verification:** `WhiteboxTools v2.4.0`
- **Tool enumeration:** PASS, 468 tools registered
- **Compatibility import:** PASS, `from whitebox_tools import WhiteboxTools`
- **Namespace import:** PASS, `from weppcloud_wbt import WhiteboxTools`
- **Bundled PROJ DLL:** PASS, `weppcloud_wbt\bin\proj_9.dll` present
- **Bundled PROJ data:** PASS, `weppcloud_wbt\bin\proj\proj.db` present
- **Wrapper PROJ environment:** PASS, `PROJ_DATA` points to the bundled `weppcloud_wbt\bin\proj` directory

### 3.1 Direct Executable Checks

```powershell
whitebox_tools.exe --version
whitebox_tools.exe --listtools
```

Observed result:

```text
WhiteboxTools v2.4.0 (c) Dr. John Lindsay 2017-2025
LISTTOOLS_EXIT=0
LISTTOOLS_FIRST_LINE=All 468 Available Tools:
LISTTOOLS_LINES=469
```

### 3.2 Python Wrapper Checks

The smoke test verified both wrapper entry points:

```python
from whitebox_tools import WhiteboxTools as CompatWhiteboxTools
from weppcloud_wbt import WhiteboxTools as NamespaceWhiteboxTools
```

Observed result:

```text
PACKAGE_VERSION=2.3.0.post2
BINARY_EXISTS=True
PROJ_DLL_EXISTS=True
PROJ_DB_EXISTS=True
COMPAT_TOOL_COUNT=468
COMPAT_MISSING_REQUIRED=[]
NAMESPACE_TOOL_COUNT=468
NAMESPACE_MISSING_REQUIRED=[]
FIND_OUTLET_HELP_NONEMPTY=True
FIND_OUTLET_HELP_HAS_D8=True
PYTHON_SMOKE_EXIT=0
```

### 3.3 Custom Tool Availability

The smoke test confirmed the following WEPPcloud-specific tools are registered through both import surfaces:

- `HillslopesTopaz`
- `FVSlope`
- `RaiseRoads`
- `IterativeFirstOrderLinkPrune`
- `RemoveShortStreams`
- `FindOutlet`

The `FindOutlet` help output was non-empty and included the expected `--d8_pntr` argument.

## 4. Conclusion

The `weppcloud-wbt` v2.3.0.post2 Windows PyPI wheel is ready for submission and production use on Windows. The repaired wheel resolves the missing `proj_9.dll` startup failure and provides the bundled PROJ data needed by the packaged executable.
