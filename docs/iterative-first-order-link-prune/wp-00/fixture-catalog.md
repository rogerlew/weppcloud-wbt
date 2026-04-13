# IFOLP WP-00 Fixture Catalog

This catalog is the checksum-pinned fixture baseline for WP-00 parity harness work.

## Clean-room note

- TopAZ is treated as a black-box behavior oracle only.
- Fixture rasters are consumed as external artifacts; no TopAZ source implementation is referenced.
- Reproducibility is enforced through SHA-256 pinning in `tools/ifolp_wp00_prepare_fixtures.py` manifest output.

## Preparation command

```bash
cd /workdir/weppcloud-wbt
python tools/ifolp_wp00_prepare_fixtures.py --run-root /tmp/ifolp_wp00/run1 --overwrite
```

Generated manifest:
- `/tmp/ifolp_wp00/run1/manifests/fixture-manifest.json`

## Fixture set

| Fixture ID | Source root | Required anchor | CSA (ha) | MSCL (m) | D8 pointer | Upstream area | Oracle stream | Raster shape | Resolution (m) |
|---|---|---|---:|---:|---|---|---|---|---:|
| `clueless_aftertaste_anchor_10_100` | `/wc1/runs/cl/clueless-aftertaste/dem/wbt` | yes | 10.0 | 100.0 | `flovec.tif` | `floaccum.tif` | `netw0.tif` | `161 x 158` | 30 |
| `blackwood_60_5` | `/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5` | no | 60.0 | 5.0 | `flovec.tif` | `floaccum.tif` | `netw0.tif` | `443 x 416` | 30 |
| `gatecreek_10m_30_2` | `/workdir/weppcloud-wbt/test_fixtures/gatecreek_10m_30_2` | no | 30.0 | 2.0 | `flovec.tif` | `floaccum.tif` | `netw0.tif` | `2037 x 2009` | 10 |

## Threshold provenance

- `clueless_aftertaste_anchor_10_100`:
  - `/wc1/runs/cl/clueless-aftertaste/watershed.log`
  - `watershed.build_channels(csa=10.0, mcl=100.0)`
- `blackwood_60_5`:
  - inferred from fixture naming convention `<name>_<csa>_<mscl>`
- `gatecreek_10m_30_2`:
  - inferred from fixture naming convention `<name>_<resolution>_<csa>_<mscl>`

## Checksum pins (SHA-256)

| Fixture ID | D8 pointer | Upstream area | Oracle stream |
|---|---|---|---|
| `clueless_aftertaste_anchor_10_100` | `0fd7ff801da759182dcc07a094c9f2c9716231dfc1e79d18d494d7dadd4bbdcf` | `b07743b5319e8de48354fcc80ebca455f6f4cea41377a5f64a689c032eed6042` | `104f259c0f4727d38eb731a76bfb144644fec7df72c2876d4a35b09bb2628a66` |
| `blackwood_60_5` | `7adf853adb922cdf50b74b6fab2cbfd154bd8ab3d34d5cda3ee46c7458c28f49` | `e832ea372a9878d69db5f1dcaaebfaa102bcad2aeb1e70f80cf06ccde202112c` | `624a53c94032450908145b50b62f90efe238b5fc2cff274278a4ead2417a55ca` |
| `gatecreek_10m_30_2` | `2b0d3a9f37a37722fd73cf050592ad907a00f2140e71e9dd3c62afe62c5a0ae8` | `970634687f9fb65843d4be2507727f43ebc1b62f4bf561017c515929b32993be` | `66aa84010d6c89752d2274d22f85f0ce950f3672b21533c6cdf7f35384d5a9d8` |
