# D8UpstreamRelief

Compute H(o) = maximum upstream raw elevation minus elevation at cell o,
using an existing WBT D8 pointer. Each cell includes itself. Upstream area A
is contributing cell count times dx*dy, in m2. H is in meters; H/sqrt(A) is
dimensionless. The user adopted this engineering contract for the Staley M3
terrain study; Staley coefficients and probabilities are outside this tool.

```bash
target/release/whitebox_tools -r=D8UpstreamRelief \
  --dem=raw.tif --d8_pntr=flovec.tif --elevation_units=m \
  --output=vertical_relief.tif --area=upstream_area.tif --coverage=coverage.tif
```

Both Python bindings expose
`d8_upstream_relief(dem, d8_pntr, output, area, elevation_units, coverage=None, callback=None)`.
Set the wrapper binary directory explicitly when testing a rebuilt executable.
The command does not derive routing, condition elevations, or convert units.
Do not pass WEPPcloud's conditioned `relief.tif` as though it were vertical relief.

Inputs must be aligned single-band north-up pixel-area GeoTIFFs in WGS84 UTM
(EPSG 32601-32660 or 32701-32760). Other CRSs, transformation-matrix grids and
contradictory explicit units are rejected. Inputs must have a single top-left
GeoTIFF tiepoint (raster coordinates 0,0) and positive original pixel scales. `--elevation_units=m` declares
raw elevations in meters even when their vertical-unit tag is absent.
DEM/pointer NoData masks must match exactly. Valid nonfinite values, invalid
pointer codes and cycles are errors. WBT pointers are NE=1, E=2, SE=4, S=8,
SW=16, W=32, NW=64, N=128; 0 is terminal. Outgoing pointers into NoData or
outside the raster terminate within the available domain.

H/A outputs are Float64 with NoData -32768. Optional UInt8 coverage has 1
where any upstream cell touches a raster edge or NoData in its eight-neighbor
window, 0 otherwise, and NoData 255. The flag propagates downstream. It warns
of potential truncation; H/A at flagged cells are within-domain quantities,
not proof of complete contributing coverage. Metadata always includes the
flagged-cell count, formula, units and input paths. Zero coverage is not a
validation of the input DEM or routing.

Output paths must be distinct new `.tif`/`.tiff` files in existing directories.
Existing paths, symlinks and aliases are rejected. On an I/O failure, discard
any partial output set and retry with new names; publication of the output set is not atomic.
Use distinct outputs per process. Existing raster I/O and wrapper boundaries
apply; this CLI does not provide an untrusted-upload service.

The independent O(N) Kahn traversal counts incoming edges, seeds sources,
and combines maxima, integer cell counts and coverage flags once per edge.
Working memory is O(N). On a 130,120,100 m chain at 10 m spacing the output
is H=0,10,30 m and A=100,200,300 m2. Raw 130,90,100 m with identical routing
has H=0,40,30 m, rather than catchment maximum-minus-minimum at the outlet.

The pinned pfdf 3.0.2/pysheds 0.4 reference fails the monotonic analytical
case; numerical differences are documented in the WEPPpy Staley terrain
package. No GPL implementation/tests were copied or translated. Matching a
reference defect is not the contract, and calibration preprocessing equivalence
is not established. Production M3 wiring and resolution policy are separate.

Validation:

```bash
cargo test -p whitebox-tools-app
cargo build --locked -p whitebox-tools-app --release
WBT_TERRAIN_BINARY="$PWD/target/release/whitebox_tools" python -m unittest discover -s tests -p test_d8_upstream_relief.py
```

Each individual output is written in a private same-parent staging directory
and published using an atomic no-replace hard link. The filesystem must support
hard links. Parent directories and input files must remain caller-controlled
and stable; this is not an adversarial shared-directory or hostile TIFF sandbox.
Cleanup failures are reported. If a later output fails, earlier published
files remain and the command fails explicitly; discard the set before retrying.
The direct GeoTIFF writer propagates final buffered I/O errors. Standard TIFF
ImageDescription records the metadata listed above.

Supporting GeoTIFF fixes preserve source sample count and original pixel-scale
metadata for validation, correctly encode single-row strip offsets/counts,
serialize tool provenance, and flush buffered writes explicitly. Existing
legacy assumed-spacing behavior remains for other callers, while this tool
rejects missing/nonpositive source spacing. Single-row compressed/uncompressed
roundtrips and invalid source-tag cases are regression-tested.

The 24-pair study recommends genuine 10 m for initial M3 support. Controlled
effects reached 10.598 probability percentage points and 12.479% inverse
threshold change; see the WEPPpy Staley package decision report. This tool
remains resolution-agnostic; production M3 enforcement is separate.
