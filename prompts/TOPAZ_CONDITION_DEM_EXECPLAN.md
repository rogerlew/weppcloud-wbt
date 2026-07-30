# Implement `TopazConditionDem` with TOPAZ FILDEP and RELIEF parity

This ExecPlan is a living document. Keep `Progress`, `Surprises & Discoveries`,
`Decision Log`, and `Outcomes & Retrospective` current as work proceeds.

Status: Completed

Owner: unassigned

Last updated: 2026-07-29

## Purpose

WEPPcloud currently uses WhiteboxTools depression conditioning before deriving
flow directions. At the `fluvial-succession/disturbed9002_wbt` outlet, the
existing fill-based workflow raises the outlet-area depression from about
533.7 m to about 910.1 m. The conditioned surface consequently gives the
eastern terrain the controlling slope and the watershed follows that flow
vector.

This work adds a standalone WhiteboxTools tool named `TopazConditionDem` that
faithfully ports TOPAZ depression filling, narrow-obstruction adjustment, and
flat resolution. The tool conditions a DEM only; existing WhiteboxTools
flow-direction and watershed tools consume its output.

Success means that the Rust tool and both shipped Python wrappers reproduce
canonical TOPAZ FILDEP and RELIEF results, emit deterministic output, and
provide inspectable cut/fill diagnostics. The exact 430-by-447 production DEM
that exposed the problem is the primary empirical fixture.

WEPPpy integration is outside this plan. The completed work must provide a
stable interface and integration handoff so the user can add the
`topaz_condition` option without reverse-engineering this repository.

## Progress

- [x] (2026-07-29 22:52Z) Created the dedicated ExecPlan and established the
  standalone-tool and WEPPpy-handoff scope.
- [x] (2026-07-29 23:04Z) Captured and checksummed the exact 430-by-447
  production DEM and ran TOPAZ in preprocessing-only mode.
- [x] (2026-07-29 23:18Z) Extracted and documented normative TOPAZ FILDEP,
  obstruction, numeric-scaling, and RELIEF behavior.
- [x] (2026-07-30 00:44Z) Established reproducible golden TOPAZ fixtures and
  zero-mismatch production parity metrics.
- [x] (2026-07-29 23:42Z) Implemented the Rust conditioning kernel and focused
  unit tests.
- [x] (2026-07-29 23:43Z) Registered `TopazConditionDem` and its CLI parameters.
- [x] (2026-07-29 23:55Z) Added matching methods to both Python wrappers.
- [x] (2026-07-30 00:48Z) Validated depression filling, obstruction adjustment,
  flat resolution, determinism, NoData handling, and raster-edge behavior.
- [x] (2026-07-29 23:48Z) Added cut/fill diagnostics, optional delta/JSON
  artifacts, and output metadata.
- [x] (2026-07-29 23:56Z) Published the WEPPpy integration handoff without
  editing WEPPpy.
- [x] (2026-07-30 00:55Z) Completed production parity, downstream D8/watershed,
  release performance, and larger-raster validation.
- [x] (2026-07-30 01:10Z) Promoted the exact checksummed production DEM into
  `test_fixtures/topaz_condition_dem/dem.tif`.

## Surprises & Discoveries

- Observation: The earlier TOPAZ comparison run,
  `srivas42-stimulant-applejack`, used a 315-by-328 DEM, not the 430-by-447 DEM
  used by `srivas42-reconciled-turf` and the WBT run.
  Evidence: The production run artifacts have different raster dimensions, so
  the earlier result does not establish what TOPAZ does on the incident DEM.

- Observation: On its different input, TOPAZ left the outlet at 533.0 m while
  filling only 164 cells, with a maximum fill of about 1.5 m. RELIEF added
  sub-millimetre perturbations, with a maximum observed change of about
  0.00052 m.
  Evidence: Comparison of that run's input elevation, FILDEP, and RELIEF
  artifacts.

- Observation: The current WBT fill workflow raises the incident outlet-area
  elevation by roughly 376.4 m. A least-cost breach configured for 1000 m
  (33 cells at 30 m resolution) still falls back to the same deep fill.
  Evidence: Production-artifact comparison and existing breach/fill outputs.

- Observation: The WEPPpy TOPAZ control template requests adjustment of
  obstructions up to two cells wide and disables aggregation and smoothing.
  Evidence:
  `/home/workdir/wepppy/wepppy/topo/topaz/topaz_templates/DNMCNT.INP.template`.

