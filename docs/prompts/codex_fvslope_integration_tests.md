# Task: FVSlope integration test suite

## Goal

Author `whitebox-tools-app/src/tools/hydro_analysis/fvslope_integration_tests.rs` and wire it into `fvslope.rs`. Run `cargo test` to confirm all new tests pass before reporting done.

## Context

`FVSlope` (`hydro_analysis/fvslope.rs`) computes slope in the D8 flow direction. Unlike the standard slope tool, it uses the downstream neighbour's elevation rather than a neighbourhood kernel, producing along-channel gradients suited to WEPP hydraulics. Inputs are a DEM raster and a D8 pointer raster; output is an F32 slope raster. Supported output units are `degrees`, `radians`, `percent`, and `ratio`.

This tool has no automated tests. The goal is a focused integration test file, not a port of the full algorithm.

## Patterns to follow

Read these two files before writing anything:

- `hydro_analysis/hillslopes_topaz_integration_tests.rs` — canonical pattern for how integration tests are structured in this repo (fixture paths via `CARGO_MANIFEST_DIR`, `temp_output_path`, `cleanup_output_path`, `Raster::new` for validation)
- `hydro_analysis/find_outlet_integration_tests.rs` — simpler example of the same pattern

Wire the new test module into `fvslope.rs` the same way `hillslopes_topaz.rs` wires in its tests (bottom of file, `#[cfg(test)]` + `#[path = ...]` + `mod` declaration).

## Fixtures

### `test_fixtures/blackwood_60_5/`

A real 416×443 watershed. Relevant files:

- `relief.tif` — DEM input
- `flovec.tif` — D8 pointer input
- `fvslop.tif` — **pre-computed reference output** produced by an earlier TOPAZ-compatible run

Use this fixture for regression and units-consistency tests.

### `test_fixtures/minimal_2pixel_stream/`

An 18×57 synthetic watershed with exactly two stream pixels. Relevant files:

- `relief.tif` — DEM input (outlet cell elev ≈ 501.7 m, upstream cell elev ≈ 505.8 m)
- `flovec.tif` — D8 pointer input

The upstream cell's expected slope ratio is derivable from the known elevation drop and the raster's cell resolution. Use this fixture for structural and range checks.

## Required tests

Write exactly three tests:

### 1. Golden-master regression (`blackwood_regression`)

Run FVSlope with `units=ratio` on the blackwood fixture inputs. Load the pre-computed `fvslop.tif` reference and the tool output. Assert:

- Output dimensions (rows, columns) match the reference.
- For every valid (non-nodata) pixel, the absolute difference between output and reference is below a tolerance appropriate for F32 raster comparison. Pick a tolerance that is tight enough to catch algorithmic regressions but tolerant of platform-level floating-point variation.
- At least one valid pixel exists (guard against an all-nodata output silently passing).

### 2. Units consistency (`blackwood_units_consistency`)

Run FVSlope four times on the blackwood fixture, once per unit type. For each valid pixel, assert the mathematical relationships hold across all four outputs simultaneously:

- `degrees = degrees(atan(ratio))`
- `radians = atan(ratio)`
- `percent = ratio * 100`
- All outputs have the same nodata mask as the ratio output.

Use a tolerance appropriate for the trigonometric conversions at typical terrain slope values (< 45°).

### 3. Structural and range check on minimal fixture (`minimal_structural`)

Run FVSlope with `units=ratio` on the minimal_2pixel_stream fixture. Assert:

- Output exists and opens without error.
- Output dimensions match the input DEM.
- All valid output values are ≥ 0.0 (negative slopes must be clamped by the tool).
- At least one valid value exists and is > 0.0 (the upstream cell must have a non-zero slope).
- The upstream cell's slope is within a reasonable tolerance of the value derived from the known elevation drop and cell resolution read from the raster metadata. Derive the expected value from the raster — do not hardcode it.

## Run requirement

After writing the tests, run:

```
cargo test -p whitebox-tools-app fvslope_integration
```

All three tests must pass. If any test fails, fix the test or the tool until they pass. Do not report done until you have seen passing output from this command.

## Constraints

- Outputs must go to temp paths (use the helper pattern from the existing integration tests). Never write into `test_fixtures/`.
- Clean up temp files after each test.
- No new dependencies. Use only crates already in `Cargo.toml`.
- Do not modify `fvslop.tif` in the blackwood fixture.
