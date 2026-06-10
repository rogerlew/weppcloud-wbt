# UnnestBasins

Use this tool to delineate overlapping nested subcatchments from multiple pour points and produce a hierarchy table describing parent/child outlet relationships.

## What This Is For

`UnnestBasins` takes multiple pour points with nested (hierarchically contained) contributing areas and produces one raster per outlet plus a CSV sidecar that records the parent/child structure.

The critical distinction from `Watershed`: when outlet A is downstream of outlet B, the watershed of A contains the watershed of B. `UnnestBasins` correctly assigns each cell to its innermost containing watershed rather than naively giving all cells to the lowest pour point. The output rasters are therefore:

- `_1.tif` — subcatchment of the first outlet (innermost / highest nesting order)
- `_2.tif` — full watershed through the second outlet (includes `_1.tif` area)
- …

Processing is driven by the Shapefile record order: the tool processes outlets in that order. The first outlet processed receives the higher nesting order (root), and inner outlets receive lower nesting orders.

The `_hierarchy.csv` sidecar records all outlet relationships so downstream tools can reconstruct the parent/child tree without re-inspecting the rasters.

## When to Use It

Use `UnnestBasins` when:

- You have multiple pour points where some are upstream of others (nested subcatchments).
- You need both the inner subcatchment mask and the full-watershed mask separately.
- Downstream tools (e.g., WEPPcloud multi-outlet routing) require the hierarchy CSV.

For a single outlet or pour points that are not nested, `Watershed` is simpler.

## Before You Begin

Required inputs:

- `--d8_pntr` — Whitebox D8 flow-direction raster; use `--esri_pntr` for ESRI encoding
- `--pour_pts` — outlet locations as a **Shapefile only** (`.shp`); GeoJSON is not accepted by this tool
- `--output` (or `-o`) — output stem path (without extension); rasters are written as `<stem>_1.tif`, `<stem>_2.tif`, …; hierarchy CSV is written as `<stem>_hierarchy.csv`

## Key Terms and Settings

| Setting | What it means | Notes |
|---------|---------------|-------|
| `--esri_pntr` | Interpret D8 pointers as ESRI encoding | Required if your pointer raster uses ESRI conventions |

Pour-point ordering: nesting order is determined by Shapefile record order, not by geography. Place the most downstream (root) outlet first in the Shapefile to get it assigned the highest nesting order.

## Steps

```bash
whitebox_tools -r=UnnestBasins \
  --d8_pntr=d8.tif \
  --pour_pts=outlets.shp \
  --output=basins/watershed
```

This produces:

- `basins/watershed_1.tif` — subcatchment mask for outlet 1 (Shapefile record 0)
- `basins/watershed_2.tif` — full watershed mask through outlet 2 (Shapefile record 1)
- `basins/watershed_hierarchy.csv` — parent/child table

## The Hierarchy CSV

The `_hierarchy.csv` file contains one row per outlet.

| Column | Description |
|--------|-------------|
| `outlet_id` | Sequential outlet index (1-based) |
| `parent_outlet_id` | Outlet that contains this one; −1 if this is the root |
| `child_count` | Number of immediate child outlets |
| `child_ids` | Comma-separated list of child `outlet_id` values |
| `nesting_order` | Depth from the root; root has the highest value, innermost has 1 |
| `hierarchy_level` | Complementary depth from innermost; innermost = 0, root = max |
| `is_root` | `true` for the outermost (most downstream) outlet |
| `row`, `column` | Pixel grid position of the outlet cell |

Example for a two-outlet run where outlet 1 (Shapefile record 0) is downstream and outlet 2 (record 1) is upstream:

```
outlet_id,parent_outlet_id,child_count,child_ids,nesting_order,hierarchy_level,is_root,row,column
1,-1,1,2,2,1,true,102,45
2,1,0,,1,0,false,88,51
```

## Interpreting Results

Each `_N.tif` raster is a binary mask (value 1 inside the subcatchment, nodata outside). Raster `_N.tif` corresponds to `outlet_id = N` in the hierarchy CSV. The full watershed (root outlet) mask is the one where `is_root = true`.

## Assumptions and Limits

- **Shapefile input only**: `--pour_pts` must be a Shapefile; GeoJSON is not supported. To use GeoJSON pour points, convert to Shapefile first.
- Nesting semantics depend on Shapefile record order, not geographic position. Wrong record order produces incorrect nesting assignments.
- All pour points should be on stream cells with valid D8 pointer values. Points off-stream may produce empty or incorrect subcatchment rasters.
- Outlets that are not hydraulically nested (i.e., one does not drain through the other) produce overlapping flat assignments; the tool does not validate that the network is hierarchical.

## Troubleshooting

- **"Pour points must be supplied as a Shapefile"** — convert your GeoJSON to a `.shp` file using GDAL (`ogr2ogr`) before running.
- **Subcatchment raster is empty** — the outlet cell does not fall on a stream or has a nodata D8 pointer value. Verify outlet coordinates align with stream cells.
- **Hierarchy CSV has unexpected parent/child assignments** — reorder the Shapefile records so the most downstream outlet is record 0. Record order is the only factor that determines nesting_order.
- **Missing `_hierarchy.csv`** — the output stem path must point to a directory that exists; the tool will not create the directory.

## Related Docs

- [Watershed End-User Guide](watershed_geojson.ENDUSER.md)
- [FindOutlet End-User Guide](find_outlet.ENDUSER.md)
- [HillslopesTopaz End-User Guide](hillslopes_topaz.ENDUSER.md)