- Observation: On the exact production input, TOPAZ keeps the requested outlet
  at 533.0 m. FILDEP modifies 581 cells (maximum fill 2.3 m, maximum cut
  5.0 m); RELIEF modifies 1,924 cells by at most 0.00148 m.
  Evidence: Checksummed preprocessing-only INELEV, FILDEP, and RELIEF arrays in
  `target/topaz-condition-dem/oracle-run/`.

- Observation: TOPAZ's 32-bit input multiplication affects 65
  half-decimetre-adjacent production cells. Performing the multiplication in
  Rust `f64` gives the wrong rounding result.
  Evidence: Cell-by-cell comparison of INELEV.OUT with both f32 and f64
  quantization.

- Observation: Temporary TOPAZ/Rust traces showed that the final five paired
  obstruction differences were caused by a mistranslated final comparison and
  candidate connectivity through already-resolved terrain. Preserving those
  source rules yields zero mismatches across all 192,210 valid cells.
  Evidence: `prompts/artifacts/topaz_condition_dem_validation.md`.

- Observation: `cargo build --release -p whitebox-tools-app` produced a current
  hashed executable under `target/release/deps/`, while the pre-existing
  top-level release executable remained stale.
  Evidence: tool discovery and benchmark were run with the freshly timestamped
  hashed executable.

## Decision Log

- Decision: Port TOPAZ behavior from source rather than approximate it with a
  composition of existing WBT tools.
  Rationale: FILDEP outlet-obstruction adjustment and RELIEF flat resolution
  require a defensible parity contract.
  Date: 2026-07-29

- Decision: Name the public tool `TopazConditionDem`.
  Rationale: The name describes the combined FILDEP and RELIEF operation while
  leaving flow direction and watershed derivation explicitly downstream.
  Date: 2026-07-29

- Decision: Treat `/workdir/topaz/src/dednm.f90` as the normative behavioral
  source and preserve its provenance and applicable license notices.
  Rationale: This is an authorized source-level translation, not a clean-room
  reimplementation.
  Date: 2026-07-29

- Decision: Default obstruction adjustment to two cells, matching the current
  WEPPpy TOPAZ control template, while exposing zero-, one-, and two-cell
  modes.
  Rationale: The production configuration is the compatibility baseline and the
  original algorithm exposes these modes.
  Date: 2026-07-29

- Decision: Require exact-input evidence before claiming that TOPAZ conditioning
  resolves the incident.
  Rationale: The existing comparison is confounded by different DEM dimensions.
  If TOPAZ also raises the exact outlet dramatically, the tool may still achieve
  faithful parity but must not be represented as an incident fix.
  Date: 2026-07-29

- Decision: Replace direct WEPPpy integration with an integration handoff.
  Rationale: The user will implement the WEPPpy option after the WBT tool is
  ready.
  Date: 2026-07-29

- Decision: Preserve TOPAZ's final spill replacement comparison and check
  candidate connectivity without crossing previously resolved cells.
  Rationale: Temporary event traces isolated these as normative source
  semantics and the resulting implementation achieves exact production parity.
  Date: 2026-07-29

## Outcomes & Retrospective

The plan is complete. Confirmed outcomes:

- Exact-input TOPAZ preserves the western/outlet elevation: the requested cell
  remains 533.0 m rather than filling to approximately 910.1 m.
- The Rust tool, CLI registration, both wrappers, delta raster, diagnostics,
  algorithm documentation, oracle materialization tool, and WEPPpy handoff are
  implemented.
- All 192,210 valid output cells match TOPAZ exactly; no deviations are
  accepted or unexplained.
- Derived WBT D8 pointer and outlet-watershed arrays are also identical.
- Five warm release runs complete in 0.48-0.53 seconds (median 0.49 seconds)
  with peak RSS of 27,668 KiB. A four-times-cell-count representative raster
  completes in 3.33 seconds with 59,408 KiB peak RSS.
- Exact-input TOPAZ and Rust both preserve the 533.0 m requested outlet instead
  of filling it toward approximately 910.1 m.
- The CLI, both wrappers, diagnostic schema, metadata, fixture tooling, and
  WEPPpy handoff are documented. WEPPpy integration remains with its owner.
- The exact 430-by-447 production DEM is committed as a durable regression
  fixture; derived TOPAZ stage rasters remain reproducibly materialized from
  the checksummed oracle outputs.

## Context and Orientation

WhiteboxTools applications are implemented under `whitebox-tools-app/src/tools/`.
Place the new Rust module at:

    whitebox-tools-app/src/tools/hydro_analysis/topaz_condition_dem.rs

Register it in:

    whitebox-tools-app/src/tools/hydro_analysis/mod.rs
    whitebox-tools-app/src/tools/mod.rs

