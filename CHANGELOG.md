# Changelog

## Unreleased
- Added PyPI installation instructions to `README.md`, including `pip install weppcloud-wbt`, a version smoke test, and import examples for both wrapper entry points.
- Updated `paper/paper.md` to explicitly state PyPI availability as `weppcloud-wbt` for macOS, Windows, and Linux.
- Updated `setup.py` distribution metadata to mark wheels as binary distributions (`has_ext_modules=True`) so packaged native executables are emitted in platlib-compatible paths and can be processed by `auditwheel`.
- Added `patchelf` to Ubuntu CI dependencies so the Linux `auditwheel repair` step can run successfully.
- Added a Linux `auditwheel repair` step in the PyPI workflow so Linux wheels are retagged from unsupported `linux_x86_64` to PyPI-accepted `manylinux` platform tags before publish.
- Upgraded GitHub Actions in the PyPI workflow to current majors (`actions/checkout@v5`, `actions/setup-python@v6`, `actions/upload-artifact@v6`, `actions/download-artifact@v6`) and removed the temporary `FORCE_JAVASCRIPT_ACTIONS_TO_NODE24` override.
- Reduced macOS Homebrew warning noise in the PyPI workflow by avoiding `brew update` during CI dependency install.
- Updated PyPI workflow publish gating: tag pushes (`v*`) still auto-publish, and manual `workflow_dispatch` runs can now publish when the new `publish` input is set to true.
- Updated Windows PyPI workflow dependency setup to export vcpkg `LIB`/`INCLUDE`/`RUSTFLAGS` plus SQLite-specific env vars and a preflight `sqlite3.lib` presence check to prevent `LNK1181` at link time.
- Added a Windows-safe fallback in the PyPI workflow installed-wheel validation: when `list_tools()` returns zero parsed entries, CI now verifies required tools via `tool_help()` instead of failing on parser-specific output differences.
- Hardened PyPI workflow tool-presence validation to check normalized `list_tools()` keys (snake_case/CamelCase tolerant) and print targeted diagnostics when required tools are missing.
- Added a packaged top-level `whitebox_tools` shim package under `python/whitebox_tools/` so `from whitebox_tools import WhiteboxTools` works from installed wheels.
- Fixed wheel packaging metadata to include the top-level `whitebox_tools` compatibility module so installed-wheel imports work across Linux/macOS/Windows.
- Added `.github/workflows/pypi-publish.yml` to build platform wheels in a Linux/macOS/Windows matrix, run installed-wheel import/tool smoke checks, aggregate all wheel artifacts, and publish tagged releases to PyPI via trusted publishing.
- Added initial Python packaging scaffold for wheel builds: `pyproject.toml`, `setup.py`, `python/whitebox_tools.py`, and `python/weppcloud_wbt/{__init__.py,_paths.py,whitebox_tools.py,bin/.gitkeep}`.
- Revised `docs/pypi-spec.md` to add explicit matrix artifact fan-in for publish, installed-wheel import-origin isolation checks, Linux strategy decision gates, stricter `WHITEBOX_TOOLS_EXE` validation, and full fork-tool acceptance assertions.
- Corrected pre-submission paper/release documentation evidence: package id references now use `whitebox-tools-app`, claims matrix reflects current IFOLP coverage, and `WhiteboxToolsTopazEmulator` evidence is scoped to the companion WEPPpy repository.
- Added direct `RemoveShortStreams --max_junctions=3` regression coverage and advertised the argument in tool parameter metadata.
- Fixed the `LICENSE.txt` "ammendments" typo.

## Backfilled history (since fork date)

