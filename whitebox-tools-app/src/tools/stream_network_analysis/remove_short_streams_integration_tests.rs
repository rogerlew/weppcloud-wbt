use super::*;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_raster_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "remove_short_streams_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn cleanup_raster_artifacts(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn write_four_way_junction_fixture(d8_path: &PathBuf, streams_path: &PathBuf) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let source_d8 = Raster::new(
        &repo_root
            .join("test_fixtures/blackwood_60_5/flovec.tif")
            .to_string_lossy(),
        "r",
    )
    .expect("source d8 fixture should open");
    let source_streams = Raster::new(
        &repo_root
            .join("test_fixtures/blackwood_60_5/netw0.tif")
            .to_string_lossy(),
        "r",
    )
    .expect("source streams fixture should open");

    let mut d8 = Raster::initialize_using_file(&d8_path.to_string_lossy(), &source_d8);
    let mut streams =
        Raster::initialize_using_file(&streams_path.to_string_lossy(), &source_streams);
    d8.reinitialize_values(0.0);
    streams.reinitialize_values(0.0);

    let center = (3isize, 3isize);
    let upstream_cells = [
        ((2isize, 3isize), 8.0),   // north cell flows south
        ((4isize, 3isize), 128.0), // south cell flows north
        ((3isize, 2isize), 2.0),   // west cell flows east
        ((3isize, 4isize), 32.0),  // east cell flows west
    ];

    streams.set_value(center.0, center.1, 1.0);
    for &((row, col), pointer_value) in &upstream_cells {
        streams.set_value(row, col, 1.0);
        d8.set_value(row, col, pointer_value);
    }

    d8.write().expect("temporary d8 raster should write");
    streams
        .write()
        .expect("temporary streams raster should write");
}

fn max_stream_inflow_count(d8: &Raster, streams: &Raster) -> usize {
    let rows = streams.configs.rows as isize;
    let columns = streams.configs.columns as isize;
    let streams_nodata = streams.configs.nodata;
    let d8_nodata = d8.configs.nodata;

    let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
    let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
    let inflowing_vals = [16i32, 32, 64, 128, 1, 2, 4, 8];

    let mut max_inflow_count = 0usize;
    for row in 0..rows {
        for col in 0..columns {
            let stream_val = streams[(row, col)];
            if stream_val == streams_nodata || stream_val <= 0.0 {
                continue;
            }

            let mut inflow_count = 0usize;
            for k in 0..8 {
                let rn = row + dy[k];
                let cn = col + dx[k];
                if rn < 0 || rn >= rows || cn < 0 || cn >= columns {
                    continue;
                }

                let upstream_stream_val = streams[(rn, cn)];
                if upstream_stream_val == streams_nodata || upstream_stream_val <= 0.0 {
                    continue;
                }

                let pointer_val = d8[(rn, cn)];
                if pointer_val == d8_nodata {
                    continue;
                }

                if (pointer_val as i32) == inflowing_vals[k] {
                    inflow_count += 1;
                }
            }

            max_inflow_count = max_inflow_count.max(inflow_count);
        }
    }

    max_inflow_count
}

fn active_stream_count(streams: &Raster) -> usize {
    let mut count = 0usize;
    for row in 0..streams.configs.rows as isize {
        for col in 0..streams.configs.columns as isize {
            let value = streams[(row, col)];
            if value != streams.configs.nodata && value > 0.0 {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn max_junctions_three_prunes_one_branch_from_four_way_junction() {
    let d8_path = temp_raster_path("four_way_d8");
    let streams_path = temp_raster_path("four_way_streams");
    let output_path = temp_raster_path("four_way_output");
    write_four_way_junction_fixture(&d8_path, &streams_path);

    let d8 = Raster::new(&d8_path.to_string_lossy(), "r").expect("d8 fixture should open");
    let streams =
        Raster::new(&streams_path.to_string_lossy(), "r").expect("streams fixture should open");
    assert_eq!(
        max_stream_inflow_count(&d8, &streams),
        4,
        "synthetic fixture should start with a four-inflow junction"
    );

    let tool = RemoveShortStreams::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", streams_path.display()),
        format!("--output={}", output_path.display()),
        "--min_length=0.0".to_string(),
        "--max_junctions=3".to_string(),
    ];
    tool.run(args, "", false)
        .expect("remove_short_streams should run on synthetic fixture");

    let output =
        Raster::new(&output_path.to_string_lossy(), "r").expect("pruned output should open");
    assert_eq!(
        max_stream_inflow_count(&d8, &output),
        3,
        "max_junctions=3 should prune one inflowing branch"
    );
    assert_eq!(
        active_stream_count(&output),
        4,
        "output should retain the receiver and three inflowing branches"
    );

    cleanup_raster_artifacts(&d8_path);
    cleanup_raster_artifacts(&streams_path);
    cleanup_raster_artifacts(&output_path);
}
