# Least-cost breach optimization

Status: closed, 2026-09-07 22:43 UTC. Full channel computation completed in
420.566 seconds on 12 forest cores; conditioning alone took 384.386 seconds.
Thirty exact parity checks and 147 Rust tests pass. See [results](artifacts/results.md).

## Scope and acceptance

Optimize the existing Rust `BreachDepressionsLeastCost` implementation for the
checksummed valid-tabletop 5 m DEM, with a target of less than 600 seconds wall
time using at most 12 physical CPU cores on forest. Preserve the 600-cell
(3,000 m) search radius, minimum-distance objective, default flat increment,
no filling, and failure on unresolved depressions. Measure the complete CLI
operation including raster I/O and diagnostics. Also measure downstream channel
construction if conditioning succeeds; do not equate a fast failure with success.

This is faithful optimization, not a replacement algorithm. Smaller existing
DEMs and synthetic interaction cases must retain exact elevation values, NoData
masks, spatial metadata, diagnostic counts, and error behavior. Timing and
operation identifiers are excluded from parity. Investigate legacy tie-order
nondeterminism before defining any intentional ordering change.

## Authority and rationale

The operator requested this package on 2026-09-07 after stopping a production
benchmark at 794.319 seconds, approximately 14% breaching progress. The target is
empirical optimization with 12 cores rather than guessing a larger RQ timeout.
Production deployment and RQ timeout changes are outside this package.

## Security and parameterization

Security impact: low. Work is local to forest, using a copied DEM, isolated
benchmark outputs, and existing Rust dependencies. No credentials or run access
changes. Ordering is made deterministic by adopting the legacy one-worker tie order;
[ADR 001](artifacts/adr-001-pit-order.md) records this choice. Formulas, thresholds,
conversions, CLI defaults, and filling/failure behavior are preserved.

## Deliverables

Fixture provenance, retained baseline binary identity, reproducible parity and
benchmark harness, optimized Rust path, regression tests, user/developer notes,
and checksummed rebuilt-binary evidence. The performance target is not met until
measured; an unresolved target keeps the package open.
