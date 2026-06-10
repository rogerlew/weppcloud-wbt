# RemoveShortStreams

Use this tool to prune short tributary branches from a stream network, with optional junction-complexity limiting that prevents any confluence from retaining more branches than a specified maximum.

## What This Is For

`RemoveShortStreams` applies two complementary pruning mechanisms:

1. **Length-based pruning** (`--min_length`): removes tributary branches shorter than the specified threshold.
2. **Junction-complexity pruning** (`--max_junctions`): at any confluence that retains more than `max_junctions` inflows after length pruning, iteratively deletes the shortest remaining branch until the confluence has at most `max_junctions` inflows. This continues until no over-junction confluences remain.

The two mechanisms work together. Length pruning runs first, reducing the network; junction pruning then sweeps iteratively to eliminate any remaining confluences that are still too complex.

The primary use in WEPPcloud is limiting junction counts to ≤ 3 before `HillslopesTopaz`, which aborts on any junction with ≥ 4 inflows.

## When to Use It

Run `RemoveShortStreams` after initial stream extraction and before `StreamJunctionIdentifier` and `HillslopesTopaz`. Use it when:

- The extracted stream network has short first-order branches that create spurious junctions.
- `StreamJunctionIdentifier` produced cells with values ≥ 4, which will abort `HillslopesTopaz`.
- You want to reduce stream complexity before TOPAZ parameterization without changing the overall drainage pattern.

## Before You Begin

Required inputs:

- `--d8_pntr` — Whitebox D8 flow-direction raster; use `--esri_pntr` for ESRI encoding
- `--streams` — binary stream mask (1 = stream, 0 or nodata = non-stream)
- `--output` (or `-o`) — output pruned stream mask

## Key Terms and Settings

| Setting | What it means | Default | Notes |
|---------|---------------|---------|-------|
| `--min_length` | Minimum tributary length to retain | — | In DEM horizontal units; branches shorter than this value are removed |
| `--max_junctions` | Maximum inflows allowed at any junction after length pruning | 3 | Iterative branch deletion removes the shortest branch at over-junction nodes until all nodes meet this limit |
| `--esri_pntr` | Interpret D8 pointers as ESRI encoding | false | Required if your pointer raster uses ESRI conventions |

`--min_length` is optional but recommended. Without it, only junction-complexity pruning applies. With both flags set, length pruning runs first and junction pruning cleans up any remaining over-connected nodes.

## Steps

Length pruning only (remove branches shorter than 500 m):

```bash
whitebox_tools -r=RemoveShortStreams \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --output=streams_pruned.tif \
  --min_length=500.0
```

Junction pruning only (limit all junctions to ≤ 3 inflows, no length threshold):

```bash
whitebox_tools -r=RemoveShortStreams \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --output=streams_pruned.tif \
  --max_junctions=3
```

Combined (remove short branches and limit junction complexity):

```bash
whitebox_tools -r=RemoveShortStreams \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --output=streams_pruned.tif \
  --min_length=200.0 \
  --max_junctions=3
```

## Interpreting Results

The output is a binary stream mask (values 0 or nodata for non-stream, 1 for retained stream cells). Cells removed by pruning become 0 or nodata in the output.

After pruning, run `StreamJunctionIdentifier` to verify no cell has value ≥ 4.

## Assumptions and Limits

- Branch length is measured in DEM horizontal units (typically metres). Ensure `--min_length` uses the correct unit for your projection.
- The iterative junction pruning always deletes the shortest branch at an over-connected node. If multiple branches have the same length, the selection is deterministic but depends on traversal order.
- `--max_junctions` defaults to 3. Setting it to 2 produces a fully binary-tree network (no node with more than two inflows), which may aggressively prune large confluences.
- The tool does not reconnect pruned branches if network topology requires it; pruning is strictly subtractive.
- The main channel (longest flow path to the outlet) is never removed, even if it would qualify as short by `--min_length`.

## Troubleshooting

- **Junction values still ≥ 4 after pruning** — reduce `--min_length` to remove more branches before junction pruning takes effect, or explicitly set `--max_junctions=3`.
- **Too many branches removed** — increase `--min_length` to retain longer branches, or increase `--max_junctions` to allow more inflows per node.
- **Output stream network is empty** — `--min_length` may be larger than all branches in the network. Check DEM units and verify the threshold is in the correct units.
- **HillslopesTopaz still aborts with "Junction count ≥ 4"** — re-run `StreamJunctionIdentifier` on the pruned output to confirm no cell exceeds 3; if values ≥ 4 still appear, the pruning may have missed a diagonal-crossing artifact.

## Related Docs

- [StreamJunctionIdentifier End-User Guide](stream_junction_identifier.ENDUSER.md)
- [HillslopesTopaz End-User Guide](hillslopes_topaz.ENDUSER.md)
- [IterativeFirstOrderLinkPrune Specification](../docs/iterative-first-order-link-prune/specification.md)
