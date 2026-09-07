#!/usr/bin/env python3
"""Validate environment precedence, persistence isolation, errors, and raster parity."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import subprocess

from benchmark_breach_least_cost import raster_fingerprint, sha


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    out = args.output_dir.resolve()
    out.mkdir(parents=True, exist_ok=False)
    binary = out / 'whitebox_tools'
    shutil.copy2(args.binary, binary)
    settings = out / 'settings.json'
    settings.write_text(json.dumps({'verbose_mode': False, 'working_directory': '',
                                    'compress_rasters': False, 'max_procs': 4}))
    dem = root / 'test_fixtures/topaz_condition_dem/dem.tif'

    def run(name, limit, extra=()):
        case = out / name
        case.mkdir()
        env = os.environ.copy()
        env.pop('WBT_MAX_PROCS', None)
        if limit is not None:
            env['WBT_MAX_PROCS'] = limit
        command = [str(binary), '-r=BreachDepressionsLeastCost', f'--dem={dem}',
                   f'--output={case / "relief.tif"}', '--dist=33', '--min_dist', '-v',
                   f'--diagnostics={case / "diagnostics.json"}',
                   '--diagnostics_id=0123456789abcdef0123456789abcdef', *extra]
        result = subprocess.run(command, env=env, capture_output=True, text=True)
        (case / 'execution.log').write_text(result.stdout + result.stderr)
        return case, result

    # Verbosity changes force the persistent save path while the override is active.
    case, proc = run('save-other-setting', '12')
    assert proc.returncode == 0, proc.stderr
    assert json.loads(settings.read_text())['max_procs'] == 4
    assert json.loads(settings.read_text())['verbose_mode'] is True
    assert 'Breach search worker threads: 12' in proc.stdout
    reference = raster_fingerprint(case / 'relief.tif')
    diagnostics = json.loads((case / 'diagnostics.json').read_text())
    saved = settings.read_bytes()
    with ThreadPoolExecutor(max_workers=2) as pool:
        results = list(pool.map(lambda limit: run('concurrent-' + limit, limit), ['1', '12']))
    for limit, (case, proc) in zip(['1', '12'], results):
        assert proc.returncode == 0, proc.stderr
        assert f'Breach search worker threads: {limit}' in proc.stdout
        assert raster_fingerprint(case / 'relief.tif') == reference
        assert json.loads((case / 'diagnostics.json').read_text()) == diagnostics
    assert settings.read_bytes() == saved
    case, proc = run('unset', None)
    assert proc.returncode == 0 and 'Breach search worker threads: 4' in proc.stdout
    assert settings.read_bytes() == saved
    case, proc = run('explicit-persistent-cli', '12', ['--max_procs=3'])
    assert proc.returncode == 0 and 'Breach search worker threads: 12' in proc.stdout
    assert json.loads(settings.read_text())['max_procs'] == 3
    saved = settings.read_bytes()
    invalid = ['', '0', '-1', 'no', '1.5', '9999999999999999999999999']
    for index, limit in enumerate(invalid):
        case, proc = run(f'invalid-{index}', limit, ['--max_procs=7'])
        assert proc.returncode != 0
        assert 'WBT_MAX_PROCS must be a positive integer' in proc.stdout + proc.stderr
        assert not (case / 'relief.tif').exists()
        assert settings.read_bytes() == saved
    settings.unlink()
    case, proc = run('unset-no-settings', None)
    assert proc.returncode == 0
    assert not settings.exists()
    report = {'binary_sha256': sha(binary), 'dem_sha256': sha(dem),
              'checks': ['runtime overrides saved setting', 'override not persisted during other saves',
                         'concurrent 1/12-worker isolation and exact parity', 'unset uses saved limit',
                         'explicit CLI limit persists while environment wins execution',
                         'invalid values fail before writes', 'unset without settings preserves -1'],
              'reference_raster': reference, 'status': 'passed'}
    (out / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report), flush=True)


if __name__ == '__main__':
    main()
