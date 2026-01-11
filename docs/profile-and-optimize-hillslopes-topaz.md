# Profile and Optimize HillslopesTopaz

## Goal
Reduce wall time for high-resolution (1.0 m) DEM runs by 10x+ without breaking
existing WEPPcloud workflows or output compatibility.

## Fixture
Use the medium-sized benchmark fixture in `test_fixtures/blackwood_60_5/`.
Pointer style for `flovec.tif` is proceduralized and consistent within the
workflow, so no detection step is required here.

Suggested run command (adjust `--esri_pntr` if needed):
```bash
./target/release/whitebox_tools --run=hillslopes_topaz \
  --dem=test_fixtures/blackwood_60_5/relief.tif \
  --d8_pntr=test_fixtures/blackwood_60_5/flovec.tif \
  --streams=test_fixtures/blackwood_60_5/netw0.tif \
  --pour_pts=test_fixtures/blackwood_60_5/outlet.geojson \
  --watershed=test_fixtures/blackwood_60_5/bound.tif \
  --chnjnt=test_fixtures/blackwood_60_5/chnjnt.tif \
  --subwta=/tmp/subwta.tif \
  --order=test_fixtures/blackwood_60_5/strahler.tif \
  --netw=/tmp/netw.tsv \
  -v
```

## Baseline Measurement
- Capture wall time and peak RSS:
  ```bash
  /usr/bin/time -v ./target/release/whitebox_tools --run=hillslopes_topaz ...
  ```
- Run 3 times and record median wall time.
- Save baseline outputs (`subwta.tif`, `netw.tsv`) for later diffing.

## Benchmark Results (blackwood_60_5, warm cache)
Host: Intel Xeon E5-2697 v2 (48 CPUs), 125 GiB RAM.

Runs: 1 warm-up + 3 measured runs (median reported).

Command (new build):
```bash
/usr/bin/time -v ./target/release/whitebox_tools --run=hillslopes_topaz \
  --dem=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/relief.tif \
  --d8_pntr=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/flovec.tif \
  --streams=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/netw0.tif \
  --pour_pts=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/outlet.geojson \
  --watershed=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/bound.tif \
  --chnjnt=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/chnjnt.tif \
  --subwta=/tmp/subwta_bw_new_1.tif \
  --order=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/strahler.tif \
  --netw=/tmp/netw_bw_new_1.tsv \
  --profile -v
```

Command (old build, no `--profile` support):
```bash
/usr/bin/time -v ./target/release/whitebox_tools --run=hillslopes_topaz \
  --dem=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/relief.tif \
  --d8_pntr=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/flovec.tif \
  --streams=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/netw0.tif \
  --pour_pts=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/outlet.geojson \
  --watershed=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/bound.tif \
  --chnjnt=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/chnjnt.tif \
  --subwta=/tmp/subwta_bw_old_1.tif \
  --order=/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/strahler.tif \
  --netw=/tmp/netw_bw_old_1.tsv \
  -v
```

Results (median of 3):

| Build | Median wall | Median peak RSS | Phase 0 | Phase 5 | Headwater scan | Flood fill (profile) |
| --- | --- | --- | --- | --- | --- | --- |
| new (dirty worktree) | 0.10s | 20.7MiB | 60.51ms | 4.56ms | 1.89ms | 4.57ms |
| old (clean HEAD) | 0.10s | 19.2MiB | 59.57ms | 3.53ms | n/a | n/a |

Notes:
- New build includes headwater scan threading + flood fill precompute.
- Old build is a clean worktree at the same HEAD commit.

## Benchmark Results (run19 1.0m, NFS share, warm cache)
Dataset: `/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt`.

Runs: 1 warm-up + 3 measured runs (median reported). Outputs written to `/tmp`.

