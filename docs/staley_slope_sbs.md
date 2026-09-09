# StaleySlopeSbs local backend contract

Status: implemented and validated, 2026-09-09 UTC. WEPPpy ADR-0058 records
scientific decisions. No production installation or caller is introduced.

## Interface

Registered tool `StaleySlopeSbs`, toolbox Hydrological Analysis. Required CLI:

    whitebox_tools -r=StaleySlopeSbs --dem=raw.tif --sbs=sbs.tif --mask=bound.tif --elevation_units=m --sbs_classes=0,1,2,3 --output_dir=fresh-result

Both Python bindings expose
`staley_slope_sbs(dem, sbs, mask, output_dir, elevation_units, sbs_classes, callback=None)`.
The comma-separated four distinct integer codes mean unburned, low, moderate,
high in that order. Normalized WEPPpy `sbs_4class.tif` uses 0,1,2,3 and 255
NoData; custom categorical sources require explicit mapping. Palette colors
are presentation only. Prepare a numeric grayscale copy without a color table
for this interface: the legacy owned decoder expands palette indices to RGB,
so paletted inputs are explicitly rejected instead of misreading class codes.
A signed Int16 grayscale GeoTIFF copy is a supported preparation target (GDAL
writes explicit SampleFormat for signed data). Unsigned TIFFs that omit
SampleFormat are rejected because the current owned reader needs that tag. A dNBR raster is not an accepted SBS substitute.

## Numerical and input contract

Raw meter elevations on a north-up square WGS84 UTM pixel-area grid (EPSG
32601–32660 or 32701–32760). All three grids must have identical dimensions,
CRS, scale and tiepoint. No warp, interpolation, conditioning or delineation.
Use full-DEM Horn 3×3 before watershed masking; missing center or any neighbor,
including off-grid neighbors, makes slope unavailable. Evaluate f64 gradient
magnitude >= tan(23 degrees), with no rounding or tolerance in classification.
Slope output covers the full DEM. Finite valid positive mask cells define the
whole watershed, including channels; zero/negative/NoData cells are outside.
Reject empty watersheds, undeclared nonfinite values and unrecognized SBS
classes anywhere in the supplied grid. An explicit parseable GDAL_NODATA tag (finite value only) is required in each
input; absent/malformed/nonfinite declarations are rejected to avoid the legacy reader
implicitly treating valid -32768 as missing. Float32 inputs also require a
NoData value representable as finite Float32; the owned reader otherwise
normalizes nonfinite sentinels/pixels to -32768 and loses observation identity. NoData is tested before class mapping;
a class code colliding with SBS NoData is invalid. Elevation units must be
explicitly `m`, and any embedded contradictory vertical units are rejected.

Initially support single-image, single-band, uncompressed classic GeoTIFF
scalar rasters with explicit SampleFormat (8/16/32-bit integers or 32/64-bit floating point); no BigTIFF,
RGB, palettes, tiles or multiband. Inputs are local immutable trusted files, not uploads.
Preflight bounded TIFF metadata before owned raster decoding: at most 10 million
cells, 512 MiB per file, 256 tags, 16 MiB per metadata tag and 64 MiB aggregate metadata and 256 GeoKeys. These are resource
bounds, not scientific resolution/coverage gates. Reject unsupported layouts
explicitly. No new external runtime dependencies.

## Products and support

A fresh output directory contains `slope.tif` (F64 degrees, NoData -32768),
`intersection.tif` (U8: 0 false, 1 true, 2 unknown, 255 outside basin),
`support.tif` (U8: bit 0 slope valid, bit 1 SBS valid, 255 outside basin), and
`summary.json` (schema_version 1, status complete). Three-state AND: either
known false operand proves false; both true proves true; otherwise unknown.
SBS-only class counts are independent of slope support.

Summary keys: tool, tool_version, schema_version, status, parameters, sources,
grid, counts, areas_m2, T, T_lower, T_upper. Counts include basin, slope_valid,
sbs_valid, jointly_valid, steep, sbs_unburned, sbs_low, sbs_moderate, sbs_high,
intersection_true, intersection_false, intersection_unknown. Areas mirror counts
multiplied by cell area. With N basin, Y true, U unknown: bounds Y/N and (Y+U)/N;
T is Y/N only for U=0 and JSON null otherwise. Sources include canonical paths,
byte lengths and explicitly labeled FNV-1a-64 content fingerprints (diagnostic,
not cryptographic authentication). Reproduction evidence additionally pins
SHA-256 inputs and executable. Parameters record algorithm, edges, threshold,
units, class mapping and raw-source intent. Grid records EPSG, rows, columns,
resolution and upper-left origin. Tool version is Cargo package version;
reproduction evidence identifies the exact rebuilt executable separately.

## Failure and publication

Require an existing output parent and a nonexistent final directory; atomic
create_dir reserves the destination with Unix mode 0700 subject to umask.
Existing files/directories/symlinks are rejected. Validate and compute before
creating it. Write each product inside this exclusively owned directory, then
write summary.json last as the completion marker. Consumers must require a
parseable summary with status complete and all three products. Failed or
interrupted writes leave a visibly incomplete reserved directory and return an
error; retry with a fresh directory. Never overwrite prior results or sources.
Local parent directories and inputs must not be concurrently modified by other
principals; this is not a hostile shared-directory publication API. No durability
claim across power loss, public upload parsing, or NoDb publication is made.

## Compatibility and verification

Additive tool and bindings; generic Slope/FVSlope and generated project artifacts
remain unchanged. Analytical tests, current CLI/binding products, filesystem
failures and the existing six main-watershed terrain fixtures verify this scope.
Controlled synthetic SBS must be labeled; sensitivity is not predictive accuracy.
NoData behavior intentionally differs from Esri missing-neighbor reweighting.

## Reproduce validation

From the WBT repository, with existing offline numpy/rasterio/matplotlib and
GDAL comparison tooling available:

    cargo check -p whitebox-tools-app
    cargo test -p whitebox-tools-app
    cargo build --release -p whitebox-tools-app
    python -m py_compile whitebox_tools.py WBT/whitebox_tools.py
    python tools/validate_staley_slope_sbs.py --output /tmp/new-staley-analytical
    python tools/validate_staley_slope_sbs_security.py --binary target/release/whitebox_tools --output /tmp/new-staley-security
    python tools/staley_slope_sbs_study.py --output /tmp/new-staley-panel

Acceptance: 157 Rust tests, 60 analytical CLI/binding cases, 24 direct security
cases, 48 terrain method rows and 432 fixed-input probability scenarios.
Small original synthetic GeoTIFFs and hashes live in
`test_fixtures/staley_slope_sbs/`; larger terrain is reused unchanged from
`test_fixtures/staley_m3_resolution/`. No real burn inference is made from
coordinate-anchored synthetic SBS. The study records source/executable SHA-256,
exact flags, timing and RSS. Both wrappers run the rebuilt binary on analytical
and Topanga real-terrain inputs. Wrapper constructors/setters change process
cwd in the existing API; reproduction scripts restore cwd around each use.

The detailed WEPPpy report and independent reviews are in
`docs/work-packages/20260908_staley_slope_sbs/artifacts/` in the companion repo.
Backend availability does not mean production installation or integration.
