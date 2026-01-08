# VRT File Support (Single SimpleSource) with Windowed GeoTIFF Read

## Background
weppcloud uses WhiteboxTools raster IO (whitebox-raster) that currently loads GeoTIFFs by fully materializing the dataset in memory. For large DEMs and many small watersheds, this is wasteful. A minimal VRT implementation enables per-watershed cropping without duplicating data on disk. The MVP is a VRTDataset with a single SimpleSource pointing to one GeoTIFF and a single band, with a SrcRect that defines the crop window.

This document specifies the supported VRT subset, the required windowed GeoTIFF read path, and a multi-phase implementation and testing plan.

## Goals
- Add VRT support for a single <VRTDataset> with one <VRTRasterBand> and one <SimpleSource>.
- Support cropped reads by implementing a windowed GeoTIFF read path in whitebox-raster.
- Keep the runtime dependency set unchanged (no GDAL linking).
- Preserve existing APIs for WhiteboxTools by integrating at Raster::new.

## Non-goals
- Full VRT support (mosaic, multiple sources, resampling, ComplexSource, derived, warped, pansharpened).
- Vector VRT support.
- Changing the internal Raster type to a lazy or tiled representation.
- Supporting band >1 or multiple bands in MVP.

## Scope and Supported VRT Subset
Supported elements and constraints:
- <VRTDataset rasterXSize="..." rasterYSize="...">
  - Must be present.
- Optional <GeoTransform> element
  - For MVP, always compute the GeoTransform from the source dataset and SrcRect offset.
  - If a VRT GeoTransform is present, it is ignored.
- Optional <SRS> element
  - For MVP, only the source GeoTIFF SRS is supported.
  - If a VRT SRS is present, it must match the source SRS or the VRT is rejected (EPSG match if present, otherwise normalized WKT match with AUTHORITY blocks removed).
- <VRTRasterBand dataType="..." band="1">
  - Only one band supported, band must be 1.
  - dataType must be either omitted or match the source type. If specified and it does not match, treat as error in MVP.
- <SimpleSource>
  - Required.
  - Must include <SourceFilename>.
  - <SourceBand> defaults to 1 if omitted; any other value is rejected.
  - <SrcRect> and <DstRect> must either both be present or both omitted.
    - If omitted, the VRT is treated as a full-size view and rasterXSize/rasterYSize must match the source dimensions.
  - <SrcRect> and <DstRect> must have equal xSize and ySize.
  - <DstRect> must have xOff=0 and yOff=0. (The VRT dataset is treated as a simple crop.)
  - Resampling attribute is not supported. If present, treat as error.

Accepted but ignored elements:
- SourceProperties, ColorInterp, Metadata, NoDataValue. (NoDataValue does not override GeoTIFF nodata in MVP.)

Rejected elements:
- ComplexSource, AveragedSource, NoDataFromMaskSource, KernelFilteredSource, ArraySource.
- Multiple SimpleSource entries.
- MaskBand or metadata derived from other sources.

## Data Mapping Rules
### Path Resolution
- If SourceFilename has relativeToVRT="1", resolve against the directory of the VRT file.
- If relativeToVRT is "0" or missing, use as-is.

### Window Mapping
- SrcRect defines the crop window in source pixel coordinates.
- DstRect must be 0,0 and the same size as SrcRect, which becomes the VRT raster size.
- VRT rasterXSize/rasterYSize must equal SrcRect xSize/ySize. If not, treat as error.
- If SrcRect/DstRect are both omitted, treat the VRT as a full-size view and require rasterXSize/rasterYSize to match the source dimensions.
- Numeric constraints:
  - xOff/yOff must be >= 0.
  - xSize/ySize must be > 0.
  - xOff + xSize and yOff + ySize must be within the source image bounds.
  - SrcRect/DstRect values must be integers (non-integer values are rejected).