### 2025-12-05
- [113c3f1](https://github.com/rogerlew/weppcloud-wbt/commit/113c3f1): fvslope

### 2026-01-07
- [caa01c8](https://github.com/rogerlew/weppcloud-wbt/commit/caa01c8): Fix VRT test fixtures and add acceptance test report
- [0672078](https://github.com/rogerlew/weppcloud-wbt/commit/0672078): Add VRT test fixtures and acceptance test report
- [13e8f86](https://github.com/rogerlew/weppcloud-wbt/commit/13e8f86): Add documentation for binary output option in PruneStrahlerStreamOrder tool
- [9dbc33d](https://github.com/rogerlew/weppcloud-wbt/commit/9dbc33d): Add VRT support for whitebox-raster
- [e455090](https://github.com/rogerlew/weppcloud-wbt/commit/e455090): Add binary output option to PruneStrahlerStreamOrder tool
- [75cf39e](https://github.com/rogerlew/weppcloud-wbt/commit/75cf39e): whitebox_tools module
- [79951ee](https://github.com/rogerlew/weppcloud-wbt/commit/79951ee): rev wbt
- [e654cbb](https://github.com/rogerlew/weppcloud-wbt/commit/e654cbb): Add VRT support specification review with findings and suggested edits
- [eed3744](https://github.com/rogerlew/weppcloud-wbt/commit/eed3744): Update task configuration in VSCode and enhance Cargo.toml with resolver and patch details
- [123f96f](https://github.com/rogerlew/weppcloud-wbt/commit/123f96f): Update README and documentation for VRT support and performance notes

### 2026-01-08
- [6a82784](https://github.com/rogerlew/weppcloud-wbt/commit/6a82784): epsilon guard rasters_share_geometry
- [908067c](https://github.com/rogerlew/weppcloud-wbt/commit/908067c): debuging
- [94900fb](https://github.com/rogerlew/weppcloud-wbt/commit/94900fb): minimal_2pixel_stream
- [82e4817](https://github.com/rogerlew/weppcloud-wbt/commit/82e4817): Fix HillslopesTopaz minimal stream handling

### 2026-01-10
- [2954199](https://github.com/rogerlew/weppcloud-wbt/commit/2954199): Improve hillslopes_topaz profiling and fixtures
- [88bea55](https://github.com/rogerlew/weppcloud-wbt/commit/88bea55): Update hillslopes_topaz spec and binary

### 2026-02-08
- [8c1b8f4](https://github.com/rogerlew/weppcloud-wbt/commit/8c1b8f4): Avoid persistent env snapshot in WhiteboxTools wrapper

### 2026-02-16
- [72b5772](https://github.com/rogerlew/weppcloud-wbt/commit/72b5772): feat: enhance UnnestBasins with hierarchy sidecar and faster order mapping

### 2026-02-17
- [930aabc](https://github.com/rogerlew/weppcloud-wbt/commit/930aabc): feat: add articles bibliography and include Lindsay 2015 and 2016 papers
- [c9b338f](https://github.com/rogerlew/weppcloud-wbt/commit/c9b338f): fix: clean up formatting in hillslopes_topaz and prune_strahler_order descriptions

### 2026-03-04
- [e3eea3b](https://github.com/rogerlew/weppcloud-wbt/commit/e3eea3b): Add RaiseRoads tool with CRS reprojection and fixture validation
- [d1ab6a0](https://github.com/rogerlew/weppcloud-wbt/commit/d1ab6a0): Track RaiseRoads prompt file and remove AGENTS prompt binding

### 2026-03-20
- [f0a1264](https://github.com/rogerlew/weppcloud-wbt/commit/f0a1264): Add RusleLsFactor terrain tool and bindings

### 2026-03-21
- [72a5130](https://github.com/rogerlew/weppcloud-wbt/commit/72a5130): whitebox_tools

### 2026-04-12
- [9f0af62](https://github.com/rogerlew/weppcloud-wbt/commit/9f0af62): docs: add iterative first-order link prune spec and integration plan
- [7cdd632](https://github.com/rogerlew/weppcloud-wbt/commit/7cdd632): ifolp: complete WP-00 parity harness baseline
- [a60398e](https://github.com/rogerlew/weppcloud-wbt/commit/a60398e): feat(ifolp): scaffold wp01 tool and parser coverage

### 2026-04-13
- [812e57f](https://github.com/rogerlew/weppcloud-wbt/commit/812e57f): ifolp: close WP-02 topology kernel and link WP-03 execplan
- [001f423](https://github.com/rogerlew/weppcloud-wbt/commit/001f423): ifolp: complete WP-03 phase A and prep WP-04 handoff
- [7efdb44](https://github.com/rogerlew/weppcloud-wbt/commit/7efdb44): ifolp: complete WP-04 pruning and prep WP-05 parity
- [4804b34](https://github.com/rogerlew/weppcloud-wbt/commit/4804b34): Finalize IFOLP WP-05 parity baseline and prep WP-06
- [9d51754](https://github.com/rogerlew/weppcloud-wbt/commit/9d51754): Close IFOLP WP-06 hardening and prepare WP-07
- [09158cf](https://github.com/rogerlew/weppcloud-wbt/commit/09158cf): Commit restored local WP-07/WP-08 prep changes
- [aeaff6e](https://github.com/rogerlew/weppcloud-wbt/commit/aeaff6e): ifolp: close WP-08 wrapper release readiness
- [fcbc6a6](https://github.com/rogerlew/weppcloud-wbt/commit/fcbc6a6): ifolp: disposition review findings and harden parsing/tests
- [9a0da39](https://github.com/rogerlew/weppcloud-wbt/commit/9a0da39): ifolp: tighten geometry contract and expand boundary coverage
- [aa60688](https://github.com/rogerlew/weppcloud-wbt/commit/aa60688): Fix IFOLP review dispositions and deterministic thread coverage
- [db95a79](https://github.com/rogerlew/weppcloud-wbt/commit/db95a79): Harden IFOLP cycle checks and pointer/header contracts
- [1130c84](https://github.com/rogerlew/weppcloud-wbt/commit/1130c84): Add IFOLP wrapper smoke tests and thread-path coverage
- [55bb3e3](https://github.com/rogerlew/weppcloud-wbt/commit/55bb3e3): Add IFOLP regression coverage for stale-skip and threaded errors
- [29bd796](https://github.com/rogerlew/weppcloud-wbt/commit/29bd796): Add IFOLP threshold-code input contract regression tests
- [fcf6040](https://github.com/rogerlew/weppcloud-wbt/commit/fcf6040): test(ifolp): add ESRI phase coverage and whitespace table parser cases
- [afd84fd](https://github.com/rogerlew/weppcloud-wbt/commit/afd84fd): fix(ifolp): close review findings and harden contracts/tests
- [3aab781](https://github.com/rogerlew/weppcloud-wbt/commit/3aab781): docs(ifolp): add end-user guide and README divergence entry
- [8dd4adf](https://github.com/rogerlew/weppcloud-wbt/commit/8dd4adf): IFOLP: add max_junctions support and WP-09 parity/docs

### 2026-04-14
- [6c68f42](https://github.com/rogerlew/weppcloud-wbt/commit/6c68f42): refactor: update WEPPpy integration plan for IFOLP implementation and compatibility checks
- [c07200a](https://github.com/rogerlew/weppcloud-wbt/commit/c07200a): Build WBT binary with IFOLP tool support
- [97f4b3f](https://github.com/rogerlew/weppcloud-wbt/commit/97f4b3f): docs: add WBT release build/install runbook

### 2026-04-19
- [4a41c71](https://github.com/rogerlew/weppcloud-wbt/commit/4a41c71): Fix IFOLP junction cap enforcement and add strained-gown fixture

### 2026-05-07
- [13cf926](https://github.com/rogerlew/weppcloud-wbt/commit/13cf926): rusle_ls_factor: add bounded small-defect noflow fallback

### 2026-05-26
- [42faf04](https://github.com/rogerlew/weppcloud-wbt/commit/42faf04): RusleLsFactor: mask residual no-flow cells and rebuild WBT release
- [55e3762](https://github.com/rogerlew/weppcloud-wbt/commit/55e3762): Ignore and untrack WBT __pycache__

### 2026-06-09
- [e9145af](https://github.com/rogerlew/weppcloud-wbt/commit/e9145af): Ignore copyrighted references
- [57774de](https://github.com/rogerlew/weppcloud-wbt/commit/57774de): WIP: JOSS paper first draft
- [1920bc8](https://github.com/rogerlew/weppcloud-wbt/commit/1920bc8): Add FindOutlet and HillslopesTopaz integration test wiring
- [934b22b](https://github.com/rogerlew/weppcloud-wbt/commit/934b22b): Add FVSlope integration tests and test wiring
- [12b8b83](https://github.com/rogerlew/weppcloud-wbt/commit/12b8b83): Fix cargo package id for whitebox-tools-app tests
- [58acb41](https://github.com/rogerlew/weppcloud-wbt/commit/58acb41): add stream junction identifier integration tests
- [caed2f2](https://github.com/rogerlew/weppcloud-wbt/commit/caed2f2): Add RaiseRoads integration tests with strategy coverage
- [4727dde](https://github.com/rogerlew/weppcloud-wbt/commit/4727dde): Add prune_strahler_order integration tests
- [ffe12ea](https://github.com/rogerlew/weppcloud-wbt/commit/ffe12ea): test: add clip-raster and watershed integration coverage
- [f32c1b4](https://github.com/rogerlew/weppcloud-wbt/commit/f32c1b4): test: align rotation-degree unit expectations to current implementation
- [68f8010](https://github.com/rogerlew/weppcloud-wbt/commit/68f8010): chore: capture metadata, references, and pending integration test prompts
- [c6c33de](https://github.com/rogerlew/weppcloud-wbt/commit/c6c33de): Add unnest_basins integration tests
- [9b63834](https://github.com/rogerlew/weppcloud-wbt/commit/9b63834): test(vrt): fix fixture paths and expand integration test coverage to all 14 valid VRT variants
- [6cd4a92](https://github.com/rogerlew/weppcloud-wbt/commit/6cd4a92): docs(paper): add claims-test-matrix mapping paper assertions to committed tests
- [b0513bf](https://github.com/rogerlew/weppcloud-wbt/commit/b0513bf): docs(paper): clarify WhiteboxTools legacy status and position weppcloud-wbt as active fork
- [41f072f](https://github.com/rogerlew/weppcloud-wbt/commit/41f072f): docs(paper): update paper.bib and paper.md with new references and improved descriptions of WEPPcloud functionality
- [f318b5f](https://github.com/rogerlew/weppcloud-wbt/commit/f318b5f): docs(paper): add figures to Software Design section
