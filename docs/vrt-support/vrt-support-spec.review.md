# Review: VRT Support Specification

## Findings (prioritized)

### High
- **Predictor correctness for windowed reads:** The spec applies PREDICTOR=2 after cropping to the output window. In whitebox-raster, predictor decoding is horizontal and depends on the left neighbor in the full row. For windows with xOff > 0, applying the predictor only within the cropped buffer yields incorrect values because the left context is missing. The windowed reader must decode each intersecting block (or row) with predictor applied to the full block/row before copying the window subset. (docs/vrt-support-spec.md:103)

### Medium
- **VRT subset rejects common gdalbuildvrt output:** Typical gdalbuildvrt outputs include elements like `<SourceProperties>`, `<ColorInterp>`, `<NoDataValue>`, and `<Metadata>`. The current subset does not mention these; if treated as errors, many single-source VRTs will fail validation even though they are otherwise compatible. Consider allowing and ignoring these elements. (docs/vrt-support-spec.md:33)
- **GeoTransform/SRS override is underspecified for whitebox-raster:** The spec defines how to compute a VRT geotransform, but does not state how to update `RasterConfigs` fields used by whitebox-raster (tiepoints/pixel scale, north/south/east/west, epsg_code, WKT). If VRT GeoTransform/SRS overrides are supported, the implementation needs explicit rules for translating into configs. (docs/vrt-support-spec.md:56)
- **Validation rules do not enumerate numeric constraints:** “Validate within bounds” is vague. The parser should explicitly reject negative xOff/yOff, non-positive xSize/ySize, and windows that exceed source bounds to avoid panics or undefined behavior. (docs/vrt-support-spec.md:88)

### Low
- **dataType matching needs an explicit mapping:** VRT `dataType` values are GDAL strings (e.g., `Float32`, `UInt16`). The spec says “match the source type,” but no mapping is specified. Define a mapping to whitebox `DataType` to avoid false mismatches. (docs/vrt-support-spec.md:68)
- **Testing plan misses key windowed edge cases:** There is no test that combines predictor=2 with non-zero xOff or that exercises edge-block padding behavior (last tile/strip). These are common failure modes. (docs/vrt-support-spec.md:169)

## Questions / Assumptions
- Is adding an XML parsing dependency acceptable, or must VRT parsing be dependency-free?
- Should `<NoDataValue>` override TAG_GDAL_NODATA, or be ignored with a warning?
- If `<SRS>` is present and differs from source geokeys, should we override `epsg_code` when the SRS is `EPSG:xxxx`, or only update the WKT string?
- Are `SourceProperties` and `ColorInterp` elements expected from gdalbuildvrt in your pipeline, and if so should they be ignored rather than rejected?
- Should the VRT reader enforce single-band source data (SamplesPerPixel=1) to align with the “single-band raster path” constraint?

## Suggested Edits / Clarifications
- **Allow common gdalbuildvrt elements:** Explicitly state that `SourceProperties`, `ColorInterp`, `NoDataValue`, and `Metadata` are accepted but ignored (or list them as warnings), to improve compatibility without expanding the MVP. (docs/vrt-support-spec.md:20)
- **Predictor handling for windows:** Update the windowed-read algorithm to decode full blocks/rows and apply predictor before copying to the output window, or include the left context when xOff > 0. (docs/vrt-support-spec.md:99)
- **Define config translation:** Add a section that maps VRT GeoTransform/SRS into whitebox `RasterConfigs` fields (west/east/north/south, resolution, epsg_code, coordinate_ref_system_wkt, tiepoint/pixel scale). (docs/vrt-support-spec.md:56)
- **Tighten numeric validation:** Require xOff/yOff >= 0, xSize/ySize > 0, window bounds within source extents, and clarify handling of non-integer SrcRect/DstRect values. (docs/vrt-support-spec.md:35)
- **Expand tests:** Add cases for predictor=2 with non-zero xOff, last-tile/strip padding, and VRTs containing typical gdalbuildvrt metadata elements. (docs/vrt-support-spec.md:169)
- **Phase 3 integration detail:** Note that `get_raster_type_from_file` needs `.vrt` mapped to a new RasterType and `Raster::new` should call `update_min_max` for VRT reads to match the GeoTIFF path. (docs/vrt-support-spec.md:141)
