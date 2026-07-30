# Tracker – TopazConditionDem parity hardening

## Quick status

**Timezone**: UTC

**Started**: 2026-07-30 00:49 UTC

**Current phase**: Closed

**Last updated**: 2026-07-30 01:14 UTC

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

- [x] Scaffolded repository-local work-package workflow and active package
  (2026-07-30 00:49 UTC).
- [x] Located the configured `nlcd/2019` source; confirmed the disturbed
  project does not persist an NLCD map (2026-07-30 00:49 UTC).
- [x] Added optional FILDEP output and a canonical stage-hash harness; all
  three production cases pass and a wrong hash exits 1
  (2026-07-30 01:04 UTC).
- [x] Added a 41-by-47 synthetic irregular-NoData fixture and exact TOPAZ
  oracle; fixed FILDEP and RELIEF open-boundary behavior, added two Rust
  regressions and a harness timeout, and matched both stages
  (2026-07-30 01:05 UTC).
- [x] Added independent width-0 and width-1 TOPAZ stage hashes on the original
  production DEM; both match Rust exactly (2026-07-30 01:07 UTC).
- [x] Retrieved the exact-grid configured NLCD source, masked 25,541 class-11
  cells from burned-out-harmonic, and matched both TOPAZ stages exactly
  (2026-07-30 01:07 UTC).
- [x] Passed the fresh release build, 132 Rust tests, Python compilation,
  seven-case harness twice, byte-identical report comparison, wrong-hash
  control, and diff checks; published evidence and closed the package
  (2026-07-30 01:14 UTC).

## Decisions

- **2026-07-30 00:49 UTC** – Use a synthetic fixture for systematic NoData
  topology and an NLCD-derived mask for production waterbody geometry.
- **2026-07-30 00:49 UTC** – Record canonical row-major little-endian TOPAZ
  internal integers so parity is independent of Fortran record markers and
  GeoTIFF metadata.
- **2026-07-30 00:49 UTC** – Retrieve NLCD read-only for the DEM extent because
  disturbed mapping leaves `_landuse_map` unset in the production project.

## Risks

| Risk | Severity | Mitigation | Status |
| --- | --- | --- | --- |
| NLCD tile alignment differs from DEM | Medium | Retrieve with explicit EPSG:32610 extent and verify exact grid before masking | Mitigated; exact match |
| NoData behavior differs between TOPAZ and GeoTIFF | Medium | Use explicit TOPAZ indeterminate value and compare masks plus valid values | Mitigated; exact match |
| Full real-data tests are too slow for default unit suite | Medium | Keep a separate release-binary parity harness with selectable fixtures | Mitigated |
| Golden hashes accidentally include container metadata | Low | Hash canonical scaled integer content only | Mitigated |
| Synthetic TIFF uses unsupported encoding | Low | Use DEFLATE without floating-point predictor 3 | Mitigated |
| Rust can loop forever on isolated valid islands | High | Treat NoData as TOPAZ's open lower boundary and enforce a per-case subprocess timeout | Mitigated |

## Verification checklist

- [x] Canonical manifests validate against TOPAZ stage outputs.
- [x] Harness fails on a deliberately wrong expected hash.
- [x] All defined parity cases pass.
- [x] Release build and full Rust tests pass.
- [x] Python wrapper compilation and harness syntax checks pass.
- [x] `git diff --check` passes.
- [x] Temporary TOPAZ and retrieval artifacts remain under `target/`.
- [x] Package prompt archived and package/tracker/project board closed.

## Progress notes

### 2026-07-30 00:49 UTC

The package was created from the user's request to convert the recommended
confidence improvements into durable repository infrastructure and evidence.
Read-only wepp1 inspection confirmed that burned-out-harmonic configures
`nlcd/2019` but has no persisted landuse raster because its active mapping is
`disturbed`.

### 2026-07-30 01:05 UTC

The synthetic fixture exposed two facets of the same NoData semantic defect:
Rust FILDEP treated cells beside NoData as closed depressions, and Rust RELIEF
could raise a valid island forever. TOPAZ's numeric indeterminate sentinel is
lower than valid terrain. Explicit open-boundary predicates now reproduce that
behavior without assigning numeric data to invalid cells. Both FILDEP and
RELIEF hashes match the TOPAZ oracle exactly.

### 2026-07-30 01:07 UTC

Widths 0 and 1 match independent TOPAZ FILDEP and RELIEF hashes, completing
mode coverage. The WMesque response for `nlcd/2019` already matched the
burned-out-harmonic grid exactly. Masking its 25,541 class-11 cells yielded
exact TOPAZ stage parity on 1,434,331 valid cells. Response metadata reports
the alias is currently backed by Annual NLCD Collection 1 year 2024.

### 2026-07-30 01:14 UTC

The final fresh release binary passed all seven canonical cases twice. The two
reports were byte-identical with SHA-256
`5ed28d52cae14acc656e2083e598f020a0ab8503871d8e719e6588a9275f7a9e`.
The negative control exited 1 on the intentionally wrong RELIEF hash. Cargo
check, the 132-test Rust suite, Python compilation, and `git diff --check`
passed. Production data and TOPAZ sources remain unmodified.
