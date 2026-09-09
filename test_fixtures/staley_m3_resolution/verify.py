"""Verify terrain snapshot hashes without geospatial dependencies."""

import hashlib
import json
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent
    manifest = json.loads((root / "manifest.json").read_text())
    count = 0
    for run in manifest["runs"]:
        for record in run["files"]:
            path = root / record["path"]
            data = path.read_bytes()
            if len(data) != record["bytes"]:
                raise ValueError(f"Size mismatch: {path}")
            if hashlib.sha256(data).hexdigest() != record["sha256"]:
                raise ValueError(f"SHA-256 mismatch: {path}")
            count += 1
    print(f"Verified {count} files across {len(manifest['runs'])} runs.")


if __name__ == "__main__":
    main()
