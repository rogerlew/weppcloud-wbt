# RaiseRoads Cross-Run Fixture

This fixture is for low-friction RaiseRoads development and regression checks.

- DEM source: `/wc1/runs/ex/exogamous-nimbleness/dem/dem.tif`
- Roads source: `/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson`

The fixture stores:
- `dem_clip.tif`: clipped DEM around reprojected road extent (buffered in DEM units)
- `roads.geojson`: copied source GeoJSON roads
- `manifest.json`: provenance, CRS metadata, bounds, and checksums

## Build/refresh fixture

```bash
python tools/prepare_raise_roads_fixture.py --overwrite
```

## Validate fixture against RaiseRoads

```bash
python tools/validate_raise_roads_fixture.py
```

Expected validation behavior:
- tool logs reprojection from `EPSG:4326` roads to DEM EPSG
- outputs open and match DEM geometry
- no-lowering guarantee holds
- non-zero modified cells are present
