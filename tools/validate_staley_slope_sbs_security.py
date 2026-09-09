#!/usr/bin/env python3
"""Independent, stdlib-only malformed-TIFF and filesystem publication checks.

Fixtures are generated directly from the documented classic TIFF structure;
no scientific raster implementation or external data is copied. Never run the
large theoretical exhaustion cases: the largest fixture payload is 14 MiB.
"""
import argparse
import hashlib
import json
from pathlib import Path
import resource
import signal
import struct
import subprocess
import time

parser = argparse.ArgumentParser(description='Bounded unmocked StaleySlopeSbs security probes (Unix).')
parser.add_argument('--binary', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path, help='Fresh evidence directory; parent must exist.')
args = parser.parse_args()
BINARY = args.binary.resolve(strict=True)
ROOT = args.output.resolve()
ROOT.mkdir(mode=0o700)
RESULTS = []

def tiff(name, values=None, mutate=None):
    keys = [1, 1, 0, 3, 1024, 0, 1, 1, 1025, 0, 1, 1, 3072, 0, 1, 32611]
    tags = {256: (4, [3]), 257: (4, [3]), 258: (3, [64]), 259: (3, [1]),
            262: (3, [1]), 273: (4, [0]), 277: (3, [1]), 278: (4, [3]),
            279: (4, [72]), 284: (3, [1]), 339: (3, [3]),
            33550: (12, [10., 10., 0.]), 33922: (12, [0., 0., 0., 500000., 4000000., 0.]),
            34735: (3, keys), 42113: (2, b'-32768\0')}
    if mutate:
        mutate(tags)
    header_size = 8 + 2 + 12 * len(tags) + 4
    payload = bytearray()
    entries = []
    for tag, (kind, vals) in sorted(tags.items()):
        if kind == 2:
            data = vals
        else:
            data = struct.pack('<' + {1:'B', 3:'H', 4:'I', 12:'d'}[kind] * len(vals), *vals)
        if len(data) <= 4:
            entry = data.ljust(4, b'\0')
        else:
            entry = struct.pack('<I', header_size + len(payload))
            payload.extend(data)
        entries.append(bytearray(struct.pack('<HHI', tag, kind, len(vals)) + entry))
    offset = header_size + len(payload)
    for entry in entries:
        if struct.unpack_from('<H', entry)[0] == 273:
            entry[8:12] = struct.pack('<I', offset)
    sample_format = '<9f' if tags[258][1][0] == 32 else '<9d'
    data = struct.pack(sample_format, *(values or [10. + 5 * (i % 3) for i in range(9)]))
    result = b'II' + struct.pack('<HI', 42, 8) + struct.pack('<H', len(tags)) + b''.join(entries) + b'\0' * 4 + payload + data
    path = ROOT / name
    path.write_bytes(result)
    return path

DEM = tiff('dem.tif')
SBS = tiff('sbs.tif', [2.] * 9)
MASK = tiff('mask.tif', [1.] * 9)
initial = {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [DEM, SBS, MASK]}

def invoke(label, dem=DEM, output=None, preexec=None, success=False, incomplete=False, existing=False):
    output = output or ROOT / label
    cmd = [str(BINARY), '-r=StaleySlopeSbs', '--dem='+str(dem), '--sbs='+str(SBS), '--mask='+str(MASK), '--elevation_units=m', '--sbs_classes=0,1,2,3', '--output_dir='+str(output)]
    start = time.monotonic()
    p = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=15, preexec_fn=preexec)
    result = {'case': label, 'returncode': p.returncode, 'seconds': round(time.monotonic()-start, 4), 'stderr': p.stderr[-1600:], 'output_exists': output.exists(), 'summary_exists': (output / 'summary.json').is_file() if output.is_dir() else False}
    if output.is_dir():
        result['mode'] = oct(output.stat().st_mode & 0o777)
        result['products'] = sorted(p.name for p in output.iterdir())
    RESULTS.append(result)
    print(json.dumps(result), flush=True)
    assert p.returncode == (0 if success else 1), result
    if success:
        summary = json.loads((output / 'summary.json').read_text())
        assert summary['status'] == 'complete', result
        assert result['products'] == ['intersection.tif', 'slope.tif', 'summary.json', 'support.tif'], result
        assert result['mode'] == '0o700', result
    elif incomplete:
        assert output.is_dir() and not result['summary_exists'], result
        assert result['mode'] == '0o700', result
    elif not existing:
        assert not output.exists() and not output.is_symlink(), result
    return result

