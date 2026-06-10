# HillslopesTopaz

Use this tool to produce TOPAZ-style channel and hillslope identifiers for a single watershed, along with the channel link metadata tables consumed by WEPPcloud.

## What This Is For

`HillslopesTopaz` takes a fully preprocessed watershed — DEM, D8 flow pointers, stream mask, junction map, Strahler order, watershed boundary, and outlet pour point — and labels every raster cell with a TOPAZ-convention ID. It also writes a link attribute table (`netw.tsv`) describing each channel segment.

The TOPAZ ID convention:

| Feature | ID pattern | Example |
|---------|------------|---------|
| Channel link | ends in 4 | 24, 34, 44, … |
| Left hillslope | channel − 2 | 22, 32, 42, … |
| Right hillslope | channel − 1 | 23, 33, 43, … |
| Headwater (top) hillslope | channel − 3 | 21, 31, 41, … |

The outlet channel is always 24. Each subsequent channel increments by 10 (34, 44, …). Left and right are defined relative to the downstream flow vector.

## When to Use It

Run this tool as the final step in a WEPPcloud watershed preparation sequence, after stream extraction, junction identification, and Strahler ordering are complete. It is the step that converts a topographic analysis into WEPP-ready model structure.

## Before You Begin

All rasters must have identical rows, columns, resolution, and spatial extent. The tool aborts with an error if any geometry mismatch is detected.

Required inputs:

- `--dem` — filled or breached DEM (Float32 or Float64)
- `--d8_pntr` — Whitebox D8 flow-direction raster; use `--esri_pntr` if your pointer was produced with ESRI encoding
- `--streams` — binary stream mask (1 = stream, 0 or nodata = non-stream)
- `--pour_pts` — single outlet location as a Shapefile, GeoJSON, or raster; must be on a stream cell inside the watershed
- `--watershed` — watershed boundary mask (1 = inside, 0 or nodata = outside)
- `--chnjnt` — junction count raster from `StreamJunctionIdentifier`; values must be 0–3 (tool aborts if ≥ 4 is encountered)
- `--order` — Strahler stream order raster; values are copied into the link table

Required outputs:

- `--subwta` — output TOPAZ ID raster (Float32, written with nodata = very negative float)
- `--netw` — output channel link table (TSV)

Optional:

- `--profile` — emit extra timing and counter diagnostics to stdout

## Key Terms and Settings

| Setting | What it means | Notes |
|---------|---------------|-------|
| `--esri_pntr` | Interpret D8 pointers as ESRI encoding | Required if your pointer raster uses ESRI conventions |
| `--profile` | Print detailed phase timing to stdout | Useful for diagnosing slow runs on large DEMs |

## Steps

1. Run stream extraction (`IterativeFirstOrderLinkPrune` or equivalent) to produce `--streams`.
2. Run `StreamJunctionIdentifier` to produce `--chnjnt`.
3. Run a Strahler-order tool to produce `--order`.
4. Run `FindOutlet` to produce `--pour_pts`.
5. Run `HillslopesTopaz`:

```bash
whitebox_tools -r=HillslopesTopaz \
  --dem=dem.tif \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --pour_pts=outlet.geojson \
  --watershed=basin.tif \
  --chnjnt=chnjnt.tif \
  --order=strahler.tif \
  --subwta=subwta.tif \
  --netw=netw.tsv
```

6. Inspect `subwta.tif` to confirm hillslope and channel labeling.
7. Inspect `netw.tsv` to confirm link table completeness.

## Interpreting Results

**`subwta.tif`** — every watershed cell carries a TOPAZ integer ID stored as Float32. Cells outside the watershed carry the nodata value.

**`netw.tsv`** — one row per channel link. Key columns:

| Column | Description |
|--------|-------------|
| `id` | Sequential link index (0-based) |
| `topaz_id` | TOPAZ channel ID (ends in 4) |
| `length_m` | Flowpath length in metres |
| `ds_z`, `us_z` | DEM elevation at downstream and upstream endpoints |
| `drop_m` | Elevation drop from upstream to downstream end |
| `order` | Strahler order at upstream endpoint |
| `areaup` | Area of left and right hillslopes draining to the link (m²) |
| `inflow0_id`, `inflow1_id`, `inflow2_id` | Upstream link indices (−1 if absent) |
| `is_headwater` | True when the link has no upstream channel inflows |
| `is_outlet` | True for the single outlet link |

## Assumptions and Limits

- Junction counts in `--chnjnt` must be 0–3; a ≥ 4 value aborts the run.
- The outlet pour point must fall on a stream pixel inside the watershed mask.
- Left/right hillslope assignment is geometric, based on the cross-product of the downstream flow vector, and can be counter-intuitive in highly sinuous channels.
- The tool does not fill sinks or extract streams; those steps must be complete before running.

## Troubleshooting

- **"Raster geometry mismatch"** — at least one input raster has different dimensions, resolution, or origin from the others.
- **"Pour point is not on a stream cell"** — the outlet GeoJSON or shapefile location does not coincide with a stream pixel; re-run `FindOutlet` or manually adjust the outlet.
- **"Junction count ≥ 4"** — the junction map has a cell with too many inflows; prune the stream network with `RemoveShortStreams --max_junctions` before running.
- **"No headwater cells found"** — the stream mask may be empty or all cells are classified as junctions.

## Related Docs

- [HillslopesTopaz Specification](../whitebox-tools-app/src/tools/hydro_analysis/hillslopes_topaz.spec.md)
- [FindOutlet End-User Guide](find_outlet.ENDUSER.md)
- [StreamJunctionIdentifier End-User Guide](stream_junction_identifier.ENDUSER.md)