Add the same wrapper method to:

    whitebox_tools.py
    WBT/whitebox_tools.py

Follow `DEVELOPING_TOOLS.md` for metadata, parameters, parsing, raster I/O,
progress, timing, and wrapper conventions. No new runtime dependency is
expected.

The normative TOPAZ implementation is `/workdir/topaz/src/dednm.f90`.
`FILDEP` discovers and fills depressions and optionally lowers one- or two-cell
outlet obstructions. `RELIEF` adds deterministic relief across flats using
surrounding rising/falling terrain and outlet proximity. `FLOVEC`/`FLOVE1`,
`UPAREA`, and `BOUND` are oracle checks, not port scope. The Rust tool ends
after writing the RELIEF-equivalent DEM.

The incident raster is the 430-by-447, 30 m, EPSG:32610 DEM from
`srivas42-reconciled-turf`. On wepp1:

    /geodata/wc1/runs/sr/srivas42-reconciled-turf/dem/dem.tif

In the production container:

    /wc1/runs/sr/srivas42-reconciled-turf/dem/dem.tif

It has 192,210 valid cells. Record a SHA-256 checksum; do not rely on the run
name alone.

A "golden fixture" is an input raster, canonical checksummed TOPAZ stage
outputs, and sufficient controls and provenance to reproduce them. "Parity"
means equality after conversion to the documented integer scale used by TOPAZ.
A floating-point GeoTIFF comparison is secondary and uses a tolerance derived
from that scale, not an arbitrary visual tolerance.

## Plan of Work

### Milestone 1: Exact-input empirical comparison

Copy the exact production DEM into a gitignored staging directory. Record its
checksum, dimensions, transform, CRS, NoData value, valid-cell count, vertical
units, and source provenance. Do not substitute a resampled, reprojected,
clipped, or separately downloaded raster.

Run original TOPAZ preprocessing on this exact raster with 30 m cells, no
aggregation, no smoothing, and two-cell obstruction adjustment. Capture the
input as interpreted by TOPAZ, post-FILDEP elevation, post-RELIEF elevation,
flow vectors, reports, and controls. Convert staged arrays to georeferenced
GeoTIFFs without changing values.

Measure the incident outlet and contributing area at each stage. Report maximum
and total fill/cut, modified-cell counts, outlet flow vector, and whether the
western route remains available. State explicitly whether TOPAZ avoids the
910 m fill on the exact input.

### Milestone 2: Normative behavior specification

Read and annotate `FILDEP`, `RELIEF`, directly called helpers, and associated
CAPLIM documentation. Create:

    docs/topaz_condition_dem_algorithm.md

Document input quantization and scaling; depression discovery, window
expansion, spill selection, and fill ordering; obstruction modes; flat relief;
tie-breaking; edges and NoData; integer limits; warnings and failures; source
revision and license; and explicit in/out-of-scope routines.

Resolve the apparent documentation/code discrepancy around large-flat limits
from source and observed behavior. Do not silently "improve" TOPAZ under the
parity name. Enhanced behavior, if ever needed, must be separate from the
default compatibility contract.

### Milestone 3: Golden fixtures and parity harness

Create:

    test_fixtures/topaz_condition_dem/
    tools/prepare_topaz_condition_dem_fixture.py

The preparation script accepts an exact DEM and TOPAZ checkout/executable,
creates controls, runs preprocessing, converts stage outputs, and writes a
manifest with checksums, revisions, parameters, raster metadata, and conversion
commands. It is development tooling, not a runtime dependency.

Include the exact production DEM and golden outputs if provenance permits
redistribution and repository size remains reviewable. Otherwise commit the
manifest, preparation script, checksums, and materialization command, while
keeping committed synthetic fixtures usable in CI. Never silently substitute a
crop: cropping can change spill elevation.

Synthetic golden cases must cover a single-cell pit; one- and two-cell outlet
obstructions; a wider obstruction; a flat with one outlet; competing outlets
and ties; interacting depressions; each raster edge; an internal NoData island;
and an external NoData boundary.

Compare canonical scaled-integer FILDEP and RELIEF arrays cell by cell. The
target is zero unexplained mismatches. GeoTIFF values may differ only within
half one final scale unit. A mismatch report includes row, column, stage,
expected/actual values, and local neighborhood.

### Milestone 4: Rust conditioning kernel

Implement inspectable FILDEP-equivalent and RELIEF-equivalent stages. Preserve
TOPAZ iteration order, integer scaling, and tie-breaking. Avoid recursion that
can overflow on large flats and avoid accidental per-cell full-raster scans.

