# FillDepressions edge-outlet validation

Validated from `/workdir/weppcloud-wbt` on 2026-07-30 UTC.

## Automated gates

| Gate | Result |
| --- | --- |
| `cargo test -p whitebox-tools-app fill_depressions_edge_outlet` | Pass: 5 tests |
| `cargo check -p whitebox-tools-app` | Pass |
| `cargo test -p whitebox-tools-app` | Pass: 140 tests |
| `python3 -m py_compile whitebox_tools.py WBT/whitebox_tools.py` | Pass |
| `git diff --check` | Pass |

The repository-wide `cargo fmt --all -- --check` gate remains non-clean because
rustfmt proposes extensive pre-existing changes outside this package,
beginning in `whitebox-raster/src/geotiff/mod.rs`. Both changed Rust files were
formatted directly; no unrelated formatting changes were retained.

## Synthetic behavior

Five focused tests construct temporary 7-by-7 Float64 rasters and verify:

- the same low region remains unchanged when connected to west, north, east,
  or south;
- closing the edge connection fills the region to the 15 m saddle;
- `--fix_flats --flat_increment=0.1` keeps outer outlets at 5 m and raises
  interior flat cells away from them;
- `--max_depth=5` leaves the 10 m deep enclosed depression unchanged;
- an interior NoData cell stays NoData and is not accepted as an outlet for a
  depression discovered from another pit.

## Exact production reproducer

Input:
`/wc1/runs/sr/srivas42-reconciled-turf/dem/dem.tif`

- Dimensions: 447 columns by 430 rows
- CRS: EPSG:32610
- Reference point: `-121.61908248267358, 47.704481876313544`
- Input elevation: `533.868286132812 m`
- Pre-fix output: `910.068481445312 m`
- Fixed output: `533.868286132812 m`

The result removes the approximately 376 m erroneous raise reported in issue
#1 while preserving the original edge-connection elevation.

## Depression-inventory parity

The tracked pre-fix `WBT/whitebox_tools` binary and rebuilt fixed
`target/debug/whitebox_tools` binary ran the exact production input with
diagnostics enabled and `--fix_flats=false`.

| Diagnostic | Pre-fix | Fixed |
| --- | ---: | ---: |
| Detected low points | 1,291 | 1,291 |
| Filled depression regions | 212 | 212 |
| Skipped depression regions | 1,079 | 1,079 |

Raster-by-raster comparison found 19,891 changed cells. Every changed fixed
value was lower than the pre-fix value; no fixed cell was higher. The maximum
removed overfill was 379.084533691406 m.

The detected inventory is structurally protected because the correction runs
only after `undefined_flow_cells` has been completely assembled. The identical
processed and skipped counts additionally prove the fixed pass still visits
the same depression candidates rather than silently dropping any.
