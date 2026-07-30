# Harden TopazConditionDem parity with canonical golden gates

This ExecPlan is a living document. Maintain `Progress`,
`Surprises & Discoveries`, `Decision Log`, and
`Outcomes & Retrospective` throughout execution. The repository work-package
rules are in `docs/work-packages/README.md`.

Status: Completed

Last updated: 2026-07-30 01:14 UTC

## Purpose

After this plan, a maintainer can build a release `whitebox_tools`, run one
command, and prove that FILDEP and RELIEF content matches checksummed TOPAZ
oracles across production, obstruction-mode, synthetic NoData, and
NLCD-waterbody cases. The proof uses canonical scaled integers and therefore
does not depend on TIFF metadata or Fortran record framing.

## Progress

- [x] (2026-07-30 00:49 UTC) Scaffolded repository-local work-package
  governance and this active plan.
- [x] (2026-07-30 00:49 UTC) Confirmed the production project configures
  `nlcd/2019` but does not persist a landuse raster.
- [x] (2026-07-30 01:04 UTC) Implemented optional FILDEP output, canonical
  manifest, positive three-fixture harness, and wrong-hash negative control.
- [x] (2026-07-30 01:05 UTC) Created the synthetic irregular-NoData
  fixture, fixed NoData open-boundary semantics in FILDEP and RELIEF, added
  termination/behavior regressions and a per-case harness timeout, and matched
  both TOPAZ stage hashes exactly.
- [x] (2026-07-30 01:07 UTC) Generated independent TOPAZ width-0 and width-1
  oracles on the original production DEM; Rust matches both FILDEP and RELIEF
  hashes for all three obstruction modes.
- [x] (2026-07-30 01:07 UTC) Retrieved the exact-grid `nlcd/2019` response,
  masked 25,541 class-11 cells from burned-out-harmonic, and matched TOPAZ
  FILDEP and RELIEF exactly.
- [x] (2026-07-30 01:14 UTC) Passed all gates, published validation evidence,
  archived this prompt, and closed the work package.

## Surprises & Discoveries

- Observation: All three existing production fixtures are fully valid rasters;
  none supplies empirical NoData-boundary evidence.
  Evidence: valid-cell counts equal rows multiplied by columns.

- Observation: `burned-out-harmonic/landuse.nodb` records
  `_nlcd_db = "nlcd/2019"` and `_landuse_map = null`.
  Evidence: read-only wepp1 inspection at `2026-07-29T17:49:51-07:00`.

- Observation: WBT rejects floating-point GeoTIFF predictor 3.
  Evidence: the first generated synthetic fixture failed explicitly with
  `The GeoTIFF reader does not currently support floating-point predictors`.

- Observation: Excluding NoData neighbors from lower-neighbor tests made a
  one-cell valid island loop forever in Rust RELIEF and incorrectly filled
  depressions adjacent to NoData in Rust FILDEP.
  Evidence: the first synthetic Rust run did not terminate; after bounding the
  run, FILDEP also had a different canonical hash. TOPAZ treats its
  indeterminate-elevation sentinel as lower than valid terrain in both stages.

- Observation: WMesque's configured `nlcd/2019` alias currently returns
  metadata for Annual NLCD Collection 1 version 1.1 (June 2025), year 2024.
  Evidence: retained WMesque response metadata and raster checksum
  `7f6c66164ce84267eb86774b23bfbf6e7db5b352876fbbefb36859a371546820`.

- Observation: The requested NLCD response exactly matched the production DEM
  dimensions, geotransform, and EPSG:32610 coordinate system.
  Evidence: both rasters are 1,233 by 1,184 with the same six geotransform
  values, so no additional warp was necessary.

## Decision Log

- Decision: Generate NoData topology synthetically and waterbody topology from
  NLCD rather than seek additional arbitrary production DEMs.
  Rationale: Synthetic geometry makes corner cases deliberate; NLCD adds a
  realistic irregular water mask.
  Date: 2026-07-30

- Decision: Canonical stage hash is SHA-256 over row-major, little-endian,
  signed 32-bit TOPAZ internal units.
  Rationale: It is stable across Fortran framing and raster container metadata.
  Date: 2026-07-30

- Decision: Keep expensive real-data parity outside the default Rust unit
  suite but make it a single automated, selectable command.
  Rationale: Large fixtures take seconds in release and much longer in debug.
  Date: 2026-07-30

- Decision: Model NoData as an open lower boundary for lower-neighbor tests,
  while excluding it from flat membership, high-neighbor propagation, and
  obstruction candidates.
  Rationale: This reproduces the observable TOPAZ sentinel behavior without
  storing a fabricated numeric elevation in invalid Rust cells.
  Date: 2026-07-30

## Outcomes & Retrospective

Completed. Seven golden cases cover 5,037,306 valid case-cells: all three
obstruction widths on the original production DEM, two larger all-valid
production DEMs, synthetic irregular NoData, and a production-scale NLCD-water
mask. Every FILDEP and RELIEF hash matches TOPAZ exactly.

The synthetic fixture found and closed a high-value defect that the three
all-valid production inputs could not expose: NoData had to act as an open
lower boundary in both conditioning stages. Two focused Rust tests and the
harness timeout retain the termination and numerical contract.

