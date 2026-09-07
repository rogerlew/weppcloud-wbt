# ADR 001: Reproduce serial pit order across worker counts

Status: adopted for optimization parity, 2026-09-07 UTC.

The legacy discovery stage appends rows as worker messages arrive and uses a
stable elevation-only sort. Equal-elevation pits consequently have no reproducible
cross-thread order, even though earlier breaches alter later searches. The target
DEM contains about 395,000 candidate pits at elevation zero, making this material.

Define tie order as the legacy one-worker row-major discovery order. Sort by the
existing elevation direction, then ascending row and column before stack popping.
Do this for both breaching and optional filling. Popping thus visits tied pits in
reverse row-major order, exactly as the legacy one-worker implementation. No cost,
threshold, flat increment, maximum-distance, or fill formula changes.

This selects a reproducible existing serial outcome; it cannot promise equality
to every legacy multi-worker run whose tie order was scheduler-dependent. Validate
against the retained one-worker binary and report legacy 12-worker differences
separately. New 1/12-worker outputs must agree exactly. Arbitrary parallel writes
and spatial reordering for speed were rejected because they change dependencies.
