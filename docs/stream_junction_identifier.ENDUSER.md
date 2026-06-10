# StreamJunctionIdentifier

Use this tool to count the number of inflowing tributary branches at every stream cell, producing a junction map that locates confluences, headwaters, and mid-link cells.

## What This Is For

`StreamJunctionIdentifier` assigns each stream pixel an integer that counts how many upstream stream pixels flow directly into it. The result is used by WEPPcloud tools to locate:

- **Confluences** (value ≥ 2): where branches join and new channel links begin
- **Mid-link cells** (value = 1): interior channel cells with exactly one inflow
- **Headwater cells** (value = 0): stream sources with no upstream stream inflows

Background (non-stream) cells receive the nodata sentinel value −32768.

`HillslopesTopaz` requires this output as its `--chnjnt` input. Values ≥ 4 will abort that tool; if you encounter values that high, prune the stream network first.

## When to Use It

Run `StreamJunctionIdentifier` after stream extraction (e.g., `IterativeFirstOrderLinkPrune`) and before `HillslopesTopaz`. It is a required intermediate step in the full TOPAZ watershed parameterization pipeline.

## Before You Begin

Required inputs:

- `--d8_pntr` — Whitebox D8 flow-direction raster; use `--esri_pntr` for ESRI encoding
- `--streams` — binary stream mask (1 = stream, 0 or nodata = non-stream)
- `--output` (or `-o`) — output junction-count raster

## Key Terms and Settings

| Setting | What it means | Notes |
|---------|---------------|-------|
| `--esri_pntr` | Interpret D8 pointers as ESRI encoding | Required if your pointer raster uses ESRI conventions |

## Steps

```bash
whitebox_tools -r=StreamJunctionIdentifier \
  --d8_pntr=d8.tif \
  --streams=streams.tif \
  --output=chnjnt.tif
```

With ESRI pointer encoding:

```bash
whitebox_tools -r=StreamJunctionIdentifier \
  --d8_pntr=d8_esri.tif \
  --streams=streams.tif \
  --output=chnjnt.tif \
  --esri_pntr
```

## Interpreting Results

| Output value | Meaning |
|---|---|
| 0 | Headwater — stream cell with no upstream stream inflows |
| 1 | Mid-link — exactly one upstream inflow |
| ≥ 2 | Junction — two or more upstream branches converge here |
| −32768 | Background (nodata sentinel) — not a stream cell |

Confluences where exactly two branches meet produce value 2, which is the most common junction type. Values ≥ 3 indicate more complex confluences; values ≥ 4 indicate a network pathology (likely caused by diagonal-crossing streams) and will cause downstream tools to abort.

## Assumptions and Limits

- The D8 pointer and stream mask must share the same geometry (rows, columns, resolution, origin).
- The tool counts only direct upstream stream neighbors, not the full upstream network.
- Non-stream cells (value 0 or nodata in `--streams`) are never counted as inflows regardless of the D8 pointer.
- Junction values ≥ 4 indicate that two stream paths cross diagonally in a way that creates a topologically invalid confluence. Prune short branches with `RemoveShortStreams` before running.

## Troubleshooting

- **Values ≥ 4 in output** — stream network is too complex or has diagonal crossings. Run `RemoveShortStreams --max_junctions 3` to eliminate over-connected junctions.
- **All cells show nodata** — check that `--streams` contains value 1 for stream cells (not just any non-zero value); the tool may be interpreting all cells as background.
- **Unexpected 0-value cells mid-channel** — the stream mask may have gaps; verify stream extraction produced a connected network.

## Related Docs

- [HillslopesTopaz End-User Guide](hillslopes_topaz.ENDUSER.md)
- [RemoveShortStreams End-User Guide](remove_short_streams.ENDUSER.md)
- [IterativeFirstOrderLinkPrune Specification](../docs/iterative-first-order-link-prune/specification.md)
