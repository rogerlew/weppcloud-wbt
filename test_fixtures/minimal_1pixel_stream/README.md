# Minimal 1-Pixel Stream Test Fixture

This fixture tests the edge case where a watershed has no natural stream network
and a minimal 1-pixel synthetic stream is created at the outlet (single cell).

## Source
- Derived from culvert batch run 120 (failed with "No outlet link found")
- Original watershed: 144 cells, 18x57 pixels
- Elevation range: 501.7 - 519.9 m

## Stream Configuration
- Single outlet pixel: row=9, col=50, elev=501.7m
- No upstream stream cells (point outlet only)

## Files
- `netw0.tif` - 1-pixel synthetic stream (outlet cell only)
- `flovec.tif` - D8 flow direction pointer
- `relief.tif` - DEM/relief raster
- `bound.tif` - Watershed boundary mask
- `strahler.tif` - Strahler stream order (from original network)
- `chnjnt.tif` - Stream junction identifier (0 junctions)
- `outlet.geojson` - Pour point at lowest elevation

## Expected Behavior
`hillslopes_topaz` should handle this minimal configuration by:
1. Recognizing the single stream cell as a point outlet
2. Creating a minimal channel (single cell or degenerate)
3. Creating 1 source hillslope (entire watershed drains to outlet)
4. Generating valid `subwta.tif` and `netw.tsv` outputs

## Use Case
This represents the absolute minimum stream configuration for WEPP modeling:
- Entire watershed treated as a single hillslope draining to a point outlet
- Useful for very small drainage areas or artificial drainage points
