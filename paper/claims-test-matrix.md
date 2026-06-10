# Claims–Test Matrix

Maps verifiable technical claims in `paper.md` to the tests that cover them. Intended for authors during revision and for reviewers assessing reproducibility. Coverage types: **regression** (output compared against committed fixture), **property** (output verified against a derivable invariant without a stored reference), **structural** (output file existence, dimensions, schema), **error** (tool returns `Err` for invalid input).

---

## HillslopesTopaz

**Paper claim** (Software Design §4): *"The central tool, `HillslopesTopaz`, implements TOPAZ-style stream and hillslope identifiers for a single watershed. It emits the rasters and channel metadata tables used by WEPPcloud, including left, right, and top hillslope classes and link-level attributes such as upstream area."*

| Test | File | Coverage |
|------|------|----------|
| `hillslopes_topaz_integration_fixture_regression` | `whitebox-tools-app/src/tools/hydro_analysis/hillslopes_topaz_integration_tests.rs` | regression — asserts hillslope raster label topology (left/right/top IDs, parent-channel relationships), link attribute table (netw.tsv) structure and per-row channel metadata against the `blackwood_60_5` fixture |

---

## FindOutlet

**Paper claim** (Summary): *"outlet discovery (`FindOutlet`)"*

| Test | File | Coverage |
|------|------|----------|
| `find_outlet_integration_from_requested_row_col_uses_requested_start` | `whitebox-tools-app/src/tools/hydro_analysis/find_outlet_integration_tests.rs` | property — when a specific row/col is requested, the snapped outlet matches the requested starting cell |
| `find_outlet_integration_from_watershed_mask_prefers_mask_candidate` | same | property — when a watershed mask is supplied, the outlet is snapped to the mask boundary rather than the raw D8 terminus |

---

## StreamJunctionIdentifier

**Paper claim** (Software Design §4): *"stream-junction counting (`StreamJunctionIdentifier`)"*

| Test | File | Coverage |
|------|------|----------|
| `blackwood_regression` | `whitebox-tools-app/src/tools/stream_network_analysis/stream_junctions_integration_tests.rs` | regression — cell-wise junction counts match `chnjnt.tif` reference; asserts at least one cell with count ≥ 2 |
| `minimal_1pixel_structural` | same | structural — single stream cell produces exactly one non-background output cell with count 0 |
| `minimal_2pixel_known_geometry` | same | property — two-cell stream: headwater has count 0, outlet has count 1; positional assertions at known row/col |
| `blackwood_esri_differs` | same | property — ESRI pointer mode produces at least one cell differing from Whitebox mode, confirming the flag is not a no-op |

---

## PruneStrahlerStreamOrder

**Paper claim** (Software Design §4): *"Strahler-order pruning (`PruneStrahlerStreamOrder`)"*

| Test | File | Coverage |
|------|------|----------|
| `blackwood_algorithm_correctness` | `whitebox-tools-app/src/tools/stream_network_analysis/prune_strahler_order_integration_tests.rs` | property — every output cell satisfies the documented cell-wise rule (order > 1 → order − 1; order ≤ 1 → nodata) against `blackwood_60_5` |
| `minimal_all_order_one_pruned` | same | property — single-stream-cell input produces no positive-valued output cells |
| `blackwood_zero_background` | same | property — `--zero_background` flag: removed cells receive 0 instead of nodata; nodata cells unaffected |
| `blackwood_binary_output` | same | property — `--binary_output` flag: retained cells collapsed to 1.0 |

---

## IterativeFirstOrderLinkPrune

**Paper claim** (Summary and Software Design §4): *"stream-network pruning"* and *"iterative first-order-link pruning with local thresholds (`IterativeFirstOrderLinkPrune`)."*