### GeoTransform
- Compute the VRT GeoTransform from the source geotransform and SrcRect xOff/yOff:
  - new_gt[0] = src_gt[0] + xOff*src_gt[1] + yOff*src_gt[2]
  - new_gt[3] = src_gt[3] + xOff*src_gt[4] + yOff*src_gt[5]
  - new_gt[1] = src_gt[1], new_gt[2] = src_gt[2], new_gt[4] = src_gt[4], new_gt[5] = src_gt[5]
- Ignore any VRT GeoTransform element.

### CRS
- For MVP, use the source GeoTIFF SRS.
- If a VRT SRS element is present, it must match the source SRS or error:
  - Prefer EPSG code matching when present in the VRT SRS.
  - Otherwise compare normalized WKT strings after removing AUTHORITY blocks.

### Data Type and NoData
- Data type is inferred from the source GeoTIFF. If VRT dataType is present and does not match the source, error.
- VRT dataType string mapping (GDAL -> whitebox DataType):
  - Byte -> U8
  - Int16 -> I16
  - UInt16 -> U16
  - Int32 -> I32
  - UInt32 -> U32
  - Int64 -> I64
  - UInt64 -> U64
  - Float32 -> F32
  - Float64 -> F64
  - Complex types (CInt16, CInt32, CFloat32, CFloat64) are unsupported and must error.
- If <NoDataValue> is present, it is ignored in MVP (use GeoTIFF TAG_GDAL_NODATA).

## Windowed GeoTIFF Read Path
A new read_geotiff_window function should support reading only a pixel window and returning a Raster that matches the VRT dataset size.

Inputs:
- file_name
- window: xOff, yOff, xSize, ySize in source pixel coordinates
- output configs and output data buffer (size xSize*ySize)

High-level algorithm:
1. Read the GeoTIFF header and IFD tags as in read_geotiff, including:
   - image width/height
   - photometric interp
   - sample format
   - bits_per_sample
   - compression
   - tile/strip layout (block sizes, offsets, byte counts)
   - geokeys and nodata
2. Validate the window is within source bounds.
3. Allocate output buffer sized xSize*ySize (not full source size).
4. Determine block layout:
   - If tiled: block_width/block_height from tags 322/323.
   - If stripped: block_height from RowsPerStrip (tag 278), block_width = image width.
5. Compute block index range that intersects the window:
   - block_x_start = xOff / block_width
   - block_x_end = (xOff + xSize - 1) / block_width
   - block_y_start = yOff / block_height
   - block_y_end = (yOff + ySize - 1) / block_height
6. Iterate only intersecting blocks:
   - Read and decompress the block using existing logic (NONE, PACKBITS, LZW, DEFLATE).
   - If PREDICTOR=2 is used, apply the predictor to the full decoded block/strip before copying any window subset (do not apply only within the cropped output).
   - Handle sparse tiles (byte count = 0) by filling nodata for the intersecting area.
   - Compute the intersection of the block with the requested window.
   - Copy only the intersecting pixels into the output buffer, re-mapping to window-local coordinates.
7. Set output RasterConfigs based on the windowed dataset size and geospatial metadata.

Notes:
- Keep all current limitations (e.g., PREDICTOR=3 unsupported) and surface the same errors.
- The decompression and pixel decoding logic can be reused; the main change is the block iteration and per-block copy into a smaller output buffer.

## Implementation Plan (Multi-phase)
### Phase 0: Alignment and Samples
Status: Partial (wepppy VRT workflow exists; `vrt_test_data` minted; baseline performance notes still pending).
- Collect representative VRT files generated by gdalbuildvrt (single source, single band, cropped).
- Confirm the minimal VRT schema matches expected MVP constraints.
- Record a baseline for memory use and wall time with current full-read path.
- Author Python VRT generation routines in wepppy for single-source cropping and confirm they produce compatible VRTs.
- Use `vrt_test_data` as the canonical fixture set for implementation and test validation.

Deliverables:
- Sample VRT fixtures and expected outputs.
- Baseline performance notes.
- wepppy VRT generation routines and example outputs.
- A validated `vrt_test_data` dataset aligned with the MVP constraints.

