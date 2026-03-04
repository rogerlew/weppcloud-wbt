# Prompt: Execute RaiseRoads End-to-End (Create, Review, Test)

You are implementing `RaiseRoads` in `/workdir/weppcloud-wbt`.

## Mandatory startup
1. Read `/workdir/weppcloud-wbt/AGENTS.md`.
2. Read `/workdir/weppcloud-wbt/prompts/RAISE_ROADS_EXECPLAN.md` completely.
3. Execute the plan milestone-by-milestone without skipping validation.

## Goal
Deliver a production-ready `RaiseRoads` WhiteboxTools command with:
- Rust tool implementation
- registration wiring
- Python wrappers (`whitebox_tools.py` and `WBT/whitebox_tools.py`)
- validation on real data
- review + fixes
- plan updates and handoff notes

## Required implementation scope
Implement tool behavior and interface from the active exec plan:
- strategies: `constant`, `profile_relative`, `cross_section`
- no-lowering guarantee (`output >= input` for valid cells)
- width/parameter fallback hierarchy
- unpaved-road conservative cross-section fallback
- GeoJSON attribute overrides where specified

Primary code targets:
- `whitebox-tools-app/src/tools/hydro_analysis/raise_roads.rs` (new)
- `whitebox-tools-app/src/tools/hydro_analysis/mod.rs`
- `whitebox-tools-app/src/tools/mod.rs`
- `whitebox_tools.py`
- `WBT/whitebox_tools.py`

Reference code:
- `whitebox-tools-app/src/tools/hydro_analysis/raise_walls.rs`
- `whitebox-tools-app/src/tools/hydro_analysis/burn_streams_at_roads.rs`
- `whitebox-tools-app/src/tools/terrain_analysis/embankment_mapping.rs`
- `DEVELOPING_TOOLS.md`

## Real-data validation inputs
- DEM: `/wc1/runs/sh/shaven-lane/dem/dem.tif`
- Roads: `/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson`

Use output workspace:
- `/tmp/raise_roads_shaven_lane`

## Validation gates (must pass)
1. `cargo check -p whitebox_tools`
2. `cargo test -p whitebox_tools`
3. `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py`
4. CLI smoke tests for `profile_relative` and `constant`
5. Python wrapper smoke test
6. Result checks:
   - output exists and opens
   - geometry matches input
   - no-lowering condition holds
   - visible/non-zero modifications near roads

## Review requirements
Before final handoff:
1. Perform a correctness-focused self-review.
2. Perform a maintainability review and simplify where needed.
3. Fix all high/critical findings.
4. Record review outcomes in the exec plan.

## ExecPlan maintenance (required)
Continuously update in `/workdir/weppcloud-wbt/prompts/RAISE_ROADS_EXECPLAN.md`:
- `Progress`
- `Surprises and Discoveries`
- `Decision Log`
- `Outcomes and Retrospective`

Do not leave the plan stale.

## Handoff output format
At completion, provide:
1. Summary of what was implemented.
2. Exact files changed.
3. Validation commands run + key outputs.
4. Review findings and fixes.
5. Any residual risks or follow-up recommendations.
