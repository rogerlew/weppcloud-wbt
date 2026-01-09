You are in the WhiteboxTools fork (WEPPcloud variant).

Goal: Patch `HillslopesTopaz` to handle minimal stream networks (2-pixel streams)
that occur in small watersheds where no natural stream intersects the watershed
boundary.

Background / problem:
- Culvert-at-risk workflows process watersheds that may have no mapped streams
  intersecting their boundaries (streams exist nearby but outside the polygon).
- For these cases, we synthesize a minimal 2-pixel stream at the outlet: the
  lowest elevation cell + one upstream neighbor that flows into it.
- `hillslopes_topaz` currently fails with:
  ```
  Found 566 headwaters.
  Walk down headwaters to identify links.
  Error: Custom { kind: InvalidInput, error: "Headwater cell is already part of a link" }
  ```
- The tool finds headwaters from the D8 flow pointer (all cells with no upstream
  neighbor), not from the stream mask. With 566 headwaters but only 2 stream
  cells, the link-walking logic fails.

Test fixture:
- Location: `test_fixtures/minimal_2pixel_stream/`
- Files: `netw0.tif` (2-pixel stream), `flovec.tif`, `relief.tif`, `bound.tif`,
  `strahler.tif`, `chnjnt.tif`, `outlet.geojson`
- Watershed: 144 cells, 18x57 pixels
- Stream: outlet pixel (9,50) + upstream pixel (10,49) connected by D8 flow

Expected behavior:
When given a 2-pixel stream, `hillslopes_topaz` should:
1. Identify the stream pixels from the stream mask (not from D8 headwater search)
2. Create a single channel link between the two stream cells
3. Create hillslopes draining to this channel:
   - Minimum: 1 source hillslope (entire watershed drains to channel)
   - Optional: left/right hillslopes if geometry allows
4. Output valid `subwta.tif` (subcatchment raster with TOPAZ IDs)
5. Output valid `netw.tsv` (network topology table)

Suggested fix approach:
1. **Filter headwaters to stream mask**: Instead of finding all D8 headwaters,
   only consider cells that are both:
   - In the stream mask (`streams > 0`)
   - Have no upstream neighbor within the stream mask
2. **Handle single-link case**: When only 1 stream link exists (2 connected
   cells), create the minimal valid output:
   - 1 channel segment
   - Source hillslope(s) draining to it
3. **Graceful degradation**: If the stream is too minimal to create left/right
   hillslopes, emit a source-only configuration.

Files to update:
- Rust tool: `whitebox-tools-app/src/tools/stream_network_analysis/hillslopes_topaz.rs`
  - Modify headwater detection to filter by stream mask
  - Handle the minimal stream case in link-walking
  - Ensure `subwta.tif` and `netw.tsv` are written for minimal cases

Validation:
1. Run against the test fixture:
   ```bash
   ./WBT --run=hillslopes_topaz \
     --dem=test_fixtures/minimal_2pixel_stream/relief.tif \
     --d8_pntr=test_fixtures/minimal_2pixel_stream/flovec.tif \
     --streams=test_fixtures/minimal_2pixel_stream/netw0.tif \
     --pour_pts=test_fixtures/minimal_2pixel_stream/outlet.geojson \
     --watershed=test_fixtures/minimal_2pixel_stream/bound.tif \
     --chnjnt=test_fixtures/minimal_2pixel_stream/chnjnt.tif \
     --subwta=test_fixtures/minimal_2pixel_stream/subwta.tif \
     --order=test_fixtures/minimal_2pixel_stream/strahler.tif \
     --netw=test_fixtures/minimal_2pixel_stream/netw.tsv \
     -v
   ```
2. Verify outputs:
   - `subwta.tif` exists with valid TOPAZ subcatchment IDs
   - `netw.tsv` exists with at least 1 channel entry
3. Run existing tests to ensure no regression

Context:
- This enables culvert-at-risk processing for small drainage areas that lack
  mapped stream networks but still need WEPP modeling for runoff estimation.
- Currently ~5 of 187 culverts in a test batch fail due to this limitation.
- Related phase: wepppy Phase 4g (culvert-at-risk integration)

Please follow the conventions in DEVELOPING_TOOLS.md and keep changes minimal.
