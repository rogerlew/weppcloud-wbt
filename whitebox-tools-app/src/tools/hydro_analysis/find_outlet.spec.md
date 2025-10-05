# FindOutlet Specification

Coded with CODEX

### Prompt:
```
You are in the WhiteboxTools fork (WEPPcloud variant).

Goal: Create a new tool in the `hydroanalysis` toolbox named `FindOutlet`.

Inputs:

| Flag | Type | Description |
|------|------|-------------|
| `--d8_pntr` | raster (u8) | Whitebox D8 flow‑direction grid. (required) |
| `--streams` | raster (u8 / bool) | 1 = stream, **nodata** or **0** = non‑stream. (required) |
| `--watershed` | raster (u8) | 1 = inside basin mask, **nodata** or **0** = outside. (required) |


Behaviour:
- Open and verify the d8, streams (channels), and watershed map have the same dimensions
- verify the watershed perimeter hass all 0 (non-stream values)
- Assume the watershed approximate and potentially not even a watershed. We want to identify (a) point(s) in the center mass of the watershed
- walk down from that point until we reach the boundary of the watershed
- verify this point is a stream
- create a geojson pourpoint file with this single outlet location with easting and northing here is the template i'm using in python
_outlet_template_geojson = """{{
"type": "FeatureCollection",
"name": "Outlet",
"crs": {{ "type": "name", "properties": {{ "name": "urn:ogc:def:crs:EPSG::{epsg}" }} }},
"features": [
{{ "type": "Feature", "properties": {{ "Id": 0 }}, 
   "geometry": {{ "type": "Point", "coordinates": [ {easting}, {northing} ] }} }}
]
}}"""


Outputs:
- geojson pourpoint file with this single outlet location
- embed relevant debug and algorithm data in the features properties

Extras:
- Build python wrapper
- Provide descriptive feedback on failure
- This is part of an automated process. some of the boundary might be not actually a watershed, but a basin around a reservoir or just a municipal boundary. think though how to harden against this.
- the pourpoint geojson will be consumed by `hillslopes_topaz`

Please update the necessary module registrations and ensure the code follows the
style in `DEVELOPING_TOOLS.md`.
```

### Documentation

#### Algorithm
- Load the D8 pointer, stream mask, and watershed mask rasters and ensure they share dimensions; abort with a descriptive error otherwise.
- Build a binary watershed mask (`Array2D<u8>`) from positive watershed cells, tracking the centroid (mean row/column) of the masked area and collecting perimeter cells (mask cells neighboured by outside cells or image edges). Record any streams that touch the perimeter for diagnostic output but do not fail immediately.
- Run a breadth-first search outward from the perimeter to assign each interior cell its integer distance from the boundary; sort all interior cells by descending distance so the deepest interior locations are tried first (capped at 512 candidates).
- Pre-compute a junction count raster using the stream network and D8 pointers so each stream cell records the number of inflowing channel neighbours.
- For each candidate cell, walk the D8 flow path by translating pointer values through the Whitebox/ESRI lookup tables, keeping a `HashSet` of visited cells and enforcing an iteration ceiling (`rows * columns * 4`) to guard against loops.
- Ignore stream hits that occur strictly inside the watershed mask and continue tracing until the path reaches the mask boundary (the next step would leave the mask). If that boundary cell is a stream, accept it; otherwise, keep stepping downstream outside the mask until a stream is encountered or the raster extent is reached.
- Apply a channel junction constraint: only accept a stream cell when its pre-computed junction count equals one. If the boundary stream does not satisfy this, continue stepping downstream until a qualifying junction is found or the raster edge is reached.
- The first candidate whose terminating cell is on a stream (junction count = 1)—preferably on the watershed boundary, or otherwise downstream of it—is selected as the outlet, capturing the number of steps taken, downstream steps, candidate index, and distance-to-boundary metrics.
- If no candidate succeeds, accumulate the first few failure reasons (loops, invalid pointers, non-stream boundaries, etc.) and raise an error summarizing them for easier debugging of problematic masks.

#### Output
- Emit a single-point GeoJSON `FeatureCollection` containing the outlet coordinates in map units with CRS metadata when an EPSG code is known.
- Populate the feature properties with diagnostics required by automation (outlet row/column, centroid, distance to boundary, candidate rank, step count, steps beyond the mask, mask value at the outlet, `outlet_in_mask`, `outlet_junction_count`, watershed cell totals, perimeter-stream stats, EPSG, and sampled perimeter stream cells when present).

#### Failure Handling
- Missing parameters, dimension mismatches, empty watershed masks, invalid or unsupported D8 pointers, downstream searches that loop or exceed the step ceiling, and candidates failing stream or junction validation all surface as `ErrorKind::InvalidInput` messages with contextual details so upstream workflows can log and remediate issues quickly.

### Implementation Plan
- Extend the existing tool rather than creating a sibling utility so the GeoJSON output, diagnostics, and CLI surface stay in one place for automation.
- CLI updates
  - Keep `--watershed` but mark it optional; emit an error unless either a watershed mask or a requested outlet location is supplied.
  - Add `--requested_outlet_lng_lat` accepting a comma-delimited `lon,lat` pair in WGS84; also support an explicit `--requested_outlet_row_col` pair for pixel-based overrides and testing.
  - Update help text, usage string, and parameter metadata; preserve backward compatibility for existing callers.
  - Amend the Python wrapper to surface the optional watershed argument and the new requested-outlet flags so UI callers can switch modes without bespoke logic.
- Requested outlet preprocessing
  - Parse the new argument, validate numeric inputs, and record the requested lon/lat in the output properties.
  - Convert lon/lat to raster indices when the grid is stored in geographic degrees (EPSG 4326); otherwise require a `--requested_outlet_row_col` override and return a helpful error when only lon/lat is supplied.
  - Project the derived start cell into raster space; if the exact cell is `nodata` or falls outside the grid, locate the nearest in-bounds cell with a valid pointer value.
- Flow-path tracing refactor
  - Extract the existing downstream walk into a helper that accepts a starting cell and returns the first qualifying stream cell or a tagged failure reason while preserving loop protection and junction checks.
  - Reuse the helper for current watershed-candidate mode; in requested-outlet mode, call it once from the derived start cell and step downstream until a non-junction stream cell is encountered or the raster edge is reached.
  - When both a watershed mask and requested location are provided, continue populating watershed diagnostics but prefer the user-supplied starting point for the walk.
- Output and diagnostics
  - Add properties for the requested coordinates, start row/column, number of cells between the request and the accepted outlet, and a mode indicator (`"start_mode": "watershed" | "requested"`).
  - If the trace exits the raster without finding a qualifying channel, return a descriptive error that includes how far the trace progressed.
- Documentation and validation
  - Expand the spec and tool docs with the new parameters and examples, and update the Python wrapper/tests once the interface stabilizes.
