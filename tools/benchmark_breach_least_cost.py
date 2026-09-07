#!/usr/bin/env python3
"""Run isolated least-cost breach cases and record exact output fingerprints."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import subprocess
import time

import numpy as np
from osgeo import gdal

gdal.UseExceptions()


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def raster_fingerprint(path):
    ds = gdal.Open(str(path))
    band = ds.GetRasterBand(1)
    data = band.ReadAsArray().astype('<f8')
    nodata = band.GetNoDataValue()
    mask = np.ones(data.shape, dtype=bool) if nodata is None else (~np.isnan(data) if np.isnan(nodata) else data != nodata)
    return {'shape': list(data.shape), 'transform': ds.GetGeoTransform(),
            'projection': ds.GetProjection(), 'nodata': nodata,
            'mask_sha256': hashlib.sha256(mask.astype('u1').tobytes()).hexdigest(),
            'values_sha256': hashlib.sha256(data[mask].tobytes()).hexdigest()}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--dem', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--cores', type=int, default=1)
    parser.add_argument('--cpus', default='0')
    parser.add_argument('--dist', type=int, default=600)
    parser.add_argument('--mode', choices=['fail', 'nofill', 'fill'], default='fail')
    parser.add_argument('--max-cost', type=float)
    parser.add_argument('--flat-increment', type=float)
    parser.add_argument('--no-min-dist', action='store_true')
    args = parser.parse_args()
    binary, dem, out = args.binary.resolve(), args.dem.resolve(), args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    # Binary must already be isolated: never edit an installed binary's settings.
    if 'breach-benchmark' not in binary.parts:
        raise ValueError('Copy binary under target/breach-benchmark before benchmarking')
    (binary.parent / 'settings.json').write_text(json.dumps({
        'verbose_mode': True, 'working_directory': '', 'compress_rasters': False,
        'max_procs': args.cores}))
    command = ['taskset', '-c', args.cpus, str(binary), '-r=BreachDepressionsLeastCost',
               f'--dem={dem}', f'--output={out / "relief.tif"}', f'--dist={args.dist}',
               f'--diagnostics={out / "diagnostics.json"}', '--diagnostics_id=0123456789abcdef0123456789abcdef', '-v']
    if not args.no_min_dist:
        command.append('--min_dist')
    if args.mode == 'fail':
        command.append('--fail_on_unresolved')
    elif args.mode == 'fill':
        command.append('--fill')
    if args.max_cost is not None:
        command.append(f'--max_cost={args.max_cost}')
    if args.flat_increment is not None:
        command.append(f'--flat_increment={args.flat_increment}')
    result = {'command': command, 'binary_sha256': sha(binary), 'dem_sha256': sha(dem),
              'host': os.uname().nodename, 'cores': args.cores, 'cpus': args.cpus,
              'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}
    start = time.perf_counter()
    with (out / 'execution.log').open('w') as log:
        proc = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT)
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    result.update(elapsed_seconds=time.perf_counter() - start, returncode=proc.returncode,
                  user_cpu_seconds=usage.ru_utime, system_cpu_seconds=usage.ru_stime,
                  peak_rss_kib=usage.ru_maxrss,
                  ended_utc=time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()))
    if (out / 'relief.tif').exists():
        result['raster'] = raster_fingerprint(out / 'relief.tif')
    if (out / 'diagnostics.json').exists():
        result['diagnostics'] = json.loads((out / 'diagnostics.json').read_text())
    result['unresolved_error'] = re.findall(r'WBT_UNRESOLVED_DEPRESSIONS count=\d+ max_dist_cells=\d+', (out / 'execution.log').read_text())
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)


if __name__ == '__main__':
    main()
