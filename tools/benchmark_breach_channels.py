#!/usr/bin/env python3
"""Time the WEPPpy channel pipeline using an isolated optimized WBT binary."""
import argparse
import json
from pathlib import Path
import resource
import time
import traceback

from benchmark_breach_least_cost import raster_fingerprint, sha
from wepppy.topo.wbt.wbt_topaz_emulator import WhiteboxToolsTopazEmulator


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--dem', type=Path, required=True)
    parser.add_argument('--output-dir', type=Path, required=True)
    args = parser.parse_args()
    binary, dem, out = args.binary.resolve(), args.dem.resolve(), args.output_dir.resolve()
    if 'breach-benchmark' not in binary.parts:
        raise ValueError('Use an isolated benchmark binary')
    out.mkdir(parents=True, exist_ok=False)
    (binary.parent / 'settings.json').write_text(json.dumps({
        'verbose_mode': True, 'working_directory': '', 'compress_rasters': False,
        'max_procs': 12}))
    emulator = WhiteboxToolsTopazEmulator(str(out / 'wbt'), str(dem), verbose=True)
    emulator.wbt.set_whitebox_dir(str(binary.parent))
    result = {'binary_sha256': sha(binary), 'dem_sha256': sha(dem),
              'started_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
              'parameters': {'csa_ha': 4.0, 'mcl_m': 40.0, 'stream_pruning_method': 'ifolp',
                             'fill_or_breach': 'breach_least_cost', 'blc_dist_m': 3000},
              'step_elapsed_seconds': {}}
    start = time.perf_counter()

    def completed(_emulator, step, _result, _args, _kwargs):
        result['step_elapsed_seconds'][step] = time.perf_counter() - start

    for step in ['relief', 'flow_vector', 'flow_accumulation', 'extract_streams',
                 'identify_stream_junctions', 'delineate_channels']:
        emulator.register_build_hook(step, completed)
    try:
        emulator.delineate_channels(csa=4.0, mcl=40.0, stream_pruning_method='ifolp',
                                   fill_or_breach='breach_least_cost', blc_dist=3000)
        result['status'] = 'success'
    except Exception as exc:  # Benchmark boundary: retain elapsed time and traceback.
        result['status'] = 'failed'
        result['error'] = str(exc)
        traceback.print_exc()
        raise
    finally:
        result['elapsed_seconds'] = time.perf_counter() - start
        result['ended_utc'] = time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())
        usage = resource.getrusage(resource.RUSAGE_CHILDREN)
        result.update(user_cpu_seconds=usage.ru_utime, system_cpu_seconds=usage.ru_stime,
                      peak_rss_kib=usage.ru_maxrss)
        (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    result['relief'] = raster_fingerprint(emulator.relief)
    result['generated_files'] = {str(f.relative_to(out)): {'bytes': f.stat().st_size, 'sha256': sha(f)}
                                 for f in sorted(out.rglob('*')) if f.is_file() and f.name != 'result.json'}
    (out / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result), flush=True)


if __name__ == '__main__':
    main()
