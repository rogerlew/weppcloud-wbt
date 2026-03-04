# ExecPlan: RaiseRoads Tool (WEPPcloud-WBT)

Status: Completed  
Owner: Codex/agent  
Repository: `/workdir/weppcloud-wbt`  
Primary target: `whitebox-tools-app/src/tools/hydro_analysis/raise_roads.rs`

## 1. Objective

Implement a new WhiteboxTools tool, `RaiseRoads`, for road embankment synthesis on DEMs, with complete registration, bindings, validation, and review.

The tool must support road-based DEM raising strategies needed by TerrainProcessor workflows and be validated with real run resources:
- DEM: `/wc1/runs/sh/shaven-lane/dem/dem.tif`
- Roads: `/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson`

## 2. Scope

In scope:
- Add a new Rust tool in `Hydrological Analysis`: `RaiseRoads`.
- Register the tool in toolbox and global tool manager.
- Add Python wrappers in:
  - `whitebox_tools.py`
  - `WBT/whitebox_tools.py`
- Add usage/docs metadata consistent with existing tools.
- Add tests (unit/smoke) and a reproducible validation script/commands.
- Run review and validation gates, fix findings, and finalize.

Out of scope for this plan:
- TerrainProcessor integration in `wepppy` (separate repo/work package).
- Full UI wiring in WEPPcloud.
- Performance tuning beyond correctness + basic practical runtime.

## 3. Functional Requirements

### 3.1 Tool Interface

Proposed command:
- `--dem` (required): input DEM raster.
- `--roads` (required): input roads line vector.
- `--output` (required): output raised DEM raster.
- `--strategy` (optional, default `profile_relative`):
  - `constant`
  - `profile_relative`
  - `cross_section`
- `--road_width` (optional, map units): fallback width.
- `--width_field` (optional): feature attribute name for width.
- `--height` (optional, default `5.0`): used by `constant` strategy.
- `--margin` (optional, default `2.0`): used by `profile_relative`.
- `--search_radius` (optional): local terrain query radius; default derived from width.
- `--taper` (optional, default `cosine`): edge taper mode.

Cross-section parameters (defaults + per-feature overrides):
- `--crown_width`, `--shoulder_width`, `--shoulder_slope`, `--backslope_angle`
- Optional fields via GeoJSON attributes (e.g., `crown_width_m`, `shoulder_width_m`, `shoulder_slope`, `backslope_angle_deg`).

### 3.2 Behavioural Guarantees

- Never lower DEM elevations (`output >= input` cell-wise).
- Apply modification only within resolved road influence width.
- Respect per-feature width when available, else fallback hierarchy:
  1. width field
  2. road class heuristic
  3. `--road_width` default
- Preserve NoData handling and metadata conventions.
- Emit reproducible metadata entries describing strategy and key parameters.

### 3.3 Strategy Semantics

`constant`:
- Raise by uniform `height` inside road influence mask, with taper at edges.

`profile_relative`:
- For candidate cells near roads, estimate local terrain max within radius.
- Compute target elevation = `local_max + margin`.
- Raise toward target with taper; clamp to no-lowering behaviour.

`cross_section`:
- Build parametric raised profile around centerline.
- For unpaved/track-like roads with missing inventory attributes, apply conservative default template.
- Allow GeoJSON attribute overrides when present.
- If cross-section profile cannot be resolved for a feature, fallback to `profile_relative` for that feature.

## 4. Implementation Map

Primary files:
- New: `whitebox-tools-app/src/tools/hydro_analysis/raise_roads.rs`
- Update: `whitebox-tools-app/src/tools/hydro_analysis/mod.rs`
- Update: `whitebox-tools-app/src/tools/mod.rs`
- Update: `whitebox_tools.py`
- Update: `WBT/whitebox_tools.py`

Reference implementations:
- `whitebox-tools-app/src/tools/hydro_analysis/raise_walls.rs`
- `whitebox-tools-app/src/tools/hydro_analysis/burn_streams_at_roads.rs`
- `whitebox-tools-app/src/tools/terrain_analysis/embankment_mapping.rs`
- `DEVELOPING_TOOLS.md`

## 5. Milestones and Acceptance Criteria

### M0: Baseline and Design Lock
Tasks:
- Read referenced tools and settle final CLI parameter names/defaults.
- Record decisions in Decision Log.

Acceptance:
- Final parameter list and defaults documented in this ExecPlan.

