#!/usr/bin/env python3
"""Compare candidate IFOLP outputs against checksum-pinned TopAZ-oracle outputs."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

try:
    import numpy as np
    import rasterio
    from scipy import ndimage
except ImportError as exc:  # pragma: no cover - dependency gate
    raise SystemExit(
        "Missing dependencies 'numpy', 'rasterio', and/or 'scipy'. "
        "Install them to run IFOLP parity comparisons."
    ) from exc


WHITEBOX_OFFSETS = {
    1: (0, 1),
    2: (-1, 1),
    4: (-1, 0),
    8: (-1, -1),
    16: (0, -1),
    32: (1, -1),
    64: (1, 0),
    128: (1, 1),
}

ESRI_OFFSETS = {
    1: (0, 1),
    2: (1, 1),
    4: (1, 0),
    8: (1, -1),
    16: (0, -1),
    32: (-1, -1),
    64: (-1, 0),
    128: (-1, 1),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, help="Path to fixture-manifest.json")
    parser.add_argument(
        "--oracle-root",
        required=True,
        help="Directory containing <fixture_id>/stream.tif oracle outputs.",
    )
    parser.add_argument(
        "--candidate-root",
        required=True,
        help="Directory containing <fixture_id>/stream.tif candidate outputs.",
    )
    parser.add_argument(
        "--output-json",
        required=True,
        help="Output path for full parity report JSON.",
    )
    parser.add_argument(
        "--canonical-json",
        required=False,
        default=None,
        help="Optional output path for canonical deterministic summary JSON.",
    )
    parser.add_argument(
        "--fail-on-mismatch",
        action="store_true",
        help="Exit non-zero if any fixture is not exact-binary equal.",
    )
    return parser.parse_args()


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as src:
        for chunk in iter(lambda: src.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def load_raster(path: Path) -> tuple[np.ndarray, float | None, dict[str, Any]]:
    with rasterio.open(path) as dataset:
        data = dataset.read(1)
        profile = {
            "width": dataset.width,
            "height": dataset.height,
            "transform": tuple(dataset.transform),
            "crs": dataset.crs.to_string() if dataset.crs else None,
            "dtype": dataset.dtypes[0],
        }
        return data, dataset.nodata, profile


def stream_mask(data: np.ndarray, nodata: float | None) -> np.ndarray:
    mask = np.isfinite(data)
    if nodata is not None:
        if np.isnan(nodata):
            mask &= ~np.isnan(data)
        else:
            mask &= data != nodata
    return mask & (data > 0)


def connected_components_count(mask: np.ndarray) -> int:
    if not np.any(mask):
        return 0
    structure = np.ones((3, 3), dtype=np.uint8)
    _, count = ndimage.label(mask, structure=structure)
    return int(count)


def inflow_junction_count(mask: np.ndarray, d8_data: np.ndarray, offsets: dict[int, tuple[int, int]]) -> int:
    inflow = np.zeros(mask.shape, dtype=np.int16)
    height, width = mask.shape

    stream_rows, stream_cols = np.where(mask)
    for row, col in zip(stream_rows.tolist(), stream_cols.tolist()):
        code = int(round(float(d8_data[row, col])))
        delta = offsets.get(code)
        if delta is None:
            continue
        dr, dc = delta
        nr = row + dr
        nc = col + dc
        if nr < 0 or nr >= height or nc < 0 or nc >= width:
            continue
        if mask[nr, nc]:
            inflow[nr, nc] += 1

    return int(np.count_nonzero(mask & (inflow >= 2)))


def outlet_reachability(
    mask: np.ndarray,
    d8_data: np.ndarray,
    offsets: dict[int, tuple[int, int]],
) -> dict[str, Any]:
    height, width = mask.shape
    state = np.zeros(mask.shape, dtype=np.int8)

    def downstream_stream_cell(row: int, col: int) -> tuple[int, int] | None:
        code = int(round(float(d8_data[row, col])))
        delta = offsets.get(code)
        if delta is None:
            return None
        dr, dc = delta
        nr = row + dr
        nc = col + dc
        if nr < 0 or nr >= height or nc < 0 or nc >= width:
            return None
        if not mask[nr, nc]:
            return None
        return (nr, nc)

    stream_cells = np.argwhere(mask)
    outlet_count = 0
    for row, col in stream_cells.tolist():
        if downstream_stream_cell(row, col) is None:
            outlet_count += 1

    for start_row, start_col in stream_cells.tolist():
        if state[start_row, start_col] != 0:
            continue

        path: list[tuple[int, int]] = []
        visited: set[tuple[int, int]] = set()
        row, col = start_row, start_col

        while True:
            if state[row, col] == 1:
                final_state = 1
                break
            if state[row, col] == -1:
                final_state = -1
                break

            current = (row, col)
            if current in visited:
                final_state = -1
                break

            visited.add(current)
            path.append(current)

            next_cell = downstream_stream_cell(row, col)
            if next_cell is None:
                final_state = 1
                break
            row, col = next_cell

        for path_row, path_col in path:
            state[path_row, path_col] = final_state

    stream_count = int(np.count_nonzero(mask))
    reachable_count = int(np.count_nonzero(state == 1))
    unreachable_count = stream_count - reachable_count

    return {
        "stream_count": stream_count,
        "reachable_count": reachable_count,
        "unreachable_count": unreachable_count,
        "outlet_count": outlet_count,
        "all_stream_cells_reach_outlet": unreachable_count == 0,
    }


def compare_fixture(
    fixture: dict[str, Any],
    run_root: Path,
    oracle_root: Path,
    candidate_root: Path,
) -> dict[str, Any]:
    fixture_id = fixture["fixture_id"]
    pointer_encoding = fixture.get("pointer_encoding", "whitebox").lower()

    if pointer_encoding == "whitebox":
        offsets = WHITEBOX_OFFSETS
    elif pointer_encoding == "esri":
        offsets = ESRI_OFFSETS
    else:
        raise SystemExit(f"Unsupported pointer encoding for fixture {fixture_id}: {pointer_encoding}")

    d8_path = run_root / fixture["staged"]["input_d8_pntr"]
    oracle_path = oracle_root / fixture_id / "stream.tif"
    candidate_path = candidate_root / fixture_id / "stream.tif"

    for required in (d8_path, oracle_path, candidate_path):
        if not required.exists():
            raise SystemExit(f"Missing required raster for fixture {fixture_id}: {required}")

    d8_data, _, d8_profile = load_raster(d8_path)
    oracle_data, oracle_nodata, oracle_profile = load_raster(oracle_path)
    candidate_data, candidate_nodata, candidate_profile = load_raster(candidate_path)

    if (
        oracle_profile["width"] != candidate_profile["width"]
        or oracle_profile["height"] != candidate_profile["height"]
        or oracle_profile["transform"] != candidate_profile["transform"]
        or oracle_profile["crs"] != candidate_profile["crs"]
    ):
        raise SystemExit(f"Candidate/oracle geometry mismatch for fixture {fixture_id}")

    if (
        d8_profile["width"] != oracle_profile["width"]
        or d8_profile["height"] != oracle_profile["height"]
    ):
        raise SystemExit(f"D8 geometry mismatch for fixture {fixture_id}")

    oracle_mask = stream_mask(oracle_data, oracle_nodata)
    candidate_mask = stream_mask(candidate_data, candidate_nodata)

    exact_binary_equal = bool(np.array_equal(candidate_mask, oracle_mask))
    differing_cell_count = int(np.count_nonzero(candidate_mask != oracle_mask))

    oracle_stream_count = int(np.count_nonzero(oracle_mask))
    candidate_stream_count = int(np.count_nonzero(candidate_mask))

    oracle_components = connected_components_count(oracle_mask)
    candidate_components = connected_components_count(candidate_mask)

    oracle_junctions = inflow_junction_count(oracle_mask, d8_data, offsets)
    candidate_junctions = inflow_junction_count(candidate_mask, d8_data, offsets)

    oracle_reachability = outlet_reachability(oracle_mask, d8_data, offsets)
    candidate_reachability = outlet_reachability(candidate_mask, d8_data, offsets)

    outlet_reachability_match = (
        oracle_reachability["all_stream_cells_reach_outlet"]
        == candidate_reachability["all_stream_cells_reach_outlet"]
    )

    return {
        "fixture_id": fixture_id,
        "thresholds": fixture["thresholds"],
        "checksums": {
            "candidate_stream_sha256": sha256_file(candidate_path),
            "oracle_stream_sha256": sha256_file(oracle_path),
        },
        "metrics": {
            "exact_binary_equal": exact_binary_equal,
            "differing_cell_count": differing_cell_count,
            "stream_cell_count": {
                "candidate": candidate_stream_count,
                "oracle": oracle_stream_count,
                "delta": candidate_stream_count - oracle_stream_count,
            },
            "connected_components": {
                "candidate": candidate_components,
                "oracle": oracle_components,
                "delta": candidate_components - oracle_components,
            },
            "junction_count": {
                "candidate": candidate_junctions,
                "oracle": oracle_junctions,
                "delta": candidate_junctions - oracle_junctions,
            },
            "outlet_reachability": {
                "candidate": candidate_reachability,
                "oracle": oracle_reachability,
                "match": outlet_reachability_match,
            },
        },
        "verdict": "match" if exact_binary_equal else "mismatch",
    }


def build_canonical_report(fixtures: list[dict[str, Any]]) -> dict[str, Any]:
    canonical_fixtures = []
    for fixture in fixtures:
        metrics = fixture["metrics"]
        canonical_fixtures.append(
            {
                "fixture_id": fixture["fixture_id"],
                "exact_binary_equal": metrics["exact_binary_equal"],
                "stream_cell_count_delta": metrics["stream_cell_count"]["delta"],
                "connected_component_count_delta": metrics["connected_components"]["delta"],
                "junction_count_delta": metrics["junction_count"]["delta"],
                "outlet_reachability_match": metrics["outlet_reachability"]["match"],
            }
        )

    exact_count = sum(1 for item in canonical_fixtures if item["exact_binary_equal"])
    return {
        "schema_version": "ifolp_wp00_parity_report_canonical/v1",
        "fixtures": canonical_fixtures,
        "summary": {
            "fixture_count": len(canonical_fixtures),
            "exact_binary_equal_count": exact_count,
            "all_exact_binary_equal": exact_count == len(canonical_fixtures),
        },
    }


def main() -> None:
    args = parse_args()

    manifest_path = Path(args.manifest).resolve()
    oracle_root = Path(args.oracle_root).resolve()
    candidate_root = Path(args.candidate_root).resolve()
    output_json = Path(args.output_json).resolve()
    canonical_json = Path(args.canonical_json).resolve() if args.canonical_json else None

    if not manifest_path.exists():
        raise SystemExit(f"Manifest not found: {manifest_path}")

    manifest = json.loads(manifest_path.read_text())
    fixtures = sorted(manifest.get("fixtures", []), key=lambda item: item["fixture_id"])
    if not fixtures:
        raise SystemExit(f"No fixtures in manifest: {manifest_path}")

    run_root = Path(manifest["run_root"]).resolve()
    fixture_reports = [
        compare_fixture(fixture, run_root=run_root, oracle_root=oracle_root, candidate_root=candidate_root)
        for fixture in fixtures
    ]

    exact_count = sum(1 for fixture in fixture_reports if fixture["metrics"]["exact_binary_equal"])
    mismatches = [
        fixture["fixture_id"]
        for fixture in fixture_reports
        if not fixture["metrics"]["exact_binary_equal"]
    ]

    report = {
        "schema_version": "ifolp_wp00_parity_report/v1",
        "fixture_manifest": str(manifest_path),
        "run_root": str(run_root),
        "oracle_root": str(oracle_root),
        "candidate_root": str(candidate_root),
        "fixtures": fixture_reports,
        "summary": {
            "fixture_count": len(fixture_reports),
            "exact_binary_equal_count": exact_count,
            "all_exact_binary_equal": exact_count == len(fixture_reports),
            "mismatched_fixture_ids": mismatches,
        },
    }

    output_json.parent.mkdir(parents=True, exist_ok=True)
    output_json.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")

    canonical_report = build_canonical_report(fixture_reports)
    if canonical_json is not None:
        canonical_json.parent.mkdir(parents=True, exist_ok=True)
        canonical_json.write_text(json.dumps(canonical_report, indent=2, sort_keys=True) + "\n")

    summary = {
        "output_json": str(output_json),
        "canonical_json": str(canonical_json) if canonical_json is not None else None,
        "summary": report["summary"],
    }
    print(json.dumps(summary, indent=2, sort_keys=True))

    if args.fail_on_mismatch and mismatches:
        raise SystemExit(
            f"Exact binary parity mismatches detected for fixtures: {', '.join(mismatches)}"
        )


if __name__ == "__main__":
    main()
