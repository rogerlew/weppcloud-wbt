#!/usr/bin/python3
"""Run TopazConditionDem and verify canonical TOPAZ stage hashes."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from pathlib import Path

import numpy as np
from osgeo import gdal

SCALE = 100_000.0


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_raster(path: Path) -> tuple[np.ndarray, np.ndarray]:
    dataset = gdal.Open(str(path))
    if dataset is None:
        raise ValueError(f"GDAL could not open {path}")
    band = dataset.GetRasterBand(1)
    values = band.ReadAsArray()
    nodata = band.GetNoDataValue()
    valid = np.isfinite(values)
    if nodata is not None:
        valid &= values != nodata
    return values, valid


def mask_hash(valid: np.ndarray) -> str:
    return hashlib.sha256(valid.astype("u1").tobytes(order="C")).hexdigest()


def stage_hash(values: np.ndarray, valid: np.ndarray) -> str:
    scaled = np.rint(values[valid] * SCALE).astype(np.int64)
    limits = np.iinfo(np.int32)
    if np.any(scaled < limits.min) or np.any(scaled > limits.max):
        raise ValueError("conditioned value exceeds canonical TOPAZ i32 range")
    canonical = np.asarray(scaled, dtype="<i4")
    return hashlib.sha256(canonical.tobytes(order="C")).hexdigest()


def run_case(
    binary: Path,
    fixture_root: Path,
    run_root: Path,
    case: dict[str, object],
    timeout_seconds: float,
) -> dict[str, object]:
    case_id = str(case["id"])
    dem = fixture_root / str(case["dem"])
    case_root = run_root / case_id
    case_root.mkdir(parents=True, exist_ok=True)
    relief = case_root / "relief.tif"
    fildep = case_root / "fildep.tif"

    checks: dict[str, object] = {}
    checks["dem_sha256"] = sha256_file(dem)
    if checks["dem_sha256"] != case["dem_sha256"]:
        raise ValueError(f"{case_id}: DEM checksum mismatch")

    command = [
        str(binary),
        "-r=TopazConditionDem",
        f"--dem={dem}",
        f"--output={relief}",
        f"--fildep={fildep}",
        f"--max_obstruction_width={case['max_obstruction_width']}",
    ]
    try:
        completed = subprocess.run(
            command,
            text=True,
            capture_output=True,
            check=False,
            timeout=timeout_seconds,
        )
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(
            f"{case_id}: tool exceeded {timeout_seconds:g}-second timeout"
        ) from error
    if completed.returncode != 0:
        raise RuntimeError(
            f"{case_id}: tool failed ({completed.returncode}): "
            f"{completed.stderr or completed.stdout}"
        )

    input_values, input_valid = read_raster(dem)
    fildep_values, fildep_valid = read_raster(fildep)
    relief_values, relief_valid = read_raster(relief)
    expected_shape = (int(case["rows"]), int(case["columns"]))
    if input_values.shape != expected_shape:
        raise ValueError(
            f"{case_id}: shape {input_values.shape} != expected {expected_shape}"
        )
    if not np.array_equal(input_valid, fildep_valid):
        raise ValueError(f"{case_id}: FILDEP changed the NoData mask")
    if not np.array_equal(input_valid, relief_valid):
        raise ValueError(f"{case_id}: RELIEF changed the NoData mask")

    checks.update(
        {
            "rows": input_values.shape[0],
            "columns": input_values.shape[1],
            "nodata_cells": int(np.count_nonzero(~input_valid)),
            "mask_sha256": mask_hash(input_valid),
            "fildep_sha256": stage_hash(fildep_values, input_valid),
            "relief_sha256": stage_hash(relief_values, input_valid),
        }
    )
    for key in (
        "nodata_cells",
        "mask_sha256",
        "fildep_sha256",
        "relief_sha256",
    ):
        if checks[key] != case[key]:
            raise ValueError(
                f"{case_id}: {key} {checks[key]} != expected {case[key]}"
            )
    return {
        "id": case_id,
        "max_obstruction_width": case["max_obstruction_width"],
        "checks": checks,
        "passed": True,
    }


def main() -> int:
    gdal.UseExceptions()
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument(
        "--fixture-root",
        type=Path,
        help="Override the manifest directory used to resolve DEM paths.",
    )
    parser.add_argument("--work-dir", type=Path, default=Path("target/topaz-parity"))
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=120.0,
        help="Maximum runtime for each parity case (default: 120).",
    )
    selection = parser.add_mutually_exclusive_group(required=True)
    selection.add_argument("--all", action="store_true")
    selection.add_argument("--case", action="append", dest="case_ids")
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")

    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f"binary does not exist: {binary}")
    manifest_path = args.manifest.resolve()
    manifest = json.loads(manifest_path.read_text())
    fixture_root = (
        args.fixture_root.resolve() if args.fixture_root else manifest_path.parent
    )
    available = {str(case["id"]): case for case in manifest["cases"]}
    selected_ids = list(available) if args.all else args.case_ids
    unknown = sorted(set(selected_ids) - set(available))
    if unknown:
        parser.error(f"unknown case(s): {', '.join(unknown)}")

    results: list[dict[str, object]] = []
    failures: list[dict[str, str]] = []
    for case_id in selected_ids:
        try:
            results.append(
                run_case(
                    binary,
                    fixture_root,
                    args.work_dir.resolve(),
                    available[case_id],
                    args.timeout_seconds,
                )
            )
        except (OSError, ValueError, RuntimeError) as error:
            failures.append({"id": case_id, "error": str(error)})

    report = {
        "schema_version": 1,
        "manifest_sha256": sha256_file(manifest_path),
        "selected_cases": selected_ids,
        "passed": not failures,
        "results": results,
        "failures": failures,
    }
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.report:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(rendered)
    sys.stdout.write(rendered)
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main())