### M1: Rust Tool Skeleton + Registration
Tasks:
- Create `raise_roads.rs` tool struct, parameters, argument parsing, usage, metadata.
- Register in hydro toolbox (`mod.rs`) and global manager (`tools/mod.rs`).

Acceptance:
- Tool appears in tool list and dispatch resolves `raise_roads`.
- `cargo check -p whitebox_tools` passes.

### M2: Core Algorithms
Tasks:
- Implement road rasterization/path influence logic.
- Implement `constant` and `profile_relative` strategies.
- Implement no-lowering guarantee and taper.

Acceptance:
- CLI run produces output raster with same geometry as DEM.
- Cell-wise assertion passes: no output cell lower than input.

### M3: Cross-Section + Fallbacks
Tasks:
- Implement cross-section profile handling.
- Add conservative unpaved fallback template.
- Add per-feature GeoJSON attribute overrides.
- Add per-feature fallback to `profile_relative` when parameters are incomplete.

Acceptance:
- Cross-section mode runs on mixed-attribution roads.
- Fallback events are deterministic and discoverable in logs/metadata.

### M4: Python Bindings + Docs
Tasks:
- Add wrappers in both Python binding files.
- Ensure argument names and defaults match Rust tool.
- Add concise docs/usage updates as needed.

Acceptance:
- Python wrapper executes tool successfully on validation dataset.

### M5: Validation and Regression Safety
Tasks:
- Run compile/tests + smoke validations.
- Validate on shaven-lane dataset.
- Verify output stats and no-lowering constraints.

Acceptance:
- Validation matrix in Section 6 fully green.

### M6: Review, Fixes, and Handoff
Tasks:
- Perform correctness review and maintainability review.
- Address review findings.
- Update Progress, Surprises, Decision Log, Outcomes.

Acceptance:
- No open critical review findings.
- ExecPlan updated with final evidence and commands.

## 6. Validation Matrix

### 6.1 Build and Static Validation
- `cargo check -p whitebox_tools`
- `cargo test -p whitebox_tools`
- `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py`

### 6.2 CLI Smoke Validation (real data)

Workspace:
- Output dir (example): `/tmp/raise_roads_shaven_lane`

Inputs:
- DEM: `/wc1/runs/sh/shaven-lane/dem/dem.tif`
- Roads: `/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson`

Commands (examples):
```bash
mkdir -p /tmp/raise_roads_shaven_lane

/workdir/weppcloud-wbt/WBT/whitebox_tools \
  --run=raise_roads \
  --dem=/wc1/runs/sh/shaven-lane/dem/dem.tif \
  --roads=/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson \
  --output=/tmp/raise_roads_shaven_lane/dem_roads_profile.tif \
  --strategy=profile_relative \
  --margin=2.0 \
  --road_width=5.0 \
  -v

/workdir/weppcloud-wbt/WBT/whitebox_tools \
  --run=raise_roads \
  --dem=/wc1/runs/sh/shaven-lane/dem/dem.tif \
  --roads=/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson \
  --output=/tmp/raise_roads_shaven_lane/dem_roads_constant.tif \
  --strategy=constant \
  --height=5.0 \
  --road_width=5.0 \
  -v
```

### 6.3 Python Wrapper Validation

```python
from whitebox_tools import WhiteboxTools
wbt = WhiteboxTools()
wbt.raise_roads(
    dem="/wc1/runs/sh/shaven-lane/dem/dem.tif",
    roads="/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson",
    output="/tmp/raise_roads_shaven_lane/dem_roads_py.tif",
    strategy="profile_relative",
    margin=2.0,
    road_width=5.0,
)
```

### 6.4 Result Checks
- Output exists and opens.
- Output dimensions/transform/CRS equal input DEM.
- NoData handling preserved.
- No lowering: `min(output - input)` over valid cells is `>= 0`.
- Non-trivial modifications exist near roads.

## 7. Review Checklist

Correctness:
- Parameter parsing accepts both `--flag=value` and `--flag value`.
- Road geometry type checks are correct.
- Width and strategy fallback logic is deterministic.
- No-lowering guarantee enforced in all code paths.

Integration:
- Tool is listed in tool manager and callable by lowercase command.
- Python wrappers present in both binding files.
- Metadata entries include strategy and primary parameters.

Maintainability:
- Naming, style, and error messages align with repository conventions.
- Complex blocks include brief, meaningful comments.
- Avoid broad exception swallowing in Python wrappers.

