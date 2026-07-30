#!/usr/bin/python3
"""Run preprocessing-only TOPAZ and emit canonical conditioning hashes."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
import time
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

TOPAZ_REVISION = "116607fc1185800ca78e387454ef1ccd3ffd73b4"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def canonical_hash(values: np.ndarray, valid: np.ndarray) -> str:
    canonical = np.asarray(values[valid], dtype="<i4")
    return hashlib.sha256(canonical.tobytes(order="C")).hexdigest()


def read_fortran_i32(path: Path, shape: tuple[int, int]) -> np.ndarray:
    raw = path.read_bytes()
    expected = shape[0] * shape[1] * 4
    if len(raw) != expected + 8:
        raise ValueError(f"{path}: expected {expected + 8} bytes, found {len(raw)}")
    first = struct.unpack("<i", raw[:4])[0]
    last = struct.unpack("<i", raw[-4:])[0]
    if first != expected or last != expected:
        raise ValueError(f"{path}: invalid Fortran record markers {first}, {last}")
    return np.frombuffer(raw[4:-4], dtype="<i4").reshape(shape, order="F")


def main() -> int:
    gdal.UseExceptions()
    parser = argparse.ArgumentParser()
    parser.add_argument("--dem", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--max-obstruction-width", required=True, type=int, choices=(0, 1, 2))
    parser.add_argument("--topaz-binary", required=True, type=Path)
    parser.add_argument("--dnmcnt-template", required=True, type=Path)
    args = parser.parse_args()

    dem = args.dem.resolve()
    output_dir = args.output_dir.resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    dataset = gdal.Open(str(dem))
    if dataset is None:
        raise ValueError(f"GDAL could not open {dem}")
    band = dataset.GetRasterBand(1)
    values = band.ReadAsArray()
    nodata = band.GetNoDataValue()
    valid = np.isfinite(values)
    if nodata is not None:
        valid &= values != nodata
    rows, columns = values.shape
    transform = dataset.GetGeoTransform()
    spatial_reference = osr.SpatialReference(wkt=dataset.GetProjectionRef())
    zone = abs(spatial_reference.GetUTMZone())
    if zone == 0:
        raise ValueError("TOPAZ oracle requires a UTM input raster")

    with (output_dir / "DEDNM.INP").open("w") as stream:
        for value, is_valid in zip(values.ravel(order="C"), valid.ravel(order="C")):
            stream.write(f"{float(value) if is_valid else 0.0}\n")

    template = args.dnmcnt_template.read_text()
    metadata = {
        "utm_zone": zone,
        "ll_x": int(round(transform[0])),
        "ll_y": int(round(transform[3] - abs(transform[5]) * rows)),
        "num_rows": rows,
        "num_cols": columns,
        "minimum_elevation": 1.0,
        "maximum_elevation": 9000.0,
        "no_data": 0,
        "cellsize": abs(transform[1]),
        "orientation": 0,
        "outlet_row": rows // 2 + 1,
        "outlet_col": columns // 2 + 1,
        "preprocessing_opt": 0,
        "preprocessing_opt_par": 5,
        "smoothing_weight": 0,
        "smoothing_passes": 2,
        "weighting_par_1": 1,
        "weighting_par_2": 1,
        "weighting_par_3": 1,
        "partial_dem_processing_opt": 1,
        "spatial_csa_par": 0,
        "csa": 5,
        "mcl": 60,
        "sbct_tab": 0,
    }
    control = template.format(**metadata)
    marker = (
        "C * DEPRESSION OUTLET ANALYSIS AND ADJUSTMENT OPTION."
    )
    marker_index = control.index(marker)
    value_index = control.index("\n 2\n", marker_index)
    control = (
        control[:value_index]
        + f"\n {args.max_obstruction_width}\n"
        + control[value_index + len("\n 2\n") :]
    )
    (output_dir / "DNMCNT.INP").write_text(control)

    started = time.monotonic()
    completed = subprocess.run(
        [str(args.topaz_binary.resolve())],
        cwd=output_dir,
        text=True,
        capture_output=True,
        check=False,
    )
    elapsed = time.monotonic() - started
    (output_dir / "dednm.log").write_text(completed.stdout + completed.stderr)
    if completed.returncode != 0:
        raise RuntimeError(
            f"TOPAZ failed ({completed.returncode}); see {output_dir / 'dednm.log'}"
        )

    stages = {}
    for stage in ("INELEV", "FILDEP", "RELIEF"):
        path = output_dir / f"{stage}.OUT"
        stage_values = read_fortran_i32(path, values.shape)
        stages[stage] = {
            "fortran_record_sha256": sha256_file(path),
            "canonical_valid_i32_sha256": canonical_hash(stage_values, valid),
        }

    manifest = {
        "schema_version": 1,
        "topaz_revision": TOPAZ_REVISION,
        "topaz_binary_sha256": sha256_file(args.topaz_binary),
        "dem": {
            "path": str(dem),
            "sha256": sha256_file(dem),
            "rows": rows,
            "columns": columns,
            "valid_cells": int(np.count_nonzero(valid)),
            "nodata_cells": int(np.count_nonzero(~valid)),
            "mask_sha256": hashlib.sha256(
                valid.astype("u1").tobytes(order="C")
            ).hexdigest(),
        },
        "parameters": {
            "aggregation_or_resampling": 0,
            "smoothing": 0,
            "max_obstruction_width": args.max_obstruction_width,
            "partial_processing": 1,
        },
        "elapsed_seconds": elapsed,
        "stages": stages,
    }
    (output_dir / "oracle.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )
    print(json.dumps(manifest, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
