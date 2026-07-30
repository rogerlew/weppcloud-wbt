# TopazConditionDem parity-hardening validation

Date: 2026-07-30 UTC

## Golden contract

`test_fixtures/topaz_condition_dem/parity_manifest.json` contains seven cases.
Each stage hash is SHA-256 over valid raster cells in row-major order as
little-endian signed 32-bit TOPAZ internal units. Each mask hash is SHA-256 over
one row-major byte per cell. The harness verifies the input TIFF checksum,
dimensions, NoData count, unchanged stage masks, FILDEP content, and RELIEF
content.

| Evidence group | Cases | Valid cells checked | Result |
| --- | ---: | ---: | --- |
| Original production DEM, widths 0/1/2 | 3 | 576,630 | Exact FILDEP and RELIEF |
| Larger all-valid production DEMs | 2 | 3,024,549 | Exact FILDEP and RELIEF |
| Synthetic irregular NoData | 1 | 1,796 | Exact FILDEP and RELIEF |
| NLCD-water-masked production DEM | 1 | 1,434,331 | Exact FILDEP and RELIEF |

The seven cases contain 5,037,306 valid case-cells in total. Repeated `--all`
runs produce byte-identical JSON reports. A deliberately corrupted expected
RELIEF hash exits nonzero and identifies the case and failed field. Every tool
invocation has a configurable timeout, defaulting to 120 seconds.

## NoData defect found and corrected

The 41-by-47 synthetic input includes an edge-connected stair-step NoData
corridor, irregular internal holes, an isolated NoData cell, and a one-cell
valid island. The initial Rust RELIEF implementation could raise the valid
island forever, and its FILDEP stage incorrectly filled cells beside NoData.

TOPAZ represents indeterminate elevation with a sentinel below valid terrain.
Rust now models that as an explicit open lower boundary in FILDEP and RELIEF,
while continuing to exclude invalid cells from region membership, obstruction
candidates, flat membership, and propagation. Focused unit tests cover both
termination and the adjacent-NoData fill behavior.

## NLCD provenance

The production run records `_nlcd_db = "nlcd/2019"` and does not persist a
landuse raster under disturbed mapping. WMesque v2 was queried read-only using
the exact burned-out-harmonic grid:

    bbox: 592572.4951871115,5224708.552325346,629562.4951871115,5260228.552325346
    bbox CRS: EPSG:32610
    cell size: 30 m
    resampling: nearest

The response already matched the DEM's 1,233-by-1,184 dimensions,
geotransform, and CRS, so no additional warp was performed. Response raster
SHA-256 was
`7f6c66164ce84267eb86774b23bfbf6e7db5b352876fbbefb36859a371546820`.
Although the configured alias is `nlcd/2019`, response metadata identifies
Annual NLCD Collection 1 (version 1.1, June 2025), year 2024. Masking its
25,541 class-11 cells produced fixture SHA-256
`4197c6689ca96f8dd349ea200b2776e27c7691ebd9a8e13ac7e7150e973ec78f`.

## Commands

Run from the repository root:

    cargo build --release -p whitebox-tools-app
    cargo test -p whitebox-tools-app
    /usr/bin/python3 tools/validate_topaz_condition_dem_parity.py \
      --binary target/release/whitebox_tools \
      --manifest test_fixtures/topaz_condition_dem/parity_manifest.json \
      --all
    /usr/bin/python3 -m py_compile whitebox_tools.py WBT/whitebox_tools.py \
      tools/create_topaz_condition_dem_nlcd_water_fixture.py \
      tools/create_topaz_condition_dem_synthetic_nodata_fixture.py \
      tools/run_topaz_condition_dem_oracle.py \
      tools/validate_topaz_condition_dem_parity.py
    git diff --check

Oracle and retrieval intermediates remain under
`target/topaz-condition-dem/parity-hardening/`. Neither production run
directories nor `/workdir/topaz` were modified.
