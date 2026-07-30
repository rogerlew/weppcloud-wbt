#!/usr/bin/python3
"""Mask NLCD open-water cells from an exactly aligned DEM fixture."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from osgeo import gdal, osr


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dem", required=True, type=Path)
    parser.add_argument("--nlcd", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--water-class", type=int, default=11)
    args = parser.parse_args()

    gdal.UseExceptions()
    dem_path = args.dem.resolve()
    nlcd_path = args.nlcd.resolve()
    dem = gdal.Open(str(dem_path))
    nlcd = gdal.Open(str(nlcd_path))
    if dem is None or nlcd is None:
        raise ValueError("GDAL could not open both input rasters")
    if (dem.RasterXSize, dem.RasterYSize) != (nlcd.RasterXSize, nlcd.RasterYSize):
        raise ValueError("NLCD dimensions do not exactly match the DEM")
    if not np.allclose(
        dem.GetGeoTransform(), nlcd.GetGeoTransform(), rtol=0.0, atol=1.0e-9
    ):
        raise ValueError("NLCD geotransform does not exactly match the DEM")
    dem_srs = osr.SpatialReference(wkt=dem.GetProjectionRef())
    nlcd_srs = osr.SpatialReference(wkt=nlcd.GetProjectionRef())
    if not dem_srs.IsSame(nlcd_srs):
        raise ValueError("NLCD coordinate system does not match the DEM")

    dem_band = dem.GetRasterBand(1)
    dem_values = dem_band.ReadAsArray()
    dem_nodata = dem_band.GetNoDataValue()
    if dem_nodata is None:
        raise ValueError("DEM must declare a NoData value")
    nlcd_values = nlcd.GetRasterBand(1).ReadAsArray()
    water = nlcd_values == args.water_class
    water_cells = int(np.count_nonzero(water))
    if water_cells == 0:
        raise ValueError(f"NLCD contains no class {args.water_class} cells")
    input_nodata = int(np.count_nonzero(dem_values == dem_nodata))
    derived = dem_values.copy()
    derived[water] = dem_nodata

    output = args.output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    driver = gdal.GetDriverByName("GTiff")
    target = driver.Create(
        str(output),
        dem.RasterXSize,
        dem.RasterYSize,
        1,
        dem_band.DataType,
        options=["COMPRESS=DEFLATE"],
    )
    target.SetGeoTransform(dem.GetGeoTransform())
    target.SetProjection(dem.GetProjectionRef())
    target_band = target.GetRasterBand(1)
    target_band.SetNoDataValue(dem_nodata)
    target_band.WriteArray(derived)
    target_band.FlushCache()
    target.FlushCache()
    target = None

    report = {
        "schema_version": 1,
        "dem": {"path": str(dem_path), "sha256": sha256_file(dem_path)},
        "nlcd": {
            "path": str(nlcd_path),
            "sha256": sha256_file(nlcd_path),
            "water_class": args.water_class,
        },
        "grid": {
            "rows": dem.RasterYSize,
            "columns": dem.RasterXSize,
            "geotransform": list(dem.GetGeoTransform()),
            "coordinate_system": dem_srs.GetAuthorityName(None),
            "coordinate_system_code": dem_srs.GetAuthorityCode(None),
        },
        "counts": {
            "input_nodata_cells": input_nodata,
            "water_cells": water_cells,
            "derived_nodata_cells": int(np.count_nonzero(derived == dem_nodata)),
        },
        "output": {"path": str(output), "sha256": sha256_file(output)},
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