### Phase 1: VRT Parsing and Validation
Status: Complete (parser + validation + tests in place).
- Add a new VRT parser module in whitebox-raster (e.g., whitebox-raster/src/vrt/mod.rs).
- Parse only the supported subset (VRTDataset, GeoTransform, SRS, VRTRasterBand, SimpleSource, SrcRect, DstRect).
- Validate constraints and provide clear error messages for unsupported features.
- Add .vrt extension mapping in get_raster_type_from_file.
- Add a lightweight XML parsing dependency (e.g., quick-xml).

Deliverables:
- VRT parsing code.
- Unit tests for parser and validation rules.

### Phase 2: Windowed GeoTIFF Read
Status: Complete (windowed read path implemented + tests for compression/predictor/sparse tiles).
- Implement read_geotiff_window with block intersection logic.
- Support both tiled and stripped layouts with existing decompression.
- Reuse existing GeoTIFF tag parsing and data type decoding.
- Ensure nodata and sparse tile behavior is preserved.
- Ensure predictor handling is applied on decoded blocks/strips before window cropping.

Deliverables:
- read_geotiff_window implementation.
- Unit tests covering tile/strip and compressed/uncompressed paths.

### Phase 3: Raster Integration
Status: Complete (Raster::new integration + integration tests + WBT tool smoke test).
- Update Raster::new to call read_vrt when file extension is .vrt.
- read_vrt should:
  - parse VRT
  - open the source GeoTIFF
  - call read_geotiff_window with the SrcRect window
  - set configs based on the computed GeoTransform and source SRS
  - call update_min_max to match GeoTIFF read behavior
- Verify that downstream tools operate without code changes.

Deliverables:
- Raster::new integration tests comparing VRT reads to reference crops.
- End-to-end VRT -> Raster -> WBT tool tests (see `docs/vrt-support-acceptance-test-report.md`).

### Phase 4: Performance and Regression
Status: Pending.
- Add a simple performance harness (documented, not necessarily automated) to compare IO time and memory between full and windowed reads.
- Run regression tests with existing GeoTIFF inputs to ensure no change for non-VRT paths.

Deliverables:
- Performance notes and regression checklist.

## Testing Plan
### Unit Tests
- VRT parser tests:
  - valid VRT with SimpleSource, SrcRect, DstRect
  - valid VRT without SrcRect/DstRect (full-size view)
  - invalid: multiple SimpleSource, mismatched sizes, non-zero DstRect offsets, unsupported resampling, band != 1
  - invalid: only one of SrcRect/DstRect present
  - invalid: negative xOff/yOff, zero/negative xSize/ySize, window outside source bounds
  - invalid: VRT SRS present but does not match source SRS
  - valid but ignored: SourceProperties, ColorInterp, Metadata, NoDataValue present
  - relativeToVRT path resolution
- GeoTransform tests:
  - computed geotransform matches expected crop
  - VRT GeoTransform present is ignored (computed transform is used)

### GeoTIFF Window Read Tests
- Use small synthetic GeoTIFFs with known values.
- Test windows aligned to tile/strip boundaries and partial overlaps.
- Test compressed and uncompressed variants if fixtures available.
- Ensure nodata fill for sparse tiles.
- Predictor=2 with non-zero xOff to confirm correct left-context decoding.
- Last-tile/strip partial-block padding behavior.
  - Implemented coverage: LZW, DEFLATE, PACKBITS, PREDICTOR=2, sparse tiles, and non-tile-aligned windows.

### Integration Tests
- Raster::new integration tests compare VRT reads against reference crops (including relative path).
- WBT tool-level smoke tests completed (see `docs/vrt-support-acceptance-test-report.md`).
- Use VRTs generated by the wepppy routines from Phase 0 as fixtures.

### Performance Tests
- Compare memory usage and wall time for:
  - full read of a large GeoTIFF
  - VRT read of a small crop window
- Target: memory proportional to window size and no full-image read IO.

## Test Fixtures: vrt_test_data
Canonical fixture set for VRT implementation and validation.

### Location
`vrt_test_data/`

