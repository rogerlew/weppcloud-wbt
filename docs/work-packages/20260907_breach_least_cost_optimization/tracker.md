# Tracker

## Status

Closed, 2026-09-07 22:43 UTC: 420.566-second full channel call on 12 cores.

## Tasks

- [x] Stop only the manual WBT benchmark on wepp1 and preserve evidence.
- [x] Copy valid-tabletop DEM to forest and hash it.
- [x] Preserve the existing release binary under target/breach-benchmark/reference.
- [x] Capture smaller-DEM baselines and study equal-elevation ordering.
- [x] Implement and test faithful optimization.
- [x] Run 30 exact 1/12-core parity cases and full-fixture conditioning (384.386 s).
- [x] Complete isolated full-channel timing: 420.566 s; exact standalone relief parity.
- [x] Repository gates: 147 tests, cargo check, Python compilation, diff check.
- [x] Close documentation with final pipeline result and generated-artifact hashes.

## Decisions and risks

Preserve sequential pit dependencies. Speculative concurrent search is acceptable
only when validated against earlier committed raster changes; independent writes
without conflict checking are not. Legacy equal-elevation ordering may depend on
thread arrival order and needs characterization. Production timing was manually
interrupted, not completed, and is only a lower bound.

## Verification

DEM SHA-256: `6d49e9e92daef8a9b5709db6833975cebfafda92ac97d6c4d6d77eeb1f05ee60`.
Baseline binary SHA-256: `9778a9c7e56c805633e02ee4c03c595d7383feaa8b0386a2f7538dd777e95c98`.
Source revision before changes: `46272ca42a5b5b95e59713deee3af20e06fc93b4`.
See artifacts/wepp1-interrupted/result.json for the 794.319-second interruption.

Final standalone conditioning: 384.386 s. Full-call conditioning: 383.758 s.
Full call: 420.566 s, from 22:36:04 UTC to 22:43:04 UTC, with 10 checksummed
generated artifacts. See artifacts/results.md and artifacts/full-channels-final.json.

Durable contract: docs/breach_depressions_least_cost_optimization.md, section
"Reproducible pit order". Exact flat-search reuse and ordered conflict checking
preserve serial decisions; no search radius, cost, or fallback changes were used.