| Test | File | Coverage |
|------|------|----------|
| `iterative_first_order_link_prune_run_integration_writes_binary_stream_output` | `whitebox-tools-app/src/tools/stream_network_analysis/iterative_first_order_link_prune_parser_tests.rs` | integration/property — runs the tool end-to-end on synthetic aligned D8/upstream-area rasters and asserts the emitted stream raster is binary with retained stream cells |
| `iterative_first_order_link_prune_run_integration_caps_receiver_inflow_for_strained_gown_fixture` | same | integration/property — runs the tool on the `strained_gown` real-data fixture with `--max_junctions=3` and asserts receiver inflow count never exceeds 3 |
| `iterative_first_order_link_prune_parser_accepts_threshold_pair_with_space_and_comments` | same | parser/property — local threshold table support accepts whitespace and comments while mapping threshold codes to `csa_ha`/`mscl_m` |
| `iterative_first_order_link_prune_prepare_phase_inputs_rejects_unmapped_threshold_code` | same | error — active stream cells with threshold codes missing from the table are rejected |
| `iterative_first_order_link_prune_phase_a_*` | `whitebox-tools-app/src/tools/stream_network_analysis/iterative_first_order_link_prune_phase_a_tests.rs` | property — phase-A stream qualification semantics, local thresholds, and ESRI/Whitebox pointer handling |
| `iterative_first_order_link_prune_phase_b_*` | `whitebox-tools-app/src/tools/stream_network_analysis/iterative_first_order_link_prune_phase_b_tests.rs` | property — phase-B first-order-link pruning semantics, deterministic selection, cycle rejection, and `max_junctions` pruning |
| `iterative_first_order_link_prune_topology_*` | `whitebox-tools-app/src/tools/stream_network_analysis/iterative_first_order_link_prune_topology_tests.rs` | property/error — topology kernel pointer decoding, inflow counts, stale-candidate rejection, tie-breaks, and threaded error propagation |
| `test_ifolp_wrapper_contract_for_both_python_surfaces` | `tests/test_ifolp_wrapper_smoke.py` | wrapper/property — both Python wrapper surfaces expose the IFOLP call contract and validate paired local-threshold arguments |

---

## RemoveShortStreams — maximum-junction pruning

**Paper claim** (Software Design §4): *"enhanced short-stream pruning with a maximum-junction constraint."*

| Test | File | Coverage |
|------|------|----------|
| `max_junctions_three_prunes_one_branch_from_four_way_junction` | `whitebox-tools-app/src/tools/stream_network_analysis/remove_short_streams_integration_tests.rs` | property — synthetic four-inflow stream network is pruned with `--max_junctions=3`; output receiver inflow count is capped at 3 while retaining stream cells |

---

## FVSlope

**Paper claim** (Software Design §4): *"FVSlope computes slope in the D8 flow direction to match TOPAZ-style flow-vector slopes used by WEPP channel hydraulics where `Slope` produces biased estimates for channels. The modified `FVSlope` tool adds ratio units and records the selected unit in output metadata."*

| Test | File | Coverage |
|------|------|----------|
| `blackwood_regression` | `whitebox-tools-app/src/tools/hydro_analysis/fvslope_integration_tests.rs` | regression — output matches `fvslop.tif` reference for the `blackwood_60_5` fixture |
| `blackwood_units_consistency` | same | property — all four unit types (ratio, degrees, percent, radians) satisfy documented mathematical relationships between them cell-wise |
| `minimal_structural` | same | structural — output opens, dimensions match input, no NaN in valid cells |

---

## RaiseRoads

**Paper claim** (Software Design §4): *"`RaiseRoads` conditions DEMs for road embankments while guaranteeing that valid DEM cells are not lowered, supporting constant, profile-relative, and cross-section strategies with attribute-based GeoJSON overrides."*

The no-lowering guarantee is stated unconditionally and is the primary verifiable claim.

| Test | File | Coverage |
|------|------|----------|
| `profile_relative_raises_without_lowering` | `whitebox-tools-app/src/tools/hydro_analysis/raise_roads_integration_tests.rs` | property — no-lowering invariant holds for every non-nodata cell; at least one cell raised (confirming WGS84→UTM reprojection succeeded) |
| `constant_and_cross_section_no_lowering` | same | property — no-lowering invariant holds for `constant` and `cross_section` strategies; at least one cell raised per strategy |
| `strategies_produce_distinct_outputs` | same | property — no two strategies produce identical output arrays, confirming `--strategy` is not a no-op |

---

