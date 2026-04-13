# IFOLP WP-00 Parity Metrics Specification

This document defines the parity metrics implemented by `tools/ifolp_wp00_compare_outputs.py`.

## Inputs

Per fixture, comparison consumes:

- `d8_pntr`: staged from fixture manifest (`fixtures/<id>/inputs/d8_pntr.tif`)
- `oracle`: `/tmp/ifolp_wp00/<run>/oracle/<id>/stream.tif`
- `candidate`: `/tmp/ifolp_wp00/<run>/candidate/<id>/stream.tif`

Stream mask definition (both oracle and candidate):
- valid raster cell (`finite` and not `nodata`), and
- raster value `> 0`

## Required metrics

1. Exact binary raster equality (primary)
- Definition: `candidate_mask == oracle_mask` for all cells.
- Report fields:
  - `metrics.exact_binary_equal`
  - `metrics.differing_cell_count`

2. Stream-cell count delta
- Definition: `count(candidate_mask) - count(oracle_mask)`.
- Report field: `metrics.stream_cell_count.delta`

3. Connected-component count delta
- Connectivity: 8-neighbor.
- Definition: `components(candidate_mask) - components(oracle_mask)`.
- Report field: `metrics.connected_components.delta`

4. Junction count delta
- A junction cell has two or more inflowing stream neighbors based on the D8 pointer raster.
- Pointer encoding follows fixture manifest (`whitebox` in WP-00 fixtures).
- Definition: `junctions(candidate_mask) - junctions(oracle_mask)`.
- Report field: `metrics.junction_count.delta`

5. Outlet reachability
- For each stream cell, follow D8 downstream links restricted to stream-mask cells.
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
  --fail-on-mismatch
```

## Deterministic output contract

- Full report: `parity-report.json` (contains absolute paths; may differ run-to-run).
- Canonical report: `parity-report.canonical.json` (path-free metric summary, deterministic for identical fixture/oracle/candidate content).
