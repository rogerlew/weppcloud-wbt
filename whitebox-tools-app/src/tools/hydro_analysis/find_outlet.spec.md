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
- For each candidate cell, walk the D8 flow path by translating pointer values through the Whitebox/ESRI lookup tables, keeping a `HashSet` of visited cells and enforcing an iteration ceiling (`rows * columns * 4`) to guard against loops.
- Stop the walk when the next step would exit the mask or raster extent; confirm the last in-mask cell is a stream (non-zero, non-nodata). The first candidate whose terminating cell is on a stream is selected as the outlet, capturing the number of steps taken, candidate index, and distance-to-boundary metrics.
- If no candidate succeeds, accumulate the first few failure reasons (loops, invalid pointers, non-stream boundaries, etc.) and raise an error summarizing them for easier debugging of problematic masks.

#### Output
- Emit a single-point GeoJSON `FeatureCollection` containing the outlet coordinates in map units with CRS metadata when an EPSG code is known.
- Populate the feature properties with diagnostics required by automation (outlet row/column, centroid, distance to boundary, candidate rank, step count, watershed cell totals, perimeter-stream stats, EPSG, and sampled perimeter stream cells when present).

#### Failure Handling
- Missing parameters, dimension mismatches, empty watershed masks, invalid D8 pointers, and candidates failing stream validation all surface as `ErrorKind::InvalidInput` messages with contextual details so upstream workflows can log and remediate issues quickly.
