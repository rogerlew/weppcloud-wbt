# IFOLP WP-00 Parity Metrics Specification

This document defines the parity metrics implemented by `tools/ifolp_wp00_compare_outputs.py`.

## Inputs

Per fixture, comparison consumes:

- `d8_pntr`: staged from fixture manifest (`fixtures/<id>/inputs/d8_pntr.tif`)
- `basin_mask`: staged from fixture manifest (`fixtures/<id>/inputs/basin_mask.tif`)
- `oracle`: `/tmp/ifolp_wp00/<run>/oracle/<id>/stream.tif`
- `candidate`: `/tmp/ifolp_wp00/<run>/candidate/<id>/stream.tif`

Stream mask definition (both oracle and candidate):
- valid raster cell (`finite` and not `nodata`), and
- raster value `> 0`

Comparison-domain definition:
- default mode: `basin_mask`
- domain mask: `basin_mask > 0` on valid basin-mask cells
- alternate mode: `full_extent` (explicitly requested via CLI)
- report fields:
  - `comparison_domain` (report-level selected mode)
  - `fixtures[*].comparison_domain.mode`
  - `fixtures[*].comparison_domain.domain_cell_count`

## Required metrics

1. Exact binary raster equality (primary)
- Definition: `candidate_mask == oracle_mask` for all cells in comparison domain.
- Report fields:
  - `metrics.exact_binary_equal`
  - `metrics.differing_cell_count`
  - `metrics.false_positives`
  - `metrics.false_negatives`

2. Stream-cell count delta
- Definition: `count(candidate_mask) - count(oracle_mask)` in comparison domain.
- Report field: `metrics.stream_cell_count.delta`

3. Connected-component count delta
- Connectivity: 8-neighbor.
- Definition: `components(candidate_mask) - components(oracle_mask)` in comparison domain.
- Report field: `metrics.connected_components.delta`

4. Junction count delta
- A junction cell has two or more inflowing stream neighbors based on the D8 pointer raster.
- Pointer encoding follows fixture manifest (`whitebox` in WP-00 fixtures).
- Definition: `junctions(candidate_mask) - junctions(oracle_mask)` in comparison domain.
- Report field: `metrics.junction_count.delta`

5. Outlet reachability
- For each stream cell, follow D8 downstream links restricted to stream-mask cells within comparison domain.
- A cell is reachable if traversal terminates at an outlet condition (downstream leaves stream mask/grid or pointer is terminal).
- Report fields:
  - `metrics.outlet_reachability.candidate.all_stream_cells_reach_outlet`
  - `metrics.outlet_reachability.oracle.all_stream_cells_reach_outlet`
  - `metrics.outlet_reachability.match`

## WP-00 acceptance thresholds

For each fixture:

- `exact_binary_equal == true`
- `stream_cell_count.delta == 0`
- `connected_components.delta == 0`
- `junction_count.delta == 0`
- `outlet_reachability.match == true`

## Command

```bash
cd /workdir/weppcloud-wbt
python tools/ifolp_wp00_compare_outputs.py \
  --manifest /tmp/ifolp_wp00/run1/manifests/fixture-manifest.json \
  --oracle-root /tmp/ifolp_wp00/run1/oracle \
  --candidate-root /tmp/ifolp_wp00/run1/candidate \
  --output-json /tmp/ifolp_wp00/run1/reports/parity-report.json \
  --canonical-json /tmp/ifolp_wp00/run1/reports/parity-report.canonical.json \
  --comparison-domain basin_mask \
  --fail-on-mismatch
```

`--comparison-domain basin_mask` is the required WP-00/WP-05 parity mode for apples-to-apples candidate/oracle comparisons.

## Deterministic output contract

- Full report: `parity-report.json` (contains absolute paths; may differ run-to-run).
- Canonical report: `parity-report.canonical.json` (path-free metric summary, deterministic for identical fixture/oracle/candidate content).
- Report schemas:
  - full report: `ifolp_wp00_parity_report/v2`
  - canonical report: `ifolp_wp00_parity_report_canonical/v2`
