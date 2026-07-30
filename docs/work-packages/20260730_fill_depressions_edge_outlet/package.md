# FillDepressions edge outlets

**Status**: Closed (2026-07-30 21:05 UTC)

**Started**: 2026-07-30 20:54 UTC

**Timezone**: UTC

## Overview

This package resolves
[weppcloud-wbt issue #1](https://github.com/rogerlew/weppcloud-wbt/issues/1):
`FillDepressions` can raise valid low terrain that reaches an outer raster
edge because its pit-outward search recognizes only lower valid neighbours as
outlets.

## Objectives

- Treat valid cells on every outer raster edge as open drainage outlets at
  their existing elevation.
- Preserve filling of genuinely enclosed depressions.
- Preserve `max_depth`, flat-gradient, data type, metadata, and NoData
  behavior.
- Preserve the pre-fix low-point and processed-depression inventory so the
  correction does not silently skip unrelated depressions.
- Add small deterministic regression cases for west, north, east, south, and
  enclosed topologies.
- Document the exact outer-edge and NoData semantics.

## Scope

### Included

- `FillDepressions` outlet discovery and flat-outlet confirmation.
- Focused Rust integration tests using synthetic 7-by-7 rasters.
- Tool documentation, changelog, work-package records, and validation evidence.

### Explicitly out of scope

- Replacing the algorithm with an outside-in priority flood.
- Changing `BreachDepressions` or other filling implementations.
- Treating interior NoData holes as outlets.
- WEPPpy deployment or production data mutation.

## Success criteria

- [x] West-, north-, east-, and south-edge low regions are not filled to an
  interior spill.
- [x] The equivalent enclosed low region is filled to its lowest spill.
- [x] Flat fixing starts from established edge outlets without changing the
  outlet elevation.
- [x] `max_depth` behavior remains covered.
- [x] Pre-fix and fixed production diagnostics report identical detected,
  filled, and skipped depression counts.
- [x] Targeted and full `whitebox-tools-app` tests pass.
- [x] Algorithm documentation and work-package evidence are complete.

## Security and parameterization gates

- **Security impact**: none
- **Dedicated security review**: no
- **Rationale**: offline raster processing and tests only; no authentication,
  secrets, network writes, or execution-policy surface changes
- **Parameterization change present**: no
- **ADR required**: no
- **Rationale**: no default, formula, threshold, conversion, or fallback value
  changes; this corrects outlet classification to match the documented
  boundary contract

## Compatibility and regression plan

The output raster schema, CLI arguments, Python bindings, metadata keys, NoData
value, and numeric type remain unchanged. Regression tests compare every valid
cell in rotated edge cases, an enclosed control, flat-fixing behavior, and a
`max_depth` control. No generated WEPP run artifact changes are in scope
because this repository exposes the native raster tool rather than run-scoped
project schemas.

## Deliverables

- Corrected `FillDepressions` boundary semantics.
- Focused synthetic integration tests.
- Updated Rust tool documentation and `CHANGELOG.md`.
- `artifacts/validation.md` with commands and results.