The final release binary passed the seven-case harness twice; reports were
byte-identical with SHA-256
`5ed28d52cae14acc656e2083e598f020a0ab8503871d8e719e6588a9275f7a9e`.
The deliberately wrong golden exited 1. Cargo check, all 132 Rust tests, Python
compilation, and diff checks passed. No runtime dependency or public default
changed. The remaining operational choice—whether and when WEPPpy adopts the
tool—was explicitly out of scope and remains with the user.

## Context and Orientation

The Rust implementation is
`whitebox-tools-app/src/tools/hydro_analysis/topaz_condition_dem.rs`. It
returns FILDEP internally and optionally writes it with `--fildep`. Both Python
wrappers live at `whitebox_tools.py` and `WBT/whitebox_tools.py`.
Fixtures and manifests live under `test_fixtures/topaz_condition_dem/`.
Original TOPAZ is `/workdir/topaz/release/libfortran5/dednm`; source revision
is pinned in the algorithm documentation.

TOPAZ stores elevation in signed integer units of 1/100000 elevation unit.
Input is first rounded to decimetres. `FILDEP.OUT` and `RELIEF.OUT` are single
Fortran unformatted records laid out in Fortran column-major storage; after
reshaping to raster rows/columns, canonical hashing writes row-major little-
endian `i32` bytes.

## Milestones

### 1. Canonical stage-output harness

Add an optional FILDEP raster output to the CLI and both wrappers. Implement
`tools/validate_topaz_condition_dem_parity.py` to read a manifest, run a
freshly built binary, canonicalize FILDEP/RELIEF to TOPAZ integers, compare
hashes, validate masks, and emit a deterministic JSON report. A deliberately
corrupted expected hash must make the harness exit nonzero.

### 2. Synthetic NoData oracle

Create a small projected GeoTIFF with an irregular internal hole, isolated
NoData island, and edge-connected NoData corridor. Feed zero as TOPAZ's
indeterminate input rather than clipping it. Require mask equality and exact
FILDEP/RELIEF values on all valid cells.

### 3. Obstruction modes

Run original TOPAZ on the 430-by-447 production fixture with obstruction widths
0 and 1, retaining the existing width-2 oracle. Add mode-specific canonical
hashes and require the Rust harness to match each stage exactly.

### 4. NLCD-derived waterbody case

Retrieve `nlcd/2019` read-only for the exact burned-out-harmonic extent with an
explicit EPSG:32610 bbox. Align nearest-neighbor to the DEM grid if necessary,
mask NLCD class 11 as NoData, and require at least one masked cell. Run TOPAZ
and Rust and require exact mask and stage parity. Record retrieval metadata,
NLCD checksum, water-cell count, and derived DEM checksum.

### 5. Closure

Run release build, full Rust tests, wrapper compilation, harness positive and
negative controls, formatting checks, and deterministic rerun. Update
algorithm/fixture/handoff/validation/changelog documents. Close `package.md`,
`tracker.md`, and `PROJECT_TRACKER.md`; move this plan to `prompts/completed/`.

## Concrete Steps

Run from `/workdir/weppcloud-wbt`.

    cargo build --release -p whitebox-tools-app
    python3 tools/validate_topaz_condition_dem_parity.py \
      --binary target/release/whitebox_tools \
      --manifest test_fixtures/topaz_condition_dem/parity_manifest.json \
      --all
    cargo test -p whitebox-tools-app
    python3 -m py_compile whitebox_tools.py WBT/whitebox_tools.py \
      tools/validate_topaz_condition_dem_parity.py
    git diff --check

Oracle generation must occur only under
`target/topaz-condition-dem/parity-hardening/`. Production sources are copied
read-only and checksummed; never modify run directories or `/workdir/topaz`.

## Validation and Acceptance

Acceptance requires:

- fresh-binary FILDEP and RELIEF canonical hashes match every manifest entry;
- input and output masks match declared contracts;
- modes 0, 1, and 2 pass on the production fixture;
- synthetic and NLCD masked cases each contain the intended NoData geometry;
- a wrong golden hash produces a nonzero harness result;
- repeat runs produce identical canonical reports;
- full Rust tests, release build, Python compilation, and diff checks pass;
- work-package lifecycle documents are closed and internally consistent.

## Idempotence and Recovery

Generated controls, TOPAZ outputs, downloads, and Rust outputs stay under the
target directory and may be safely regenerated. Fixture promotion is a
separate deliberate copy with checksum verification. If NLCD retrieval fails,
retain the URL/metadata and retry; do not substitute an undocumented dataset.
If parity fails, keep the case active, localize stage and cell differences, and
do not weaken or regenerate expected hashes from Rust output.

## Interfaces and Dependencies

No new runtime dependency is permitted. The parity harness may use the
development host's GDAL Python bindings and NumPy. NLCD retrieval uses the
existing WEPPpy WMesque client only as a read-only development utility.
`TopazConditionDem` remains deterministic and defaults to obstruction width 2.

Revision note (2026-07-30): Initial plan created from the user-approved parity
confidence recommendations; synthetic NoData and NLCD water masking are
explicit scope decisions from the conversation.

Revision note (2026-07-30): Closed after all seven canonical cases, repeated
determinism, negative control, full tests, and lifecycle documentation passed.
