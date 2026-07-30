# AGENTS.md
> AI coding agent guide for `/workdir/weppcloud-wbt`

## Purpose
- Provide high-signal execution guidance for coding agents in this repository.
- Keep instructions concise; put task-specific detail in `prompts/*.md` exec plans.

## Active ExecPlan
- None.

## Most Recent Completed ExecPlan
- `docs/work-packages/20260729_topaz_condition_dem_parity_hardening/prompts/completed/topaz_condition_dem_parity_hardening_execplan.md`

## Required Workflow for Active ExecPlan Work
1. Read this `AGENTS.md` first.
2. Read the active exec plan end-to-end before code changes.
3. Execute milestone by milestone; do not skip validation gates.
4. Keep the exec plan updated as a living artifact:
 - `Progress`
 - `Surprises and Discoveries`
 - `Decision Log`
 - `Outcomes and Retrospective`
5. If blocked by missing external dependencies/data, record the blocker in the plan and stop.
6. Keep `CHANGELOG.md` up to date with commit-level or grouped changes for repository work.
7. For plans under `docs/work-packages/`, update the package tracker and
   `PROJECT_TRACKER.md` before handoff.

## Tool Development Conventions
- Follow `DEVELOPING_TOOLS.md` for:
  - Rust tool placement and registration
  - Argument parsing conventions
  - Metadata output structure
  - Python wrapper updates in both bindings
- Prefer minimal, deterministic implementations before optimization.
- Preserve existing CLI and binding conventions.

## Validation Baseline
- Build/check:
  - `cargo check -p whitebox-tools-app`
  - `cargo test -p whitebox-tools-app`
- Python wrapper sanity:
  - `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py`
- Real-data smoke test input for TopazConditionDem:
  - DEM: `/wc1/runs/sr/srivas42-reconciled-turf/dem/dem.tif`

## Deliverables for TopazConditionDem Work
- Faithful Rust implementation of TOPAZ FILDEP and RELIEF conditioning.
- Exact-input and synthetic golden parity fixtures.
- Registered CLI tool and both Python wrapper methods.
- Cut/fill diagnostics, metadata, tests, and algorithm documentation.
- Production validation evidence and a WEPPpy integration handoff.
