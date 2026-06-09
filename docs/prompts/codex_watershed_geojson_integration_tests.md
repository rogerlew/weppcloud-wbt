# Task: Watershed GeoJSON pour-point integration test suite

## Goal

Author `whitebox-tools-app/src/tools/hydro_analysis/watershed_integration_tests.rs` and wire it into `watershed.rs`. Run `cargo test` to confirm all new tests pass before reporting done.

## Context

`Watershed` (`hydro_analysis/watershed.rs`) delineates the upslope drainage area for a set of pour points. Given a D8 flow-pointer raster and a pour-point source, it traces all cells that drain to each outlet and labels them with the outlet's ID. Cells that drain to no pour point receive nodata (`-32768.0`, hardcoded).

The tool accepts three pour-point input formats, distinguished by file extension at runtime:

- **`.shp`** — Shapefile point features (original upstream code)
- **`.geojson` / `.json`** — GeoJSON FeatureCollection of Point or MultiPoint features (**added in this fork**)
- **anything else** — treated as a raster; non-zero cells are pour points

The GeoJSON branch assigns sequential FIDs (1, 2, 3 …) to features in order. A non-FeatureCollection GeoJSON root returns `Err`. Non-Point/Non-MultiPoint features inside a FeatureCollection are silently skipped.

The integration tests must cover the GeoJSON path specifically — that is the modification this fork introduced, and what the JOSS paper describes as "GeoJSON pour-point watershed support."

## Patterns to follow

Read before writing:

- `hydro_analysis/hillslopes_topaz_integration_tests.rs` — canonical pattern (`CARGO_MANIFEST_DIR`, `temp_output_path`, `cleanup_output_path`, `Raster::new`)
- `hydro_analysis/find_outlet_integration_tests.rs` — simpler two-input example

Wire the new module at the bottom of `watershed.rs` via `#[cfg(test)]` + `#[path = ...]` + `mod` declaration. There is no existing `mod tests` block in `watershed.rs`; add only the integration test module line.

## Fixtures

### `test_fixtures/minimal_1pixel_stream/`

An 18×57 synthetic watershed. 144 cells drain to a single outlet. Relevant files:

- `flovec.tif` — D8 pointer input
- `outlet.geojson` — FeatureCollection with one Point feature at the outlet coordinate
- `bound.tif` — watershed boundary mask; positive values identify the 144 in-watershed cells

The `outlet.geojson` uses projected coordinates (same CRS as `flovec.tif`); no reprojection is required.

### `test_fixtures/minimal_2pixel_stream/`

Same 18×57 watershed with a two-cell stream. Relevant files:

- `flovec.tif` — D8 pointer input
- `outlet.geojson` — FeatureCollection with one Point feature (same coordinate as minimal_1pixel)
- `netw0.tif` — stream raster; the outlet cell has a positive value; all non-stream cells are 0

## Required tests

Write exactly four tests.

### 1. GeoJSON outlet delineates the expected watershed (`geojson_delineates_watershed_matching_bound`)

Run `Watershed` with `minimal_1pixel_stream/flovec.tif` and `minimal_1pixel_stream/outlet.geojson`. Load the output and `bound.tif`. Assert:

- Output dimensions match the D8 pointer.
- Every cell that is valid (non-nodata, non-zero) in `bound.tif` has a labeled (non-nodata) value in the output.
- Every cell that is nodata or zero in `bound.tif` has output value `−32768.0` (the tool's hardcoded nodata).
- All labeled output cells have value `1.0` — the single feature's sequential FID.

### 2. GeoJSON and raster pour-point delineate the same watershed extent (`geojson_raster_parity`)

Run `Watershed` twice on `minimal_2pixel_stream`:

- Run A: D8 pointer `flovec.tif`, pour points `outlet.geojson`
- Run B: D8 pointer `flovec.tif`, pour points `netw0.tif` (raster mode; outlet cell is the positive-valued stream cell)

Assert that the set of labeled (non-nodata) cells is identical between the two outputs. The label values may differ (GeoJSON assigns FID=1; raster assigns the raster value at the outlet). This test verifies that the GeoJSON code path produces the same spatial delineation as the existing raster path for the same physical outlet location.

### 3. MultiPoint GeoJSON geometry is supported (`multipoint_geojson_produces_watershed`)

The GeoJSON parser handles both `Point` and `MultiPoint` feature geometries. Test the `MultiPoint` branch by constructing a temporary GeoJSON file at runtime.

Read `minimal_1pixel_stream/outlet.geojson` to obtain the outlet coordinate. Write a temporary FeatureCollection GeoJSON file containing one feature whose geometry is `MultiPoint` with that single coordinate as its only member. Run `Watershed` with this synthetic file and `minimal_1pixel_stream/flovec.tif`.

Assert:

- The tool completes without error.
- The set of labeled cells matches the expected watershed (compare against `bound.tif` as in test 1).

Clean up the temporary GeoJSON file after the test.

### 4. Non-FeatureCollection GeoJSON root returns an error (`non_feature_collection_returns_error`)

The GeoJSON branch requires a FeatureCollection at the root. A bare `Point` geometry object is not a FeatureCollection and must be rejected.

Write a minimal temporary GeoJSON file containing only a bare `Point` geometry (not wrapped in a Feature or FeatureCollection). Pass it as `--pour_pts` to `Watershed` with `minimal_1pixel_stream/flovec.tif`. Assert the call returns `Err`. Clean up the temporary file.

## Run requirement

After writing the tests, run:

```
cargo test -p whitebox-tools-app watershed_integration
```

All four tests must pass. If any test fails, fix the test or investigate the tool until they pass. Do not report done until you have seen passing output from this command.

## Constraints

- Write output rasters to temp paths using the standard helper pattern. Never write into `test_fixtures/`.
- Temporary GeoJSON files written in tests 3 and 4 must also be cleaned up after each test.
- No new dependencies. Use only crates already in `Cargo.toml`.
- Do not modify any fixture file.
