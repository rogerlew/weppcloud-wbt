// Independently derived upstream maximum/area traversal. License: MIT.
use std::collections::VecDeque;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::tools::*;
use whitebox_raster::{DataType, PhotometricInterpretation, Raster, RasterConfigs};

/// Maximum upstream raw elevation minus local elevation, using supplied WBT D8
/// routing. Includes each cell in its upstream area. See docs/d8_upstream_relief.md.
pub struct D8UpstreamRelief {
    parameters: Vec<ToolParameter>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidInput, message.into())
}

impl D8UpstreamRelief {
    pub fn new() -> Self {
        let parameters = [
            ("dem", "Raw elevation DEM", false, false),
            ("d8_pntr", "WBT D8 pointer", false, false),
            ("output", "Vertical relief (m)", true, false),
            ("area", "Upstream area (m2)", true, false),
            (
                "coverage",
                "Potential upstream truncation (0/1)",
                true,
                true,
            ),
        ]
        .iter()
        .map(|(flag, description, output, optional)| ToolParameter {
            name: description.to_string(),
            flags: vec![format!("--{}", flag)],
            description: description.to_string(),
            parameter_type: if *output {
                ParameterType::NewFile(ParameterFileType::Raster)
            } else {
                ParameterType::ExistingFile(ParameterFileType::Raster)
            },
            default_value: None,
            optional: *optional,
        })
        .chain(std::iter::once(ToolParameter {
            name: "Elevation units declaration".into(),
            flags: vec!["--elevation_units".into()],
            description: "Required declaration that raw elevations are meters: m".into(),
            parameter_type: ParameterType::OptionList(vec!["m".into()]),
            default_value: None,
            optional: false,
        }))
        .collect();
        Self { parameters }
    }
}

#[derive(Default)]
struct Inputs {
    dem: String,
    pointer: String,
    relief: String,
    area: String,
    coverage: String,
    units: String,
}

fn unquote(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('\'') && value.ends_with('\''))
            || (value.starts_with('"') && value.ends_with('"')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn parse(args: &[String]) -> Result<Inputs, Error> {
    let mut inputs = Inputs::default();
    let mut i = 0;
    while i < args.len() {
        let (flag, inline) = match args[i].split_once('=') {
            Some((key, value)) => (key, Some(value)),
            None => (args[i].as_str(), None),
        };
        let slot = match flag.trim_start_matches('-').to_lowercase().as_str() {
            "dem" => &mut inputs.dem,
            "d8_pntr" => &mut inputs.pointer,
            "output" | "o" => &mut inputs.relief,
            "area" => &mut inputs.area,
            "coverage" => &mut inputs.coverage,
            "elevation_units" => &mut inputs.units,
            _ => return Err(invalid(format!("Unknown argument: {}", flag))),
        };
        if !slot.is_empty() {
            return Err(invalid(format!("Duplicate argument: {}", flag)));
        }
        let value = match inline {
            Some(value) => value,
            None => {
                i += 1;
                args.get(i)
                    .ok_or_else(|| invalid(format!("Missing value for {}", flag)))?
            }
        };
        if value.is_empty() || value.starts_with("--") {
            return Err(invalid(format!("Missing value for {}", flag)));
        }
        *slot = unquote(value).to_string();
        i += 1;
    }
    if inputs.dem.is_empty()
        || inputs.pointer.is_empty()
        || inputs.relief.is_empty()
        || inputs.area.is_empty()
    {
        return Err(invalid(
            "Required: --dem, --d8_pntr, --output, --area, --elevation_units=m",
        ));
    }
    if inputs.units != "m" {
        return Err(invalid(
            "Declare raw elevation units with --elevation_units=m; other units are unsupported.",
        ));
    }
    Ok(inputs)
}

fn resolved(name: &str, wd: &str) -> Result<PathBuf, Error> {
    let path = Path::new(name);
    let joined = if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(wd).join(path)
    };
    let extension = joined
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if extension != "tif" && extension != "tiff" {
        return Err(invalid("D8UpstreamRelief supports .tif/.tiff files only."));
    }
    Ok(joined)
}

fn new_output(path: &Path) -> Result<PathBuf, Error> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(invalid(format!(
                "Output already exists: {}",
                path.display()
            )))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => (),
        Err(error) => return Err(error),
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize()?;
    let name = path
        .file_name()
        .ok_or_else(|| invalid("Output needs a filename."))?;
    Ok(parent.join(name))
}

