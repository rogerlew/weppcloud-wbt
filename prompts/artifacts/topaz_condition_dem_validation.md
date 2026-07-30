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

The generated release binary discovers `TopazConditionDem`. Five warm runs on
the production DEM took 0.48-0.53 seconds (median 0.49 seconds), with peak RSS
of 27,668 KiB. The corresponding TOPAZ `libfortran5` oracle runs took
0.22-0.24 seconds (median 0.23 seconds), with peak RSS of 5,376 KiB.

Current strict final-array comparison:

    matching valid cells: 192,210
    mismatching cells:          0

Temporary observation logging in TOPAZ and Rust showed identical ordering for
all 155 two-cell obstruction events. The last five paired differences came
from two source-level details: TOPAZ's final equal-distance replacement
compares outside drop with the current best cut, and a replacement candidate
must still be connected without crossing terrain resolved by an earlier
depression. The production comparison is exact after preserving both rules.

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

## Validation status

- Rust compilation: pass
- Full Rust tests: pass (130 passed)
- Tool discovery: pass
- Exact outlet behavior: pass
- Runtime budget under 30 seconds: pass
- Strict production parity: pass (zero mismatches)
- Determinism, mode coverage, invalid input, NoData, edge behavior: pass
- Release benchmark series and larger-raster scaling: pass
- Derived D8/watershed value comparison: pass
- Both Python wrappers compile and expose equivalent signatures: pass
