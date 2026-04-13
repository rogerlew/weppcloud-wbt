#!/usr/bin/env python3
"""Prepare checksum-pinned fixtures for IFOLP WP-00 parity harness runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    import rasterio
except ImportError as exc:  # pragma: no cover - dependency gate
    raise SystemExit(
        "Missing dependency 'rasterio'. Install rasterio to prepare IFOLP fixtures."
    ) from exc


DEFAULT_RUN_ROOT = Path("/tmp/ifolp_wp00")


@dataclass(frozen=True)
class FixtureSpec:
    fixture_id: str
    source_root: Path
    d8_pntr: str
    upstream_area: str
    oracle_stream: str
    basin_mask: str
    csa_ha: float
    mscl_m: float
    pointer_encoding: str
    threshold_source: str
    notes: str


FIXTURE_SPECS = [
    FixtureSpec(
        fixture_id="clueless_aftertaste_anchor_10_100",
        source_root=Path("/wc1/runs/cl/clueless-aftertaste/dem/wbt"),
        d8_pntr="flovec.tif",
        upstream_area="floaccum.tif",
        oracle_stream="netw0.tif",
        basin_mask="bound.tif",
        csa_ha=10.0,
        mscl_m=100.0,
        pointer_encoding="whitebox",
        threshold_source=(
            "/wc1/runs/cl/clueless-aftertaste/watershed.log -> "
            "watershed.build_channels(csa=10.0, mcl=100.0)"
        ),
        notes="Required real-world anchor fixture.",
    ),
    FixtureSpec(
        fixture_id="blackwood_60_5",
        source_root=Path("/workdir/weppcloud-wbt/test_fixtures/blackwood_60_5"),
        d8_pntr="flovec.tif",
        upstream_area="floaccum.tif",
        oracle_stream="netw0.tif",
        basin_mask="bound.tif",
        csa_ha=60.0,
        mscl_m=5.0,
        pointer_encoding="whitebox",
        threshold_source=(
            "Fixture naming convention '<name>_<csa>_<mscl>' for "
            "test_fixtures/blackwood_60_5"
        ),
        notes="Repository fixture with moderate network complexity.",
    ),
    FixtureSpec(
        fixture_id="gatecreek_10m_30_2",
        source_root=Path("/workdir/weppcloud-wbt/test_fixtures/gatecreek_10m_30_2"),
        d8_pntr="flovec.tif",
        upstream_area="floaccum.tif",
        oracle_stream="netw0.tif",
        basin_mask="bound.tif",
        csa_ha=30.0,
        mscl_m=2.0,
        pointer_encoding="whitebox",
        threshold_source=(
            "Fixture naming convention '<name>_<resolution>_<csa>_<mscl>' for "
            "test_fixtures/gatecreek_10m_30_2"
        ),
        notes="Repository fixture with large-area, 10m-cell network.",
    ),
]


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as src:
        for chunk in iter(lambda: src.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def raster_info(path: Path) -> dict[str, Any]:
    with rasterio.open(path) as dataset:
        return {
            "width": dataset.width,
            "height": dataset.height,
            "count": dataset.count,
            "dtype": dataset.dtypes[0],
            "nodata": dataset.nodata,
            "crs": dataset.crs.to_string() if dataset.crs else None,
            "resolution_x": dataset.transform.a,
            "resolution_y": abs(dataset.transform.e),
            "transform": tuple(dataset.transform),
        }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--run-root",
        default=str(DEFAULT_RUN_ROOT),
        help="Run directory that receives staged fixture inputs and manifest.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Delete and recreate --run-root if it already exists.",
    )
    parser.add_argument(
        "--fixtures",
        nargs="*",
        default=None,
        help="Optional fixture_id subset. Defaults to all known fixtures.",
    )
    return parser.parse_args()


def select_specs(fixture_ids: list[str] | None) -> list[FixtureSpec]:
    by_id = {spec.fixture_id: spec for spec in FIXTURE_SPECS}
    if fixture_ids is None or len(fixture_ids) == 0:
        return list(FIXTURE_SPECS)

    selected: list[FixtureSpec] = []
    for fixture_id in fixture_ids:
        spec = by_id.get(fixture_id)
        if spec is None:
            known = ", ".join(sorted(by_id))
            raise SystemExit(f"Unknown fixture_id '{fixture_id}'. Known: {known}")
        selected.append(spec)
    return selected


def copy_file(src: Path, dst: Path) -> None:
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst)


def main() -> None:
    args = parse_args()
    run_root = Path(args.run_root).resolve()

    if run_root.exists():
        if not args.overwrite:
            raise SystemExit(
                f"Run root already exists: {run_root}. Pass --overwrite to recreate it."
            )
        shutil.rmtree(run_root)

    fixtures_dir = run_root / "fixtures"
    manifests_dir = run_root / "manifests"
    fixtures_dir.mkdir(parents=True, exist_ok=True)
    manifests_dir.mkdir(parents=True, exist_ok=True)

    selected_specs = select_specs(args.fixtures)

    fixture_records = []
    for spec in selected_specs:
        source_root = spec.source_root.resolve()
        d8_src = source_root / spec.d8_pntr
        area_src = source_root / spec.upstream_area
        oracle_src = source_root / spec.oracle_stream
        basin_src = source_root / spec.basin_mask

        for required in (d8_src, area_src, oracle_src, basin_src):
            if not required.exists():
                raise SystemExit(f"Required fixture input missing: {required}")

        fixture_root = fixtures_dir / spec.fixture_id
        inputs_root = fixture_root / "inputs"
        d8_dst = inputs_root / "d8_pntr.tif"
        area_dst = inputs_root / "upstream_area.tif"
        basin_dst = inputs_root / "basin_mask.tif"

        copy_file(d8_src, d8_dst)
        copy_file(area_src, area_dst)
        copy_file(basin_src, basin_dst)

        d8_src_sha = sha256_file(d8_src)
        area_src_sha = sha256_file(area_src)
        oracle_src_sha = sha256_file(oracle_src)
        basin_src_sha = sha256_file(basin_src)
        d8_dst_sha = sha256_file(d8_dst)
        area_dst_sha = sha256_file(area_dst)
        basin_dst_sha = sha256_file(basin_dst)

        if d8_src_sha != d8_dst_sha:
            raise SystemExit(f"Checksum mismatch after staging pointer raster for {spec.fixture_id}")
        if area_src_sha != area_dst_sha:
            raise SystemExit(
                f"Checksum mismatch after staging upstream-area raster for {spec.fixture_id}"
            )
        if basin_src_sha != basin_dst_sha:
            raise SystemExit(f"Checksum mismatch after staging basin mask for {spec.fixture_id}")

        d8_info = raster_info(d8_dst)
        area_info = raster_info(area_dst)
        oracle_info = raster_info(oracle_src)
        basin_info = raster_info(basin_dst)

        if (
            d8_info["width"] != area_info["width"]
            or d8_info["height"] != area_info["height"]
            or d8_info["width"] != oracle_info["width"]
            or d8_info["height"] != oracle_info["height"]
            or d8_info["width"] != basin_info["width"]
            or d8_info["height"] != basin_info["height"]
        ):
            raise SystemExit(f"Geometry mismatch among fixture rasters for {spec.fixture_id}")

        fixture_records.append(
            {
                "fixture_id": spec.fixture_id,
                "notes": spec.notes,
                "thresholds": {
                    "csa_ha": spec.csa_ha,
                    "mscl_m": spec.mscl_m,
                    "source": spec.threshold_source,
                },
                "pointer_encoding": spec.pointer_encoding,
                "source": {
                    "root": str(source_root),
                    "d8_pntr": str(d8_src),
                    "upstream_area": str(area_src),
                    "oracle_stream": str(oracle_src),
                    "basin_mask": str(basin_src),
                },
                "staged": {
                    "input_d8_pntr": str(d8_dst.relative_to(run_root)),
                    "input_upstream_area": str(area_dst.relative_to(run_root)),
                    "input_basin_mask": str(basin_dst.relative_to(run_root)),
                    "oracle_stream_expected": f"oracle/{spec.fixture_id}/stream.tif",
                },
                "checksums": {
                    "source_d8_pntr_sha256": d8_src_sha,
                    "source_upstream_area_sha256": area_src_sha,
                    "source_oracle_stream_sha256": oracle_src_sha,
                    "source_basin_mask_sha256": basin_src_sha,
                    "staged_d8_pntr_sha256": d8_dst_sha,
                    "staged_upstream_area_sha256": area_dst_sha,
                    "staged_basin_mask_sha256": basin_dst_sha,
                },
                "raster_info": {
                    "d8_pntr": d8_info,
                    "upstream_area": area_info,
                    "oracle_stream": oracle_info,
                    "basin_mask": basin_info,
                },
            }
        )

    fixture_records.sort(key=lambda item: item["fixture_id"])

    manifest = {
        "schema_version": "ifolp_wp00_fixture_manifest/v2",
        "prepared_by": "tools/ifolp_wp00_prepare_fixtures.py",
        "prepared_at_utc": datetime.now(timezone.utc).isoformat(),
        "run_root": str(run_root),
        "comparison_domain": {
            "default_mode": "basin_mask",
            "source": "fixtures/<id>/inputs/basin_mask.tif (staged from bound.tif)",
        },
        "fixture_count": len(fixture_records),
        "fixtures": fixture_records,
    }

    manifest_path = manifests_dir / "fixture-manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")

    summary = {
        "run_root": str(run_root),
        "manifest": str(manifest_path),
        "fixture_ids": [item["fixture_id"] for item in fixture_records],
    }
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
