# HillslopesTopaz — Design Specification
*A WhiteboxTools Rust plugin implementing Garbrecht & Martz TOPAZ‐style channel & hillslope IDs for a single watershed.*

---

## 1 Purpose  
Generate a raster (`subwta.tif`) whose cell values follow the TOPAZ convention:

| Feature | Example ID | Rule |
|---------|------------|------|
| Main‑stem channels | …4 | ends in 4 |
| Left side hillslopes | …2 | channel ID − 2 |
| Right side hillslopes | …3 | channel ID − 1 |
| Headwater (top) hillslopes | …1 | channel ID − 3 |

Outlet channel starts with 24. Channels enumerate by 10. (e.g. 34, 44, ...)

A link table (`netw.tsv`) describing every channel link segment is also produced.

---

## 2 Inputs

| Flag | Type | Description |
|------|------|-------------|
| `--dem` | raster (f32/f64) | Filled or breached DEM. |
| `--d8_pntr` | raster (u8) | Whitebox D8 flow‑direction grid. |
| `--streams` | raster (u8 / bool) | 1 = stream, **nodata** or **0** = non‑stream. |
| `--pour_pts` | point vector **or** raster | **Exactly one** outlet location; no clipping performed. |
| `--watershed` | raster (u8) | 1 = inside basin mask, **nodata** or **0** = outside. |
| `--chnjnt` | raster (u8) | 0 = headwater, 1 = mid‑link, 2 = junction (≥3 ⇒ error). |
| `--order` | raster (u8) | Stream order (copied to link table;  |
| `--subwta` | output raster (f32) | Resulting TOPAZ IDs (nodata initialized to a very negative float). |

All rasters **must share identical rows, columns, grid origin, cell size, and nodata**; the tool aborts if any mismatch is detected.

---

## 3 Outputs  

### 3.1 `subwta.tif`  
*Type `f32`, nodata `-1.7976931348623157e308`* — TOPAZ identifier for every cell (hillslopes and channels stored as floats; integers are not enforced).

### 3.2 `netw.tsv`  
One row per channel link (ordered by walk order).

| Column | Units | Notes |
|--------|-------|-------|
| `id` | — | Sequential link index (0‑based). |
| `topaz_id` | — | Final TOPAZ channel ID (`…4`). |
| `ds_x`, `ds_y` | grid coords | Downstream end (toward outlet). |
| `us_x`, `us_y` | grid coords | Upstream end (headwater or junction). |
| `inflow0_id` | — | Index of first upstream link or `-1` if none. |
| `inflow1_id` | — | Index of second upstream link or `-1` if none. |
| `inflow2_id` | — | Index of third upstream link or `-1` if absent. |
| `length_m` | m | Flowpath length computed from the stored link path (orthogonal vs diagonal steps honoured). |
| `ds_z`, `us_z` | m | DEM elevation at downstream and upstream endpoints. |
| `drop_m` | m | `us_z - ds_z`. |
| `order` | — | Value sampled from the `--order` raster at the upstream endpoint. |
| `areaup` | m² | Area of labelled hillslopes draining to the link (left + right only). |
| `is_headwater` | bool | `true` when the upstream end is a headwater. |
| `is_outlet` | bool | `true` for the outlet link only. |

---

## 4 Core Data Structures

```rust
/// Per‑link info; stored in Vec<Link>. use -1 for unassigned/null i32 values
struct Link {
    id:          i32,
    topaz_id:    i32,          // …4 only
    ds:          (isize, isize),
    us:          (isize, isize),
    inflow0_id:  i32,
    inflow1_id:  i32,
    inflow2_id:  i32,
    length_m:    f64,
    ds_z:        f64,
    us_z:        f64,
    drop_m:      f64,
    order:       u8,
    areaup:      f64,
    is_headwater: bool,
    is_outlet:    bool,
    path:        Vec<(isize, isize)>,
}
```

Auxiliary:

* `Raster<f32>` `subwta` (mutable; initialized to `f64::MIN`)  
* `Vec<(row,col)>` link_path cache for each `Link` (optional, freed after use)

---

## 5 Algorithm Overview

| Phase | Action | Notes |
|-------|--------|-------|
| **0 Sanity** | Confirm identical grid geometry; fail if ≥2 pour points; error if any `chnjnt≥3`. |
| **1 Pourpoint** | identify pour point coordinate (row, col) from shapefile, geojson or raster. This will be based on existing implementation in watershed.rs tool. validate pour pouint is on a channel|
| **2 Channel tree build** | *Iterative BFS/queue* starting at outlet. At each channel pixel: push upstream pixels until junction (`chnjnt==2`) or headwater (`chnjnt==0`) is encountered. Create a `Link` per segment. Assign `inflow0_id` and `inflow1_id` for non-headwater channels. These are the two upstream links flowing into the current link’s upstream end. Deterministic `id` = incremental counter. |
| **3 TOPAZ channel IDs** | Bottom‑up traversal of `Vec<Link>`: first link = 24. For every junction, decide left/right child ordering **relative** to downstream flow vector: 1. compute unit vector of parent link `a` (`us-ds`); 2. for each child `b` calculate vector as (`ds-us`) such that the junction is considered the origin for the comparison. Then we can calculate $\theta = \text{atan2}(a_x b_y - a_y b_x, a_x b_x + a_y b_y)$ for inflow0 and inflow1. after normalizing `theta` to 0-360 degrees the smaller positive angle = left ⇒ last_id + 10, larger = right ⇒ last_id + 20. Update last_id after each assignment. In the case where `us` == `ds` (e.g. the channel is 1 pixel) the `a` vector should be determined from the D8 flow direction.
| **4 Stamp channels in subwta** | Initialize an `f32` raster filled with `f64::MIN` and stamp each link’s channel pixels with its `topaz_id`. Junction cells belong to the downstream channel, while non-outlet downstream endpoints are left untouched. |
| **5 Headwater hillslopes (…1)** | For each headwater `Link`, flood upstream (D8) from `us` within watershed; label visited cells with `Link.topaz_id - 3`. |
| **6 Side hillslopes buffer cells (…2 & …3)** | Single pass **along the channel path** from outlet upward: for each channel pixel `c`, compute flow vector to its downstream pixel `d` (`c-d`) (assumes outlet is not on the edge of the map); examine the 8 neighbours `n`: if `n` is in watershed, not yet labelled in subwta, and drains into `c`, compute vector `c-n` such that `c` and atan2 of the downstream vector and the inflow path from the hillslope. left and right hillslopes are more or less perpendicular to the channel. So atan2 resuls < 180 shoudl be left ≡ ID - 2, and atan2 results >= 180 should be right ≡ ID - 1; write label and continue walking up the channel. Label **only immediate buffer cells** (no flood fill, next step). Edge cases raise exceptions. |
| **7 Residual fill hillslope cells** | For any remaining `subwta` cell == 0 and `watershed` cell == 1: walk its flow path until hitting a labelled cell; back‑fill path with that ID (reuse existing WBT implementation). |
| **8 Write outputs** | Flush `subwta` (stored as `f32`, nodata `f64::MIN`) and `netw.tsv`. Upstream link ids use `-1` when missing. Floats are formatted to three decimals. |

**note:**
`atan2` calculations are on pixel grid and tolerance is not critical

---

## 6 Implementation Notes

### Performance:
- No early optimizations required
- Queue allocation: Prefer readable implementation
- Link paths: Calculate lengths during initial BFS (avoids cache complexity)
- All loops single‑threaded; no Rayon ye

## 7 Failure Modes (Errors)

- Abort if pour point not on stream pixel.
- More or fewer than one pour-point.  
- Edge cases in flow vector calculations raise exceptions.
- Grid alignment checks only require matching dimensions (no CRS check).
- `chnjnt` value ≥ 3 anywhere.  
- Any in-basin D8 pointer cell with value `0` (or any code not mapped by the chosen pointer style) causes the tool to abort with "Invalid D8 pointer value".

*End of specification.*
