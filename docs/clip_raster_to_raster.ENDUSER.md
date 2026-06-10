# ClipRasterToRaster

Use this tool to mask a raster to the valid, non-zero extent of another raster, writing nodata outside that extent.

## What This Is For

`ClipRasterToRaster` performs a cell-wise mask operation:

- Where the mask raster is **valid (non-nodata) and non-zero**: the output carries the input value unchanged.
- Where the mask raster is **nodata or zero**: the output carries nodata.

This is useful for cutting a large raster (e.g., a DEM tile, flow-direction grid, or soil attribute layer) down to a smaller watershed extent defined by a binary mask — without resampling, reprojecting, or changing raster geometry.

Both the input and mask must have identical geometry (same rows, columns, resolution, and origin). The tool rejects mismatched geometry with an error.

## When to Use It

Use `ClipRasterToRaster` when you need to restrict a raster to the extent of a watershed mask before passing it to tools that should see only cells inside the watershed. It is lighter than GDAL clip operations because it does not resample or reproject — it simply zeroes out cells outside the mask.

Common uses in WEPPcloud workflows:

- Applying a watershed mask to a DEM before TOPAZ parameterization
- Restricting a flow-direction grid to a subcatchment boundary
- Trimming a soil or landuse attribute raster to a delineated watershed

## Before You Begin

Required inputs:

- `--input` (or `-i`) — input raster to clip
- `--mask` (or `-m`) — binary mask raster; cells with value non-zero and non-nodata define the "keep" region
- `--output` (or `-o`) — output raster path

The input and mask rasters must share identical geometry. The output raster has the same dimensions, CRS, and resolution as the input.

## Key Terms and Settings

No optional flags beyond the three required parameters.

| Parameter | What it means |
|-----------|---------------|
| `--input` | Raster to clip; data type is preserved |
| `--mask` | Binary raster defining the clip region; any non-zero, non-nodata value is treated as "keep" |
| `--output` | Result raster; nodata outside the mask, input values inside |

## Steps

```bash
whitebox_tools -r=ClipRasterToRaster \
  --input=dem.tif \
  --mask=watershed.tif \
  --output=dem_clipped.tif
```

Clip a flow-direction grid to a watershed mask:

```bash
whitebox_tools -r=ClipRasterToRaster \
  --input=d8.tif \
  --mask=watershed.tif \
  --output=d8_clipped.tif
```

## Interpreting Results

The output raster has:

- The same data type as the input.
- Input values wherever the mask is non-zero and non-nodata.
- Nodata everywhere else.
- Identical geometry (extent, resolution, CRS) to the input.

The output does NOT resize to the mask extent. If the mask is smaller than the input, the output retains the same dimensions with large areas of nodata.

## Assumptions and Limits

- The input and mask must have identical rows, columns, cell size, and spatial extent. The tool aborts with a geometry mismatch error if they differ.
- Mask value 0 is treated as "exclude" regardless of whether 0 is the nodata value; ensure your mask uses a non-zero integer for included cells.
- The tool does not crop to the bounding box of the mask; it retains the full input extent. Use GDAL `gdalwarp` or `gdal_translate` after clipping if you need a physically smaller output file.

## Troubleshooting

- **"Geometry mismatch"** — the input and mask rasters do not have the same dimensions or cell size. Resample one to match the other before running.
- **Output is all nodata** — the mask may use 0 for the interior and nodata for the exterior, which is the reverse of the expected convention. Ensure the mask uses value 1 (or any positive value) for cells to keep and nodata or 0 for cells to exclude.
- **Large output file** — the output retains the full input grid extent; nodata cells still occupy space in uncompressed GeoTIFFs. Add `--compress=lzw` via GDAL if file size is a concern.

## Related Docs

- [HillslopesTopaz End-User Guide](hillslopes_topaz.ENDUSER.md)
- [FindOutlet End-User Guide](find_outlet.ENDUSER.md)
