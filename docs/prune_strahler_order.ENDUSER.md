# PruneStrahlerStreamOrder

Use this tool to drop the lowest-order branches from a Strahler stream order raster, reducing the network by one order level across the entire grid.

## What This Is For

`PruneStrahlerStreamOrder` applies a single pruning pass to a Strahler order raster:

- Cells with order > 1 have their order decremented by 1.
- Cells with order = 1 (the finest headwater branches) become nodata (removed from the network).
- Cells with order = 0 or nodata are unaffected by default.

The result is a simplified stream network with all first-order branches removed. Repeated application prunes successive orders.

Two output modes are available:

- **Order mode** (default): output carries the decremented order value; useful when order labels matter downstream.
- **Binary mode** (`--binary_output`): retained cells (originally order ≥ 2) are collapsed to value 1; useful when only stream presence/absence is needed and order values are irrelevant.

## When to Use It

Use `PruneStrahlerStreamOrder` when you want to reduce stream network complexity after Strahler ordering but before TOPAZ parameterization. It is a lighter alternative to threshold-based stream extraction and is particularly useful when you already have a high-density stream grid and need to coarsen it.

## Before You Begin

Required inputs:

- `--streams` (or `-i`) — Strahler order raster; cells contain integer order values (1, 2, 3, …)
- `--output` (or `-o`) — output pruned raster

## Key Terms and Settings

| Setting | What it means | Notes |
|---------|---------------|-------|
| `--zero_background` | Write 0 for removed cells instead of nodata | Useful when downstream tools require a numeric background rather than nodata |
| `--binary_output` | Collapse all retained cells to value 1 | Use when only stream presence matters, not order |

## Steps

Standard pruning (decrement orders, remove order-1 links, nodata background):

```bash
whitebox_tools -r=PruneStrahlerStreamOrder \
  --streams=strahler.tif \
  --output=strahler_pruned.tif
```

Pruning with zero background (some downstream tools require a numeric grid):

```bash
whitebox_tools -r=PruneStrahlerStreamOrder \
  --streams=strahler.tif \
  --output=strahler_pruned.tif \
  --zero_background
```

Binary output (stream mask with first-order branches removed):

```bash
whitebox_tools -r=PruneStrahlerStreamOrder \
  --streams=strahler.tif \
  --output=streams_pruned_binary.tif \
  --binary_output
```

## Interpreting Results

In order mode, the maximum output value equals `max(input order) − 1`. If the input had orders 1–4, the output has orders 1–3 (original order 4 → 3, order 3 → 2, order 2 → 1, order 1 → nodata/0).

In binary mode, all retained cells equal 1 regardless of original order.

An input grid with only order-1 cells produces an output where all stream cells are removed (all nodata or 0).

## Assumptions and Limits

- The tool applies exactly one pruning pass. To prune multiple order levels, run the tool iteratively.
- Background cells (nodata or 0) in the input are treated as non-stream and are not modified.
- The tool does not recompute Strahler order after pruning; decremented values may not satisfy Strahler ordering conventions (e.g., cells that were order 2 only because of an order-1 tributary will be reduced to order 1 even though they no longer have a matching-order confluent).
- If topological consistency is required after pruning, re-run Strahler ordering on the pruned stream mask.

## Troubleshooting

- **Output is all nodata** — the input grid may contain only order-1 cells; check the input raster range.
- **Unexpected 0 cells in output** — `--zero_background` was set; if nodata was intended, omit that flag.
- **Downstream tools reject the output** — some tools require a binary stream mask (values 0 and 1 only); use `--binary_output` to produce one.

## Related Docs

- [IterativeFirstOrderLinkPrune Specification](../docs/iterative-first-order-link-prune/specification.md)
- [StreamJunctionIdentifier End-User Guide](stream_junction_identifier.ENDUSER.md)
- [RemoveShortStreams End-User Guide](remove_short_streams.ENDUSER.md)
