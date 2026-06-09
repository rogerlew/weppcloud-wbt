# Task: PruneStrahlerStreamOrder integration test suite

## Goal

Author `whitebox-tools-app/src/tools/stream_network_analysis/prune_strahler_order_integration_tests.rs` and wire it into `prune_strahler_order.rs`. Run `cargo test` to confirm all new tests pass before reporting done.

## Context

`PruneStrahlerStreamOrder` (`stream_network_analysis/prune_strahler_order.rs`) applies a cell-wise transformation to a Strahler-order raster:

- Cells with value > 1 → value − 1 (orders shift down by one, first-order links are removed)
- Cells with value ≤ 1 (order-one stream cells and non-stream background cells) → background value (nodata by default, 0 if `--zero_background`)
- Cells equal to the input nodata → nodata always, regardless of flags

Two optional flags modify the output:

- `--zero_background`: removed and background cells receive 0 instead of the input's nodata value
- `--binary_output`: retained stream cells (value > 0 after the shift) are collapsed to 1; removed and background cells receive the background value

The combination `--binary_output --zero_background` produces a binary stream mask where retained streams are 1 and everything else is 0 (non-watershed nodata remains nodata). This is the production pattern shown in the tool's example usage.

Because the transformation is a simple cell-wise algebraic rule, the correct output for every cell is derivable from the input without a separate reference raster. Tests should compute expected values from the input and compare them to the actual output rather than comparing against a stored golden master.

## Patterns to follow

Read before writing:

- `hydro_analysis/hillslopes_topaz_integration_tests.rs` — canonical fixture path convention (`CARGO_MANIFEST_DIR`), `temp_output_path`, `cleanup_output_path`, `Raster::new` for validation
- `hydro_analysis/find_outlet_integration_tests.rs` — simpler example of the same pattern

Wire the new module into `prune_strahler_order.rs` the same way `hillslopes_topaz.rs` wires in its tests (bottom of file, `#[cfg(test)]` + `#[path = ...]` + `mod` declaration).

## Fixtures

### `test_fixtures/blackwood_60_5/`

A real watershed. Relevant file: `strahler.tif` — a Strahler-order raster containing multiple order levels (1 through N, where N ≥ 2), nodata for cells outside the watershed, and likely 0 or nodata for non-stream cells inside the watershed. Use this fixture for the algorithm-correctness and flag tests.

Before writing tests that iterate over every cell, read the input raster to understand the distribution of values actually present. Verify at runtime that the fixture contains at least one cell with order ≥ 2; if the fixture lacks order-2-or-higher cells the shift-down property cannot be observed and the test should fail with a clear message.

### `test_fixtures/minimal_1pixel_stream/`

A synthetic 18×57 watershed with a single stream cell. Relevant file: `strahler.tif`. With only one stream cell and no upstream tributaries, all stream cells are expected to be order 1. After pruning, no stream cells should remain. Use this fixture to test the all-order-one case.

## Required tests

Write exactly four tests.

### 1. Algorithm correctness (`blackwood_algorithm_correctness`)

Run `PruneStrahlerStreamOrder` on `blackwood_60_5/strahler.tif` with no optional flags. Load both the input and output rasters. For every cell, assert that the output satisfies the documented transformation rule:

- Input is nodata → output is nodata
- Input > 1 → output equals input − 1
- Input ≤ 1 and input is not nodata → output equals the input's nodata value (because `--zero_background` was not passed)

Assert that at least one cell satisfies the input > 1 case, confirming the test exercises the order-shift path.

### 2. All-order-one input is fully pruned (`minimal_all_order_one_pruned`)

Run `PruneStrahlerStreamOrder` on `minimal_1pixel_stream/strahler.tif` with no optional flags. Assert:

- Output exists and opens.
- No output cell has a positive value (all stream cells were order 1 and have been removed).
- Output dimensions match the input.

### 3. Zero-background flag (`blackwood_zero_background`)

Run with `--zero_background` on `blackwood_60_5/strahler.tif`. Load input and output. For every cell assert the transformation rule with `background_val = 0`:

- Input is nodata → output is nodata (nodata is unaffected by `--zero_background`)
- Input > 1 → output equals input − 1
- Input ≤ 1 and not nodata → output equals 0

Assert that at least one cell satisfies the ≤ 1 non-nodata case, confirming the zero-background path is exercised.

### 4. Binary-output flag (`blackwood_binary_output`)

Run with `--binary_output` on `blackwood_60_5/strahler.tif` (no `--zero_background`). Load input and output. For every cell assert:

- Input is nodata → output is nodata
- Input > 1 → output equals 1.0 exactly (order collapsed to presence)
- Input ≤ 1 and not nodata → output equals input nodata value

Assert that at least one cell satisfies the input > 1 case.

## Run requirement

After writing the tests, run:

```
cargo test -p whitebox-tools-app prune_strahler_order_integration
```

All four tests must pass. If any test fails, fix the test or investigate the tool until they pass. Do not report done until you have seen passing output from this command.

## Constraints

- Write outputs to temp paths using the pattern from the existing integration tests. Never write into `test_fixtures/`.
- Clean up temp files after each test.
- No new dependencies. Use only crates already in `Cargo.toml`.
- Do not modify any file in `test_fixtures/`.
- Do not add a golden-master reference file to any fixture directory. Derive expected values from the input raster in each test.
