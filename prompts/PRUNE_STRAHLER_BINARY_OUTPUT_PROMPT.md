You are in the WhiteboxTools fork (WEPPcloud variant).

Goal: Extend the existing `PruneStrahlerStreamOrder` tool to optionally emit a
binary stream mask directly (so downstream pipelines can keep using `netful.tif`
as a 0/1 stream raster).

Background / problem:
- Culvert stream rasters from culvert-at-risk can be binary (all stream cells = 1).
- `PruneStrahlerStreamOrder` expects a Strahler-order raster and currently
  outputs an order raster (orders shifted down, first-order removed).
- When a binary stream raster is passed through pruning, the first pass removes
  the entire network (order-1 -> background), and even when order rasters are
  used, downstream code (e.g., `polygonize_netful`) expects stream cells == 1.
- We need a flag that converts the pruned order raster into a binary stream mask.

Inputs (new flag):
- `--binary_output` (bool, optional, default: false)
  - When true, the output is a binary stream raster:
    - `> 0` order cells => `1`
    - background => `0` if `--zero_background` is set, otherwise NoData
    - NoData cells remain NoData (regardless of `--zero_background`)

Behavior:
- Keep existing behavior when `--binary_output` is false (order raster output).
- When `--binary_output` is true:
  1) Run the same pruning logic (drop order 1, decrement others by 1).
  2) Convert the pruned output to a binary stream mask as described above.
  3) Preserve NoData handling:
     - If input cell is NoData, output NoData (regardless of `--zero_background`).
     - If `--zero_background` is true, use `0` for background/non-stream cells.

Outputs:
- Same output raster path (`--output`).
- Add metadata entries:
  - `Binary output: true/false`
  - `Zero background: true/false` (already present; keep it)

Files to update:
- Rust tool: `whitebox-tools-app/src/tools/stream_network_analysis/prune_strahler_order.rs`
  - Add the new parameter in `parameters` and parse it in `run`.
  - Implement the binary conversion when flag is set.
- Python bindings:
  - `whitebox_tools.py`
  - `WBT/whitebox_tools.py`
  - Update the `prune_strahler_stream_order` wrapper to accept
    `binary_output: bool = False` and include `--binary_output` when true.

Example usage:
- CLI:
  `whitebox_tools -r=PruneStrahlerStreamOrder --streams=strahler.tif -o=netful.tif --binary_output --zero_background`
- Python:
  ```python
  wbt.prune_strahler_stream_order(
      streams="strahler.tif",
      output="netful.tif",
      binary_output=True,
      zero_background=True,
  )
  ```

Notes:
- This tool still expects a Strahler-order raster as input; it does NOT compute
  order internally. The new flag only changes the output representation.
- Keep the output raster data type consistent with the input (do not force u8).
- Palette/photometric changes for binary output are out-of-scope.
- Keep flag parsing consistent with other boolean flags (accept `--binary_output`
  without an explicit value).

Please follow the conventions in DEVELOPING_TOOLS.md and keep changes minimal.
