# Results

Completed on forest, 2026-09-07 UTC. The final full-channel call ran from
22:36:04 UTC to 22:43:04 UTC, pinned to 12 distinct physical cores (CPUs 0-11).

| Measurement | Wall time | Outcome |
| --- | ---: | --- |
| Final standalone conditioning, CPUs 12-23 | 384.386 s | 400,026 solved pits; zero unresolved |
| Conditioning inside full WEPPpy call, CPUs 0-11 | 383.758 s | Exact relief parity with standalone call |
| Complete WEPPpy channel-delineation call, CPUs 0-11 | 420.566 s | Success; 10 generated artifacts checksummed |

The full call has 179.434 seconds of margin below the requested 600-second target.
It uses the unchanged 5 m project DEM, 3,000 m breach search, minimum-distance
objective, no filling, failure on unresolved depressions, CSA 4 ha, MCL 40 m, and
IFOLP pruning. This measures computation directly, excluding RQ bookkeeping.

Thirty real/synthetic comparisons at one and 12 cores match the retained legacy
one-core reference exactly, including values, masks, spatial metadata, diagnostic
counts/volumes, and error behavior. All 147 Rust tests pass; cargo check, both
Python bindings and the five fixture/benchmark tools compile, and diff checks pass.

The deployed legacy binary is retained locally under target/breach-benchmark/reference
and its SHA-256 matches wepp1. Legacy 12-core output differs on tied-height cases;
new pit ordering reproduces its serial outcome deterministically. The durable
ordering contract is docs/breach_depressions_least_cost_optimization.md,
section "Reproducible pit order"; rationale is ADR 001 in this artifact directory.

Production was not deployed or requeued. The earlier wepp1 timing attempt was
stopped at the user's request after 794.319 seconds and is only a lower bound.
Two threaded prototypes also missed 600 seconds; their interrupted timings are
retained in benchmark-history.json. Exact flat-search reuse enabled the target.

Raw binary and raster scratch outputs remain under target/breach-benchmark.
The copied DEM and deterministic synthetic fixtures live under
test_fixtures/breach_least_cost. See parity.json, conditioning-final.json,
full-channels-final.json, pipeline-affinity.json, and validation.json for evidence.
