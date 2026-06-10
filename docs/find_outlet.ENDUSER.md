# FindOutlet

Use this tool to automatically locate a watershed outlet pour point and write it as a GeoJSON file suitable for downstream WEPPcloud tools.

## What This Is For

`FindOutlet` takes a D8 flow-pointer raster, a stream mask, and a watershed mask and returns a single outlet location as a GeoJSON `FeatureCollection`. It works by finding the deepest interior point of the watershed (farthest from the boundary), walking downhill from there until the flow path reaches the watershed edge, and confirming the exit point is on a stream with exactly one inflow (to avoid landing mid-junction).

The output GeoJSON includes diagnostic properties that help downstream steps — and human reviewers — understand how the outlet was selected.

## When to Use It

Use `FindOutlet` when you have a watershed mask (a raster that marks a region of interest) but no pre-selected pour point. It is designed for automated pipelines where the outlet is unknown and must be inferred from terrain, and for interactive WEPPcloud sessions where a user picks an approximate area and the tool resolves the snapped outlet.

Two operating modes:

- **Watershed mode** (default): infers the outlet from the mask geometry and D8 flow directions.
- **Requested outlet mode**: the caller supplies a preferred location and the tool snaps downhill to the nearest qualifying stream cell.

## Before You Begin

Required inputs:

- `--d8_pntr` — Whitebox D8 flow-direction raster (or ESRI if `--esri_pntr` is set)
- `--streams` — binary stream mask (1 = stream, 0 or nodata = non-stream)
- `--watershed` — watershed boundary mask (1 = inside, 0 or nodata = outside)

Optional inputs:

- `--requested_outlet_lng_lat` — preferred outlet as `longitude,latitude` in WGS84 (EPSG:4326); only valid when the raster CRS is geographic degrees
- `--requested_outlet_row_col` — preferred outlet as `row,col` pixel indices; use this when the raster is in a projected CRS and you want to override from row/column

At least `--watershed` or a requested outlet location must be supplied; the tool rejects calls that provide neither.

## Key Terms and Settings

| Setting | What it means | Notes |
|---------|---------------|-------|
| `--esri_pntr` | Interpret D8 pointers as ESRI encoding | Required if your pointer raster uses ESRI conventions |
| `--requested_outlet_lng_lat` | Preferred outlet as WGS84 lon,lat | Tool snaps downhill from this location to the nearest qualifying stream cell |
| `--requested_outlet_row_col` | Preferred outlet as pixel row,col | Use instead of `--requested_outlet_lng_lat` for projected rasters |

## Steps

Watershed mode:

```bash
whitebox_tools -r=FindOutlet \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --watershed=basin.tif \
  --output=outlet.geojson
```

Requested-location mode:

```bash
whitebox_tools -r=FindOutlet \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --watershed=basin.tif \
  --requested_outlet_lng_lat=-116.45,46.73 \
  --output=outlet.geojson
```

## Interpreting Results

The output is a single-point GeoJSON `FeatureCollection`. The feature's `geometry` holds the outlet coordinates in the raster CRS. The `properties` object contains:

| Property | Description |
|----------|-------------|
| `outlet_row`, `outlet_col` | Pixel indices of the accepted outlet |
| `outlet_in_mask` | Whether the accepted outlet is inside the watershed mask |
| `outlet_junction_count` | Junction inflow count at the accepted cell (always 1) |
| `candidate_rank` | Which interior candidate succeeded (0-based) |
| `steps_taken` | Cells traced from candidate to outlet |
| `steps_beyond_mask` | Cells traced outside the mask before finding a stream |
| `start_mode` | `"watershed"` or `"requested"` |
| `epsg` | EPSG code of the raster CRS, when available |

If the outlet cannot be determined, the tool raises an error with a summary of failure reasons for each candidate attempted.

## Assumptions and Limits

- The watershed mask is treated as approximate; the boundary does not need to be a hydraulically perfect watershed.
- The junction constraint (exactly 1 inflow) prevents the outlet from landing at a stream confluence. If your stream network has no qualifying single-inflow cell reachable from any interior candidate, the tool will fail.
- In requested-outlet mode, the supplied location is a starting point, not necessarily the final outlet — the tool still walks downhill to the nearest qualifying stream cell.
- Rasters must share identical dimensions; the tool aborts with a dimension mismatch error otherwise.

## Troubleshooting

- **"No qualifying outlet found"** — the stream network may be too sparse, the watershed mask may be very small, or all boundary exits land at multi-inflow junctions. Try running `RemoveShortStreams` to simplify the network before re-running.
- **"Dimension mismatch"** — all three rasters must have identical rows, columns, and resolution.
- **"Requested outlet is outside the raster extent"** — check that the lon/lat or row/col values fall within the raster bounds.
- **"Loop detected in flow path"** — the D8 pointer raster has a cycle; check for unfilled sinks.

## Related Docs

- [FindOutlet Specification](../whitebox-tools-app/src/tools/hydro_analysis/find_outlet.spec.md)
- [HillslopesTopaz End-User Guide](hillslopes_topaz.ENDUSER.md)
