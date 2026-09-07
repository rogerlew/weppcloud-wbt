# Least-cost breaching execution and parity

`BreachDepressionsLeastCost` retains its cost equations, neighbor traversal, heap
ordering, flat increment, path limits, optional filling, and unresolved-depression
error contract. The `WBT_MAX_PROCS` process environment override or existing `max_procs` setting
limits compute workers. See [runtime configuration](wbt_runtime_configuration.md)
for precedence and the distinction between runtime and persistent settings.

## Reproducible pit order

Pit discovery may run concurrently, but equal-elevation pits are sorted in the
legacy one-worker row-major discovery order before stack popping. Breaching visits
low elevations first and tied cells in reverse row-major order. Optional filling
uses its original elevation direction with the same row/column tie rule.

Legacy multi-worker discovery used message arrival order for equal heights, so its
results could vary with scheduling. Compatibility is exact equality with the
retained legacy one-worker result; it is not equality with every possible legacy
multi-worker outcome. New one- and multi-worker runs must agree exactly.

## Parallel searches and ordered terrain changes

Each worker searches an immutable view of the current terrain with private heap,
backlink, path-length, and generation-tagged visited storage. Generation tags
avoid clearing all explored cells after every pit. Results are applied in the
serial pit order. A result is recomputed against the latest terrain if any earlier
write falls within its read rectangle, including the initial pit check. The
rectangle is conservative: unnecessary retries are safe; stale writes are not.

The worker pool uses the existing Rayon dependency. Scratch memory grows with
raster size and worker count: approximately seven bytes per cell per worker,
plus heaps and cached search descriptions. Benchmark with explicit `max_procs`
and CPU affinity when comparing timings.

## Exact reuse on flat terrain

Some no-write searches cross unchanged flat terrain to an outer raster edge.
Their translated execution is identical when all interior values and the edge
band in the entire translated read rectangle match bit for bit, and the distance
to the same edge is unchanged. The implementation caches only such successful
no-write searches. It excludes corners and rejects nonuniform rectangles.

A 32-cell tile index summarizes interior values for exact equality checks.
Mixed and partial tiles and the separate boundary band are checked against
actual raster values; this is not a tolerance or elevation
approximation. The index is refreshed after committed writes. Reused results
retain their read rectangle and therefore undergo the same ordered conflict check.
Search settings are fixed for the lifetime of the worker cache.

## Reproducing validation on forest

The target fixture and provenance are in `test_fixtures/breach_least_cost/`.
Reference binary identity, parity evidence, and timings are recorded in
`docs/work-packages/20260907_breach_least_cost_optimization/`.

Copy reference and candidate binaries into separate directories under
`target/breach-benchmark/`. The benchmark harness deliberately requires isolated
binaries because WBT reads and writes `settings.json` beside its executable.
Use separate binary directories for concurrent invocations with different settings.

    cargo build --release -p whitebox-tools-app --locked
    mkdir -p target/breach-benchmark/candidate
    cp target/release/whitebox_tools target/breach-benchmark/candidate/whitebox_tools
    /workdir/wepppy/.venv/bin/python tools/run_breach_parity_suite.py target/breach-benchmark/candidate/whitebox_tools candidate

Each case produces `result.json`, raw execution logs, and applicable raster and
conditioning diagnostics. Compare a reference one-core summary with each candidate
summary using `tools/compare_breach_parity.py`. Comparison includes exact valid-cell
float64 hashes, masks, georeferencing, diagnostic values, and machine-readable
failure behavior. An unexpected failure is never accepted as parity.

Run the large fixture with its original 3,000 m radius (600 cells), minimum-distance
objective, no fill, and failure on unresolved depressions:

    /workdir/wepppy/.venv/bin/python tools/benchmark_breach_least_cost.py \
      --binary target/breach-benchmark/candidate/whitebox_tools \
      --dem test_fixtures/breach_least_cost/valid-tabletop/dem.tif \
      --output-dir target/breach-benchmark/valid-tabletop-12 \
      --cores 12 --cpus 0-11 --dist 600 --mode fail

The target is less than 600 seconds including raster I/O and diagnostics on
12 physical forest cores. Record completed timing; interrupted progress is only
a lower bound. An unresolved-depression error preserves safety semantics and
must not be reported as successful channel construction. Production installation
and RQ timeout changes require a separate deployment decision.

The retained deployed reference has SHA-256
`9778a9c7e56c805633e02ee4c03c595d7383feaa8b0386a2f7538dd777e95c98`.
If that local binary is unavailable, materialize source revision
`46272ca42a5b5b95e59713deee3af20e06fc93b4` with `git archive` into a separate
`target/breach-benchmark/reference-source/` directory and build it with
`cargo build --release -p whitebox-tools-app --locked` there. Copy its executable
into an isolated reference directory and record its new binary hash; compiler
versions can change binary hashes without changing raster results.

For a complete WEPPpy channel-call measurement, use the existing WEPPpy environment
and the same isolated candidate binary after other benchmarks finish:

    PYTHONPATH=/workdir/wepppy taskset -c 0-11 /workdir/wepppy/.venv/bin/python -u \
      tools/benchmark_breach_channels.py \
      --binary target/breach-benchmark/candidate/whitebox_tools \
      --dem test_fixtures/breach_least_cost/valid-tabletop/dem.tif \
      --output-dir target/breach-benchmark/full-channels-12

This runs CSA 4 ha, MCL 40 m, and IFOLP pruning, recording cumulative build-step
times and checksums of the generated flow, stream, and polygon artifacts. It
measures the computation directly; Redis/RQ bookkeeping is outside this timer.
