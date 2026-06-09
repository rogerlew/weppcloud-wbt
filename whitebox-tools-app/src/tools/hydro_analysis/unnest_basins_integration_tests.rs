use super::*;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use whitebox_vector::{AttributeField, FieldData, FieldDataType, ShapeType, Shapefile};

fn temp_output_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "unnest_basins_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn temp_output_shapefile(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "unnest_basins_{}_{}_{}.shp",
        stem,
        process::id(),
        nanos
    ))
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn cleanup_shapefile(path: &PathBuf) {
    let base = path.with_extension("").to_string_lossy().to_string();
    let _ = fs::remove_file(format!("{}.shp", base));
    let _ = fs::remove_file(format!("{}.shx", base));
    let _ = fs::remove_file(format!("{}.dbf", base));
    let _ = fs::remove_file(format!("{}.prj", base));
    let _ = fs::remove_file(format!("{}.cpg", base));
}

fn output_order_path(output: &PathBuf, order: usize) -> PathBuf {
    let output_path = output.to_string_lossy();
    let pos_of_dot = output_path.rfind('.').unwrap_or(0);
    let ext = &output_path[pos_of_dot..];
    PathBuf::from(output_path.replace(ext, &format!("_{}{}", order, ext)))
}

fn hierarchy_path(output: &PathBuf) -> PathBuf {
    let stem = output
        .file_stem()
        .expect("output path should include a stem")
        .to_string_lossy();
    output.with_file_name(format!("{}_hierarchy.csv", stem))
}

fn labeled_cell_count(raster: &Raster, nodata: f64) -> usize {
    let mut count = 0usize;
    for row in 0..raster.configs.rows as isize {
        for col in 0..raster.configs.columns as isize {
            let value = raster[(row, col)];
            if value != nodata {
                count += 1;
            }
        }
    }
    count
}

fn parse_hierarchy_csv(path: &PathBuf) -> Vec<HashMap<String, String>> {
    let text = fs::read_to_string(path).expect("hierarchy sidecar should read");
    let mut lines = text.lines();
    let _header = lines
        .next()
        .expect("hierarchy sidecar should include a header line");
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            [
                "outlet_id",
                "parent_outlet_id",
                "child_count",
                "child_ids",
                "nesting_order",
                "hierarchy_level",
                "is_root",
                "row",
                "column",
            ]
            .into_iter()
            .zip(parts.into_iter())
            .map(|(k, v)| (k.to_string(), v.trim_matches('"').to_string()))
            .collect()
        })
        .collect()
}

fn write_outlet_shapefile(path: &PathBuf, points: &[(f64, f64)]) {
    let mut output = Shapefile::new(path.to_string_lossy().as_ref(), ShapeType::Point)
        .expect("temporary Shapefile should create");
    output
        .attributes
        .add_field(&AttributeField::new("ID", FieldDataType::Int, 7u8, 0u8));

    for (i, (x, y)) in points.iter().enumerate() {
        output.add_point_record(*x, *y);
        output
            .attributes
            .add_record(vec![FieldData::Int((i + 1) as i32)], false);
    }

    output.write().expect("temporary Shapefile should write")
}

fn fixture_root(fixture_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve")
        .join(format!("test_fixtures/{}", fixture_name))
}

fn read_d8_and_bound(fixture_name: &str) -> (Raster, Raster) {
    let fixture_root = fixture_root(fixture_name);
    let d8 = Raster::new(&fixture_root.join("flovec.tif").to_string_lossy(), "r")
        .expect("fixture D8 raster should open");
    let bound = Raster::new(&fixture_root.join("bound.tif").to_string_lossy(), "r")
        .expect("fixture bound raster should open");
    (d8, bound)
}

fn point_from_row_col(raster: &Raster, row: isize, col: isize) -> (f64, f64) {
    (raster.get_x_from_column(col), raster.get_y_from_row(row))
}

#[test]
fn single_outlet_end_to_end() {
    let (d8, bound) = read_d8_and_bound("minimal_1pixel_stream");
    let outlet_point = point_from_row_col(&d8, 9, 50);

    let shapefile_path = temp_output_shapefile("single_outlet");
    write_outlet_shapefile(&shapefile_path, &[outlet_point]);

    let output_path = temp_output_path("single_outlet");
    let sidecar_path = hierarchy_path(&output_path);

    let tool = UnnestBasins::new();
    let args = vec![
        format!("--d8_pntr={}", d8.file_name),
        format!("--pour_pts={}", shapefile_path.to_string_lossy()),
        format!("--output={}", output_path.display()),
    ];

    tool.run(args, "", false)
        .expect("unnest_basins should run for single outlet fixture");

    let output_1 = output_order_path(&output_path, 1);
    let output_2 = output_order_path(&output_path, 2);
    assert!(output_1.exists(), "single outlet output _1.tif should exist");
    assert!(!output_2.exists(), "single outlet should not emit _2.tif");
    assert!(
        sidecar_path.exists(),
        "single outlet hierarchy sidecar should exist"
    );

    let output_raster = Raster::new(&output_1.to_string_lossy(), "r")
        .expect("single outlet _1 output should open");
    assert_eq!(
        output_raster.configs.rows,
        d8.configs.rows,
        "single outlet output rows should match input"
    );
    assert_eq!(
        output_raster.configs.columns,
        d8.configs.columns,
        "single outlet output columns should match input"
    );

    let mut labeled_cells = 0usize;
    let mut labeled_cells_in_watershed = 0usize;
    for row in 0..d8.configs.rows as isize {
        for col in 0..d8.configs.columns as isize {
            let output_value = output_raster[(row, col)];
            if output_value != output_raster.configs.nodata {
                labeled_cells += 1;
                assert!(
                    bound[(row, col)] > 0.0,
                    "output labels should remain inside the input watershed mask"
                );
            }
            if bound[(row, col)] > 0.0 && output_value > 0.0 {
                labeled_cells_in_watershed += 1;
            }
        }
    }

    assert_eq!(
        output_raster[(9, 50)],
        1.0,
        "single outlet run should label the pour point cell"
    );
    assert_eq!(labeled_cells, labeled_cells_in_watershed, "single outlet labels should only be inside watershed mask");
    assert!(labeled_cells > 0, "single outlet should produce non-trivial delineation");

    cleanup_output_path(&output_1);
    let _ = fs::remove_file(&sidecar_path);
    cleanup_shapefile(&shapefile_path);
}