### Directory Structure
```
vrt_test_data/
├── source/           # Source GeoTIFF files (crop targets)
├── vrt/              # Valid VRT files for testing
├── reference/        # Reference cropped TIFFs for value comparison
├── invalid_vrt/      # Invalid VRT files for error handling tests
└── README.md
```

### Source Files
All source files were created from a real DEM from the culvert-at-risk integration testing:
- Original: `/wc1/culverts/8121e6c0-50ff-4777-b61f-f743058f0fe4/topo/hydro-enforced-dem.tif`
- 8910x5510 pixels, 1m resolution, EPSG:32619 (UTM zone 19N), Float64

Sources:
- `source/dem_500x500.tif`: 500x500, Float64, stripped (500x2), uncompressed
- `source/dem_500x500_lzw.tif`: same, LZW
- `source/dem_500x500_deflate.tif`: same, DEFLATE
- `source/dem_500x500_tiled.tif`: tiled 64x64, uncompressed
- `source/dem_500x500_tiled_lzw.tif`: tiled 64x64, LZW
- `source/dem_100x100_int16.tif`: 100x100, Int16
- `source/dem_100x100_float32.tif`: 100x100, Float32
- `source/dem_100x100_int16_lzw_pred2.tif`: 100x100, Int16, LZW + PREDICTOR=2
- `source/dem_100x100_int16_packbits.tif`: 100x100, Int16, PACKBITS
- `source/dem_500x500_sparse.tif`: 500x500, Float64, tiled sparse (all nodata)

### VRT Files (Valid)
Each VRT was created with `gdal_translate -of VRT -srcwin ...` and has a corresponding
reference TIF in `reference/` created with `gdal_translate -of GTiff -srcwin ...`.

| VRT File | Source | Window (xOff,yOff,xSize,ySize) | Test Purpose |
|----------|--------|-------------------------------|--------------|
| `crop_center_100x100.vrt` | dem_500x500.tif | 200,200,100,100 | Basic center crop |
| `crop_topleft_150x150.vrt` | dem_500x500.tif | 0,0,150,150 | Top-left corner |
| `crop_bottomright_200x200.vrt` | dem_500x500.tif | 300,300,200,200 | Bottom-right corner |
| `crop_lzw_200x200.vrt` | dem_500x500_lzw.tif | 100,100,200,200 | LZW source |
| `crop_deflate_300x300.vrt` | dem_500x500_deflate.tif | 50,50,300,300 | DEFLATE source |
| `crop_tiled_nonaligned.vrt` | dem_500x500_tiled.tif | 37,41,67,83 | Non-tile-aligned |
| `crop_tiled_lzw_150x150.vrt` | dem_500x500_tiled_lzw.tif | 50,50,150,150 | Tiled + LZW |
| `crop_relative_path.vrt` | ../source/dem_500x500.tif | 200,200,100,100 | relativeToVRT="1" |
| `crop_int16_50x50.vrt` | dem_100x100_int16.tif | 25,25,50,50 | Int16 type |
| `crop_float32_50x50.vrt` | dem_100x100_float32.tif | 25,25,50,50 | Float32 type |
| `crop_int16_pred2_50x50.vrt` | dem_100x100_int16_lzw_pred2.tif | 25,25,50,50 | LZW + PREDICTOR=2 |
| `crop_int16_packbits_50x50.vrt` | dem_100x100_int16_packbits.tif | 25,25,50,50 | PACKBITS compression |
| `crop_sparse_120x120.vrt` | dem_500x500_sparse.tif | 0,0,120,120 | Sparse tiles (all nodata) |
| `fullsize_no_rect.vrt` | dem_100x100_int16.tif | full size (no SrcRect/DstRect) | Full-size view without rects |

VRT structure (MVP subset):
```xml
<VRTDataset rasterXSize="W" rasterYSize="H">
  <SRS>...</SRS>
  <GeoTransform>...</GeoTransform>
  <VRTRasterBand dataType="..." band="1">
    <NoDataValue>0</NoDataValue>
    <SimpleSource>
      <SourceFilename relativeToVRT="0|1">path/to/source.tif</SourceFilename>
      <SourceBand>1</SourceBand>
      <SrcRect xOff="X" yOff="Y" xSize="W" ySize="H" />
      <DstRect xOff="0" yOff="0" xSize="W" ySize="H" />
    </SimpleSource>
  </VRTRasterBand>
</VRTDataset>
```
If SrcRect/DstRect are omitted, the VRT is treated as a full-size view and rasterXSize/rasterYSize must match the source dimensions.

