# Articles Bibliography

Developer reference notes for papers in `docs/articles/`.

---

## lindsay2015.pdf

Lindsay JB, Dhun K. 2015. Modelling surface drainage patterns in altered landscapes using LiDAR. *International Journal of Geographical Information Science*, 29(3): 397-411. DOI: 10.1080/13658816.2014.975715

### Summary

Introduces the **least-cost breaching algorithm** for resolving topographic depressions in LiDAR DEMs containing road/railway embankments and drainage ditches. This is the algorithm behind WBT's `BreachDepressionsLeastCost` tool.

### Key points

- LiDAR DEMs represent road surfaces but not culverts/underpasses, creating artificial dams that block flow. Traditional depression filling floods valleys upstream of embankments, losing all surface drainage information within those areas.
- The least-cost breach algorithm uses **least-cost path (LCP) analysis** to find the breach channel requiring the minimum cumulative elevation change. Cost (*C*) for each cell is the difference between the cell's elevation and the pit cell's elevation. A cost-distance accumulation surface (*A*) is built via priority queue, then the breach trench follows the steepest descent path on the *A*-surface from pit to target.
- The `d_max` parameter (maximum allowable breach distance in grid cells) controls the search radius. The paper recommends **iterative runs with increasing `d_max`** (e.g. 5, 50, 150, 750, 1500 cells) feeding each output into the next, followed by a final depression fill for unresolvable pits. This avoids expensive LCP analysis over large neighbourhoods for the majority of pits that have short-distance solutions.
- Maximum allowable decrement height rejects breach paths requiring excessive depth, preventing unrealistic trenching.

### Results (Rondeau Bay, 1m LiDAR, 28219x33380 = ~940M cells)

| Metric | Depression breaching | Depression filling | Stream burning + fill |
|---|---|---|---|
| Cells modified | baseline | 86.5% more | 81.2% more |
| Volumetric impact (m^3) | 6,224,882 | 55,923,148 | 33,766,908 |
| Underpass accuracy (major) | 87.7% | N/A | N/A |
| Underpass accuracy (overall) | 79.3% | N/A | N/A |

- Breaching modified the DEM **at least 80% less** than filling or stream burning.
- Breached 87.7% of major embankment-crossing underpasses without ancillary culvert data.
- Curved breach channels follow natural stream courses; other breaching methods produce straight-line trenches that artificially straighten channels.

### Limitations noted

- Minor culverts (ditch-connecting) breached only 61.0% of the time.
- On gentle-gradient roadside ditches, the algorithm sometimes trenched across embankments where no underpass existed (false breach). This is inherent when along-ditch gradients are too flat for the cost model to prefer following the ditch.

### Relevance to weppcloud-wbt

This paper is the theoretical basis for `BreachDepressionsLeastCost` — the recommended depression removal tool after `BurnStreamsAtRoads`. Key parameters map directly:

| Paper concept | WBT parameter | Guidance |
|---|---|---|
| `d_max` (search radius) | `--dist` | Use iterative increasing values or a single conservative value. Larger = slower but more complete. |
| Max decrement height | `--max_cost` | Prevents excessive trenching. Set based on expected maximum embankment height. |
| Final fill pass | `--fill true` | Resolves pits that exceed cost/distance thresholds. |
| `--min_dist` | `--min_dist` | When `true`, uses minimum-distance breach path instead of minimum-cost. Default `true` in WBT. |

The iterative `d_max` strategy from the paper is built into the WBT tool — a single call with appropriate `--dist` handles this internally.

---

## lindsay2016.pdf

Lindsay JB. 2016. The practice of DEM stream burning revisited. *Earth Surface Processes and Landforms*, 41(5): 658-668. DOI: 10.1002/esp.3888

### Summary

Introduces `TopologicalBreachBurn`, a stream burning method that preserves vector stream topology during rasterization. Also recommends `BurnStreamsAtRoads` as a conservative alternative for LiDAR DEMs where only road-crossing enforcement is needed.

### Key points

- Traditional stream burning (*FillBurn*: rasterize + constant elevation offset + depression fill) suffers from four problems: (1) difficulty choosing offset magnitude, (2) parallel stream artifacts from misaligned hydrography, (3) topological errors from rasterization (stream adjacency, collisions, piracy), (4) need for manual vector editing (lakes, wide streams, braided channels).
- These errors worsen as the **scale mismatch** between vector hydrography and DEM resolution increases. At coarser DEM resolutions, FillBurn's Kappa accuracy dropped from 0.953 (SRTM-1) to 0.490 (GTOPO-30); TopologicalBreachBurn maintained 0.952 to 0.921.
- `TopologicalBreachBurn` addresses this by:
  1. Computing **Total Upstream Channel Length (TUCL)** from the vector hydrography layer to establish stream hierarchy
  2. **Pruning low-TUCL links** to match network detail to DEM resolution (optimization: minimize collisions + minimize pruning)
  3. Rasterizing with **link ID values** (not boolean), resolving collisions by TUCL priority
  4. A **modified priority-flood** that assigns flow direction during traversal, giving stream cells higher priority than land cells
  5. Simultaneous **flow accumulation + breach-burn** that lowers cells by the minimum amount (0.001 elevation units) for monotonic drainage
- **Depression removal is integrated into the priority-flood** — no separate `BreachDepressions` or `FillDepressions` step is needed. The flow-direction raster is the primary output; the burned DEM is secondary.
- The priority-flood confines in-stream flow to cells of the same link ID (except at link end-nodes), preventing stream piracy between adjacent channels.

### On LiDAR DEMs and road crossings (p. 667)

Lindsay explicitly recommends a conservative approach for LiDAR DEMs:

> "an alternative, conservative method, may be to only burn a LiDAR DEM for a short distance upstream/downstream of road crossings, with the intent of removing road embankments while preserving the DEM's representation of drainage features elsewhere. This approach has been implemented as Whitebox GAT plugin tool called *Burn Streams At Roads*."

This is particularly relevant when the LiDAR DEM's own drainage representation is more accurate than the available mapped hydrography (common in headwater areas and areas of complex hydrology).

### Relevance to weppcloud-wbt

Two WBT tools implement concepts from this paper:

| Paper concept | WBT tool | Use case |
|---|---|---|
| TopologicalBreachBurn (full stream enforcement) | `TopologicalBreachBurn` | When vector hydrography is more detailed than DEM and full network enforcement is needed |
| Conservative road-crossing enforcement | `BurnStreamsAtRoads` | LiDAR DEMs where only road embankment removal is needed (recommended for Culvert_web_app) |

For the Culvert_web_app hydro-enforcement pipeline, the recommended sequence is:

1. `BurnStreamsAtRoads(dem, streams, roads, width)` — localized, elevation-aware road-crossing enforcement
2. `BreachDepressionsLeastCost(dem, dist, max_cost, fill=true)` — cost-constrained depression removal for remaining natural depressions (from Lindsay & Dhun, 2015)

This replaces the current 4-step pipeline (create_breaklines + fill roads + burn breaklines + unconstrained BreachDepressions) with two tool calls that are spatially controlled and elevation-aware. See the migration guide for detailed comparison.