Use checked conversions and explicit errors for unsupported dimensions,
invalid parameters, non-finite valid elevations, and internal scale overflow.
Preserve NoData and raster geometry. Keep raster I/O separate from the kernel
so small grids can be tested directly.

### Milestone 5: Tool registration and CLI

Register `TopazConditionDem` in the hydro-analysis and global tool registries.
The initial public contract is:

    --dem, -i                    required input DEM
    --output, -o                 required conditioned DEM
    --max_obstruction_width      optional 0, 1, or 2; default 2
    --delta                      optional signed output-minus-input raster
    --diagnostics                optional JSON diagnostics path

The primary output is post-RELIEF. Do not expose configurable flat increment or
elevation precision until Milestone 2 proves a coherent contract. Defaults use
TOPAZ-derived scaling and increment. Any advanced controls must be explicitly
outside strict parity.

Verify discovery and parameter JSON from a newly built binary.

### Milestone 6: Both Python wrappers

Add `topaz_condition_dem(...)` to `whitebox_tools.py` and
`WBT/whitebox_tools.py`. Signatures, defaults, generated arguments, callback
behavior, and return contracts must match. Test that both wrappers emit the
same CLI and omit optional artifacts when not requested.

### Milestone 7: Behavioral and edge validation

In addition to golden parity, validate repeat-run determinism; traversal and
thread independence; unchanged NoData/georeferencing; valid edge behavior;
obstruction modes zero/one/two; no TOPAZ-resolvable depression or flat left
unresolved; and explicit failure for bad parameters and unusable rasters.

Run existing WBT D8 pointer and watershed processing on both the golden TOPAZ
RELIEF raster and the new output. These derived WBT artifacts must match each
other exactly. Compare TOPAZ FLOVEC as a secondary semantic check and document
direction-code or tie-breaking differences outside conditioning scope.

### Milestone 8: Diagnostics and metadata

Always add output metadata identifying the tool, input, obstruction width,
TOPAZ source/compatibility revision, and elapsed time.

Versioned diagnostics JSON includes raster identity and parameters; depression
and flat counts; unchanged, filled, lowered, and synthetic-relief-only cells;
obstruction adjustments by width; signed extrema; maximum fill/cut/relief;
positive fill and cut volume when units permit; warnings or unresolved
features; and output/delta checksums.

Use projected cell area for volume and state units. If horizontal or vertical
units are ambiguous, emit `null` with a reason rather than guessing. The delta
raster is signed `conditioned - input`, preserves NoData, and states its sign
and units. Derive stage diagnostics from kernel data, not final-value
thresholding.

### Milestone 9: WEPPpy integration handoff

Do not edit `/home/workdir/wepppy`. Add:

    docs/topaz_condition_dem_wepppy_handoff.md

Document the released tool/version or commit; CLI and wrapper signatures;
minimal CLI/Python examples; output, unit, NoData, metadata, and diagnostics
contracts; defaults; placement before D8; contrast with existing fill/breach
options; exact production result; limitations; and suggested WEPPpy config,
tests, and artifact names. Warn clearly if exact-input TOPAZ does not resolve
the motivating incident.

### Milestone 10: Production parity and performance

Run the newly built binary on the exact production DEM and write:

    prompts/artifacts/topaz_condition_dem_validation.md

Record commands, commits, checksums, parameters, mismatch counts, outlet
elevations and flow direction, watershed statistics, and diagnostics. Include
compact outlet/mismatch maps only when they materially explain behavior.

Benchmark at least five warm release runs and the available TOPAZ oracle on the
same host/input. Report median/range wall time and peak RSS. Require the
430-by-447 case to finish under 30 seconds on the documented development host,
O(cell count) retained raster storage, and no pathological scaling on one
larger representative DEM. Exact parity takes priority over optimization.

Update `CHANGELOG.md`, this plan, and the handoff with final evidence.

## Concrete Steps

Run from `/workdir/weppcloud-wbt` unless stated otherwise.

1. Record revisions:

       git rev-parse HEAD
       git -C /workdir/topaz rev-parse HEAD

2. Stage and checksum the exact input:

       mkdir -p target/topaz-condition-dem/oracle
       cp /wc1/runs/sr/srivas42-reconciled-turf/dem/dem.tif \
         target/topaz-condition-dem/oracle/dem.tif
       sha256sum target/topaz-condition-dem/oracle/dem.tif
       gdalinfo -json target/topaz-condition-dem/oracle/dem.tif \
         > target/topaz-condition-dem/oracle/dem.gdalinfo.json

   If `/wc1` is unavailable, copy the exact file from wepp1 using established
   operator procedures and verify its checksum.

