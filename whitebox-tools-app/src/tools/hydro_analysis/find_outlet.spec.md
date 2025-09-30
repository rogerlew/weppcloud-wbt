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