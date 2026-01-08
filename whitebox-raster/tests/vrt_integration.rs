use std::path::PathBuf;

use whitebox_raster::{DataType, Raster};

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vrt_test_data")
}

fn data_path(rel: &str) -> PathBuf {
    data_root().join(rel)
}

fn assert_raster_matches_reference(raster: &Raster, reference: &Raster) {
    assert_eq!(raster.configs.rows, reference.configs.rows);
    assert_eq!(raster.configs.columns, reference.configs.columns);
    assert_eq!(raster.configs.data_type, reference.configs.data_type);
    assert_eq!(
        raster.configs.photometric_interp,
        reference.configs.photometric_interp
    );
    assert_eq!(raster.configs.epsg_code, reference.configs.epsg_code);

    let geo_tol = 1e-6;
    assert!((raster.configs.west - reference.configs.west).abs() <= geo_tol);
    assert!((raster.configs.east - reference.configs.east).abs() <= geo_tol);
    assert!((raster.configs.north - reference.configs.north).abs() <= geo_tol);
    assert!((raster.configs.south - reference.configs.south).abs() <= geo_tol);
    assert!(
        (raster.configs.resolution_x - reference.configs.resolution_x).abs() <= geo_tol
    );
    assert!(
        (raster.configs.resolution_y - reference.configs.resolution_y).abs() <= geo_tol
    );

    let tol = match reference.configs.data_type {
        DataType::F32 => 1e-6,
        DataType::F64 => 1e-9,
        _ => 0.0,
    };

    for row in 0..raster.configs.rows {
        for col in 0..raster.configs.columns {
            let expected = reference[(row as isize, col as isize)];
            let actual = raster[(row as isize, col as isize)];
            if reference.configs.data_type.is_float() {
                assert!(
                    (actual - expected).abs() <= tol,
                    "Mismatch at row {} col {}: {} vs {}",
                    row,
                    col,
                    actual,
                    expected
                );
            } else {
                assert!(
                    actual == expected,
                    "Mismatch at row {} col {}: {} vs {}",
                    row,
                    col,
                    actual,
                    expected
                );
            }
        }
    }
}

#[test]
fn test_vrt_raster_loads_center_crop() {
    let vrt_path = data_path("vrt/crop_center_100x100.vrt");
    let reference_path = data_path("reference/crop_center_100x100.tif");

    let raster = Raster::new(vrt_path.to_str().unwrap(), "r")
        .expect("Expected VRT raster to load.");
    let reference = Raster::new(reference_path.to_str().unwrap(), "r")
        .expect("Expected reference raster to load.");

    assert_raster_matches_reference(&raster, &reference);
}

#[test]
fn test_vrt_raster_loads_relative_path() {
    let vrt_path = data_path("vrt/crop_relative_path.vrt");
    let reference_path = data_path("reference/crop_center_100x100.tif");

    let raster = Raster::new(vrt_path.to_str().unwrap(), "r")
        .expect("Expected VRT raster to load.");
    let reference = Raster::new(reference_path.to_str().unwrap(), "r")
        .expect("Expected reference raster to load.");

    assert_raster_matches_reference(&raster, &reference);
}

#[test]
fn test_vrt_raster_loads_fullsize_no_rect() {
    let vrt_path = data_path("vrt/fullsize_no_rect.vrt");
    let source_path = data_path("source/dem_100x100_int16.tif");

    let raster = Raster::new(vrt_path.to_str().unwrap(), "r")
        .expect("Expected VRT raster to load.");
    let reference = Raster::new(source_path.to_str().unwrap(), "r")
        .expect("Expected source raster to load.");

    assert_raster_matches_reference(&raster, &reference);
}
