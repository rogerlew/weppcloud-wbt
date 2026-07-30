# Preserve outer-edge outlets in FillDepressions

This ExecPlan is a living document. Maintain `Progress`,
`Surprises & Discoveries`, `Decision Log`, and
`Outcomes & Retrospective` throughout execution. The repository work-package
rules are in `docs/work-packages/README.md`.

Status: Completed

Last updated: 2026-07-30 21:05 UTC

## Purpose

After this plan, `FillDepressions` preserves a valid low region connected to
any outer raster edge instead of raising it to a higher internal spill. A
maintainer can run focused synthetic regressions to distinguish all four
open-edge orientations from the equivalent enclosed depression and verify
flat-gradient and `max_depth` behavior.

## Progress

- [x] (2026-07-30 20:54 UTC) Retrieved issue #1, confirmed the implicated
  source path, and created this work package.
- [x] (2026-07-30 20:57 UTC) Implemented explicit outer-edge outlet
  classification.
- [x] (2026-07-30 21:00 UTC) Added and passed focused synthetic regression
  coverage.
- [x] (2026-07-30 21:04 UTC) Proved pre-fix/fixed depression-inventory parity
  on the exact production fixture: 1,291 detected, 212 filled, 1,079 skipped.
- [x] (2026-07-30 21:05 UTC) Completed documentation, full validation,
  evidence, and package closure.

## Surprises & Discoveries

- Observation: Initial pit detection scans only rows and columns
  `1..dimension-1`, but a pit region can grow onto an edge cell.
  Evidence: `fill_depressions.rs` pit scan and priority-growth bounds.

- Observation: Flat fixing revalidates candidate outlets using only lower
  valid neighbours, so correcting initial outlet discovery alone would not
  establish an edge source for the optional gradient.
  Evidence: the `possible_outlets` confirmation loop.

- Observation: The narrow correction changes fill elevations without changing
  which depression regions are detected or processed.
  Evidence: pre-fix and fixed production diagnostics both report
  1,291 detected, 212 filled, and 1,079 skipped regions; the changed code runs
  only after `undefined_flow_cells` is finalized.

## Decision Log

- Decision: Add a shared outer-edge predicate and use it in both initial
  outlet discovery and flat-outlet confirmation.
  Rationale: Both phases implement the same outlet contract; sharing the
  predicate prevents the original classification mismatch from recurring.
  Date: 2026-07-30

- Decision: Do not replace the current implementation with an outside-in
  priority flood in this package.
  Rationale: The explicit boundary fix is sufficient for the confirmed defect
  and has substantially lower compatibility and performance risk.
  Date: 2026-07-30

- Decision: Require differential diagnostic parity with the tracked pre-fix
  binary on the exact production fixture.
  Rationale: Synthetic topology proves boundary behavior; identical inventory
  counts directly prove the fix still catches and processes the same
  depression candidates.
  Date: 2026-07-30

## Outcomes & Retrospective

Completed. A shared four-edge predicate now establishes valid outer cells as
outlets during priority growth and retains them when flat-gradient candidates
are confirmed. Five focused tests cover west/north/east/south rotations, the
equivalent enclosed depression, explicit flat increments, `max_depth`, and
interior NoData behavior.

The tracked pre-fix and rebuilt fixed binaries reported the identical
production depression inventory: 1,291 detected low points, 212 filled
regions, and 1,079 skipped regions. All 19,891 changed output cells moved
downward from the erroneous baseline; none moved upward. At the issue reference
point, the fixed output preserves 533.868286 m instead of 910.068481 m.

Cargo check, all 140 Rust tests, Python wrapper compilation, and diff checks
passed. No CLI, wrapper, raster schema, metadata, dependency, or
`BreachDepressions` change was introduced.

## Context and Orientation

The implementation is
`whitebox-tools-app/src/tools/hydro_analysis/fill_depressions.rs`. It first
finds interior cells without lower neighbours, then grows a priority region
until it finds a lower valid neighbour. Cells outside the raster return NoData,
which currently prevents the outer boundary from being recognized.

Candidate outlets are retained in `possible_outlets`. When `--fix_flats` is
enabled, a later loop confirms those candidates before growing the small
gradient. The same edge semantic must apply there.

Focused integration tests will live beside the tool under
`whitebox-tools-app/src/tools/hydro_analysis/` and will construct temporary
7-by-7 rasters, invoke `FillDepressions::run`, and inspect the resulting raster
values.

## Milestones

### 1. Boundary classification

Introduce a small predicate for valid outer-grid coordinates. When the
priority search pops an outer-edge cell before finding another outlet, record
that cell as an outlet at its current elevation. Reuse the predicate when
confirming candidates for flat fixing. Do not classify interior NoData as an
outlet and do not change the initial pit scan.

### 2. Synthetic regression matrix

Create a 7-by-7 low flat with a higher saddle and lower terrain beyond it.
Verify the west-connected form is preserved, rotate it to cover north, east,
and south, and verify an enclosed form fills to its saddle. Add controls for
flat fixing and `max_depth`.

### 3. Documentation and closure

Update the tool documentation to state that valid outer-edge cells are outlets
at their current elevations and that NoData regions are not filled or treated
as open drainage. Update `CHANGELOG.md`, run all validation gates, record
results in `artifacts/validation.md`, and consistently close the active plan,
package, tracker, project board, and root `AGENTS.md` pointer.

## Concrete Steps

Run from `/workdir/weppcloud-wbt`.

    cargo fmt --all -- --check
    cargo test -p whitebox-tools-app fill_depressions_edge_outlet
    cargo check -p whitebox-tools-app
    cargo test -p whitebox-tools-app
    python3 -m py_compile whitebox_tools.py WBT/whitebox_tools.py
    git diff --check

## Validation and Acceptance

Acceptance requires:

- unchanged low-region values with `--fix_flats=false` for every outer edge;
- an unchanged edge outlet and monotonic small gradient away from it with
  `--fix_flats=true`;
- fill to the lowest valid spill for the enclosed equivalent;
- no fill when the enclosed depth exceeds `max_depth`;
- unchanged NoData mask and no new public argument or metadata schema;
- identical pre-fix and fixed `detected_low_point_count`,
  `filled_depression_count`, and `skipped_depression_count` on the production
  reproducer;
- targeted and full Rust tests, Cargo check, Python compilation, formatting,
  and diff checks pass;
- package lifecycle documents close consistently.

## Idempotence and Recovery

Tests use uniquely named files in the operating-system temporary directory and
remove their raster and sidecar outputs after assertions. Re-running tests is
safe. The code change is localized and can be reverted independently from the
work-package documentation. If the narrow fix cannot satisfy the enclosed or
flat tests, record the evidence before considering a larger algorithm change.

## Interfaces and Dependencies

No new runtime or development dependency is allowed. CLI flags, Python
wrappers, raster output schema, metadata keys, and `BreachDepressions` remain
unchanged.

Revision note (2026-07-30): Initial plan created from issue #1 and the user's
request to execute the fix as a repository work package.

Revision note (2026-07-30 21:05 UTC): Closed after focused, full-suite,
production-reproducer, and differential depression-inventory validation.