Command (new build):
```bash
/usr/bin/time -v ./target/release/whitebox_tools --run=hillslopes_topaz \
  --dem=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/relief.vrt \
  --d8_pntr=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/flovec.vrt \
  --streams=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/netw0.tif \
  --pour_pts=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/outlet.geojson \
  --watershed=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/bound.tif \
  --chnjnt=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/chnjnt.tif \
  --subwta=/tmp/subwta_run19_new_1.tif \
  --order=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/strahler.tif \
  --netw=/tmp/netw_run19_new_1.tsv \
  --profile -v
```

Command (old build):
```bash
/usr/bin/time -v ./target/release/whitebox_tools --run=hillslopes_topaz \
  --dem=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/relief.vrt \
  --d8_pntr=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/flovec.vrt \
  --streams=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/netw0.tif \
  --pour_pts=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/outlet.geojson \
  --watershed=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/bound.tif \
  --chnjnt=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/chnjnt.tif \
  --subwta=/tmp/subwta_run19_old_1.tif \
  --order=/wc1/culverts/9bb6b47b-2256-4e14-a9c2-929b54c61568/runs/19/dem/wbt/strahler.tif \
  --netw=/tmp/netw_run19_old_1.tsv \
  -v
```

Results (median of 3):

| Build | Median wall | Median peak RSS | Phase 0 | Phase 5 | Headwater scan | Flood fill (profile) |
| --- | --- | --- | --- | --- | --- | --- |
| new (dirty worktree) | 41.65s | 8063.2MiB | 25810.00ms | 10610.00ms | 85.15ms | 10610.00ms |
| old (clean HEAD) | 41.27s | 7928.2MiB | 26180.00ms | 9730.00ms | n/a | n/a |

Notes:
- New build includes headwater scan threading + flood fill precompute.
- NFS share likely dominates Phase 0 I/O.

## Profiling Plan
Start with low-friction sampling and move to instrumentation only if needed.

### Sampling (preferred)
```bash
perf record -g -- ./target/release/whitebox_tools --run=hillslopes_topaz ...
perf report
```
If `perf` is not available or lacks permissions, use:
```bash
samply record ./target/release/whitebox_tools --run=hillslopes_topaz ...
```

### Phase Timing (coarse)
Use the existing verbose phase prints (or `--profile`) to identify dominant phases:
- Phase 1: link building
- Phase 5: hillslope flood fill
- Phase 6: area calculations

## Data to Capture Per Run
- Git SHA and branch
- CPU model, core count, and RAM
- Fixture name and file list
- Wall time, peak RSS
- Whether `--esri_pntr` is used

## Likely Hotspots to Validate
- Headwater discovery and link walking loops
- Flood fill / residual fill in Phase 5
- Repeated `get_value` calls on rasters inside inner loops
- HashMap lookups for `subwta_counts`
- Bounds checks inside 8-neighbor scans

## Optimization Ideas (in order of safety)
1. Precompute stream mask and pointer offsets as packed arrays to reduce
   repeated `get_value` calls in tight loops.
2. Replace `HashMap` for `subwta_counts` with a `Vec<i32>` keyed by TOPAZ ID
   range when feasible.
3. Collapse repeated neighbor scans by caching upstream counts for stream cells.
4. Reduce per-cell branching by splitting loops for stream and non-stream cells.
5. Consider single-pass labeling if flood fill and residual fill overlap.

## Validation
- Pixel-wise compare `subwta.tif` with baseline (exact match expected for
  integer TOPAZ IDs stored in float).
- Compare `netw.tsv` rows for identical topology and IDs.

Example comparison snippet:
```bash
python - <<'PY'
import numpy as np
import rasterio

def load(path):
    with rasterio.open(path) as ds:
        return ds.read(1), ds.nodata

a, na = load('/tmp/subwta.tif')
b, nb = load('test_fixtures/blackwood_60_5/subwta.tif')
assert na == nb
if not np.array_equal(a, b):
    diff = np.count_nonzero(a != b)
    raise SystemExit(f'diff cells: {diff}')
print('subwta match')
PY
```

## Success Criteria
- 10x+ wall-time reduction on a 1.0 m DEM case.
- No change to output topology and TOPAZ labeling.
- No regressions on existing WEPPcloud workflows.
