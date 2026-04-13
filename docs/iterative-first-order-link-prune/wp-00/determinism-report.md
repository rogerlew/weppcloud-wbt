# IFOLP WP-00 Determinism Report

## Scope

WP-00 validation verifies:

1. Harness command set runs end-to-end from clean directories.
2. Two consecutive reruns produce identical comparison outputs.
3. Required WP-00 gates pass.

## End-to-end harness runs

Run 1:

```bash
cd /workdir/weppcloud-wbt
python tools/ifolp_wp00_prepare_fixtures.py --run-root /tmp/ifolp_wp00/run1 --overwrite
./tools/ifolp_wp00_run_topaz_oracle.sh --manifest /tmp/ifolp_wp00/run1/manifests/fixture-manifest.json --oracle-root /tmp/ifolp_wp00/run1/oracle --overwrite
rm -rf /tmp/ifolp_wp00/run1/candidate
mkdir -p /tmp/ifolp_wp00/run1/candidate
for d in /tmp/ifolp_wp00/run1/oracle/*; do
  if [ -d "$d" ]; then
    id="$(basename "$d")"
    mkdir -p "/tmp/ifolp_wp00/run1/candidate/$id"
    cp "$d/stream.tif" "/tmp/ifolp_wp00/run1/candidate/$id/stream.tif"
  fi
done
python tools/ifolp_wp00_compare_outputs.py \
  --manifest /tmp/ifolp_wp00/run1/manifests/fixture-manifest.json \
  --oracle-root /tmp/ifolp_wp00/run1/oracle \
  --candidate-root /tmp/ifolp_wp00/run1/candidate \
  --output-json /tmp/ifolp_wp00/run1/reports/parity-report.json \
  --canonical-json /tmp/ifolp_wp00/run1/reports/parity-report.canonical.json \
  --fail-on-mismatch
```

Run 2:

```bash
cd /workdir/weppcloud-wbt
python tools/ifolp_wp00_prepare_fixtures.py --run-root /tmp/ifolp_wp00/run2 --overwrite
./tools/ifolp_wp00_run_topaz_oracle.sh --manifest /tmp/ifolp_wp00/run2/manifests/fixture-manifest.json --oracle-root /tmp/ifolp_wp00/run2/oracle --overwrite
rm -rf /tmp/ifolp_wp00/run2/candidate
mkdir -p /tmp/ifolp_wp00/run2/candidate
for d in /tmp/ifolp_wp00/run2/oracle/*; do
  if [ -d "$d" ]; then
    id="$(basename "$d")"
    mkdir -p "/tmp/ifolp_wp00/run2/candidate/$id"
    cp "$d/stream.tif" "/tmp/ifolp_wp00/run2/candidate/$id/stream.tif"
  fi
done
python tools/ifolp_wp00_compare_outputs.py \
  --manifest /tmp/ifolp_wp00/run2/manifests/fixture-manifest.json \
  --oracle-root /tmp/ifolp_wp00/run2/oracle \
  --candidate-root /tmp/ifolp_wp00/run2/candidate \
  --output-json /tmp/ifolp_wp00/run2/reports/parity-report.json \
  --canonical-json /tmp/ifolp_wp00/run2/reports/parity-report.canonical.json \
  --fail-on-mismatch
```

## Parity result summary (both runs)

- Fixture count: `3`
- Exact binary parity: `3/3`
- Mismatches: `[]`

Per fixture (run1 report):

| Fixture ID | Exact equality | Differing cells | Stream delta | Component delta | Junction delta | Outlet reachability match |
|---|---|---:|---:|---:|---:|---|
| `clueless_aftertaste_anchor_10_100` | true | 0 | 0 | 0 | 0 | true |
| `blackwood_60_5` | true | 0 | 0 | 0 | 0 | true |
| `gatecreek_10m_30_2` | true | 0 | 0 | 0 | 0 | true |

## Determinism evidence

Canonical report SHA-256:

- `/tmp/ifolp_wp00/run1/reports/parity-report.canonical.json`
  - `9a171ade68bfc94b31b28285bf2393ea30b3b631ac54d1f83c6f606c1d40237e`
- `/tmp/ifolp_wp00/run2/reports/parity-report.canonical.json`
  - `9a171ade68bfc94b31b28285bf2393ea30b3b631ac54d1f83c6f606c1d40237e`

`diff -u` between run1 and run2 canonical reports: no differences.

## Validation gates

Required WP-00 gates executed:

```bash
cd /workdir/weppcloud-wbt
cargo check -p whitebox_tools
python -m py_compile tools/ifolp_wp00_prepare_fixtures.py tools/ifolp_wp00_compare_outputs.py
```

Results:

- `cargo check -p whitebox_tools`: pass (existing non-blocking warnings in unrelated modules).
- `py_compile`: pass.
- End-to-end harness runs (run1 and run2): pass.

## Review phase record

- Code review status: completed.
- Findings: no blocking findings in WP-00 harness scripts/docs.
- Disposition: none required.

## Residual risks

1. Two fixture threshold pairs (`blackwood_60_5`, `gatecreek_10m_30_2`) are inferred from fixture naming conventions rather than explicit run logs.
2. WP-00 baseline compares candidate outputs staged from oracle copies; non-trivial candidate-vs-oracle divergence will be exercised in later implementation packages.
