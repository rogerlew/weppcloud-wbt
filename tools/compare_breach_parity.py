#!/usr/bin/env python3
"""Compare benchmark summaries, including raster hashes and failure semantics."""
import argparse
import json
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reference', type=Path)
    parser.add_argument('candidate', type=Path)
    args = parser.parse_args()
    reference = json.loads(args.reference.read_text())
    candidate = json.loads(args.candidate.read_text())
    keys = ['dem_sha256', 'returncode', 'raster', 'diagnostics', 'unresolved_error']
    checks = {key: reference.get(key) == candidate.get(key) for key in keys}
    # Unexpected tool failure is not evidence of parity, even if both runs failed.
    checks['meaningful_result'] = (reference['returncode'] == 0 and
                                   'raster' in reference and 'diagnostics' in reference) or bool(reference['unresolved_error'])
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
