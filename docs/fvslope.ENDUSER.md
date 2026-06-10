# FVSlope

Use this tool to compute slope in the D8 flow direction (flow-vector slope) rather than the steepest-descent slope used by generic terrain tools.

## What This Is For

`FVSlope` measures how steeply a cell's terrain drops toward its D8 downstream neighbor. This matches the TOPAZ convention for flow-vector slope and is the value expected by WEPP channel hydraulics. A generic `Slope` tool computes the steepest descent across all eight neighbors, which systematically overestimates slope for channel cells that flow diagonally and underestimates it for cells where flow and gradient are aligned. `FVSlope` avoids this bias by restricting the slope calculation to the single downstream neighbor indicated by the D8 pointer.

The chosen unit is recorded in the output raster's metadata so downstream tools and inspection scripts can read it without requiring the caller to track it separately.

## When to Use It

Use `FVSlope` when preparing slope inputs for WEPP channel or hillslope hydraulics, or any workflow that expects TOPAZ-style flow-vector slopes. Do not use a generic slope tool as a substitute; the results will differ, particularly for diagonal flow cells.

## Before You Begin

Required inputs:

- `--dem` (or `-i`) — DEM raster; must be in the same CRS and resolution as the D8 pointer
- `--d8_pntr` — Whitebox D8 flow-direction raster; use `--esri_pntr` for ESRI encoding
- `--output` (or `-o`) — output raster path

Optional inputs:

- `--units` — output slope unit; default is `degrees`
- `--zfactor` — vertical unit multiplier when horizontal and vertical units differ (e.g., feet DEM over a metre-projected grid)
- `--esri_pntr` — treat D8 pointer values as ESRI encoding

## Key Terms and Settings

| Setting | What it means | Units or values | Notes |
|---------|---------------|-----------------|-------|
| `--units` | Output slope representation | `degrees`, `radians`, `percent`, `ratio` | `ratio` is rise-over-run; `percent` is ratio × 100. Default: `degrees` |
| `--zfactor` | Vertical/horizontal unit conversion factor | numeric, default 1.0 | Set to 0.3048 when DEM is in feet and horizontal CRS is in metres |
| `--esri_pntr` | Use ESRI D8 pointer encoding | boolean | Required if pointer raster was produced with ArcGIS or similar |

Unit conversion relationships (per cell):

- `ratio` = vertical drop / horizontal distance to downstream neighbor
- `degrees` = atan(ratio) × (180 / π)
- `radians` = atan(ratio)
- `percent` = ratio × 100

## Steps

```bash
# Degrees output (default)
whitebox_tools -r=FVSlope \
  --dem=dem.tif \
  --d8_pntr=d8.tif \
  --output=fvslope_deg.tif

# Ratio output (rise/run, as expected by some WEPP inputs)
whitebox_tools -r=FVSlope \
  --dem=dem.tif \
  --d8_pntr=d8.tif \
  --output=fvslope_ratio.tif \
  --units=ratio

# Feet DEM, metre-projected horizontal CRS
whitebox_tools -r=FVSlope \
  --dem=dem_feet.tif \
  --d8_pntr=d8.tif \
  --output=fvslope.tif \
  --units=ratio \
  --zfactor=0.3048
```

## Interpreting Results

Output values are slope magnitudes in the chosen unit for each cell. Nodata cells in the DEM or pointer raster produce nodata in the output. Flat cells (upstream and downstream elevations equal) produce slope = 0. The chosen unit string is written into the raster metadata and can be read by inspection tools without needing to recall which unit was used.

## Assumptions and Limits

- The DEM and D8 pointer must share the same geometry (rows, columns, resolution, origin).
- `--zfactor` applies a simple scalar to the elevation difference before computing the ratio; it does not reproject or resample.
- The tool computes slope only toward the single D8 downstream neighbor, so the output is direction-specific, not a symmetric terrain attribute. Cells at the watershed outlet or raster boundary may have no valid downstream neighbor and will receive nodata.

## Troubleshooting

- **Unexpected nodata cells** — check that the DEM and D8 pointer cover the same extent and that sink-filling was applied before D8 computation.
- **Slope values seem wrong** — confirm the unit setting matches what the downstream tool expects. WEPP channel input files expect `ratio` (rise/run), not degrees.
- **All zeros near flat areas** — expected; flat-area flow paths are assigned by the D8 sink-fill algorithm and may produce zero slope where terrain is truly flat.
