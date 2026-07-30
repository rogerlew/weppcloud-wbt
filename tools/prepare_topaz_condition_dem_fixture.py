#!/usr/bin/env python3
"""Manifest and convert TOPAZ DEDNM conditioning-stage fixtures."""

import argparse
import hashlib
import json
import struct
from pathlib import Path

def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_fortran_i32(path, rows, columns):
    import numpy as np

    raw = Path(path).read_bytes()
    marker = struct.unpack("<i", raw[:4])[0]
    expected = rows * columns * 4
    if marker != expected or struct.unpack("<i", raw[-4:])[0] != expected:
        raise ValueError(f"{path}: unexpected Fortran record markers")
    return np.frombuffer(raw[4:-4], dtype="<i4").reshape(
        (rows, columns), order="F"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dem", required=True, type=Path)
    parser.add_argument("--topaz-output-dir", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--topaz-revision", required=True)
    args = parser.parse_args()
    import numpy as np
    import rasterio

    args.output_dir.mkdir(parents=True, exist_ok=True)

    with rasterio.open(args.dem) as source:
        profile = source.profile.copy()
        rows, columns = source.height, source.width
        raster_metadata = {
            "rows": rows,
            "columns": columns,
            "crs": str(source.crs),
            "transform": list(source.transform),
            "nodata": source.nodata,
        }
    profile.update(dtype="float64", count=1)

    stage_hashes = {}
    for stage in ("INELEV", "FILDEP", "RELIEF"):
        source_path = args.topaz_output_dir / f"{stage}.OUT"
        values = read_fortran_i32(source_path, rows, columns)
        target = args.output_dir / f"{stage.lower()}.tif"
        with rasterio.open(target, "w", **profile) as output:
            output.write(values.astype("float64") / 100000.0, 1)
        stage_hashes[stage] = {
            "out_sha256": sha256(source_path),
            "tif_sha256": sha256(target),
        }

    manifest = {
        "schema_version": 1,
        "dem": {"sha256": sha256(args.dem), **raster_metadata},
        "topaz_revision": args.topaz_revision,
        "parameters": {
            "aggregation_or_resampling": 0,
            "smoothing": 0,
            "max_obstruction_width": 2,
            "partial_processing": 1,
        },
        "stages": stage_hashes,
    }
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )


if __name__ == "__main__":
    main()