3. Implement and run the reproducible TOPAZ oracle preparation in Milestones
   1-3. Save commands and checksums in the manifest and validation artifact.

4. Implement the Rust module, registrations, wrappers, tests, documentation,
   diagnostics, and handoff in milestone order.

5. During iteration:

       cargo fmt --all -- --check
       cargo check -p whitebox-tools-app
       cargo test -p whitebox-tools-app topaz_condition_dem -- --nocapture
       python3 -m py_compile whitebox_tools.py WBT/whitebox_tools.py

6. Exercise the generated binary:

       cargo build -p whitebox-tools-app
       target/debug/whitebox_tools --toolparameters=TopazConditionDem
       target/debug/whitebox_tools \
         --run=TopazConditionDem \
         --dem=target/topaz-condition-dem/oracle/dem.tif \
         --output=target/topaz-condition-dem/conditioned.tif \
         --max_obstruction_width=2 \
         --delta=target/topaz-condition-dem/delta.tif \
         --diagnostics=target/topaz-condition-dem/diagnostics.json

7. Before handoff:

       cargo test -p whitebox-tools-app
       cargo build --release -p whitebox-tools-app
       git diff --check

8. Run golden parity, derived D8/watershed, determinism, and performance
   harnesses. Put concise evidence in the validation artifact.

## Validation and Acceptance

Acceptance requires all of the following:

- Original TOPAZ has processed the exact 430-by-447 input and the result is not
  conflated with the 315-by-328 run.
- The algorithm document covers FILDEP, obstruction adjustment, RELIEF,
  scaling, tie-breaking, boundaries, and provenance.
- Synthetic and production fixtures have zero unexplained canonical
  scaled-integer mismatches at FILDEP and RELIEF. An unresolved production
  mismatch keeps the plan active.
- A freshly built binary discovers the tool and reports the documented schema.
- Both wrappers produce equivalent invocations.
- Output, delta, diagnostics, and metadata satisfy their contracts.
- Determinism, NoData, edge, obstruction, invalid-input, and downstream
  D8/watershed tests pass.
- Full Rust tests, release build, Python compilation, and `git diff --check`
  pass.
- Production parity/performance evidence and the WEPPpy handoff are committed.
- WEPPpy itself remains unchanged.

Passing tests alone is insufficient. Acceptance requires checksummed evidence
from the newly built binary.

## Idempotence and Recovery

Generate temporary data only under `target/topaz-condition-dem/` until
deliberately promoting fixtures or documentation. Identical inputs, revisions,
and parameters must reproduce canonical arrays and stable diagnostics.

Never overwrite production artifacts. Copy and checksum them. Do not modify
`/workdir/topaz` or `/home/workdir/wepppy`.

Retain failed oracle controls, logs, and partial outputs in a timestamped
staging directory. Reduce mismatches to small fixtures when possible, but never
waive the full production comparison. Public parameter or semantic changes
must update implementation, fixtures, both wrappers, docs, handoff, and this
decision log together.

## Artifacts and Notes

Expected durable artifacts:

    docs/topaz_condition_dem_algorithm.md
    docs/topaz_condition_dem_wepppy_handoff.md
    prompts/artifacts/topaz_condition_dem_validation.md
    test_fixtures/topaz_condition_dem/
    tools/prepare_topaz_condition_dem_fixture.py

The fixture manifest records production DEM checksum; TOPAZ revision and
executable checksum; WBT revision; controls; stage checksums; geometry, CRS,
NoData, and units; and conversion-tool versions and commands. Do not commit
credentials, private run metadata, unrelated run artifacts, or temporary TOPAZ
working files.

## Interfaces and Dependencies

The public Rust tool implements `WhiteboxTool` and is advertised under hydro
analysis as `TopazConditionDem`. The kernel exposes internal stage results for
tests and diagnostics without making implementation details public CLI.

Both Python interfaces are conceptually:

    topaz_condition_dem(
        dem,
        output,
        max_obstruction_width=2,
        delta=None,
        diagnostics=None,
        callback=None,
    )

Exact ordering follows adjacent wrapper conventions; the CLI is the
compatibility authority.

Runtime code uses existing repository raster and serialization facilities.
Development fixture tooling may use the established GDAL/rasterio/TOPAZ
environment, but users of the built tool must not need TOPAZ, Python, GDAL
command-line tools, WEPPpy, or a new external library.

Revision note (2026-07-29): Initial plan created from the production
conditioning investigation. The proposed WEPPpy integration milestone is a
documented handoff because the user owns that integration.