### Invalid VRT Files
`vrt_test_data/invalid_vrt/` cases:
- `multiple_sources.vrt`: multiple SimpleSource elements
- `nonzero_dstrect_offset.vrt`: DstRect xOff/yOff != 0
- `size_mismatch.vrt`: SrcRect size != DstRect size
- `window_out_of_bounds.vrt`: SrcRect exceeds source bounds
- `band_not_one.vrt`: band attribute != 1
- `complex_source.vrt`: ComplexSource instead of SimpleSource
- `negative_offset.vrt`: negative xOff
- `vrt_size_mismatch.vrt`: rasterXSize/YSize != SrcRect xSize/ySize
- `missing_srcrect.vrt`: DstRect present but SrcRect missing
- `missing_dstrect.vrt`: SrcRect present but DstRect missing

### Validation Approach
For each valid VRT:
1. Load VRT via Raster::new (parser + windowed GeoTIFF read).
2. Read pixel data using the windowed GeoTIFF path.
3. Compare against the reference TIF loaded directly.
4. Verify values match (exact for integer, tolerance for float).

### Creation Commands
```
# Source DEM (uncompressed, stripped)
gdal_translate -of GTiff -srcwin 1000 1000 500 500 -co COMPRESS=NONE \
  /wc1/culverts/.../hydro-enforced-dem.tif source/dem_500x500.tif

# Compressed variants
gdal_translate -of GTiff -co COMPRESS=LZW source/dem_500x500.tif source/dem_500x500_lzw.tif
gdal_translate -of GTiff -co COMPRESS=DEFLATE source/dem_500x500.tif source/dem_500x500_deflate.tif

# Tiled variants
gdal_translate -of GTiff -co TILED=YES -co BLOCKXSIZE=64 -co BLOCKYSIZE=64 \
  source/dem_500x500.tif source/dem_500x500_tiled.tif

# LZW with predictor=2 (Int16)
gdal_translate -of GTiff -co COMPRESS=LZW -co PREDICTOR=2 \
  source/dem_100x100_int16.tif source/dem_100x100_int16_lzw_pred2.tif

# PACKBITS compression (Int16)
gdal_translate -of GTiff -co COMPRESS=PACKBITS \
  source/dem_100x100_int16.tif source/dem_100x100_int16_packbits.tif

# Sparse tiled dataset (all nodata = 0)
gdal_calc.py -A source/dem_500x500.tif --calc="A*0" --NoDataValue=0 --type Float64 \
  --format GTiff --creation-option TILED=YES --creation-option BLOCKXSIZE=64 \
  --creation-option BLOCKYSIZE=64 --creation-option SPARSE_OK=YES \
  --outfile source/dem_500x500_sparse.tif --overwrite

# VRT creation (example)
gdal_translate -of VRT -srcwin 200 200 100 100 \
  source/dem_500x500.tif vrt/crop_center_100x100.vrt

# Reference TIF creation (example)
gdal_translate -of GTiff -srcwin 200 200 100 100 \
  source/dem_500x500.tif reference/crop_center_100x100.tif
```

### Geospatial Metadata
All files share these properties (inherited from source):
- CRS: EPSG:32619 (WGS 84 / UTM zone 19N)
- Pixel Size: ~1.0m x 1.0m
- NoData: 0
- Origin: varies by crop window (source origin + offset * pixel_size)

## Open Questions
- None (full-size VRTs without SrcRect/DstRect are supported when the VRT size matches the source).

## Acceptance Criteria
- .vrt files load via Raster::new without changes to downstream tools.
- Only the requested window is read and allocated in memory.
- Supported VRTs behave identically to equivalent cropped GeoTIFFs.
- Clear errors for unsupported VRT features.
