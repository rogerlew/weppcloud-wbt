# strained_gown IFOLP Regression Fixture

Purpose: reproduce IFOLP `--max_junctions=3` fan-in behavior that previously allowed
junction inflow counts above 3 on real WEPPcloud data.

## Source

- Run root: `/wc1/runs/st/strained-gown`
- Extraction date: 2026-04-19
- Files copied from: `/wc1/runs/st/strained-gown/dem/wbt/`
  - `flovec.tif`
  - `floaccum.tif`

## Intended Test Use

This fixture is consumed by the IFOLP integration test in
`iterative_first_order_link_prune_parser_tests.rs` to assert that running:

- `--csa=3.0`
- `--mscl=60.0`
- `--max_junctions=3`

produces a stream raster whose receiver inflow count never exceeds 3.
