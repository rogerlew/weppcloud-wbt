# TopazConditionDem validation

Date: 2026-07-29 PDT

## Revisions and input

- weppcloud-wbt base: `5b57eab546da5a97e4bca5f4e986305e99b198de`
- TOPAZ source: `116607fc1185800ca78e387454ef1ccd3ffd73b4`
- Production DEM SHA-256:
  `b87f189bf3aa79b7f25542f0982378e193d11164fec55a68f7310e6256a8282a`
- Raster: 430 rows by 447 columns, 30 m, EPSG:32610, 192,210 valid cells
- TOPAZ controls: no aggregation/resampling, no smoothing, obstruction width 2,
  preprocessing only

Read-only wepp1 verification:

    ssh wepp1 'hostname; pwd; date -Is; stat ...; sha256sum .../dem.tif'

reported host `wepp1`, `/home/roger`, timestamp
`2026-07-29T16:04:02-07:00`, size 769,856 bytes, and the checksum above.

## Exact-input TOPAZ oracle

The preprocessing-only `dednm` run completed normally. Canonical Fortran
unformatted-stage checksums:

- `INELEV.OUT`: `9c6ebdcb9638925d7995f33863aa79b80c64619aebb67a6af26aafeae32809c2`
- `FILDEP.OUT`: `60956300daeba030c7671b251f06ffaf07ca17f962f289b5820276bc96d74444`
- `RELIEF.OUT`: `9f15579622db0f30de79bd7ce123d0834bba7f6196e787fc4f847cb9fcbbc84e`

At requested raster cell `(row=220, column=71)`, the source elevation
533.0064 m is rounded to 533.0 m and remains 533.0 m after both stages. At the
selected outlet `(223, 74)`, 533.4083 m rounds to and remains 533.4 m.

FILDEP changes 581 cells: maximum fill 2.3 m and maximum cut 5.0 m. RELIEF
changes 1,924 cells, only upward, with maximum 0.00148 m. TOPAZ therefore does
not reproduce the approximately 910.1 m filled outlet.

## Rust generated-binary evidence

The generated release binary discovers `TopazConditionDem`. After the
expanding-window correction, five warm runs on the production DEM took
0.17-0.50 seconds (median 0.18 seconds), with peak RSS of 27,664 KiB. The
corresponding TOPAZ `libfortran5` oracle runs took
0.22-0.24 seconds (median 0.23 seconds), with peak RSS of 5,376 KiB.

Current strict final-array comparison:

    matching valid cells: 192,210
    mismatching cells:          0

Temporary observation logging in TOPAZ and Rust showed identical ordering for
all 155 two-cell obstruction events. The last five paired differences came
from TOPAZ's final equal-distance replacement comparison and the membership
state produced by its bounded search window. The production comparison is
exact after preserving both rules.

Modes 0, 1, and 2 produced respectively:

- width 0: 157 depressions, 831 filled cells, no cuts, maximum fill 6.9 m;
- width 1: 83 depressions, 527 fills, 128 cuts, maximum fill 3.0 m and cut
  4.4 m;
- width 2: 46 depressions, 388 fills, 193 cuts, maximum fill 2.3 m and cut
  5.0 m.

Two repeated width-2 runs had identical raster values and diagnostics JSON.
The invalid width 3 is rejected explicitly. A bilinearly enlarged 894-by-860
raster (four times as many cells) completed in 3.33 seconds with 59,408 KiB
peak RSS.

The release build produced the current executable as the hashed Cargo artifact
`target/release/deps/whitebox_tools-b410b1e427d40d0b`; the pre-existing
top-level `target/release/whitebox_tools` remained stale. All release evidence
uses the freshly built hashed executable. This artifact-selection behavior is
a build-system friction to resolve before packaging, not a tool correctness
failure.

## Downstream flow evidence

`D8Pointer` rasters derived from materialized TOPAZ RELIEF and Rust output have
identical values at every cell. Their GeoTIFF hashes differ only because of
metadata. `Watershed` run from the production outlet GeoJSON likewise produces
identical value arrays: two watershed cells and 192,208 NoData cells. Thus the
conditioning translation does not introduce a downstream flow-vector or
watershed difference relative to TOPAZ RELIEF.

## Additional production fixtures