## 8. Risks and Mitigations

Risk: Cross-section scope balloons.  
Mitigation: Keep v1 cross-section minimal, deterministic, no-lowering, and fallback-heavy.

Risk: Runtime is too slow on large DEMs.  
Mitigation: validate first on shaven-lane; profile only if correctness is achieved.

Risk: Road attribute heterogeneity.  
Mitigation: strict fallback hierarchy + metadata reporting of applied defaults.

## 9. Progress

- [x] M0 Baseline and design lock
- [x] M1 Rust skeleton + registration
- [x] M2 Core algorithms
- [x] M3 Cross-section + fallbacks
- [x] M4 Python bindings + docs
- [x] M5 Validation and regression safety
- [x] M6 Review, fixes, and handoff

Implementation checkpoint:
- Added `RaiseRoads` Rust tool with strategy implementations (`constant`, `profile_relative`, `cross_section`) and enforced no-lowering clamp on all write paths.
- Added width fallback hierarchy and GeoJSON/shapefile ingestion with attribute heuristics and overrides.
- Added conservative unpaved cross-section defaults and per-feature fallback to `profile_relative` when cross-section params are invalid.
- Added CRS-aware road handling: infer source EPSG (GeoJSON `crs`, shapefile projection text, or lon/lat bounds heuristic), infer DEM EPSG, and reproject roads to DEM CRS when they differ.
- Added reprojection metadata/logging (`Road source CRS`, source/target EPSG, transformed point count, reprojection flag).
- Registered tool in hydro analysis module and global tool manager dispatch.
- Added Python wrapper methods in both binding files.
- Added unit coverage for taper bounds, width heuristic, cross-section non-negative raise, singleton-safe progress math, and EPSG parsing regressions.
- Added low-friction fixture tooling:
  - `tools/prepare_raise_roads_fixture.py` to materialize a portable fixture from run data sources.
  - `tools/validate_raise_roads_fixture.py` to run reproducible smoke assertions (reprojection, no-lowering, non-zero modifications).
  - fixture assets under `test_fixtures/raise_roads_exogamous_shavenlane/` (`dem_clip.tif`, `roads.geojson`, `manifest.json`, `README.md`).

## 10. Surprises and Discoveries

- Existing tools already include GeoJSON parsing patterns (`Watershed`, `HillslopesTopaz`) via the `geojson` crate; `RaiseRoads` can follow this path for line-feature ingestion.
- Validation inputs appear to have mixed spatial references/location ranges (DEM and roads are not obviously co-located by extent). Implementation will preserve explicit behaviour and surface overlap statistics in metadata/logging for reproducibility.
- Confirmed with numeric checks: DEM is `EPSG:32611` with bounds near `(555710, 5028225, 563720, 5036055)` while the provided roads GeoJSON contains lon/lat coordinates near `(-123.469, 45.358, -123.456, 45.367)`. On the required inputs, no spatial overlap exists, so outputs are unchanged (`modified_cells=0`) by design.
- Even after automatic reprojection from `EPSG:4326` to `EPSG:32611`, required-input roads remain outside DEM bounds (projected road bounds near `(-6654, 5043075, -5551, 5044043)`), confirming a true location mismatch rather than only a CRS mismatch.
- Python wrapper smoke run from repo root hit an existing local `settings.json` schema mismatch (`KeyError: 'raise_on_error'`). Running wrapper smoke from `/tmp` (same code path, no local settings override) succeeds with return code `0`.
- A clipped fixture can remain representative while staying tiny: the cross-run fixture clip is only `124x125` pixels (`63 KB`) and still preserves reprojection + non-zero modification behavior.

## 11. Decision Log

- `RaiseRoads` CLI defaults locked:
  - `strategy=profile_relative`
  - `height=5.0`
  - `margin=2.0`
  - `road_width` fallback default: `5.0` map units
  - `taper=cosine`
  - `crown_width=4.0`, `shoulder_width=1.0`, `shoulder_slope=0.08`, `backslope_angle=30.0`
- Supported strategy tokens: `constant`, `profile_relative`, `cross_section` (case-insensitive).
- Taper modes: `cosine`, `linear`, `none`; unknown values error fast.
- Width fallback hierarchy fixed as:
  1. feature width from `width_field` when supplied and numeric/parseable
  2. road-class heuristic from common class/surface attributes
  3. `--road_width`, else internal default (`5.0`)