## ClipRasterToRaster

**Paper claim** (Software Design §4): *"`ClipRasterToRaster` … reduce[s] unnecessary format conversions and full-raster reads in cloud workflows."*

| Test | File | Coverage |
|------|------|----------|
| `blackwood_watershed_clip` | `whitebox-tools-app/src/tools/gis_analysis/clip_raster_to_raster_integration_tests.rs` | property — every cell satisfies the documented mask rule (pass-through where mask valid and non-zero; nodata elsewhere) |
| `minimal_zero_mask_becomes_nodata` | same | property — exactly 2 cells pass through when `netw0.tif` is used as mask, exercising the `mask == 0` exclusion branch specifically |
| `mismatched_geometry_returns_error` | same | error — mismatched raster dimensions return `Err` |

---

## Watershed — GeoJSON pour-point support

**Paper claim** (Summary and Software Design §4): *"GeoJSON pour-point watershed support"* and *"GeoJSON support in `Watershed`"*

| Test | File | Coverage |
|------|------|----------|
| `geojson_delineates_watershed_matching_bound` | `whitebox-tools-app/src/tools/hydro_analysis/watershed_integration_tests.rs` | regression — GeoJSON path produces labeled cells that match `bound.tif` cell-for-cell; all labeled cells carry FID 1.0 |
| `geojson_raster_parity` | same | property — GeoJSON path and raster path produce the same set of labeled cells for the same physical outlet |
| `multipoint_geojson_produces_watershed` | same | structural — `MultiPoint` geometry branch handles a synthetic temp file without error; labeled extent matches `bound.tif` |
| `non_feature_collection_returns_error` | same | error — non-FeatureCollection GeoJSON root returns `Err` |

---

## UnnestBasins — basin hierarchy export

**Paper claim** (Summary and Software Design §4): *"basin hierarchy export in `UnnestBasins`"*

The sidecar CSV (`<stem>_hierarchy.csv`) with columns `outlet_id, parent_outlet_id, child_count, child_ids, nesting_order, hierarchy_level, is_root, row, column` is the fork-specific contribution.

| Test | File | Coverage |
|------|------|----------|
| `single_outlet_end_to_end` | `whitebox-tools-app/src/tools/hydro_analysis/unnest_basins_integration_tests.rs` | structural — one output raster (`_1.tif`) created, no `_2.tif`, hierarchy CSV created; labeled cells match `bound.tif` |
| `single_outlet_hierarchy_csv_fields` | same | property — CSV header exact match; single data row with `outlet_id=1`, `parent_outlet_id=0`, `is_root=true`, `child_count=0`, `nesting_order=1`, `hierarchy_level=0` |
| `two_nested_outlets_produce_nested_outputs` | same | property — two outlets produce `_1.tif` and `_2.tif`; root watershed covers at least as many cells as subcatchment; CSV has exactly one root row and one row with nonzero parent |

---

## Read-only single-source VRT support

**Paper claim** (Summary and Software Design §4): *"limited read-only VRT support"* and *"read-only single-source VRT support reduce[s] unnecessary format conversions and full-raster reads in cloud workflows."*

### Parser-level tests (`whitebox-raster/tests/vrt_parser.rs`)

| Test | Coverage |
|------|----------|
| `test_parse_valid_vrt` | regression — parsed fields (dimensions, SRS, GeoTransform, band, data\_type, SrcRect/DstRect offsets and sizes) match expected values for `crop_center_100x100.vrt` |
| `test_parse_relative_path_vrt` | property — `relative_to_vrt = true`, source filename starts with `..` |
| `test_parse_float32_vrt` | property — `data_type = F32` parsed correctly |
| `test_parse_fullsize_no_rect` | property — absent SrcRect/DstRect defaults to full raster extent |
| `test_invalid_band_not_one` | error — band ≠ 1 rejected |
| `test_invalid_complex_source` | error — `ComplexSource` rejected |
| `test_invalid_multiple_sources` | error — more than one `SimpleSource` rejected |
| `test_invalid_negative_offset` | error — negative rect offset rejected |
| `test_invalid_nonzero_dstrect_offset` | error — non-zero DstRect offset rejected |
| `test_invalid_size_mismatch` | error — SrcRect/DstRect size mismatch rejected |
| `test_invalid_vrt_size_mismatch` | error — VRT declared size ≠ SrcRect size rejected |
| `test_invalid_missing_dstrect` | error — SrcRect present without DstRect rejected |
| `test_invalid_missing_srcrect` | error — DstRect present without SrcRect rejected |
| `test_invalid_window_out_of_bounds` | error — SrcRect exceeding source raster bounds rejected |

