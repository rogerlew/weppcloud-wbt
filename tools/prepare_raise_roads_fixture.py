#!/usr/bin/env python3
"""Materialize a local RaiseRoads development fixture from run data sources."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable, Optional, Tuple

try:
    import rasterio
    from rasterio.windows import from_bounds
except ImportError as exc:  # pragma: no cover - dependency gate
    raise SystemExit(
        "Missing dependency 'rasterio'. Install rasterio to prepare fixtures."
    ) from exc

try:
    from pyproj import Transformer
except ImportError as exc:  # pragma: no cover - dependency gate
    raise SystemExit("Missing dependency 'pyproj'. Install pyproj to prepare fixtures.") from exc


DEFAULT_SOURCE_DEM = "/wc1/runs/ex/exogamous-nimbleness/dem/dem.tif"
DEFAULT_SOURCE_ROADS = "/wc1/runs/sh/shaven-lane/roads/UM1_roads_info.geojson"
DEFAULT_OUTPUT_DIR = "test_fixtures/raise_roads_exogamous_shavenlane"
EPSG_PATTERN = re.compile(r"EPSG[^0-9]*([0-9]{3,5})", re.IGNORECASE)


@dataclass
class Bounds:
    min_x: float
    min_y: float
    max_x: float
    max_y: float

    def buffered(self, distance: float) -> "Bounds":
        return Bounds(
            min_x=self.min_x - distance,
            min_y=self.min_y - distance,
            max_x=self.max_x + distance,
            max_y=self.max_y + distance,
        )

    def clamped(self, limit: "Bounds") -> "Bounds":
        return Bounds(
            min_x=max(self.min_x, limit.min_x),
            min_y=max(self.min_y, limit.min_y),
            max_x=min(self.max_x, limit.max_x),
            max_y=min(self.max_y, limit.max_y),
        )

    def as_tuple(self) -> Tuple[float, float, float, float]:
        return (self.min_x, self.min_y, self.max_x, self.max_y)


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as src:
        for chunk in iter(lambda: src.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def parse_epsg_from_text(text: str) -> Optional[int]:
    if not text:
        return None
    upper = text.upper()
    if "CRS84" in upper:
        return 4326
    match = EPSG_PATTERN.search(text)
    if match is None:
        return None
    return int(match.group(1))


def looks_like_lon_lat(min_x: float, min_y: float, max_x: float, max_y: float) -> bool:
    return min_x >= -180.0 and max_x <= 180.0 and min_y >= -90.0 and max_y <= 90.0


def iter_geojson_linestring_points(root: dict) -> Iterable[Tuple[float, float]]:
    if root.get("type") != "FeatureCollection":
        raise ValueError("Roads GeoJSON must be a FeatureCollection.")
    for feature in root.get("features", []):
        geom = feature.get("geometry") or {}
        geom_type = geom.get("type")
        coords = geom.get("coordinates") or []
        if geom_type == "LineString":
            lines = [coords]
        elif geom_type == "MultiLineString":
            lines = coords
        else:
            continue
        for line in lines:
            for point in line:
                if len(point) >= 2:
                    yield (float(point[0]), float(point[1]))


def infer_geojson_epsg(root: dict, bounds: Bounds) -> Optional[int]:
    crs_name = (
        (((root.get("crs") or {}).get("properties") or {}).get("name"))
        if isinstance(root.get("crs"), dict)
        else None
    )
    epsg = parse_epsg_from_text(crs_name or "")
    if epsg is not None:
        return epsg
    if looks_like_lon_lat(*bounds.as_tuple()):
        return 4326
    return None


def bounds_from_points(points: Iterable[Tuple[float, float]]) -> Bounds:
    min_x = float("inf")
    min_y = float("inf")
    max_x = float("-inf")
    max_y = float("-inf")
    count = 0
    for x, y in points:
        count += 1
        min_x = min(min_x, x)
        min_y = min(min_y, y)
        max_x = max(max_x, x)
        max_y = max(max_y, y)
    if count == 0:
        raise ValueError("Roads GeoJSON does not contain LineString/MultiLineString coordinates.")
    return Bounds(min_x=min_x, min_y=min_y, max_x=max_x, max_y=max_y)


def transform_bounds(bounds: Bounds, source_epsg: int, target_epsg: int) -> Bounds:
    transformer = Transformer.from_crs(
        f"EPSG:{source_epsg}", f"EPSG:{target_epsg}", always_xy=True
    )
    corners = [
        (bounds.min_x, bounds.min_y),
        (bounds.min_x, bounds.max_y),
        (bounds.max_x, bounds.min_y),
        (bounds.max_x, bounds.max_y),
    ]
    tx = []
    ty = []
    for x, y in corners:
        out_x, out_y = transformer.transform(x, y)
        tx.append(out_x)
        ty.append(out_y)
    return Bounds(min_x=min(tx), min_y=min(ty), max_x=max(tx), max_y=max(ty))


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dem", default=DEFAULT_SOURCE_DEM)
    parser.add_argument("--source-roads", default=DEFAULT_SOURCE_ROADS)
    parser.add_argument("--output-dir", default=DEFAULT_OUTPUT_DIR)
    parser.add_argument("--buffer", type=float, default=100.0, help="Clip buffer in map units.")
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Overwrite output directory if it exists.",
    )
    args = parser.parse_args()

    source_dem = Path(args.source_dem).resolve()
    source_roads = Path(args.source_roads).resolve()
    output_dir = Path(args.output_dir).resolve()

    if not source_dem.exists():
        raise SystemExit(f"Source DEM not found: {source_dem}")
    if not source_roads.exists():
        raise SystemExit(f"Source roads not found: {source_roads}")
    generated_files = ("dem_clip.tif", "roads.geojson", "manifest.json")
    if output_dir.exists():
        if args.overwrite:
            for filename in generated_files:
                target = output_dir / filename
                if target.exists():
                    target.unlink()
        elif any((output_dir / filename).exists() for filename in generated_files):
            raise SystemExit(
                f"Output directory already contains fixture artifacts: {output_dir}. "
                "Pass --overwrite to replace generated files."
            )
    else:
        output_dir.mkdir(parents=True, exist_ok=True)

    roads_root = json.loads(source_roads.read_text())
    roads_points = list(iter_geojson_linestring_points(roads_root))
    roads_bounds_source = bounds_from_points(roads_points)
    roads_epsg = infer_geojson_epsg(roads_root, roads_bounds_source)
    if roads_epsg is None:
        raise SystemExit("Unable to infer roads EPSG from GeoJSON metadata or coordinate ranges.")

    with rasterio.open(source_dem) as dem_src:
        dem_crs = dem_src.crs
        if dem_crs is None:
            raise SystemExit("Source DEM CRS is undefined.")
        dem_epsg = dem_crs.to_epsg()
        if dem_epsg is None:
            raise SystemExit("Source DEM EPSG is undefined.")

        roads_bounds_dem = transform_bounds(roads_bounds_source, roads_epsg, dem_epsg)
        dem_bounds = Bounds(*dem_src.bounds)
        clip_bounds = roads_bounds_dem.buffered(args.buffer).clamped(dem_bounds)
        if clip_bounds.min_x >= clip_bounds.max_x or clip_bounds.min_y >= clip_bounds.max_y:
            raise SystemExit("Clip bounds collapsed after clamping; roads do not overlap DEM.")

        window = from_bounds(*clip_bounds.as_tuple(), transform=dem_src.transform)
        window = window.round_offsets().round_lengths()

        subset = dem_src.read(1, window=window)
        transform = dem_src.window_transform(window)
        profile = dem_src.profile.copy()
        profile.update(
            {
                "height": subset.shape[0],
                "width": subset.shape[1],
                "transform": transform,
                "compress": "lzw",
            }
        )

        out_dem = output_dir / "dem_clip.tif"
        with rasterio.open(out_dem, "w", **profile) as dst:
            dst.write(subset, 1)

    out_roads = output_dir / "roads.geojson"
    shutil.copy2(source_roads, out_roads)

    manifest = {
        "name": "raise_roads_exogamous_shavenlane",
        "created_utc": datetime.now(timezone.utc).isoformat(),
        "source": {
            "dem": str(source_dem),
            "roads": str(source_roads),
            "dem_sha256": sha256_file(source_dem),
            "roads_sha256": sha256_file(source_roads),
        },
        "crs": {
            "dem_epsg": dem_epsg,
            "roads_epsg": roads_epsg,
            "reprojection_expected": roads_epsg != dem_epsg,
        },
        "bounds": {
            "roads_source": roads_bounds_source.as_tuple(),
            "roads_in_dem_crs": roads_bounds_dem.as_tuple(),
            "clip_in_dem_crs": clip_bounds.as_tuple(),
        },
        "outputs": {
            "dem_clip": {
                "path": out_dem.name,
                "sha256": sha256_file(out_dem),
            },
            "roads_geojson": {
                "path": out_roads.name,
                "sha256": sha256_file(out_roads),
            },
        },
    }
    manifest_path = output_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")

    print(f"Fixture written: {output_dir}")
    print(f"DEM clip: {out_dem}")
    print(f"Roads: {out_roads}")
    print(f"Manifest: {manifest_path}")


if __name__ == "__main__":
    main()
