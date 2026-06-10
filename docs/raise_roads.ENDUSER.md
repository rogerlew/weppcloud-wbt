# RaiseRoads

Use this tool to condition a DEM by raising cells near road centrelines to represent embankment fill, while guaranteeing that no valid cell is ever lowered.

## What This Is For

Road embankments are a significant surface-flow barrier in erosion and hydrologic modeling. A raw DEM does not represent the fill material that props a road above the surrounding terrain, so water that should be blocked or redirected by an embankment flows straight through in the model. `RaiseRoads` corrects this by elevating DEM cells within a search corridor along each road centreline using one of three embankment strategies.

The **no-lowering guarantee** is unconditional: for every valid (non-nodata) DEM cell, the output value is always ≥ the input value. The tool will never lower a cell, regardless of strategy or parameter settings.

Roads can be supplied as GeoJSON or Shapefile. When the road CRS differs from the DEM CRS, the tool automatically reprojects the road geometry before rasterising it.

## When to Use It

Run `RaiseRoads` after DEM acquisition and before sink-filling, depression-breaching, and D8 computation. Embankment conditioning should precede flow-direction derivation so that raised cells deflect modeled flow paths correctly.

## Before You Begin

Required inputs:

- `--dem` — input DEM raster
- `--roads` — road centreline vector (GeoJSON or Shapefile); any CRS; automatically reprojected to DEM CRS
- `--output` (or `-o`) — output conditioned DEM raster

Required:

- `--strategy` — embankment model to apply (see below)

## Strategy Overview

Three strategies are available. Choose based on available road data and modeling requirements.

### `constant`

Raises cells within the search corridor by a fixed `--height` increment, tapered toward the road edge. Use when road heights are unknown and a uniform estimate is acceptable.

Key parameters: `--height` (raise amount in DEM vertical units), `--road_width`, `--search_radius`, `--taper`.

### `profile_relative`

Raises cells to the local terrain maximum within the search corridor plus a `--margin`. Adapts to terrain slope rather than adding a flat increment. Suitable for roads that follow ridgelines or contours where a constant height estimate would be either too low (flat areas) or too high (steep slopes).

Key parameters: `--margin`, `--road_width`, `--search_radius`, `--taper`.

### `cross_section`

Applies a geometric road cross-section profile with distinct zones: crown, shoulder, and backslope. Supports conservative unpaved-road behavior and per-feature parameter overrides via GeoJSON properties.

Key parameters: `--road_width`, `--crown_width`, `--shoulder_width`, `--shoulder_slope`, `--backslope_angle`, `--height`.

GeoJSON attribute overrides: individual features may carry `width`, `height`, `shoulder_width`, `shoulder_slope`, or `backslope_angle` properties that override global parameter values for that specific road segment.

## Key Terms and Settings

| Setting | What it means | Units | Notes |
|---------|---------------|-------|-------|
| `--strategy` | Embankment model | `constant`, `profile_relative`, `cross_section` | Required |
| `--road_width` | Default road width | DEM horizontal units | Fallback when `--width_field` is absent or missing |
| `--width_field` | GeoJSON/Shapefile attribute name holding per-feature width | — | Optional; overrides `--road_width` per feature |
| `--height` | Fixed raise amount (constant) or crown height (cross_section) | DEM vertical units | Used by `constant` and `cross_section` |
| `--margin` | Terrain-max padding above local maximum | DEM vertical units | Used by `profile_relative` |
| `--search_radius` | Half-width of the rasterisation corridor | DEM horizontal units | Cells within this distance of the centreline are candidates |
| `--taper` | Apply linear height taper from road edge to search radius boundary | boolean | Smooths the transition between embankment and natural terrain |
| `--crown_width` | Width of the flat crown zone | DEM horizontal units | `cross_section` only |
| `--shoulder_width` | Width of the shoulder zone | DEM horizontal units | `cross_section` only |
| `--shoulder_slope` | Slope of the shoulder | ratio (rise/run) | `cross_section` only |
| `--backslope_angle` | Angle of the fill backslope | degrees | `cross_section` only |

## Steps

Constant strategy:

```bash
whitebox_tools -r=RaiseRoads \
  --dem=dem.tif \
  --roads=roads.geojson \
  --output=dem_conditioned.tif \
  --strategy=constant \
  --height=2.0 \
  --road_width=8.0 \
  --search_radius=15.0
```

Profile-relative strategy:

```bash
whitebox_tools -r=RaiseRoads \
  --dem=dem.tif \
  --roads=roads.geojson \
  --output=dem_conditioned.tif \
  --strategy=profile_relative \
  --margin=1.0 \
  --road_width=8.0 \
  --search_radius=15.0
```

Cross-section strategy with GeoJSON attribute overrides:

```bash
whitebox_tools -r=RaiseRoads \
  --dem=dem.tif \
  --roads=roads_with_attrs.geojson \
  --output=dem_conditioned.tif \
  --strategy=cross_section \
  --height=2.5 \
  --road_width=8.0 \
  --crown_width=4.0 \
  --shoulder_width=2.0 \
  --shoulder_slope=0.02 \
  --backslope_angle=34.0
```

## Interpreting Results

The output is a DEM of the same dimensions and CRS as the input. Where the road corridor intersects valid terrain, cells are raised by the strategy formula. All other cells are unchanged. Inspect the output by subtracting the input DEM from the output; positive differences indicate raised cells.

If reprojection occurred (roads in a different CRS), verify that at least one cell was raised; if no cells changed, the reprojection may have failed silently due to an unrecognized CRS.

## Assumptions and Limits

- The no-lowering guarantee is absolute: `output[cell] ≥ input[cell]` for all valid cells.
- Road CRS inference is heuristic (bounding box and GeoJSON `crs` property). If inference produces incorrect results, reproject roads to DEM CRS manually before running.
- The tool does not model culverts, ditches, or road drainage. It only raises terrain; flow-path routing through embankments requires additional modeling.
- Very wide `--search_radius` values on dense road networks can raise large areas of terrain.

## Troubleshooting

- **No cells raised** — verify that the road geometry overlaps the DEM extent and that CRS reprojection produced valid projected coordinates. Check that `--search_radius` is large enough relative to the DEM resolution.
- **"Road CRS could not be inferred"** — reproject the road file to the DEM CRS manually.
- **Unrealistically large raises** — reduce `--height` or `--margin`, or switch to `profile_relative` which adapts to local terrain rather than adding a constant.
