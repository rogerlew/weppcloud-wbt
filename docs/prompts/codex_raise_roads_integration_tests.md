# Task: RaiseRoads integration test suite

## Goal

Author `whitebox-tools-app/src/tools/hydro_analysis/raise_roads_integration_tests.rs` and wire it into `raise_roads.rs`. Run `cargo test` to confirm all new tests pass before reporting done.

## Context

`RaiseRoads` (`hydro_analysis/raise_roads.rs`) conditions a DEM by raising cells near road centrelines to simulate embankment fill. It accepts a DEM raster and a roads vector (GeoJSON or Shapefile), and writes a conditioned DEM raster.

Three raise strategies are available via `--strategy`:

- **`constant`**: adds a fixed `--height` increment, tapered toward the road's edge
- **`profile_relative`**: raises cells to `local_terrain_max + --margin`, tapered; adapts to terrain slope rather than adding a flat increment
- **`cross_section`**: applies a geometric road cross-section profile (crown → shoulder → backslope); supports conservative unpaved mode and per-feature GeoJSON attribute overrides

The tool automatically reprojects roads to the DEM's CRS when the road source EPSG and DEM EPSG differ. The DEM EPSG is inferred from raster metadata; the road EPSG is inferred from GeoJSON `crs` property or bounding-box heuristics.

**The one unconditional invariant the paper explicitly guarantees:** valid DEM cells (non-nodata) are never lowered. The code enforces `if candidate < z { candidate = z; }` before writing any output value.

The existing `mod tests` block in `raise_roads.rs` contains unit tests for internal functions. Do not modify or replace it. Add the integration test module as a second `#[cfg(test)]` block at the bottom of the file using `#[path = ...]`.

## Patterns to follow

Read before writing:

- `hydro_analysis/hillslopes_topaz_integration_tests.rs` — canonical pattern (fixture paths via `CARGO_MANIFEST_DIR`, `temp_output_path`, `cleanup_output_path`, `Raster::new`)
- `hydro_analysis/fvslope_integration_tests.rs` — simpler two-input example of the same pattern

## Fixture

### `test_fixtures/raise_roads_exogamous_shavenlane/`

Files:
- `dem_clip.tif` — DEM in UTM (EPSG:32610)
- `roads.geojson` — road centrelines in WGS84 (EPSG:4326)
- `manifest.json` — provenance metadata; `crs.reprojection_expected = true` confirms the road-to-DEM CRS transformation is required for this fixture

Read `manifest.json` to understand the fixture's CRS details. The tool must reproject the WGS84 roads into UTM before rasterising them. If reprojection silently fails, no cells will be raised. The tests must verify that cells were actually modified to confirm reprojection worked end-to-end.

There is no pre-computed reference output raster for this fixture. Derive expected properties from the input DEM and the tool's documented behaviour.

## Required tests

Write exactly three tests.

### 1. Profile-relative end-to-end with reprojection (`profile_relative_raises_without_lowering`)

Run `RaiseRoads` with `--strategy=profile_relative` on the fixture. Assert:

- The output raster exists and opens.
- Output rows and columns match the input DEM.
- **No-lowering invariant**: for every non-nodata cell in the input DEM, `output[cell] >= input[cell]`. This is the paper's stated guarantee.
- Output nodata cells match input nodata cells (nodata mask is preserved).
- At least one cell satisfies `output[cell] > input[cell]` — confirming that roads intersected the DEM extent and the reprojection pipeline produced valid projected coordinates. If this assertion fails, the reprojection most likely failed silently.

### 2. Constant and cross-section strategies also satisfy no-lowering (`constant_and_cross_section_no_lowering`)

Run `RaiseRoads` twice on the fixture — once with `--strategy=constant --height=3.0` and once with `--strategy=cross_section`. For each run assert:

- Output dimensions match the input DEM.
- No-lowering invariant holds.
- At least one cell is raised above input.

This covers the paper's claim that all three strategies guarantee no lowering.

### 3. Strategies produce distinct raster outputs (`strategies_produce_distinct_outputs`)

Run all three strategies on the fixture and collect the output arrays. Assert that no two strategy outputs are identical (i.e., for each pair, at least one valid cell differs). This confirms the `--strategy` parameter is not a no-op and that each code path applies a materially different transformation to the same DEM.

## Run requirement

After writing the tests, run:

```
cargo test -p whitebox-tools-app raise_roads_integration
```

All three tests must pass. If any test fails, diagnose the failure, fix the test or the cause, and confirm passing output before reporting done.

## Constraints

- Write outputs to temp paths using the pattern from the existing integration tests. Never write into `test_fixtures/`.
- Clean up all temp files after each test.
- No new dependencies. Use only crates already in `Cargo.toml`.
- Do not modify `dem_clip.tif` or `roads.geojson` in the fixture.
- Do not add a golden-master reference raster to the fixture directory.