#[test]
fn single_outlet_hierarchy_csv_fields() {
    let (d8, _bound) = read_d8_and_bound("minimal_1pixel_stream");
    let outlet_point = point_from_row_col(&d8, 9, 50);

    let shapefile_path = temp_output_shapefile("single_outlet_csv");
    write_outlet_shapefile(&shapefile_path, &[outlet_point]);

    let output_path = temp_output_path("single_outlet_csv");
    let sidecar_path = hierarchy_path(&output_path);

    let tool = UnnestBasins::new();
    let args = vec![
        format!("--d8_pntr={}", d8.file_name),
        format!("--pour_pts={}", shapefile_path.to_string_lossy()),
        format!("--output={}", output_path.display()),
    ];

    tool
        .run(args, "", false)
        .expect("unnest_basins should run for CSV assertions");

    let text = fs::read_to_string(&sidecar_path).expect("hierarchy CSV should read");
    let mut lines = text.lines();
    let header = lines
        .next()
        .expect("hierarchy CSV should include header");
    assert_eq!(
        header,
        "outlet_id,parent_outlet_id,child_count,child_ids,nesting_order,hierarchy_level,is_root,row,column",
        "hierarchy header should match expected schema"
    );

    let data_rows: Vec<&str> = lines.filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(data_rows.len(), 1, "single outlet should emit one data row");

    let fields: Vec<&str> = data_rows[0].split(',').collect();
    assert_eq!(fields[0], "1", "outlet_id should be 1");
    assert_eq!(fields[1], "0", "parent_outlet_id should be 0");
    assert_eq!(fields[2], "0", "child_count should be 0");
    assert_eq!(
        fields[3].trim_matches('"'),
        "",
        "child_ids should be empty for no children"
    );
    assert_eq!(fields[4], "1", "nesting_order should be 1");
    assert_eq!(fields[5], "0", "hierarchy_level should be 0");
    assert_eq!(fields[6], "true", "is_root should be true");

    cleanup_output_path(&output_order_path(&output_path, 1));
    let _ = fs::remove_file(&sidecar_path);
    cleanup_shapefile(&shapefile_path);
}

#[test]
fn two_nested_outlets_produce_nested_outputs() {
    let (d8, _bound) = read_d8_and_bound("minimal_2pixel_stream");
    let points = [
        point_from_row_col(&d8, 9, 50),
        point_from_row_col(&d8, 10, 49),
    ];

    let shapefile_path = temp_output_shapefile("nested_outlets");
    write_outlet_shapefile(&shapefile_path, &points);

    let output_path = temp_output_path("nested_outlets");
    let sidecar_path = hierarchy_path(&output_path);

    let tool = UnnestBasins::new();
    let args = vec![
        format!("--d8_pntr={}", d8.file_name),
        format!("--pour_pts={}", shapefile_path.to_string_lossy()),
        format!("--output={}", output_path.display()),
    ];

    tool
        .run(args, "", false)
        .expect("unnest_basins should run for nested outlets fixture");

    let output_1 = output_order_path(&output_path, 1);
    let output_2 = output_order_path(&output_path, 2);
    assert!(output_1.exists(), "nested case should emit _1.tif");
    assert!(output_2.exists(), "nested case should emit _2.tif");
    assert!(sidecar_path.exists(), "nested case should emit hierarchy sidecar");

    let output_raster_1 = Raster::new(&output_1.to_string_lossy(), "r")
        .expect("_1 output should open");
    let output_raster_2 = Raster::new(&output_2.to_string_lossy(), "r")
        .expect("_2 output should open");

    assert!(
        labeled_cell_count(&output_raster_2, output_raster_2.configs.nodata)
            >= labeled_cell_count(&output_raster_1, output_raster_1.configs.nodata),
        "_2 outlet should have at least as many labeled cells as _1"
    );

    let rows = parse_hierarchy_csv(&sidecar_path);
    assert_eq!(rows.len(), 2, "nested case should have two data rows");

    let root_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.get("is_root") == Some(&"true".to_string())
                && row.get("parent_outlet_id") == Some(&"0".to_string())
        })
        .collect();
    assert_eq!(root_rows.len(), 1, "exactly one row should be root");

    let nested_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.get("is_root") == Some(&"false".to_string())
                && row
                    .get("parent_outlet_id")
                    .and_then(|p| p.parse::<i32>().ok())
                    .unwrap_or(-1)
                    > 0
        })
        .collect();
    assert_eq!(nested_rows.len(), 1, "exactly one nested row should have a parent");

    let root_child_count = root_rows
        .first()
        .and_then(|row| row.get("child_count"))
        .and_then(|value| value.parse::<usize>().ok())
        .expect("root row should expose child_count as an integer");
    assert_eq!(root_child_count, 1, "root outlet should have one child");

    cleanup_output_path(&output_1);
    cleanup_output_path(&output_2);
    let _ = fs::remove_file(&sidecar_path);
    cleanup_shapefile(&shapefile_path);
}
