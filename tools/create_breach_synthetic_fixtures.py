#!/usr/bin/env python3
"""Materialize deterministic tied-pit and flat-edge regression DEMs."""
from pathlib import Path

import numpy as np
from osgeo import gdal, osr


def write(name, data):
    path = Path(__file__).resolve().parents[1] / 'test_fixtures/breach_least_cost' / name
    gdal.UseExceptions()
    ds = gdal.GetDriverByName('GTiff').Create(str(path), data.shape[1], data.shape[0], 1, gdal.GDT_Float64)
    ds.SetGeoTransform((500000, 5, 0, 4500000, 0, -5))
    spatial = osr.SpatialReference()
    spatial.ImportFromEPSG(32610)
    ds.SetProjection(spatial.ExportToWkt())
    ds.GetRasterBand(1).SetNoDataValue(-9999)
    ds.GetRasterBand(1).WriteArray(data)
    ds = None


def main():
    tied = np.full((33, 33), 20., dtype=np.float64)
    tied[0, :] = tied[-1, :] = tied[:, 0] = tied[:, -1] = 0
    tied[3:30:3, 3:30:3] = 10
    tied[12:15, 12:15] = -9999
    tied[22:26, 22:26] = 10
    write('tied_pits.tif', tied)
    flat = np.zeros((65, 65), dtype=np.float64)
    write('flat_edge.tif', flat)
    flat[20:25, 20:25] = 1
    flat[40, 40] = -0.5
    flat[50, 50] = -9999
    write('flat_edge_perturbed.tif', flat)


if __name__ == '__main__':
    main()
