# Task: StreamJunctionIdentifier integration test suite

## Goal

Author `whitebox-tools-app/src/tools/stream_network_analysis/stream_junctions_integration_tests.rs` and wire it into `stream_junctions.rs`. Run `cargo test` to confirm all new tests pass before reporting done.

## Context

`StreamJunctionIdentifier` (`stream_network_analysis/stream_junctions.rs`) counts the number of inflowing channel pixels for each stream cell. For every cell in the streams raster with a positive value, it inspects all 8 neighbours: a neighbour contributes to the count if it is also a stream cell AND its D8 pointer value indicates it drains toward the current cell. Non-stream cells receive the background value (hardcoded −32768, regardless of input nodata). Output is an integer-valued F64 raster. The `--esri_pntr` flag switches the expected inflowing direction encodings from Whitebox to ESRI convention.

The tool's output (`chnjnt.tif`) is used as an input to `HillslopesTopaz`. Every fixture that contains `chnjnt.tif` holds a pre-computed reference for this tool.

## Patterns to follow

Read before writing:

- `hydro_analysis/hillslopes_topaz_integration_tests.rs` — canonical integration test structure (fixture paths via `CARGO_MANIFEST_DIR`, `temp_output_path`, `cleanup_output_path`, `Raster::new` for validation)
- `hydro_analysis/find_outlet_integration_tests.rs` — simpler example of the same pattern

Wire the new module into `stream_junctions.rs` the same way `hillslopes_topaz.rs` wires in its tests (bottom of file, `#[cfg(test)]` + `#[path = ...]` + `mod` declaration).

## Fixtures

### `test_fixtures/minimal_1pixel_stream/`

A synthetic 18×57 watershed with a single stream cell at the outlet (row=9, col=50). Relevant files:

- `flovec.tif` — D8 pointer input
- `netw0.tif` — streams input (1 positive-valued cell)
- `chnjnt.tif` — reference output

Because there is exactly one stream cell and no stream neighbour can flow into it, the expected junction count at that cell is 0.

### `test_fixtures/minimal_2pixel_stream/`

Same watershed with two stream cells: the outlet at (row=9, col=50, elev=501.7m) and one upstream cell at (row=10, col=49, elev=505.8m). Relevant files:

- `flovec.tif` — D8 pointer input
- `netw0.tif` — streams input (2 positive-valued cells)
- `chnjnt.tif` — reference output

The upstream cell is a headwater (no stream cell flows into it, count = 0). The outlet cell has exactly one inflowing stream neighbour (the upstream cell, count = 1). The fixture README notes "0 junctions" meaning no cell reaches count ≥ 2.

### `test_fixtures/blackwood_60_5/`

A real 416×443 watershed with a full channel network. Relevant files:

- `flovec.tif` — D8 pointer input
- `netw0.tif` — streams input
- `chnjnt.tif` — reference output

This fixture contains genuine junctions (cells with count ≥ 2). Use it for the golden-master regression.

## Required tests

Write exactly four tests.

### 1. Golden-master regression (`blackwood_regression`)

Run `StreamJunctionIdentifier` on the blackwood fixture. Load the pre-computed `chnjnt.tif` reference. Assert:

- Output dimensions (rows, columns) match the reference.
- Use the stream mask from `netw0.tif` as the validity mask.
- For every stream cell (`netw0 > 0`), output value equals the reference value (exact integer comparison is appropriate; counts are whole numbers).
- Every non-stream cell in output should be fixed background `−32768`.
- At least one cell has a junction count ≥ 2 (confirms the fixture has real junctions and the test is non-trivial).

### 2. Minimal 1-pixel structural check (`minimal_1pixel_structural`)

Run `StreamJunctionIdentifier` on the minimal_1pixel_stream fixture. Assert:

- Output exists and opens.
- Output dimensions match the input streams raster.
- Exactly one cell has a value other than −32768.
- That cell's value is 0 (the single stream cell has no inflowing stream neighbours).

### 3. Known-geometry check on 2-pixel fixture (`minimal_2pixel_known_geometry`)

Run `StreamJunctionIdentifier` on the minimal_2pixel_stream fixture. Assert:

- Exactly two cells have a value other than −32768.
- No cell has a value ≥ 2 (no junctions in a 2-cell stream).
- Exactly one cell has value 0 (headwater) and exactly one cell has value 1 (the outlet with one inflowing neighbour).
- The cell at (row=10, col=49) has value 0 and the cell at (row=9, col=50) has value 1. Read these coordinates from the fixture README rather than hardcoding — use the known outlet and upstream positions.

### 4. ESRI pointer mode produces different counts (`blackwood_esri_differs`)

Run `StreamJunctionIdentifier` twice on the blackwood fixture: once without `--esri_pntr` and once with it. Assert:

- Both runs complete without error.
- The two outputs differ on at least one valid (non-background) cell, confirming that the ESRI flag actually changes the direction-encoding lookup and is not a no-op on Whitebox-encoded inputs.

## Run requirement

After writing the tests, run:

```
cargo test -p whitebox-tools-app stream_junctions_integration
```

All four tests must pass. If any test fails, fix the test or investigate the tool until they pass. Do not report done until you have seen passing output from this command.

## Constraints

- Write outputs to temp paths using the existing helper pattern. Never write into `test_fixtures/`.
- Clean up temp files after each test.
- No new dependencies. Use only crates already in `Cargo.toml`.
- Do not modify any file in `test_fixtures/`.