### Integration-level tests via `Raster::new` (`whitebox-raster/tests/vrt_integration.rs`)

Covers all 14 valid VRT fixtures. Each test loads the VRT via `Raster::new` and compares every cell against a reference TIF.

| Test | VRT source encoding |
|------|---------------------|
| `test_vrt_raster_loads_center_crop` | Float64, stripped, no compression |
| `test_vrt_raster_loads_relative_path` | relative `SourceFilename` |
| `test_vrt_raster_loads_fullsize_no_rect` | absent SrcRect/DstRect (full-raster alias) |
| `test_vrt_raster_loads_bottomright_crop` | bottom-right crop window |
| `test_vrt_raster_loads_topleft_crop` | top-left crop window |
| `test_vrt_raster_loads_lzw_compressed_source` | LZW-compressed source |
| `test_vrt_raster_loads_deflate_compressed_source` | DEFLATE-compressed source |
| `test_vrt_raster_loads_tiled_source` | tiled layout, non-aligned window |
| `test_vrt_raster_loads_tiled_lzw_source` | tiled + LZW |
| `test_vrt_raster_loads_int16_source` | Int16 data type |
| `test_vrt_raster_loads_int16_lzw_pred2_source` | Int16, LZW, predictor-2 |
| `test_vrt_raster_loads_int16_packbits_source` | Int16, PackBits compression |
| `test_vrt_raster_loads_float32_source` | Float32 data type |
| `test_vrt_raster_loads_sparse_source` | sparse tile layout |

### Windowed GeoTIFF tests (`whitebox-raster/tests/geotiff_window.rs`)

14 tests covering `read_geotiff_window` (the primitive underlying VRT reads): stripped, LZW, DEFLATE, tiled, tiled+LZW, Int16, Float32, predictor-2, PackBits, sparse, and relative-path inputs; plus boundary-condition error cases (negative offset, zero size, out-of-bounds).

---

## Claims with no direct test coverage

The following claims from the paper describe properties or behaviours that are not verified by any committed test:

| Claim | Location | Notes |
|-------|----------|-------|
| Structured Rust error propagation to Python callers (`raise_on_error`) | Software Design §4, State of the Field | Requires Python-layer test; not covered in Rust test suite |
| `WhiteboxToolsTopazEmulator` orchestrates WEPPcloud preprocessing | Software Design §4 | Adapter coverage lives in the companion WEPPpy repository: `/workdir/wepppy/tests/topo/test_terrain_processor_wbt_integration.py` covers IFOLP/RemoveShortStreams routing and real-WBT flow-stack generation |
| Build-time environment variable removal preventing stacktrace leakage | State of the Field | Security property; not unit-testable in the conventional sense |

---

## Running the full test suite

```bash
# whitebox_raster: VRT parser + integration + geotiff window (42 tests)
cargo test -p whitebox_raster --tests

# whitebox-tools-app: all tool integration tests
cargo test -p whitebox-tools-app hillslopes_topaz_integration
cargo test -p whitebox-tools-app find_outlet_integration
cargo test -p whitebox-tools-app fvslope_integration
cargo test -p whitebox-tools-app stream_junctions_integration
cargo test -p whitebox-tools-app prune_strahler_order_integration
cargo test -p whitebox-tools-app iterative_first_order_link_prune
cargo test -p whitebox-tools-app remove_short_streams_integration
cargo test -p whitebox-tools-app raise_roads_integration
cargo test -p whitebox-tools-app clip_raster_to_raster_integration
cargo test -p whitebox-tools-app watershed_integration
cargo test -p whitebox-tools-app unnest_basins_integration
```
