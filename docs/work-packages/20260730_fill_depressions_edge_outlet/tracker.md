# Tracker – FillDepressions edge outlets

## Quick status

**Timezone**: UTC

**Started**: 2026-07-30 20:54 UTC

**Current phase**: Closed

**Last updated**: 2026-07-30 21:05 UTC

**Next milestone**: none

**Security impact**: none

**Dedicated security review**: no

## Task board

### In progress

- None.

### Ready

- None.

### Blocked

- None.

### Done

- [x] Confirmed issue #1 against the current source and opened this work
  package (2026-07-30 20:54 UTC).
- [x] Corrected edge-outlet discovery and confirmation and added focused
  synthetic coverage (2026-07-30 21:00 UTC).
- [x] Proved exact production depression-inventory parity and confirmed all
  changed elevations move only downward from the erroneous baseline
  (2026-07-30 21:04 UTC).
- [x] Passed targeted tests, Cargo check, all 140 Rust tests, wrapper
  compilation, and diff checks; closed the package (2026-07-30 21:05 UTC).
- [x] Built the locked release, installed the byte-identical tracked WEPPpy
  runtime binary, and passed host/container execution smoke tests
  (2026-07-30 21:09 UTC).
- [x] Closed the CI-only shared-raster worker-lifetime race exposed by the
  small synthetic fixture and repeated the validation/release cycle
  (2026-07-30 22:15 UTC).

## Decisions

- **2026-07-30 20:54 UTC** – Apply the narrow explicit-edge correction before
  considering an outside-in rewrite. This directly fixes the confirmed path
  while minimizing parity and performance risk.
- **2026-07-30 20:54 UTC** – Keep interior NoData closed. The existing pit
  detector intentionally excludes cells adjacent to NoData, and issue #1 does
  not authorize changing that contract.

## Risks

| Risk | Severity | Mitigation | Status |
| --- | --- | --- | --- |
| Edge cells are discovered but not retained as flat outlets | High | Apply the same edge predicate during outlet confirmation and test `--fix_flats` | Mitigated |
| Narrow fix changes enclosed-depression behavior | Medium | Retain an enclosed control with the same synthetic topology | Mitigated |
| Rotation exposes row/column asymmetry | Medium | Exercise all four edges by rotating one matrix | Mitigated |
| Edge correction skips unrelated depressions | High | Compare pre-fix and fixed diagnostics on the exact production fixture | Mitigated: identical 1291/212/1079 inventory |
| Broad algorithm rewrite creates parity/performance drift | High | Keep pit-outward structure and change only boundary classification | Mitigated |

## Verification checklist

- [x] Targeted integration tests pass.
- [x] `cargo check -p whitebox-tools-app` passes.
- [x] `cargo test -p whitebox-tools-app` passes.
- [x] Python wrapper compilation passes.
- [x] `git diff --check` passes.
- [x] Package prompt, tracker, package, project board, and root pointer close
  consistently.

## Progress notes

### 2026-07-30 20:54 UTC

Issue #1 identifies that `Raster::get_value` returns NoData outside the grid,
while outlet discovery requires a lower valid neighbour. The current search
therefore walks through a valid edge-connected flat to a higher internal
spill. The implementation already excludes outer cells from initial pit
detection, so the correction belongs where a growing region reaches an edge
cell and where candidate outlets are confirmed for flat fixing.

### 2026-07-30 21:04 UTC

The tracked pre-fix `WBT/whitebox_tools` and rebuilt fixed CLI processed the
exact production fixture with diagnostics enabled. Both reported 1,291
detected low points, 212 filled depression regions, and 1,079 skipped regions.
The fixed raster never exceeded the baseline raster: all 19,891 changed cells
were lower, with a maximum removed erroneous raise of 379.084534 m. This
confirms the fix preserves the depression inventory while changing only fill
levels selected for affected edge-connected regions.
