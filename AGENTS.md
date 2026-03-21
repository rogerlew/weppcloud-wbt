# AGENTS.md
> AI coding agent guide for `/workdir/weppcloud-wbt`

## Purpose
- Provide high-signal execution guidance for coding agents in this repository.
- Keep instructions concise; put task-specific detail in `prompts/*.md` exec plans.

## Active ExecPlan
- `none` (most recently completed: `prompts/RAISE_ROADS_EXECPLAN.md`)

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
  - `cargo check -p whitebox_tools`
  - `cargo test -p whitebox_tools`
- Python wrapper sanity:
  - `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py`
- Real-data smoke test inputs for RaiseRoads:
  - DEM: `/wc1/runs/sh/shaven-lane/dem/dem.tif`
  - Roads: `/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson`

## Deliverables for RaiseRoads Work
- Code changes implementing tool + registration + wrappers.
- Validation evidence (commands and key outputs).
- Updated exec plan sections with final outcomes.