- `cross_section` per-feature fallback behaviour fixed:
  - If parameters are invalid/unresolvable for a feature, route that feature through `profile_relative`.
  - For unpaved/track-like roads without explicit profile attributes, apply conservative template defaults.
- GeoJSON attribute overrides accepted for:
  - `crown_width_m`
  - `shoulder_width_m`
  - `shoulder_slope`
  - `backslope_angle_deg`
  - plus any user-specified `--width_field`.
- Reprojection policy fixed:
  - if both road EPSG and DEM EPSG are known and differ, reproject all road vertices to DEM CRS;
  - if EPSG is unknown on either side, skip reprojection and preserve explicit warning/metadata.
- EPSG parsing hardened for WKT:
  - ignore known non-CRS EPSG unit/axis codes (e.g., `9001`) so CRS detection does not misidentify projection codes.
- No-overlap handling is explicit and non-destructive:
  - tool writes an unchanged DEM when no roads intersect valid DEM cells;
  - verbose mode prints an explicit warning to guide CRS/location troubleshooting.
- Correctness fix applied after review:
  - replaced inline progress percentage division with `progress_percent(index, total)` helper to prevent divide-by-zero on singleton part/row cases;
  - added test `test_progress_percent_handles_singleton_total`.
- Fixture policy fixed for future low-friction development:
  - maintain a stable fixture pairing (`exogamous-nimbleness` DEM + `shaven-lane` roads) under `test_fixtures`;
  - include a manifest with source paths/checksums and CRS/bounds provenance;
  - validate fixture behavior via script-level assertions instead of ad hoc manual checks.

## 12. Outcomes and Retrospective

Validation evidence:
- `cargo check -p whitebox_tools` passed.
- `cargo test -p whitebox_tools` passed (`12 passed; 0 failed`).
- `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py` passed.
- CLI smoke (`profile_relative`) passed on required inputs and logged reprojection: `Reprojected roads from EPSG:4326 to EPSG:32611 (191 vertices).`
- CLI smoke (`constant`) passed on required inputs and logged reprojection: `Reprojected roads from EPSG:4326 to EPSG:32611 (191 vertices).`
- Python wrapper smoke passed (`wrapper_return_code=0`).

Result checks on required dataset (`/tmp/raise_roads_shaven_lane` outputs):
- Output files exist and open.
- Output geometry matches input DEM (`shape/transform/CRS` all match).
- No-lowering guarantee holds (`min_diff=0.0`, `lowered_cells=0`).
- Non-zero modifications near roads: **not observed on the required inputs** because provided DEM and roads do not overlap spatially (CRS/extent mismatch).

Additional validation on corrected run dataset (`/wc1/runs/ex/exogamous-nimbleness`):
- DEM (`EPSG:32610`) and roads (`EPSG:4326`) were correctly auto-reprojected (`4326 -> 32610`) with `191` transformed vertices.
- CLI smoke (`profile_relative`) and (`constant`) both passed with output writes and no overlap warnings.
- Python wrapper smoke passed (`wrapper_return_code=0`).
- Result checks passed with non-zero modifications and no lowering:
  - `profile_relative`: `modified_cells=607`, `min_diff=0.0`, `max_diff=17.36676025390625`, `lowered_cells=0`
  - `constant`: `modified_cells=607`, `min_diff=0.0`, `max_diff=4.99114990234375`, `lowered_cells=0`

Fixture tooling validation:
- `python tools/prepare_raise_roads_fixture.py --overwrite` passed and wrote:
  - `test_fixtures/raise_roads_exogamous_shavenlane/dem_clip.tif`
  - `test_fixtures/raise_roads_exogamous_shavenlane/roads.geojson`
  - `test_fixtures/raise_roads_exogamous_shavenlane/manifest.json`
- `python tools/validate_raise_roads_fixture.py` passed with assertions green for `profile_relative` and `constant` (`modified_cells=607`, `lowered_cells=0`).

Review outcomes:
- Correctness review found two high-severity issues and both were fixed:
  - progress divide-by-zero for singleton totals (fixed via `progress_percent` helper + regression test);
  - EPSG parser could select non-CRS unit codes such as `EPSG:9001` from WKT (fixed by filtering known non-CRS codes + regression test).
- Maintainability review simplified duplicated progress math via helper function and kept CRS inference/reprojection localized to dedicated helper functions.
- No remaining critical/high findings in the implemented RaiseRoads path.
