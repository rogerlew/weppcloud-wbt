# IFOLP WP-00 TopAZ Oracle Manifest

## Oracle policy (clean-room)

- Oracle inputs are pre-generated stream rasters from external TopAZ-compatible workflows.
- WP-00 consumes only raster outputs (`netw0.tif`) and validates checksums.
- No TopAZ source logic is copied, referenced, or translated.

## Oracle staging command

```bash
cd /workdir/weppcloud-wbt
./tools/ifolp_wp00_run_topaz_oracle.sh \
  --manifest /tmp/ifolp_wp00/run1/manifests/fixture-manifest.json \
  --oracle-root /tmp/ifolp_wp00/run1/oracle \
  --overwrite
```

Generated capture manifest:
- `/tmp/ifolp_wp00/run1/oracle/oracle-capture-manifest.json`

Capture mode:
- `snapshot_copy` (copy and checksum-verify pinned oracle rasters)

## Apples-to-apples parity interpretation contract

- Oracle stream rasters are channel-only (`1` on channel cells; `NoData` elsewhere).
- Candidate IFOLP stream rasters are full-extent binary (`0/1`) in the valid raster domain.
- Direct full-extent visual comparison can therefore conflate basin behavior with background-stage encoding.
- Required WP-00/WP-05 parity mode is basin-masked comparison (`bound.tif > 0`, staged as `inputs/basin_mask.tif`).
- `tools/ifolp_wp00_compare_outputs.py` defaults to `--comparison-domain basin_mask` and should only use `full_extent` for diagnostics.

## Oracle artifact checksums (SHA-256)

| Fixture ID | Source oracle path | Oracle SHA-256 |
|---|---|---|
| `clueless_aftertaste_anchor_10_100` | `/wc1/runs/cl/clueless-aftertaste/dem/wbt/netw0.tif` | `104f259c0f4727d38eb731a76bfb144644fec7df72c2876d4a35b09bb2628a66` |
| `blackwood_60_5` | `/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5/netw0.tif` | `624a53c94032450908145b50b62f90efe238b5fc2cff274278a4ead2417a55ca` |
| `gatecreek_10m_30_2` | `/workdir/weppcloud-wbt/test_fixtures/gatecreek_10m_30_2/netw0.tif` | `66aa84010d6c89752d2274d22f85f0ce950f3672b21533c6cdf7f35384d5a9d8` |

## Verification contract

`tools/ifolp_wp00_run_topaz_oracle.sh` must fail if any condition is violated:

1. A fixture oracle source file is missing.
2. A staged oracle file checksum differs from pinned `source_oracle_stream_sha256`.
3. Existing oracle output directory is present without explicit `--overwrite`.

This enforces checksum-pinned, rerunnable oracle capture for parity testing.
