#!/usr/bin/env python3
"""Run the real-DEM and synthetic breach parity matrix with isolated binaries."""
import argparse
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
SMALL = ROOT / 'test_fixtures/topaz_condition_dem'
TIED = ROOT / 'test_fixtures/breach_least_cost/tied_pits.tif'
CASES = {
    'small': (SMALL / 'dem.tif', ['--mode', 'nofill']),
    'small-fill': (SMALL / 'dem.tif', ['--mode', 'fill']),
    'small-fail': (SMALL / 'dem.tif', ['--mode', 'fail']),
    'nodata': (SMALL / 'synthetic_irregular_nodata.tif', ['--mode', 'nofill', '--dist', '10']),
    'burned': (SMALL / 'burned-out-harmonic_dem.tif', ['--mode', 'nofill']),
    'water': (SMALL / 'burned-out-harmonic_nlcd-water-mask_dem.tif', ['--mode', 'nofill']),
    'portland': (SMALL / 'portland_BRnearMultnoma_HighSevS.202009.chn_cs200_dem.tif', ['--mode', 'nofill']),
    'tied': (TIED, ['--mode', 'nofill', '--dist', '4']),
    'tied-fill': (TIED, ['--mode', 'fill', '--dist', '4']),
    'limited-cost': (SMALL / 'dem.tif', ['--mode', 'nofill', '--max-cost', '1']),
    'no-min-dist': (SMALL / 'dem.tif', ['--mode', 'nofill', '--no-min-dist']),
    'flat-zero': (TIED, ['--mode', 'nofill', '--flat-increment', '0']),
    'flat-edge': (TIED.parent / 'flat_edge.tif', ['--mode', 'nofill', '--dist', '64']),
    'flat-edge-perturbed': (TIED.parent / 'flat_edge_perturbed.tif', ['--mode', 'nofill', '--dist', '64']),
    'short-distance': (TIED, ['--mode', 'fail', '--dist', '1']),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary')
    parser.add_argument('label')
    parser.add_argument('--cases', default=','.join(CASES))
    parser.add_argument('--cores', default='1,12')
    args = parser.parse_args()
    for cores in map(int, args.cores.split(',')):
        for name in args.cases.split(','):
            dem, extra = CASES[name]
            out = ROOT / 'target/breach-benchmark' / f'{args.label}-{name}-{cores}'
            command = [sys.executable, str(ROOT / 'tools/benchmark_breach_least_cost.py'),
                       '--binary', args.binary, '--dem', str(dem), '--output-dir', str(out),
                       '--dist', '33', '--cores', str(cores),
                       '--cpus', '0' if cores == 1 else '0-11', *extra]
            with Path(str(out) + '.log').open('w') as log:
                subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)
            print(out, flush=True)


if __name__ == '__main__':
    main()