if __name__ == '__main__':
    print(json.dumps({'scratch': str(ROOT), 'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest()}), flush=True)
    invoke('valid', success=True)
    whitespace = tiff('whitespace_nodata.tif', mutate=lambda t: t.update({42113: (2, b' -32768 \0')}))
    invoke('whitespace_nodata', dem=whitespace, success=True)
    planar = tiff('planar2.tif', mutate=lambda t: t.update({284: (3, [2])}))
    invoke('planar2', dem=planar, success=True)
    overflow = tiff('f32_nodata_overflow.tif', mutate=lambda t: t.update({
        258: (3, [32]), 279: (4, [36]), 42113: (2, b'1e100\0')}))
    invoke('f32_nodata_overflow', dem=overflow)
    nan_nodata = tiff('nan_nodata.tif', [-32768.] * 9,
                      mutate=lambda t: t.update({42113: (2, b'nan\0')}))
    invoke('nan_nodata', dem=nan_nodata)
    variants = {
        'short_geokey': lambda t: t.update({34735: (3, [1])}),
        'excess_geokey_count': lambda t: t[34735][1].__setitem__(3, 200),
        'bad_geokey_ascii_ref': lambda t: (t[34735][1].extend([1026, 34737, 200, 10]), t[34735][1].__setitem__(3, 4)),
        'bad_geokey_double_ref': lambda t: (t[34735][1].extend([2057, 34736, 20, 10]), t[34735][1].__setitem__(3, 4)),
        'unknown_units': lambda t: (t[34735][1].extend([3076, 0, 1, 65535]), t[34735][1].__setitem__(3, 4)),
        'byte_width': lambda t: t.update({256: (1, [3])}),
        'wrong_scale_type': lambda t: t.update({33550: (3, [10, 10, 0])}),
        'two_tiepoints': lambda t: t.update({33922: (12, t[33922][1] * 2)}),
        'invalid_utf8_nodata': lambda t: t.update({42113: (2, b'\xff\0')}),
        'huge_cells': lambda t: t.update({256: (4, [10000001])}),
    }
    for label, mutate in variants.items():
        invoke(label, tiff(label + '.tif', mutate=mutate))
    existing_file = ROOT / 'existing_file'
    existing_file.write_bytes(b'prior-product')
    invoke('existing_file', output=existing_file, existing=True)
    assert existing_file.read_bytes() == b'prior-product'
    existing_dir = ROOT / 'existing_dir'
    existing_dir.mkdir()
    (existing_dir / 'summary.json').write_bytes(b'prior-summary')
    invoke('existing_dir', output=existing_dir, existing=True)
    assert (existing_dir / 'summary.json').read_bytes() == b'prior-summary'
    symlink = ROOT / 'dangling_symlink'
    symlink.symlink_to(ROOT / 'absent-target')
    invoke('dangling_symlink', output=symlink, existing=True)
    assert symlink.is_symlink() and not (ROOT / 'absent-target').exists()
    invoke('missing_parent', output=ROOT / 'absent-parent' / 'result')
    def limited():
        signal.signal(signal.SIGXFSZ, signal.SIG_IGN)
        resource.setrlimit(resource.RLIMIT_FSIZE, (500, 500))
    invoke('midwrite_limit', preexec=limited, incomplete=True)
    strip = tiff('inflated_strip.tif', mutate=lambda t: t.update({279: (4, [128])}))
    with strip.open('ab') as file:
        file.write(bytes(56))
    invoke('inflated_strip', dem=strip)

    # Five metadata tags alias one 14 MiB region: expanded metadata is 70 MiB.
    metadata = tiff('aliased_metadata.tif')
    data = bytearray(metadata.read_bytes())
    old_ifd = struct.unpack_from('<I', data, 4)[0]
    count = struct.unpack_from('<H', data, old_ifd)[0]
    entries = data[old_ifd + 2:old_ifd + 2 + count * 12]
    new_ifd = len(data)
    new_count = count + 5
    shared_start = new_ifd + 2 + 12 * new_count + 4
    extra = b''.join(struct.pack('<HHII', tag, 2, 14 * 1024 * 1024, shared_start)
                     for tag in range(50000, 50005))
    struct.pack_into('<I', data, 4, new_ifd)
    with metadata.open('wb') as file:
        file.write(data)
        file.write(struct.pack('<H', new_count) + entries + extra + bytes(4))
        file.write(bytes(14 * 1024 * 1024))
    invoke('aliased_metadata', dem=metadata)

    def expanded_keys(tags, extra_keys):
        tags[34735][1][3] += extra_keys
        for tag in range(10000, 10000 + extra_keys):
            tags[34735][1].extend([tag, 34737, 65535, 0])
        tags[34737] = (2, b'A' * 65534 + b'|')

    # Bounded expansion remains valid; a count over the 256-key cap is rejected.
    permitted = tiff('permitted_geokeys.tif', mutate=lambda t: expanded_keys(t, 128))
    invoke('permitted_geokeys', dem=permitted, success=True)
    excessive = tiff('excessive_geokeys.tif', mutate=lambda t: expanded_keys(t, 257))
    invoke('excessive_geokeys', dem=excessive)
    assert initial == {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in [DEM, SBS, MASK]}
    report = {'binary': str(BINARY), 'binary_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
              'sources_preserved': True, 'cases': RESULTS, 'passed': len(RESULTS)}
    (ROOT / 'security_results.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'sources_preserved': True, 'passed': len(RESULTS)}), flush=True)
