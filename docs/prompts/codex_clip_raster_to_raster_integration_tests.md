# Task: ClipRasterToRaster integration test suite

## Goal

Author `whitebox-tools-app/src/tools/gis_analysis/clip_raster_to_raster_integration_tests.rs` and wire it into `clip_raster_to_raster.rs`. Run `cargo test` to confirm all new tests pass before reporting done.

## Context

`ClipRasterToRaster` (`gis_analysis/clip_raster_to_raster.rs`) is a cell-wise masking tool. Given an input raster and a mask raster of identical geometry, each output cell is set to the input value when the mask cell is valid (non-nodata) and non-zero; otherwise the output cell receives the input raster's nodata value.

The tool rejects input/mask pairs that differ in rows, columns, or resolution — it returns an `Err` rather than panicking.

The full transformation rule, directly from the source:

```
if mask[cell] != mask_nodata AND mask[cell] != 0.0:
    output[cell] = input[cell]
else:
    output[cell] = input_nodata
```

Because the algorithm is cell-wise and fully specified by this rule, the correct output for every cell is derivable from the inputs without a golden-master reference raster. Do not add a reference raster to any fixture directory.

## Patterns to follow

Read before writing:

- `hydro_analysis/hillslopes_topaz_integration_tests.rs` — canonical pattern (fixture paths via `CARGO_MANIFEST_DIR`, `temp_output_path`, `cleanup_output_path`, `Raster::new`)
- `stream_network_analysis/prune_strahler_order_integration_tests.rs` — example of property-based verification against the input rather than a stored reference

Wire the new test module into `clip_raster_to_raster.rs` the same way other integration test files are wired (bottom of file, `#[cfg(test)]` + `#[path = ...]` + `mod` declaration).

## Fixtures

### `test_fixtures/blackwood_60_5/`

A 416×443 watershed. Relevant pair:
- `relief.tif` — input DEM (full raster extent, nodata outside valid area)
- `bound.tif` — watershed boundary mask (positive values inside the watershed, zero or nodata outside)

The mask and input share the same geometry. Clipping relief with bound should produce a raster where cells inside the watershed retain their elevation and cells outside become nodata.

### `test_fixtures/minimal_2pixel_stream/`

An 18×57 synthetic watershed. Relevant pair:
- `relief.tif` — input DEM
- `netw0.tif` — stream mask: exactly 2 cells have positive values (the stream cells); all other cells are zero

Using `netw0.tif` as the mask isolates the zero-exclusion branch: non-stream cells have mask value 0 (not nodata), so this fixture exercises `m_val != 0.0` rather than `m_val != nodata_m`.

### Mismatched pair

`blackwood_60_5/relief.tif` (416×443) and `minimal_1pixel_stream/bound.tif` (18×57) have different geometries. Passing them to the tool should produce an `Err`.

## Required tests

Write exactly three tests.

### 1. Watershed clip pass-through and exclusion (`blackwood_watershed_clip`)

Run `ClipRasterToRaster` using `blackwood_60_5/relief.tif` as input and `blackwood_60_5/bound.tif` as mask. Load the input and output rasters. For every cell assert the transformation rule:

- Where `mask != mask_nodata AND mask != 0.0`: output equals input value.
- Where `mask == mask_nodata OR mask == 0.0`: output equals input nodata.

Additionally assert:

- At least one cell satisfies the pass-through branch (mask has valid cells inside the watershed).
- At least one cell satisfies the exclusion branch (mask has nodata or zero cells outside the watershed).
- Output dimensions (rows, columns) match the input.

### 2. Zero mask values produce nodata output (`minimal_zero_mask_becomes_nodata`)

Run `ClipRasterToRaster` using `minimal_2pixel_stream/relief.tif` as input and `minimal_2pixel_stream/netw0.tif` as mask. Assert:

- Exactly 2 output cells have a value other than input nodata (the 2 stream cells where `netw0 > 0`).
- All other output cells equal input nodata.
- The 2 retained cells have the same elevation values as the input at those positions.

This test specifically targets the `m_val != 0.0` branch. The non-stream cells have mask value 0 (not nodata), so this is distinct from the nodata-exclusion path.

### 3. Geometry mismatch returns error (`mismatched_geometry_returns_error`)

Attempt to run `ClipRasterToRaster` with `blackwood_60_5/relief.tif` (416×443) as input and `minimal_1pixel_stream/bound.tif` (18×57) as mask. Assert that the call returns `Err`. No output cleanup is needed since no file should be written; pass a temp path anyway and clean it up defensively.

## Run requirement

After writing the tests, run:

```
cargo test -p whitebox-tools-app clip_raster_to_raster_integration
```

All three tests must pass. If any test fails, fix the test or investigate the tool until they pass. Do not report done until you have seen passing output from this command.

## Constraints

- Write outputs to temp paths using the pattern from the existing integration tests. Never write into `test_fixtures/`.
- Clean up temp files after each test.
- No new dependencies. Use only crates already in `Cargo.toml`.
- Do not add a golden-master raster to any fixture directory.
