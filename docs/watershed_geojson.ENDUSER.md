# Watershed (GeoJSON pour-point support)

Use this tool to delineate one or more subcatchment masks from a set of pour points, using D8 flow directions.

## What This Is For

`Watershed` traces D8 flow paths uphill from each pour point and marks all contributing cells. It produces a labeled raster where each cell carries the sequential index (1, 2, 3, …) of the pour point whose watershed it belongs to. Cells that do not drain to any pour point carry nodata.

This tool is an extension of the classic WhiteboxTools `Watershed` tool. The weppcloud-wbt fork adds **GeoJSON pour-point support**: in addition to Shapefiles and raster pour points, the tool now accepts GeoJSON `FeatureCollection` files containing `Point` or `MultiPoint` features. Each feature becomes one pour point.

## When to Use It

Use `Watershed` when you have pour points (outlet locations) and want a raster mask of the contributing area above each one. It is most commonly run after `FindOutlet` produces a pour-point GeoJSON, or after manually selecting outlet locations.

For nested watersheds with hierarchical parent/child outlet relationships, use `UnnestBasins` instead — it adds the hierarchy CSV sidecar and handles nested assignment correctly.

## Before You Begin

Required inputs:

- `--d8_pntr` — Whitebox D8 flow-direction raster; use `--esri_pntr` for ESRI encoding
- `--pour_pts` — pour-point layer; one of:
  - Shapefile (`.shp`)
  - GeoJSON or JSON file containing a `FeatureCollection` of `Point` or `MultiPoint` features
  - Raster (any non-nodata, non-zero cell is a pour point)
- `--output` (or `-o`) — output raster path

GeoJSON format requirement: the root object must be a `FeatureCollection`. A bare `Feature` or `Point` object is rejected with an error.

## Key Terms and Settings

| Setting | What it means | Notes |
|---------|---------------|-------|
| `--esri_pntr` | Interpret D8 pointers as ESRI encoding | Required if your pointer raster uses ESRI conventions |

Pour-point ordering: features are assigned sequential FIDs starting at 1 in the order they appear in the input file (Shapefile record order, GeoJSON features array order, or raster scan order). The output raster carries these FIDs so you can trace which watershed belongs to which pour point.

## Steps

GeoJSON pour points (weppcloud-wbt extension):

```bash
whitebox_tools -r=Watershed \
  --d8_pntr=d8.tif \
  --pour_pts=outlets.geojson \
  --output=watershed.tif
```

Shapefile pour points:

```bash
whitebox_tools -r=Watershed \
  --d8_pntr=d8.tif \
  --pour_pts=outlets.shp \
  --output=watershed.tif
```

Single outlet from `FindOutlet`:

```bash
whitebox_tools -r=FindOutlet \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --watershed=basin_mask.tif \
  --output=outlet.geojson

whitebox_tools -r=Watershed \
  --d8_pntr=d8.tif \
  --pour_pts=outlet.geojson \
  --output=watershed.tif
```

## Interpreting Results

The output raster contains integer FID values:

| Cell value | Meaning |
|---|---|
| 1 | Drains to pour point 1 |
| 2 | Drains to pour point 2 |
| … | … |
| nodata | Does not drain to any pour point |

For a single-outlet run, every contributing cell has value 1 and nodata covers the rest of the raster extent.

## Assumptions and Limits

- Pour points must lie within the raster extent; points outside produce no contributing-area assignment.
- When pour points overlap or one point's watershed contains another, both FIDs may be valid contributors but only one is assigned (the downstream one takes precedence in overlapping regions).
- The GeoJSON input must be a `FeatureCollection`; a non-FeatureCollection root object returns an error. Validate your GeoJSON before running.
- `MultiPoint` features are expanded: each coordinate in the feature becomes one pour point, all assigned the same FID.
- The output geometry matches the D8 pointer geometry exactly; no resampling occurs.

## Troubleshooting

- **"Expected FeatureCollection"** — the GeoJSON file has a bare `Feature` or geometry object as its root. Wrap it in a `FeatureCollection`.
- **Output is all nodata** — the pour points may not snap to valid flow-direction cells. Verify that pour points fall within the raster extent and on cells with valid (non-nodata) D8 pointer values.
- **Wrong watershed extent** — pour point snapping lands on a cell that drains a different area than expected. Use `FindOutlet` to produce a snapped outlet rather than supplying approximate coordinates directly.

## Related Docs

- [FindOutlet End-User Guide](find_outlet.ENDUSER.md)
- [UnnestBasins End-User Guide](unnest_basins.ENDUSER.md)
- [HillslopesTopaz End-User Guide](hillslopes_topaz.ENDUSER.md)