Two larger DEMs were copied read-only from wepp1 and verified before and after
transfer. Both use the same controls as the original oracle: no aggregation,
no smoothing, width-two obstruction adjustment, and preprocessing only.
The source preflight at `2026-07-29T17:07:00-07:00` verified host `wepp1`,
working directory `/home/roger`, and these paths:

- `/geodata/wc1/runs/bu/burned-out-harmonic/dem/dem.tif`
- `/geodata/weppcloud_runs/portland_BRnearMultnoma_HighSevS.202009.chn_cs200/dem/dem.tif`

| Fixture | Valid cells | Input SHA-256 | TOPAZ time / RSS | Rust release time / RSS | Exact mismatches |
| --- | ---: | --- | ---: | ---: | ---: |
| burned-out-harmonic | 1,459,872 | `a2535e3564f8ebc488d3af18f05f0eaf80b25d305eac22bfd59b56c8cbe9757f` | 3.76 s / 20,236 KiB | 2.51 s / 101,480 KiB | 0 |
| Portland high severity | 1,564,677 | `d0e4c4f1cd32f03ba4cda6d092935a138102214c2199c4a61da5f58266626106` | 3.68 s / 21,620 KiB | 2.77 s / 107,744 KiB | 0 |

The first unbounded Rust traversal differed at 137 and 2,082 cells,
respectively. Porting TOPAZ's initial 11-by-11 search window, directional
five-cell expansion, per-window visitation, and resolved-state handling
removed every mismatch while retaining zero mismatches on the original DEM.
Checksums and stage statistics are recorded in
`test_fixtures/topaz_condition_dem/additional_parity.json`.

## Canonical parity-hardening gate

`test_fixtures/topaz_condition_dem/parity_manifest.json` now makes FILDEP and
RELIEF parity reproducible from a fresh release binary. It hashes valid stage
values in row-major order as little-endian signed 32-bit TOPAZ internal units,
independent of TIFF metadata and Fortran record framing.

All seven cases pass:

- widths 0, 1, and 2 on the 430-by-447 production DEM;
- the two larger all-valid production DEMs;
- a 41-by-47 synthetic irregular-NoData case; and
- burned-out-harmonic with 25,541 NLCD class-11 cells masked as NoData.

The synthetic case exposed a real defect: invalid neighbors were excluded from
Rust lower-boundary tests, so RELIEF could loop forever on a valid island
surrounded by NoData, while FILDEP could fill cells that TOPAZ drains to its
indeterminate sentinel. Rust now models NoData as an open lower boundary while
excluding it from region/flat membership and propagation. Both stages match
TOPAZ exactly on all 1,796 synthetic valid cells.

The production project configures the `nlcd/2019` WMesque alias. An exact
EPSG:32610 request already matched the 1,233-by-1,184 burned-out-harmonic grid,
so no additional warp was used. The response raster SHA-256 is
`7f6c66164ce84267eb86774b23bfbf6e7db5b352876fbbefb36859a371546820`;
its metadata currently identifies Annual NLCD Collection 1 version 1.1,
year 2024. The derived masked DEM and all 1,434,331 valid FILDEP/RELIEF values
match TOPAZ exactly.

Two complete harness runs produced byte-identical JSON reports with SHA-256
`5ed28d52cae14acc656e2083e598f020a0ab8503871d8e719e6588a9275f7a9e`.
A wrong expected RELIEF hash exits 1 and names the failed case and field.
Detailed provenance and commands are in
`docs/work-packages/20260729_topaz_condition_dem_parity_hardening/artifacts/validation.md`.

## Validation status

- Rust compilation: pass
- Full Rust tests: pass (132 passed)
- Tool discovery: pass
- Exact outlet behavior: pass
- Runtime budget under 30 seconds: pass
- Strict production parity: pass (zero mismatches)
- Two larger production fixtures: pass (zero mismatches over 3,024,549 cells)
- Determinism, mode coverage, invalid input, NoData, edge behavior: pass
- Release benchmark series and larger-raster scaling: pass
- Derived D8/watershed value comparison: pass
- Both Python wrappers compile and expose equivalent signatures: pass
- Seven-case canonical FILDEP/RELIEF harness: pass
- Repeated canonical report determinism and wrong-hash control: pass
