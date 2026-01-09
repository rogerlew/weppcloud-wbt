# Minimal 2-Pixel Stream Test Fixture

This fixture tests the edge case where a watershed has no natural stream network
and a minimal 2-pixel synthetic stream is created at the outlet.

## Source
- Derived from culvert batch run 120 (failed with "No outlet link found")
- Original watershed: 144 cells, 18x57 pixels
- Elevation range: 501.7 - 519.9 m

## Stream Configuration
- Outlet pixel: row=9, col=50, elev=501.7m
- Upstream pixel: row=10, col=49, elev=505.8m
- Flow direction: 128 (upstream flows to outlet)

## Files
- `netw0.tif` - 2-pixel synthetic stream (outlet + 1 upstream cell)
- `flovec.tif` - D8 flow direction pointer
- `relief.tif` - DEM/relief raster
- `bound.tif` - Watershed boundary mask
- `strahler.tif` - Strahler stream order (from original network)
- `chnjnt.tif` - Stream junction identifier (0 junctions for 2-pixel stream)
- `outlet.geojson` - Pour point at lowest elevation

## Expected Behavior
`hillslopes_topaz` should handle this minimal configuration by:
1. Recognizing the 2-pixel stream as a single link
2. Creating 1 channel segment
3. Creating hillslopes draining to the channel (source + optionally left/right)
4. Generating valid `subwta.tif` and `netw.tsv` outputs

## Current Error
```
Found 566 headwaters.
Walk down headwaters to identify links.
Error: Custom { kind: InvalidInput, error: "Headwater cell is already part of a link" }
```

The tool finds headwaters from the D8 pointer (not the stream mask) and fails
when processing the minimal 2-pixel stream configuration.
