# Task: Fix VRT test fixture path and complete integration test coverage

## Goal

Fix a broken fixture path in three existing test files, then expand `whitebox-raster/tests/vrt_integration.rs` to cover all valid VRT files in the fixture directory. Run `cargo test -p whitebox-raster` and confirm all tests pass before reporting done.

## Context

`whitebox-raster` implements read-only, single-source VRT support added in this fork. The VRT parser is in `whitebox-raster/src/vrt/mod.rs` (`read_vrt`), and it is invoked by `Raster::new` when the file extension is `.vrt`. The fork's constraints are:

- **Read-only**: no VRT write support
- **Single-source**: exactly one `SimpleSource` element per `VRTRasterBand`
- **Band 1 only**: `VRTRasterBand band=1`, `SourceBand=1`
- **No complex source types**: `ComplexSource`, `AveragedSource`, etc. are rejected with `Err`
- **SrcRect/DstRect must be congruent**: sizes must match and `DstRect` offsets must be 0

There are three test files in `whitebox-raster/tests/`:

- `geotiff_window.rs` — tests for `read_geotiff_window` (windowed reads on plain TIFFs)
- `vrt_parser.rs` — unit tests for `read_vrt` directly; covers 4 valid VRTs and all 10 invalid VRTs
- `vrt_integration.rs` — integration tests for `Raster::new` on `.vrt` files; only covers 3 of 14 valid VRTs

## Step 1: Fix the broken fixture path (all 3 test files)

All three test files contain:

```rust
fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vrt_test_data")   // ← WRONG
}
```

`CARGO_MANIFEST_DIR` for `whitebox-raster` resolves to the `whitebox-raster/` directory. The VRT test fixtures live at `test_fixtures/vrt_test_data/` relative to the repo root — one level up from `whitebox-raster/`. The correct path is:

```rust
fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("test_fixtures")
        .join("vrt_test_data")   // ← CORRECT
}
```

Apply this fix to all three files:
- `whitebox-raster/tests/geotiff_window.rs`
- `whitebox-raster/tests/vrt_parser.rs`
- `whitebox-raster/tests/vrt_integration.rs`

Do not change anything else in these files during this step.

## Step 2: Expand `vrt_integration.rs`

After fixing the path, the existing three integration tests will pass:

| Test | VRT | Reference |
|------|-----|-----------|
| `test_vrt_raster_loads_center_crop` | `vrt/crop_center_100x100.vrt` | `reference/crop_center_100x100.tif` |
| `test_vrt_raster_loads_relative_path` | `vrt/crop_relative_path.vrt` | `reference/crop_center_100x100.tif` |
| `test_vrt_raster_loads_fullsize_no_rect` | `vrt/fullsize_no_rect.vrt` | `source/dem_100x100_int16.tif` |

Add one test for each of the remaining 11 valid VRT files. Each test follows the same pattern as the existing ones: load VRT via `Raster::new`, load reference via `Raster::new`, call `assert_raster_matches_reference`.

### New tests to add

| Test name | VRT | Reference |
|-----------|-----|-----------|
| `test_vrt_raster_loads_bottomright_crop` | `vrt/crop_bottomright_200x200.vrt` | `reference/crop_bottomright_200x200.tif` |
| `test_vrt_raster_loads_topleft_crop` | `vrt/crop_topleft_150x150.vrt` | `reference/crop_topleft_150x150.tif` |
| `test_vrt_raster_loads_lzw_compressed_source` | `vrt/crop_lzw_200x200.vrt` | `reference/crop_lzw_200x200.tif` |
| `test_vrt_raster_loads_deflate_compressed_source` | `vrt/crop_deflate_300x300.vrt` | `reference/crop_deflate_300x300.tif` |
| `test_vrt_raster_loads_tiled_source` | `vrt/crop_tiled_nonaligned.vrt` | `reference/crop_tiled_nonaligned.tif` |
| `test_vrt_raster_loads_tiled_lzw_source` | `vrt/crop_tiled_lzw_150x150.vrt` | `reference/crop_tiled_lzw_150x150.tif` |
| `test_vrt_raster_loads_int16_source` | `vrt/crop_int16_50x50.vrt` | `reference/crop_int16_50x50.tif` |
| `test_vrt_raster_loads_int16_lzw_pred2_source` | `vrt/crop_int16_pred2_50x50.vrt` | `reference/crop_int16_pred2_50x50.tif` |
| `test_vrt_raster_loads_int16_packbits_source` | `vrt/crop_int16_packbits_50x50.vrt` | `reference/crop_int16_packbits_50x50.tif` |
| `test_vrt_raster_loads_float32_source` | `vrt/crop_float32_50x50.vrt` | `reference/crop_float32_50x50.tif` |
| `test_vrt_raster_loads_sparse_source` | `vrt/crop_sparse_120x120.vrt` | `reference/crop_sparse_120x120.tif` |

Use the existing `assert_raster_matches_reference` helper already defined in the file. Do not modify that helper.

## Run requirement

After both steps, run:

```
cargo test -p whitebox-raster
```

All tests must pass — including the 11 pre-existing tests in `geotiff_window.rs`, the tests in `vrt_parser.rs`, and all 14 integration tests (3 existing + 11 new) in `vrt_integration.rs`. Do not report done until you have seen passing output from this command.

If any test fails after the path fix, investigate the failure. Do not mask failures by skipping or ignoring tests.

## Constraints

- Fix only `data_root()` in the three test files — do not change test logic, helper functions, or test names.
- No new dependencies.
- Do not add, remove, or modify any file in `test_fixtures/`.
- Do not create new test files — add to the existing `vrt_integration.rs`.
