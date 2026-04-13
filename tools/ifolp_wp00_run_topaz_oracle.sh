#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  tools/ifolp_wp00_run_topaz_oracle.sh \
    --manifest /tmp/ifolp_wp00/manifests/fixture-manifest.json \
    [--oracle-root /tmp/ifolp_wp00/oracle] \
    [--overwrite]

Behavior:
- Clean-room mode only (WP-00): stages pre-generated TopAZ-oracle rasters from fixture sources.
- Validates each staged oracle raster against checksum pinned in fixture-manifest.json.
USAGE
}

manifest=""
oracle_root=""
overwrite=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)
      manifest="$2"
      shift 2
      ;;
    --oracle-root)
      oracle_root="$2"
      shift 2
      ;;
    --overwrite)
      overwrite=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "$manifest" ]]; then
  echo "--manifest is required" >&2
  usage >&2
  exit 2
fi

if [[ ! -f "$manifest" ]]; then
  echo "Manifest not found: $manifest" >&2
  exit 1
fi

if [[ -z "$oracle_root" ]]; then
  oracle_root="$(dirname "$(dirname "$manifest")")/oracle"
fi

python - "$manifest" "$oracle_root" "$overwrite" <<'PY'
from __future__ import annotations

import hashlib
import json
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

manifest_path = Path(sys.argv[1]).resolve()
oracle_root = Path(sys.argv[2]).resolve()
overwrite = sys.argv[3] == "1"

manifest = json.loads(manifest_path.read_text())
fixtures = manifest.get("fixtures", [])
if not fixtures:
    raise SystemExit(f"No fixtures in manifest: {manifest_path}")

if oracle_root.exists():
    if not overwrite:
        raise SystemExit(
            f"Oracle root already exists: {oracle_root}. Pass --overwrite to recreate it."
        )
    shutil.rmtree(oracle_root)

oracle_root.mkdir(parents=True, exist_ok=True)

def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as src:
        for chunk in iter(lambda: src.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()

capture_records = []
for fixture in sorted(fixtures, key=lambda item: item["fixture_id"]):
    fixture_id = fixture["fixture_id"]
    source_oracle = Path(fixture["source"]["oracle_stream"]).resolve()
    expected_sha = fixture["checksums"]["source_oracle_stream_sha256"]

    if not source_oracle.exists():
        raise SystemExit(f"Oracle source missing for {fixture_id}: {source_oracle}")

    staged_dir = oracle_root / fixture_id
    staged_dir.mkdir(parents=True, exist_ok=True)
    staged_path = staged_dir / "stream.tif"

    shutil.copy2(source_oracle, staged_path)

    staged_sha = sha256_file(staged_path)
    if staged_sha != expected_sha:
        raise SystemExit(
            f"Checksum mismatch for {fixture_id}: staged={staged_sha} expected={expected_sha}"
        )

    capture_records.append(
        {
            "fixture_id": fixture_id,
            "source_oracle_stream": str(source_oracle),
            "staged_oracle_stream": str(staged_path),
            "sha256": staged_sha,
        }
    )

capture_manifest = {
    "schema_version": "ifolp_wp00_oracle_capture/v1",
    "capture_mode": "snapshot_copy",
    "clean_room_note": (
        "TopAZ behavior treated as black-box oracle; only raster outputs are staged and"
        " checksum-verified."
    ),
    "captured_at_utc": datetime.now(timezone.utc).isoformat(),
    "fixture_manifest": str(manifest_path),
    "oracle_root": str(oracle_root),
    "fixtures": capture_records,
}

capture_manifest_path = oracle_root / "oracle-capture-manifest.json"
capture_manifest_path.write_text(json.dumps(capture_manifest, indent=2, sort_keys=True) + "\n")

summary = {
    "oracle_root": str(oracle_root),
    "capture_manifest": str(capture_manifest_path),
    "fixture_ids": [record["fixture_id"] for record in capture_records],
}
print(json.dumps(summary, indent=2, sort_keys=True))
PY
