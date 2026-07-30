#!/usr/bin/python3
"""Create the deterministic TopazConditionDem irregular-NoData fixture."""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

ROWS = 41
COLUMNS = 47
NODATA = -9999.0


def main() -> None:
    gdal.UseExceptions()
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    row, column = np.indices((ROWS, COLUMNS))
    values = 250.0 + row * 0.4 + column * 0.7

    # A closed depression and a flat exercise conditioning next to masked
    # geometry while the regional trend supplies unambiguous drainage.
    depression = (row - 27) ** 2 + (column - 33) ** 2 <= 9
    values[depression] -= 8.0
    values[16:20, 8:14] = 263.0

    valid = np.ones((ROWS, COLUMNS), dtype=bool)

    # Edge-connected stair-step corridor.
    for r in range(4, 16):
        valid[r, : 2 + (r - 4) // 3] = False

    # Irregular internal hole with a one-cell valid island.
    internal_hole = ((row - 12) ** 2 / 16.0 + (column - 29) ** 2 / 25.0) <= 1.0
    valid[internal_hole] = False
    valid[12, 29] = True

    # L-shaped hole and isolated NoData cell.
    valid[29:35, 10:12] = False
    valid[33:35, 10:19] = False
    valid[22, 21] = False

    output_values = values.astype(np.float32)
    output_values[~valid] = NODATA

    args.output.parent.mkdir(parents=True, exist_ok=True)
    driver = gdal.GetDriverByName("GTiff")
    dataset = driver.Create(
        str(args.output),
        COLUMNS,
        ROWS,
        1,
        gdal.GDT_Float32,
        options=["COMPRESS=DEFLATE"],
    )
    dataset.SetGeoTransform((500000.0, 30.0, 0.0, 5100000.0, 0.0, -30.0))
    spatial_reference = osr.SpatialReference()
    spatial_reference.ImportFromEPSG(32610)
    dataset.SetProjection(spatial_reference.ExportToWkt())
    band = dataset.GetRasterBand(1)
    band.SetNoDataValue(NODATA)
    band.WriteArray(output_values)
    band.FlushCache()
    dataset = None

    print(
        f"wrote {args.output}: {ROWS}x{COLUMNS}, "
        f"valid={np.count_nonzero(valid)}, nodata={np.count_nonzero(~valid)}"
    )


if __name__ == "__main__":
    main()
