#!/usr/bin/env python3
"""Run RaiseRoads smoke assertions against a prepared fixture."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path

try:
    import numpy as np
    import rasterio
except ImportError as exc:  # pragma: no cover - dependency gate
    raise SystemExit(
        "Missing dependencies 'numpy' and/or 'rasterio'. Install them to run validation."
    ) from exc


DEFAULT_FIXTURE_DIR = "test_fixtures/raise_roads_exogamous_shavenlane"
DEFAULT_BINARY = "target/debug/whitebox_tools"
REPROJ_PATTERN = re.compile(r"Reprojected roads from EPSG:(\d+) to EPSG:(\d+)")


def run_tool(binary: Path, dem: Path, roads: Path, out_raster: Path, strategy: str) -> str:
    args = [
        str(binary),
        "--run=raise_roads",
        f"--dem={dem}",
        f"--roads={roads}",
        f"--output={out_raster}",
        f"--strategy={strategy}",
        "--road_width=5.0",
        "-v",
    ]
    if strategy == "profile_relative":
        args.append("--margin=2.0")
    elif strategy == "constant":
        args.append("--height=5.0")

    proc = subprocess.run(args, check=False, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"RaiseRoads failed for strategy={strategy}.\nSTDOUT:\n{proc.stdout}\nSTDERR:\n{proc.stderr}"
        )
    return proc.stdout + proc.stderr


def raster_checks(dem_path: Path, out_path: Path) -> dict:
    with rasterio.open(dem_path) as src:
        dem = src.read(1)
        dem_nodata = src.nodata
        dem_shape = src.shape
        dem_transform = tuple(src.transform)
        dem_crs = src.crs.to_string() if src.crs else None

    with rasterio.open(out_path) as out:
        arr = out.read(1)
        out_nodata = out.nodata
        out_shape = out.shape
        out_transform = tuple(out.transform)
        out_crs = out.crs.to_string() if out.crs else None

    valid = np.isfinite(dem) & np.isfinite(arr)
    if dem_nodata is not None:
        valid &= dem != dem_nodata
    if out_nodata is not None:
        valid &= arr != out_nodata

    diff = arr[valid] - dem[valid]
    return {
        "exists": out_path.exists(),
        "opens": True,
        "shape_matches": out_shape == dem_shape,
        "transform_matches": out_transform == dem_transform,
        "crs_matches": out_crs == dem_crs,
        "min_diff": float(diff.min()) if diff.size else None,
        "max_diff": float(diff.max()) if diff.size else None,
        "lowered_cells": int((diff < 0).sum()) if diff.size else None,
        "modified_cells": int((diff > 0).sum()) if diff.size else None,
    }


def assert_checks(name: str, checks: dict) -> None:
    assert checks["exists"], f"{name}: output missing"
    assert checks["opens"], f"{name}: output failed to open"
    assert checks["shape_matches"], f"{name}: DEM shape mismatch"
    assert checks["transform_matches"], f"{name}: DEM transform mismatch"
    assert checks["crs_matches"], f"{name}: DEM CRS mismatch"
    assert checks["min_diff"] is not None, f"{name}: no valid cells for diff checks"
    assert checks["min_diff"] >= 0.0, f"{name}: no-lowering violated (min_diff={checks['min_diff']})"
    assert checks["lowered_cells"] == 0, f"{name}: lowered cells detected ({checks['lowered_cells']})"
    assert checks["modified_cells"] is not None and checks["modified_cells"] > 0, (
        f"{name}: expected non-zero modified cells"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture-dir", default=DEFAULT_FIXTURE_DIR)
    parser.add_argument("--binary", default=DEFAULT_BINARY)
    parser.add_argument("--output-dir", default="/tmp/raise_roads_fixture_smoke")
    args = parser.parse_args()

    fixture_dir = Path(args.fixture_dir).resolve()
    dem = fixture_dir / "dem_clip.tif"
    roads = fixture_dir / "roads.geojson"
    manifest = fixture_dir / "manifest.json"
    binary = Path(args.binary).resolve()
    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    for required in (dem, roads, manifest, binary):
        if not required.exists():
            raise SystemExit(f"Required input missing: {required}")

    manifest_json = json.loads(manifest.read_text())
    expected_dem_epsg = manifest_json["crs"]["dem_epsg"]
    expected_roads_epsg = manifest_json["crs"]["roads_epsg"]

    results = {}
    logs = {}
    for strategy in ("profile_relative", "constant"):
        out_raster = output_dir / f"raise_roads_{strategy}.tif"
        log_text = run_tool(binary, dem, roads, out_raster, strategy)
        logs[strategy] = log_text
        match = REPROJ_PATTERN.search(log_text)
        if expected_roads_epsg != expected_dem_epsg:
            assert match is not None, f"{strategy}: expected reprojection log entry"
            src_epsg = int(match.group(1))
            dst_epsg = int(match.group(2))
            assert src_epsg == expected_roads_epsg, (
                f"{strategy}: reprojection source EPSG mismatch ({src_epsg} != {expected_roads_epsg})"
            )
            assert dst_epsg == expected_dem_epsg, (
                f"{strategy}: reprojection target EPSG mismatch ({dst_epsg} != {expected_dem_epsg})"
            )
        checks = raster_checks(dem, out_raster)
        assert_checks(strategy, checks)
        results[strategy] = checks

    summary = {
        "fixture_dir": str(fixture_dir),
        "binary": str(binary),
        "output_dir": str(output_dir),
        "results": results,
    }
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