// Private staging plus hard-link publication prevents concurrent writers from
// replacing an existing destination. Each output is published independently.
fn publish(out: &mut Raster, destination: &Path) -> Result<(), Error> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("Output needs parent."))?;
    let mut directory;
    loop {
        directory = parent.join(format!(
            ".d8-relief-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&directory) {
            Ok(()) => break,
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    let staged = directory.join("output.tif");
    out.file_name = staged
        .to_str()
        .ok_or_else(|| invalid("Non-UTF8 staging path."))?
        .to_string();
    let result = whitebox_raster::geotiff::write_geotiff(out)
        .and_then(|_| std::fs::hard_link(&staged, destination));
    let cleanup = std::fs::remove_dir_all(&directory);
    if let Err(error) = &cleanup {
        eprintln!(
            "D8UpstreamRelief staging cleanup failed for {}: {}",
            directory.display(),
            error
        );
    }
    result.and(cleanup)
}

fn meters(units: &str) -> bool {
    matches!(
        units.to_lowercase().as_str(),
        "m" | "meter" | "meters" | "metre" | "metres" | "linear_meter"
    )
}

fn validate_grid(c: &RasterConfigs) -> Result<(), Error> {
    if !((32601..=32660).contains(&c.epsg_code) || (32701..=32760).contains(&c.epsg_code)) {
        return Err(invalid(
            "Only WGS84 UTM meter grids (EPSG:32601-32660/32701-32760) are supported.",
        ));
    }
    let point_pixels = c
        .geo_key_directory
        .get(4..)
        .unwrap_or(&[])
        .chunks_exact(4)
        .any(|key| key[0] == 1025 && (key[1] != 0 || key[2] != 1 || key[3] != 1));
    if c.bands != 1
        || point_pixels
        || !c.pixel_is_area
        || c.model_transformation.iter().any(|v| *v != 0.0)
    {
        return Err(invalid(
            "Require a single-band, north-up pixel-area grid without a transformation matrix.",
        ));
    }
    if c.xy_units != "not specified" && !meters(&c.xy_units) {
        return Err(invalid("Horizontal units must be meters."));
    }
    let source_scale = c
        .source_pixel_scale
        .ok_or_else(|| invalid("Original GeoTIFF scale unavailable."))?;
    if source_scale[0] <= 0.0 || source_scale[1] <= 0.0 {
        return Err(invalid(
            "Missing or nonpositive original GeoTIFF pixel scale.",
        ));
    }
    if c.rows == 0
        || c.columns == 0
        || ![
            c.north,
            c.south,
            c.east,
            c.west,
            c.resolution_x,
            c.resolution_y,
        ]
        .iter()
        .all(|x| x.is_finite())
        || c.resolution_x <= 0.0
        || c.resolution_y <= 0.0
        || c.north < c.south
        || c.east < c.west
    {
        return Err(invalid("Invalid raster extent, size or spacing."));
    }
    // The owned GeoTIFF reader stores the last pixel origin as east/south.
    // Require the conventional single top-left tiepoint, avoiding its general
    // tiepoint-offset path; compare input grids using the same representation.
    if c.model_tiepoint.len() != 6 || c.model_tiepoint[0] != 0.0 || c.model_tiepoint[1] != 0.0 {
        return Err(invalid("Require a single top-left GeoTIFF tiepoint."));
    }
    let width = c.resolution_x * c.columns.saturating_sub(1) as f64;
    let height = c.resolution_y * c.rows.saturating_sub(1) as f64;
    if ((c.east - c.west) - width).abs() > 1e-8 * c.resolution_x
        || ((c.north - c.south) - height).abs() > 1e-8 * c.resolution_y
    {
        return Err(invalid("Raster extent does not agree with cell spacing."));
    }
    Ok(())
}

const OFFSETS: [(isize, isize); 8] = [
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
];
const NONE: usize = usize::MAX;

#[derive(Debug)]
struct Terrain {
    relief: Vec<f64>,
    count: Vec<u64>,
    coverage: Vec<u8>,
}

// Kahn traversal: every valid edge is visited once. Maxima and counts combine
// associatively, so queue order cannot change the integer area count or maximum.
fn accumulate(
    z: &[f64],
    pointers: &[f64],
    valid: &[bool],
    rows: usize,
    cols: usize,
) -> Result<Terrain, Error> {
    let n = rows
        .checked_mul(cols)
        .ok_or_else(|| invalid("Raster size overflow."))?;
    if z.len() != n || pointers.len() != n || valid.len() != n {
        return Err(invalid("Array size mismatch."));
    }
    let mut downstream = vec![NONE; n];
    let mut indegree = vec![0u8; n];
    let mut maximum = z.to_vec();
    let mut count = vec![0u64; n];
    let mut coverage = vec![255u8; n];
    for cell in 0..n {
        if !valid[cell] {
            continue;
        }
        count[cell] = 1;
        coverage[cell] = 0;
        let row = (cell / cols) as isize;
        let col = (cell % cols) as isize;
        for (dr, dc) in OFFSETS {
            let (r, c) = (row + dr, col + dc);
            if r < 0
                || c < 0
                || r >= rows as isize
                || c >= cols as isize
                || !valid[r as usize * cols + c as usize]
            {
                coverage[cell] = 1;
            }
        }
        let p = pointers[cell];
        if !z[cell].is_finite() || !p.is_finite() {
            return Err(invalid("Nonfinite valid raster value."));
        }
        if p == 0.0 {
            continue;
        }
        let d = match p {
            1.0 => 0,
            2.0 => 1,
            4.0 => 2,
            8.0 => 3,
            16.0 => 4,
            32.0 => 5,
            64.0 => 6,
            128.0 => 7,
            _ => {
                return Err(invalid(format!(
                    "Invalid WBT D8 pointer {} at row {}, column {}",
                    p, row, col
                )))
            }
        };
        let (r, c) = (row + OFFSETS[d].0, col + OFFSETS[d].1);
        if r >= 0 && c >= 0 && r < rows as isize && c < cols as isize {
            let target = r as usize * cols + c as usize;
            if valid[target] {
                downstream[cell] = target;
                indegree[target] += 1;
            }
        }
    }
    let mut queue = VecDeque::new();
    for cell in 0..n {
        if valid[cell] && indegree[cell] == 0 {
            queue.push_back(cell);
        }
    }
    let mut visited = 0;
    while let Some(cell) = queue.pop_front() {
        visited += 1;
        let target = downstream[cell];
        if target == NONE {
            continue;
        }
        maximum[target] = maximum[target].max(maximum[cell]);
        count[target] += count[cell];
        coverage[target] |= coverage[cell];
        indegree[target] -= 1;
        if indegree[target] == 0 {
            queue.push_back(target);
        }
    }
    if visited != valid.iter().filter(|v| **v).count() {
        return Err(invalid("Cycle in supplied D8 routing."));
    }
    for cell in 0..n {
        maximum[cell] = if valid[cell] {
            maximum[cell] - z[cell]
        } else {
            -32768.0
        };
        if valid[cell] && !maximum[cell].is_finite() {
            return Err(invalid("Relief overflow."));
        }
    }
    Ok(Terrain {
        relief: maximum,
        count,
        coverage,
    })
}

impl WhiteboxTool for D8UpstreamRelief {
    fn get_source_file(&self) -> String {
        file!().into()
    }
    fn get_tool_name(&self) -> String {
        "D8UpstreamRelief".into()
    }
    fn get_tool_description(&self) -> String {
        "Computes maximum upstream raw elevation minus local elevation and contributing area using supplied WBT D8 routing.".into()
    }
    fn get_tool_parameters(&self) -> String {
        format!(
            "{{\"parameters\":{}}}",
            serde_json::to_string(&self.parameters).expect("serializable tool parameters")
        )
    }
    fn get_example_usage(&self) -> String {
        "whitebox_tools -r=D8UpstreamRelief --dem=raw.tif --d8_pntr=pointer.tif --output=height.tif --area=area.tif --elevation_units=m --coverage=coverage.tif".into()
    }
    fn get_toolbox(&self) -> String {
        "Hydrological Analysis".into()
    }
    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let input = parse(&args)?;
        let dem_path = resolved(&input.dem, working_directory)?.canonicalize()?;
        let pointer_path = resolved(&input.pointer, working_directory)?.canonicalize()?;
        let mut outputs = vec![
            new_output(&resolved(&input.relief, working_directory)?)?,
            new_output(&resolved(&input.area, working_directory)?)?,
        ];
        if !input.coverage.is_empty() {
            outputs.push(new_output(&resolved(&input.coverage, working_directory)?)?);
        }
        for (i, path) in outputs.iter().enumerate() {
            if path == &dem_path || path == &pointer_path || outputs[..i].contains(path) {
                return Err(invalid(
                    "Output paths must be distinct from inputs and each other.",
                ));
            }
        }
        let dem = Raster::new(
            dem_path
                .to_str()
                .ok_or_else(|| invalid("Non-UTF8 DEM path."))?,
            "r",
        )?;
        let pointer = Raster::new(
            pointer_path
                .to_str()
                .ok_or_else(|| invalid("Non-UTF8 pointer path."))?,
            "r",
        )?;
        let c = &dem.configs;
        let p = &pointer.configs;
        validate_grid(c)?;
        validate_grid(p)?;
        if c.z_units != "not specified" && !meters(&c.z_units) {
            return Err(invalid(
                "DEM vertical units contradict --elevation_units=m.",
            ));
        }
        if c.rows != p.rows
            || c.columns != p.columns
            || c.epsg_code != p.epsg_code
            || [
                c.north,
                c.south,
                c.east,
                c.west,
                c.resolution_x,
                c.resolution_y,
            ] != [
                p.north,
                p.south,
                p.east,
                p.west,
                p.resolution_x,
                p.resolution_y,
            ]
        {
            return Err(invalid("DEM and pointer grids must align exactly."));
        }
        let n = c
            .rows
            .checked_mul(c.columns)
            .ok_or_else(|| invalid("Raster size overflow."))?;
        let mut z = Vec::with_capacity(n);
        let mut flow = Vec::with_capacity(n);
        let mut valid = Vec::with_capacity(n);
        for row in 0..c.rows as isize {
            for col in 0..c.columns as isize {
                let a = dem.get_value(row, col);
                let b = pointer.get_value(row, col);
                let an = a == c.nodata || (c.nodata.is_nan() && a.is_nan());
                let bn = b == p.nodata || (p.nodata.is_nan() && b.is_nan());
                if an != bn {
                    return Err(invalid("DEM and pointer NoData masks must match exactly."));
                }
                z.push(a);
                flow.push(b);
                valid.push(!an);
            }
        }
        if !valid.iter().any(|v| *v) {
            return Err(invalid("No valid terrain cells."));
        }
        let start = Instant::now();
        let terrain = accumulate(&z, &flow, &valid, c.rows, c.columns)?;
        let cell_area = c.resolution_x * c.resolution_y;
        if !(cell_area * n as f64).is_finite() {
            return Err(invalid("Area overflow."));
        }
        let flagged = terrain.coverage.iter().filter(|v| **v == 1).count();
        let elapsed = start.elapsed();
        for (kind, path) in outputs.iter().enumerate() {
            let mut out = Raster::initialize_using_file(
                path.to_str()
                    .ok_or_else(|| invalid("Non-UTF8 output path."))?,
                &dem,
            );
            out.configs.data_type = if kind == 2 {
                DataType::U8
            } else {
                DataType::F64
            };
            out.configs.nodata = if kind == 2 { 255.0 } else { -32768.0 };
            // Regenerate UTM keys so area/coverage do not inherit elevation units.
            out.configs.geo_key_directory.clear();
            out.configs.geo_double_params.clear();
            out.configs.geo_ascii_params.clear();
            out.configs.photometric_interp = PhotometricInterpretation::Continuous;
            out.configs.z_units = if kind == 0 {
                "meters".into()
            } else {
                "not specified".into()
            };
            out.configs.minimum = f64::INFINITY;
            out.configs.maximum = f64::NEG_INFINITY;
            for row in 0..c.rows {
                for col in 0..c.columns {
                    let i = row * c.columns + col;
                    let value = match kind {
                        0 => terrain.relief[i],
                        1 => {
                            if valid[i] {
                                terrain.count[i] as f64 * cell_area
                            } else {
                                -32768.0
                            }
                        }
                        _ => terrain.coverage[i] as f64,
                    };
                    if value != out.configs.nodata {
                        out.configs.minimum = out.configs.minimum.min(value);
                        out.configs.maximum = out.configs.maximum.max(value);
                    }
                    out.set_value(row as isize, col as isize, value);
                }
            }
            for entry in [
                "Created by D8UpstreamRelief".to_string(),
                "Relief: maximum upstream raw elevation minus local elevation; includes local cell".into(),
                format!("Raw DEM: {}; elevation_units=m",dem_path.display()),
                format!("Pointer: {}; WBT D8 encoding; 0=terminal",pointer_path.display()),
                "Area: contributing cell count including local cell times dx*dy; m2".into(),
                format!("Potential truncation: D8 edge/NoData adjacency propagated downstream; flagged cells={}",flagged),
                format!("Output quantity: {}",["relief (m)","area (m2)","potential truncation (0/1)"][kind]),
                format!("Traversal elapsed seconds: {}",elapsed.as_secs_f64()),
            ] {out.add_metadata_entry(entry);}
            publish(&mut out, path)?;
        }
        if verbose {
            println!("D8UpstreamRelief complete: {} cells; {} potentially truncated cells; traversal {:?}",n,flagged,elapsed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytical_chains_nested_outlets_and_conditioning() {
        for (z, expected) in [
            (vec![130., 120., 100.], vec![0., 10., 30.]),
            (vec![110., 130., 100.], vec![0., 0., 30.]),
            (vec![130., 90., 100.], vec![0., 40., 30.]),
            (vec![100., 100., 100.], vec![0., 0., 0.]),
        ] {
            let t = accumulate(&z, &[2., 2., 0.], &[true; 3], 1, 3).unwrap();
            assert_eq!(t.relief, expected);
            assert_eq!(t.count, vec![1, 2, 3]);
        }
    }
    #[test]
    fn unequal_tributaries_and_coverage_propagation() {
        let mut z = vec![100.; 25];
        let mut p = vec![0.; 25];
        z[6] = 140.;
        z[8] = 170.;
        z[12] = 120.;
        p[6] = 4.;
        p[8] = 16.;
        p[12] = 8.;
        let t = accumulate(&z, &p, &[true; 25], 5, 5).unwrap();
        assert_eq!(t.count[12], 3);
        assert_eq!(t.relief[12], 50.);
        assert_eq!(t.count[17], 4);
        assert_eq!(t.relief[17], 70.);
        assert_eq!(t.coverage[12], 0);
        p[0] = 4.;
        let t = accumulate(&z, &p, &[true; 25], 5, 5).unwrap();
        assert_eq!(t.coverage[12], 1);
        assert_eq!(t.coverage[17], 1);
    }
    #[test]
    fn invalid_cycles_values_and_nodata_boundary() {
        assert!(accumulate(&[2., 1.], &[2., 32.], &[true; 2], 1, 2)
            .unwrap_err()
            .to_string()
            .contains("Cycle"));
        for p in [3., -1., 1.5, f64::INFINITY] {
            assert!(accumulate(&[1.], &[p], &[true], 1, 1).is_err());
        }
        let t = accumulate(&[2., -9999.], &[2., -9999.], &[true, false], 1, 2).unwrap();
        assert_eq!(t.relief, vec![0., -32768.]);
        assert_eq!(t.count, vec![1, 0]);
    }
    #[test]
    fn publication_never_replaces_existing_destination_and_propagates_io_error() {
        let root =
            std::env::temp_dir().join(format!("terrain-publish-test-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let destination = root.join("existing.tif");
        std::fs::write(&destination, b"preserve").unwrap();
        let mut configs = RasterConfigs::default();
        configs.rows = 1;
        configs.columns = 1;
        configs.north = 1.;
        configs.south = 0.;
        configs.east = 1.;
        configs.west = 0.;
        configs.resolution_x = 1.;
        configs.resolution_y = 1.;
        configs.data_type = DataType::F64;
        configs.photometric_interp = PhotometricInterpretation::Continuous;
        let mut out = Raster::initialize_using_config("unused.tif", &configs);
        out.set_value(0, 0, 1.);
        assert_eq!(
            publish(&mut out, &destination).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert_eq!(std::fs::read(&destination).unwrap(), b"preserve");
        assert!(publish(&mut out, &root.join("missing").join("out.tif")).is_err());
        // A writer-level failure (unknown storage type) must not publish a file.
        out.configs.data_type = DataType::Unknown;
        assert!(publish(&mut out, &root.join("failed.tif")).is_err());
        assert!(!root.join("failed.tif").exists());
        assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parser_rejects_missing_unknown_and_duplicate() {
        for args in [
            vec!["--dem"],
            vec!["--unknown=x"],
            vec!["--dem=a", "--dem=b"],
        ] {
            assert!(parse(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
        }
    }
}
