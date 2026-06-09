/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: WEPPcloud development team
Created: 04/03/2026
Last Modified: 04/03/2026
License: MIT
*/

use crate::tools::*;
use geojson::{GeoJson, Geometry, Value as GeoValue};
use proj::Proj;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path;
use whitebox_common::structures::{Array2D, DistanceMetric, FixedRadiusSearch2D};
use whitebox_raster::*;
use whitebox_vector::{FieldData, ShapeType, Shapefile};

const DEFAULT_HEIGHT: f64 = 5.0;
const DEFAULT_MARGIN: f64 = 2.0;
const DEFAULT_ROAD_WIDTH: f64 = 5.0;
const DEFAULT_CROWN_WIDTH: f64 = 4.0;
const DEFAULT_SHOULDER_WIDTH: f64 = 1.0;
const DEFAULT_SHOULDER_SLOPE: f64 = 0.08;
const DEFAULT_BACKSLOPE_ANGLE: f64 = 30.0;

const CONSERVATIVE_CROSS_CROWN_WIDTH: f64 = 3.0;
const CONSERVATIVE_CROSS_SHOULDER_WIDTH: f64 = 0.5;
const CONSERVATIVE_CROSS_SHOULDER_SLOPE: f64 = 0.12;
const CONSERVATIVE_CROSS_BACKSLOPE_ANGLE: f64 = 18.0;
const CONSERVATIVE_CROSS_MAX_HEIGHT: f64 = 1.5;

#[derive(Clone, Copy, PartialEq)]
enum Strategy {
    Constant,
    ProfileRelative,
    CrossSection,
}

impl Strategy {
    fn from_str(value: &str) -> Result<Strategy, Error> {
        match value.to_lowercase().as_str() {
            "constant" => Ok(Strategy::Constant),
            "profile_relative" => Ok(Strategy::ProfileRelative),
            "cross_section" => Ok(Strategy::CrossSection),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Unrecognized strategy. Expected one of: constant, profile_relative, cross_section.",
            )),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Strategy::Constant => "constant",
            Strategy::ProfileRelative => "profile_relative",
            Strategy::CrossSection => "cross_section",
        }
    }
}

#[derive(Clone, Copy)]
enum TaperMode {
    Cosine,
    Linear,
    None,
}

impl TaperMode {
    fn from_str(value: &str) -> Result<TaperMode, Error> {
        match value.to_lowercase().as_str() {
            "cosine" => Ok(TaperMode::Cosine),
            "linear" => Ok(TaperMode::Linear),
            "none" => Ok(TaperMode::None),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "Unrecognized taper mode. Expected one of: cosine, linear, none.",
            )),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            TaperMode::Cosine => "cosine",
            TaperMode::Linear => "linear",
            TaperMode::None => "none",
        }
    }
}

#[derive(Clone)]
struct FeatureAttributes {
    values: HashMap<String, String>,
}

impl FeatureAttributes {
    fn new() -> FeatureAttributes {
        FeatureAttributes {
            values: HashMap::new(),
        }
    }

    fn insert(&mut self, key: &str, value: String) {
        self.values.insert(key.to_lowercase(), value);
    }

    fn get_text(&self, key: &str) -> Option<&str> {
        self.values
            .get(&key.to_lowercase())
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
    }

    fn get_text_from_candidates(&self, keys: &[&str]) -> Option<String> {
        for key in keys {
            if let Some(value) = self.get_text(key) {
                return Some(value.to_string());
            }
        }
        None
    }

    fn get_number(&self, key: &str) -> Option<f64> {
        self.get_text(key)?.parse::<f64>().ok()
    }

    fn get_number_from_candidates(&self, keys: &[&str]) -> Option<f64> {
        for key in keys {
            if let Some(value) = self.get_number(key) {
                return Some(value);
            }
        }
        None
    }

    fn has_any_key(&self, keys: &[&str]) -> bool {
        keys.iter().any(|key| self.values.contains_key(&key.to_lowercase()))
    }
}

#[derive(Clone)]
struct RawFeature {
    attrs: FeatureAttributes,
    parts: Vec<Vec<(f64, f64)>>,
}

#[derive(Clone)]
struct CrossSectionParams {
    crown_width: f64,
    shoulder_width: f64,
    shoulder_slope: f64,
    backslope_angle: f64,
}

impl CrossSectionParams {
    fn defaults() -> CrossSectionParams {
        CrossSectionParams {
            crown_width: DEFAULT_CROWN_WIDTH,
            shoulder_width: DEFAULT_SHOULDER_WIDTH,
            shoulder_slope: DEFAULT_SHOULDER_SLOPE,
            backslope_angle: DEFAULT_BACKSLOPE_ANGLE,
        }
    }

    fn conservative_unpaved() -> CrossSectionParams {
        CrossSectionParams {
            crown_width: CONSERVATIVE_CROSS_CROWN_WIDTH,
            shoulder_width: CONSERVATIVE_CROSS_SHOULDER_WIDTH,
            shoulder_slope: CONSERVATIVE_CROSS_SHOULDER_SLOPE,
            backslope_angle: CONSERVATIVE_CROSS_BACKSLOPE_ANGLE,
        }
    }

    fn is_valid(&self) -> bool {
        self.crown_width > 0.0
            && self.shoulder_width >= 0.0
            && self.shoulder_slope >= 0.0
            && self.backslope_angle > 0.0
            && self.backslope_angle < 89.5
    }
}

#[derive(Clone)]
enum CrossSectionMode {
    Direct,
    ConservativeUnpaved,
    FallbackProfileRelative,
}

#[derive(Clone)]
struct FeatureConfig {
    search_radius_cells: isize,
    influence_radius: f64,
    cross_section_mode: CrossSectionMode,
    cross_params: CrossSectionParams,
    cross_height: f64,
}

#[derive(Clone)]
struct FlattenedRoadPart {
    feature_idx: usize,
    points: Vec<(f64, f64)>,
}

struct RoadNetwork {
    raw_features: Vec<RawFeature>,
    source_epsg: Option<u16>,
    source_crs_note: String,
}

#[derive(Default)]
struct ResolutionStats {
    width_from_field_count: usize,
    width_from_heuristic_count: usize,
    width_from_default_count: usize,
    cross_direct_count: usize,
    cross_conservative_count: usize,
    cross_fallback_count: usize,
}

#[derive(Default)]
struct ReprojectionSummary {
    source_epsg: Option<u16>,
    target_epsg: Option<u16>,
    reprojected: bool,
    transformed_points: usize,
}

pub struct RaiseRoads {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RaiseRoads {
    pub fn new() -> RaiseRoads {
        let name = "RaiseRoads".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Raises road embankments in a DEM using constant, profile-relative, or cross-section strategies."
                .to_string();

        let mut parameters = vec![];

        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Roads File".to_owned(),
            flags: vec!["--roads".to_owned()],
            description: "Input roads vector lines file (.shp, .geojson, or .json).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Line,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raised DEM raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Raise strategy".to_owned(),
            flags: vec!["--strategy".to_owned()],
            description: "Raise strategy; options include 'constant', 'profile_relative', and 'cross_section'."
                .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "constant".to_string(),
                "profile_relative".to_string(),
                "cross_section".to_string(),
            ]),
            default_value: Some("profile_relative".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Default road width (map units)".to_owned(),
            flags: vec!["--road_width".to_owned()],
            description: "Fallback road width in map units when no feature width/class value is available."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_ROAD_WIDTH.to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Width attribute field".to_owned(),
            flags: vec!["--width_field".to_owned()],
            description: "Optional feature attribute name containing per-road widths (map units)."
                .to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Constant height increment".to_owned(),
            flags: vec!["--height".to_owned()],
            description: "Height increment for constant strategy and cross-section crest height (map units)."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_HEIGHT.to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Profile-relative margin".to_owned(),
            flags: vec!["--margin".to_owned()],
            description: "Margin above local terrain max for profile_relative strategy (map units).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_MARGIN.to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Local terrain search radius".to_owned(),
            flags: vec!["--search_radius".to_owned()],
            description: "Optional local terrain search radius in map units. Defaults to a width-derived value."
                .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Taper mode".to_owned(),
            flags: vec!["--taper".to_owned()],
            description: "Edge taper mode for constant/profile_relative strategies; options include 'cosine', 'linear', and 'none'."
                .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "cosine".to_string(),
                "linear".to_string(),
                "none".to_string(),
            ]),
            default_value: Some("cosine".to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Cross-section crown width".to_owned(),
            flags: vec!["--crown_width".to_owned()],
            description: "Cross-section crown width in map units.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_CROWN_WIDTH.to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Cross-section shoulder width".to_owned(),
            flags: vec!["--shoulder_width".to_owned()],
            description: "Cross-section shoulder width in map units.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_SHOULDER_WIDTH.to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Cross-section shoulder slope".to_owned(),
            flags: vec!["--shoulder_slope".to_owned()],
            description: "Cross-section shoulder slope (rise per run).".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_SHOULDER_SLOPE.to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Cross-section backslope angle".to_owned(),
            flags: vec!["--backslope_angle".to_owned()],
            description: "Cross-section backslope angle in degrees.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_BACKSLOPE_ANGLE.to_string()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }

        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=dem.tif --roads=roads.shp --output=raised.tif --strategy=profile_relative --margin=2.0 --road_width=5.0\n>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=dem.tif --roads=roads.geojson --output=raised_cross.tif --strategy=cross_section --height=1.5 --crown_width=3.0 --shoulder_width=0.5 --shoulder_slope=0.12 --backslope_angle=18.0",
            short_exe, name
        )
        .replace("*", &sep);

        RaiseRoads {
            name,
            description,
            toolbox,
            parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RaiseRoads {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => format!("{{\"parameters\":{}}}", json_str),
            Err(err) => format!("{:?}", err),
        }
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut dem_file = String::new();
        let mut roads_file = String::new();
        let mut output_file = String::new();

        let mut strategy = Strategy::ProfileRelative;
        let mut road_width = DEFAULT_ROAD_WIDTH;
        let mut width_field = String::new();
        let mut height = DEFAULT_HEIGHT;
        let mut margin = DEFAULT_MARGIN;
        let mut search_radius: Option<f64> = None;
        let mut taper_mode = TaperMode::Cosine;

        let mut cross_defaults = CrossSectionParams::defaults();

        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }

        for i in 0..args.len() {
            let mut arg = args[i].replace('"', "");
            arg = arg.replace("\'", "");
            let cmd = arg.split('=');
            let vec = cmd.collect::<Vec<&str>>();
            let keyval = vec.len() > 1;
            let flag_val = vec[0].to_lowercase().replace("--", "-");

            if flag_val == "-dem" {
                dem_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-roads" {
                roads_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-strategy" {
                let strategy_str = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                strategy = Strategy::from_str(&strategy_str)?;
            } else if flag_val == "-road_width" {
                road_width = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-width_field" {
                width_field = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-height" {
                height = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-margin" {
                margin = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-search_radius" {
                let radius = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
                search_radius = Some(radius);
            } else if flag_val == "-taper" {
                let taper = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                taper_mode = TaperMode::from_str(&taper)?;
            } else if flag_val == "-crown_width" {
                cross_defaults.crown_width = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-shoulder_width" {
                cross_defaults.shoulder_width = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-shoulder_slope" {
                cross_defaults.shoulder_slope = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            } else if flag_val == "-backslope_angle" {
                cross_defaults.backslope_angle = if keyval {
                    vec[1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                } else {
                    args[i + 1]
                        .to_string()
                        .parse::<f64>()
                        .expect(&format!("Error parsing {}", flag_val))
                };
            }
        }

        if dem_file.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "Input DEM not specified."));
        }
        if roads_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input roads vector not specified.",
            ));
        }
        if output_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Output raster file not specified.",
            ));
        }

        if road_width <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "road_width must be greater than 0.",
            ));
        }
        if height < 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "height must be >= 0.",
            ));
        }
        if margin < 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "margin must be >= 0.",
            ));
        }
        if let Some(radius) = search_radius {
            if radius <= 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "search_radius must be greater than 0.",
                ));
            }
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28);
            println!("{}", "*".repeat(welcome_len));
            println!(
                "* Welcome to {} {}*",
                tool_name,
                " ".repeat(welcome_len - 15 - tool_name.len())
            );
            println!(
                "* Powered by WhiteboxTools {}*",
                " ".repeat(welcome_len - 28)
            );
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();
        if !dem_file.contains(&sep) && !dem_file.contains('/') {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !roads_file.contains(&sep) && !roads_file.contains('/') {
            roads_file = format!("{}{}", working_directory, roads_file);
        }
        if !output_file.contains(&sep) && !output_file.contains('/') {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading DEM...");
        }
        let dem = Raster::new(&dem_file, "r")?;
        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;
        let grid_res = ((dem.configs.resolution_x.abs() + dem.configs.resolution_y.abs()) / 2.0)
            .max(1e-9);
        let dem_epsg = infer_dem_epsg(&dem);

        if verbose {
            println!("Reading roads vector...");
        }
        let mut road_network = read_roads(&roads_file)?;
        let reproj_summary = maybe_reproject_roads_to_dem(&mut road_network, dem_epsg, verbose)?;

        if road_network.raw_features.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "No valid line features were found in roads input.",
            ));
        }

        let width_field_opt = if width_field.trim().is_empty() {
            None
        } else {
            Some(width_field.trim().to_lowercase())
        };

        let start = Instant::now();

        let mut resolution_stats = ResolutionStats::default();
        let (feature_configs, road_parts) = build_feature_configs_and_parts(
            &road_network,
            &width_field_opt,
            road_width,
            search_radius,
            grid_res,
            strategy,
            taper_mode,
            height,
            margin,
            &cross_defaults,
            &mut resolution_stats,
        )?;

        let max_influence_radius = feature_configs
            .iter()
            .fold(0.0f64, |acc, cfg| acc.max(cfg.influence_radius));

        if max_influence_radius <= 0.0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Resolved influence radius is zero; check width and cross-section settings.",
            ));
        }

        let mut frs: FixedRadiusSearch2D<i32> =
            FixedRadiusSearch2D::new(max_influence_radius, DistanceMetric::Euclidean);
        let sample_spacing = (grid_res * 0.5).max(0.1);

        if verbose {
            println!("Sampling road centerlines...");
        }

        let mut sampled_points = 0usize;
        let mut invalid_part_count = 0usize;
        let total_parts = road_parts.len().max(1);
        let mut old_progress = 1usize;

        for (part_index, part) in road_parts.iter().enumerate() {
            if part.points.len() < 2 {
                invalid_part_count += 1;
                continue;
            }

            for i in 0..part.points.len() - 1 {
                let (x1, y1) = part.points[i];
                let (x2, y2) = part.points[i + 1];
                let dx = x2 - x1;
                let dy = y2 - y1;
                let seg_len = (dx * dx + dy * dy).sqrt();

                if seg_len <= 0.0 {
                    frs.insert(x1, y1, part.feature_idx as i32);
                    sampled_points += 1;
                    continue;
                }

                let steps = (seg_len / sample_spacing).ceil() as usize;
                let num_steps = steps.max(1);
                for step in 0..=num_steps {
                    let t = step as f64 / num_steps as f64;
                    let x = x1 + t * dx;
                    let y = y1 + t * dy;
                    frs.insert(x, y, part.feature_idx as i32);
                    sampled_points += 1;
                }
            }

            if verbose {
                let progress = progress_percent(part_index, total_parts);
                if progress != old_progress {
                    println!("Sampling road centerlines: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if sampled_points == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "No valid road geometry could be sampled.",
            ));
        }

        let mut nearest_feature: Array2D<i32> = Array2D::new(rows, columns, -1, -1)?;
        let mut nearest_distance: Array2D<f64> =
            Array2D::new(rows, columns, f64::INFINITY, f64::INFINITY)?;
        let mut feature_overlap_cells = vec![0usize; feature_configs.len()];

        if verbose {
            println!("Resolving nearest roads...");
        }
        old_progress = 1;

        for row in 0..rows {
            for col in 0..columns {
                let z = dem.get_value(row, col);
                if z == nodata {
                    continue;
                }

                let x = dem.get_x_from_column(col);
                let y = dem.get_y_from_row(row);
                let neighbours = frs.search(x, y);
                if neighbours.is_empty() {
                    continue;
                }

                let mut best_dist = f64::INFINITY;
                let mut best_feature = -1i32;

                for (fid, dist) in neighbours {
                    if fid < 0 {
                        continue;
                    }
                    let feature = &feature_configs[fid as usize];
                    if dist <= feature.influence_radius && dist < best_dist {
                        best_dist = dist;
                        best_feature = fid;
                    }
                }

                if best_feature >= 0 {
                    nearest_feature.set_value(row, col, best_feature);
                    nearest_distance.set_value(row, col, best_dist);
                    feature_overlap_cells[best_feature as usize] += 1;
                }
            }

            if verbose {
                let progress = progress_percent(row as usize, rows as usize);
                if progress != old_progress {
                    println!("Resolving nearest roads: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let overlapping_features = feature_overlap_cells.iter().filter(|&&v| v > 0).count();

        let mut output = Raster::initialize_using_file(&output_file, &dem);
        output.set_data_from_raster(&dem)?;

        if verbose {
            println!("Applying raise strategy '{}'...", strategy.as_str());
        }
        old_progress = 1;

        let mut modified_cells = 0usize;
        let mut max_raise = 0.0f64;
        let mut total_raise = 0.0f64;
        let mut cross_fallback_cells = 0usize;

        for row in 0..rows {
            for col in 0..columns {
                let z = dem.get_value(row, col);
                if z == nodata {
                    continue;
                }

                let fid = nearest_feature.get_value(row, col);
                if fid < 0 {
                    continue;
                }

                let feature = &feature_configs[fid as usize];
                let dist = nearest_distance.get_value(row, col);
                if dist > feature.influence_radius {
                    continue;
                }

                let mut candidate: f64;

                match strategy {
                    Strategy::Constant => {
                        let weight = taper_weight(taper_mode, dist, feature.influence_radius);
                        candidate = z + height.max(0.0) * weight;
                    }
                    Strategy::ProfileRelative => {
                        let local_max = local_max_within_radius(
                            &dem,
                            row,
                            col,
                            feature.search_radius_cells,
                            nodata,
                            grid_res,
                        );
                        let weight = taper_weight(taper_mode, dist, feature.influence_radius);
                        let target = local_max + margin;
                        let raise = (target - z).max(0.0);
                        candidate = z + raise * weight;
                    }
                    Strategy::CrossSection => {
                        match feature.cross_section_mode {
                            CrossSectionMode::FallbackProfileRelative => {
                                let local_max = local_max_within_radius(
                                    &dem,
                                    row,
                                    col,
                                    feature.search_radius_cells,
                                    nodata,
                                    grid_res,
                                );
                                let weight = taper_weight(
                                    taper_mode,
                                    dist,
                                    feature.influence_radius.max(grid_res),
                                );
                                let target = local_max + margin;
                                let raise = (target - z).max(0.0);
                                candidate = z + raise * weight;
                                cross_fallback_cells += 1;
                            }
                            _ => {
                                let raise = cross_section_raise(feature, dist).unwrap_or(0.0);
                                candidate = z + raise;
                            }
                        }
                    }
                }

                if candidate < z {
                    candidate = z;
                }

                let delta = candidate - z;
                if delta > 0.0 {
                    output.set_value(row, col, candidate);
                    modified_cells += 1;
                    total_raise += delta;
                    if delta > max_raise {
                        max_raise = delta;
                    }
                }
            }

            if verbose {
                let progress = progress_percent(row as usize, rows as usize);
                if progress != old_progress {
                    println!("Applying raise strategy: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        output.add_metadata_entry(format!(
            "Created by whitebox_tools' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input DEM: {}", dem_file));
        output.add_metadata_entry(format!("Input roads: {}", roads_file));
        output.add_metadata_entry(format!(
            "Road source CRS: {}",
            if road_network.source_crs_note.is_empty() {
                "unknown".to_string()
            } else {
                road_network.source_crs_note.clone()
            }
        ));
        output.add_metadata_entry(format!(
            "Road source EPSG: {}",
            reproj_summary
                .source_epsg
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ));
        output.add_metadata_entry(format!(
            "DEM EPSG: {}",
            reproj_summary
                .target_epsg
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ));
        output.add_metadata_entry(format!("Road reprojection applied: {}", reproj_summary.reprojected));
        output.add_metadata_entry(format!(
            "Road points transformed: {}",
            reproj_summary.transformed_points
        ));
        output.add_metadata_entry(format!("Strategy: {}", strategy.as_str()));
        output.add_metadata_entry(format!("Taper mode: {}", taper_mode.as_str()));
        output.add_metadata_entry(format!("Default road width: {}", road_width));
        output.add_metadata_entry(format!("Width field: {}", width_field));
        output.add_metadata_entry(format!("Height: {}", height));
        output.add_metadata_entry(format!("Margin: {}", margin));
        output.add_metadata_entry(format!(
            "Search radius override: {}",
            search_radius
                .map(|v| v.to_string())
                .unwrap_or_else(|| "auto".to_string())
        ));
        output.add_metadata_entry(format!("Road features loaded: {}", feature_configs.len()));
        output.add_metadata_entry(format!("Road parts loaded: {}", road_parts.len()));
        output.add_metadata_entry(format!(
            "Road parts skipped (invalid): {}",
            invalid_part_count
        ));
        output.add_metadata_entry(format!("Road sampled points: {}", sampled_points));
        output.add_metadata_entry(format!(
            "Overlapping road features: {}",
            overlapping_features
        ));
        output.add_metadata_entry(format!("Cells modified: {}", modified_cells));
        output.add_metadata_entry(format!("Max raise: {}", max_raise));
        output.add_metadata_entry(format!("Total raise: {}", total_raise));
        output.add_metadata_entry(format!(
            "Width source counts (field, heuristic, default): {}, {}, {}",
            resolution_stats.width_from_field_count,
            resolution_stats.width_from_heuristic_count,
            resolution_stats.width_from_default_count
        ));
        output.add_metadata_entry(format!(
            "Cross-section feature counts (direct, conservative, fallback): {}, {}, {}",
            resolution_stats.cross_direct_count,
            resolution_stats.cross_conservative_count,
            resolution_stats.cross_fallback_count
        ));
        output.add_metadata_entry(format!(
            "Cross-section fallback cells: {}",
            cross_fallback_cells
        ));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...");
        }

        let _ = match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written");
                }
            }
            Err(e) => return Err(e),
        };

        if overlapping_features == 0 && verbose {
            println!(
                "Warning: No roads intersected the DEM extent. Check that raster and vector inputs share a common projection and location."
            );
        }

        if verbose {
            println!("{}", format!("Elapsed Time (excluding I/O): {}", elapsed_time));
        }

        Ok(())
    }
}

fn read_roads(roads_file: &str) -> Result<RoadNetwork, Error> {
    let lower = roads_file.to_lowercase();
    if lower.ends_with(".shp") {
        return read_roads_shapefile(roads_file);
    }
    if lower.ends_with(".geojson") || lower.ends_with(".json") {
        return read_roads_geojson(roads_file);
    }

    read_roads_shapefile(roads_file)
}

fn read_roads_shapefile(roads_file: &str) -> Result<RoadNetwork, Error> {
    let roads = Shapefile::read(roads_file)?;

    if roads.header.shape_type.base_shape_type() != ShapeType::PolyLine {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "The input roads data must be of polyline base shape type.",
        ));
    }

    let mut raw_features = vec![];

    for record_num in 0..roads.num_records {
        let record = roads.get_record(record_num);
        let attrs = attributes_from_shapefile_record(&roads, record_num);

        let mut parts = vec![];
        for part in 0..record.num_parts as usize {
            let start_point = record.parts[part] as usize;
            let end_point = if part < record.num_parts as usize - 1 {
                record.parts[part + 1] as usize - 1
            } else {
                record.num_points as usize - 1
            };

            let mut points = vec![];
            for i in start_point..=end_point {
                points.push((record.points[i].x, record.points[i].y));
            }

            if points.len() >= 2 {
                parts.push(points);
            }
        }

        if !parts.is_empty() {
            raw_features.push(RawFeature { attrs, parts });
        }
    }

    let mut source_epsg = infer_epsg_from_projection_text(&roads.projection);
    let mut source_crs_note = if let Some(epsg) = source_epsg {
        format!("EPSG:{}", epsg)
    } else if !roads.projection.trim().is_empty() {
        "projection WKT detected (EPSG unresolved)".to_string()
    } else {
        String::new()
    };

    if source_epsg.is_none() {
        if let Some((min_x, min_y, max_x, max_y)) = feature_bounds(&raw_features) {
            if looks_like_lon_lat_bounds(min_x, min_y, max_x, max_y) {
                source_epsg = Some(4326);
                source_crs_note = "EPSG:4326 (inferred from lon/lat coordinate ranges)".to_string();
            }
        }
    }

    Ok(RoadNetwork {
        raw_features,
        source_epsg,
        source_crs_note,
    })
}

fn attributes_from_shapefile_record(roads: &Shapefile, record_num: usize) -> FeatureAttributes {
    let mut attrs = FeatureAttributes::new();
    let record = roads.attributes.get_record(record_num);

    for (idx, value) in record.iter().enumerate() {
        let field_name = roads
            .attributes
            .get_field(idx)
            .name
            .to_string()
            .to_lowercase();

        let value_string = match value {
            FieldData::Int(v) => v.to_string(),
            FieldData::Real(v) => v.to_string(),
            FieldData::Text(v) => v.to_string(),
            FieldData::Date(v) => v.to_string(),
            FieldData::Bool(v) => v.to_string(),
            FieldData::Null => String::new(),
        };

        if !value_string.is_empty() {
            attrs.insert(&field_name, value_string);
        }
    }

    attrs
}

fn read_roads_geojson(roads_file: &str) -> Result<RoadNetwork, Error> {
    let geojson_str = fs::read_to_string(roads_file)?;
    let root_json: JsonValue = serde_json::from_str(&geojson_str)
        .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
    let mut source_epsg = infer_geojson_source_epsg(&root_json);
    let mut source_crs_note = source_epsg
        .map(|epsg| format!("EPSG:{}", epsg))
        .unwrap_or_else(String::new);

    let gj: GeoJson = geojson_str
        .parse()
        .map_err(|err| Error::new(ErrorKind::InvalidData, err))?;

    let mut raw_features = vec![];

    match gj {
        GeoJson::FeatureCollection(fc) => {
            for feature in fc.features {
                let mut attrs = FeatureAttributes::new();
                if let Some(props) = feature.properties {
                    for (key, value) in props {
                        let lower_key = key.to_lowercase();
                        match value {
                            JsonValue::String(v) => attrs.insert(&lower_key, v),
                            JsonValue::Number(v) => attrs.insert(&lower_key, v.to_string()),
                            JsonValue::Bool(v) => attrs.insert(&lower_key, v.to_string()),
                            JsonValue::Null => {}
                            _ => attrs.insert(&lower_key, value.to_string()),
                        }
                    }
                }

                if let Some(Geometry { value, .. }) = feature.geometry {
                    let mut parts: Vec<Vec<(f64, f64)>> = vec![];
                    match value {
                        GeoValue::LineString(coords) => {
                            let mut points = vec![];
                            for coord in coords {
                                if coord.len() >= 2 {
                                    points.push((coord[0], coord[1]));
                                }
                            }
                            if points.len() >= 2 {
                                parts.push(points);
                            }
                        }
                        GeoValue::MultiLineString(lines) => {
                            for line in lines {
                                let mut points = vec![];
                                for coord in line {
                                    if coord.len() >= 2 {
                                        points.push((coord[0], coord[1]));
                                    }
                                }
                                if points.len() >= 2 {
                                    parts.push(points);
                                }
                            }
                        }
                        _ => {
                            continue;
                        }
                    }

                    if !parts.is_empty() {
                        raw_features.push(RawFeature { attrs, parts });
                    }
                }
            }
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "GeoJSON roads input must be a FeatureCollection of LineString or MultiLineString features.",
            ));
        }
    }

    if source_epsg.is_none() {
        if let Some((min_x, min_y, max_x, max_y)) = feature_bounds(&raw_features) {
            if looks_like_lon_lat_bounds(min_x, min_y, max_x, max_y) {
                source_epsg = Some(4326);
                source_crs_note = "EPSG:4326 (inferred from lon/lat coordinate ranges)".to_string();
            }
        }
    }

    Ok(RoadNetwork {
        raw_features,
        source_epsg,
        source_crs_note,
    })
}

fn infer_dem_epsg(dem: &Raster) -> Option<u16> {
    if dem.configs.epsg_code > 0 {
        return Some(dem.configs.epsg_code);
    }

    parse_last_epsg_code(&dem.configs.coordinate_ref_system_wkt)
        .or_else(|| parse_last_epsg_code(&dem.configs.projection))
}

fn maybe_reproject_roads_to_dem(
    road_network: &mut RoadNetwork,
    dem_epsg: Option<u16>,
    verbose: bool,
) -> Result<ReprojectionSummary, Error> {
    let mut summary = ReprojectionSummary {
        source_epsg: road_network.source_epsg,
        target_epsg: dem_epsg,
        ..Default::default()
    };

    let source_epsg = match road_network.source_epsg {
        Some(v) => v,
        None => return Ok(summary),
    };
    let target_epsg = match dem_epsg {
        Some(v) => v,
        None => return Ok(summary),
    };

    if source_epsg == target_epsg {
        return Ok(summary);
    }

    let source_crs = format!("EPSG:{}", source_epsg);
    let target_crs = format!("EPSG:{}", target_epsg);
    let projector = Proj::new_known_crs(&source_crs, &target_crs, None).map_err(|err| {
        Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Unable to initialize CRS transformation from {} to {}: {}",
                source_crs, target_crs, err
            ),
        )
    })?;

    for feature in &mut road_network.raw_features {
        for part in &mut feature.parts {
            for point in part.iter_mut() {
                let (src_x, src_y) = *point;
                let transformed = projector.convert((src_x, src_y)).map_err(|err| {
                    Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Failed transforming road coordinate ({}, {}) from {} to {}: {}",
                            src_x, src_y, source_crs, target_crs, err
                        ),
                    )
                })?;
                if !transformed.0.is_finite() || !transformed.1.is_finite() {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!(
                            "Non-finite transformed coordinate ({}, {}) from source ({}, {}) while converting {} to {}.",
                            transformed.0, transformed.1, src_x, src_y, source_crs, target_crs
                        ),
                    ));
                }
                *point = transformed;
                summary.transformed_points += 1;
            }
        }
    }

    summary.reprojected = true;
    road_network.source_epsg = Some(target_epsg);
    road_network.source_crs_note = format!("EPSG:{} (reprojected from EPSG:{})", target_epsg, source_epsg);

    if verbose {
        println!(
            "Reprojected roads from {} to {} ({} vertices).",
            source_crs, target_crs, summary.transformed_points
        );
    }

    Ok(summary)
}

fn infer_geojson_source_epsg(root_json: &JsonValue) -> Option<u16> {
    let crs_name = root_json
        .get("crs")
        .and_then(|crs| crs.get("properties"))
        .and_then(|props| props.get("name"))
        .and_then(|name| name.as_str())?;

    let upper = crs_name.to_uppercase();
    if upper.contains("CRS84") {
        return Some(4326);
    }

    parse_last_epsg_code(crs_name)
}

fn infer_epsg_from_projection_text(projection: &str) -> Option<u16> {
    if projection.trim().is_empty() {
        return None;
    }

    let upper = projection.to_uppercase();
    if upper.contains("CRS84") {
        return Some(4326);
    }

    parse_last_epsg_code(projection).or_else(|| infer_utm_epsg_from_projection_text(&upper))
}

fn parse_last_epsg_code(text: &str) -> Option<u16> {
    let upper = text.to_uppercase();
    let bytes = upper.as_bytes();
    let mut idx = 0usize;
    let mut last_code: Option<u16> = None;

    while idx + 4 <= bytes.len() {
        if &bytes[idx..idx + 4] == b"EPSG" {
            let mut start = idx + 4;
            while start < bytes.len() && !bytes[start].is_ascii_digit() {
                start += 1;
            }
            let mut end = start;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end > start {
                if let Ok(code) = upper[start..end].parse::<u16>() {
                    if !is_known_non_crs_epsg(code) {
                        last_code = Some(code);
                    }
                }
                idx = end;
                continue;
            }
        }
        idx += 1;
    }

    last_code
}

fn is_known_non_crs_epsg(code: u16) -> bool {
    matches!(code, 9001 | 9002 | 9003 | 9101 | 9102 | 9122 | 9123 | 9124 | 9201)
}

fn infer_utm_epsg_from_projection_text(upper_projection: &str) -> Option<u16> {
    let (zone, is_northern) = extract_utm_zone_and_hemisphere(upper_projection)?;
    if zone == 0 || zone > 60 {
        return None;
    }

    if upper_projection.contains("NAD_1983")
        || upper_projection.contains("NAD83")
        || upper_projection.contains("NORTH_AMERICAN_1983")
    {
        return Some(26900 + zone);
    }

    if upper_projection.contains("WGS_1984")
        || upper_projection.contains("WGS 84")
        || upper_projection.contains("WGS_84")
    {
        return Some(if is_northern { 32600 + zone } else { 32700 + zone });
    }

    None
}

fn extract_utm_zone_and_hemisphere(upper_projection: &str) -> Option<(u16, bool)> {
    let zone_pos = upper_projection.find("ZONE")?;
    let tail = &upper_projection[zone_pos + 4..];

    let mut digit_start = None;
    for (i, ch) in tail.char_indices() {
        if ch.is_ascii_digit() {
            digit_start = Some(i);
            break;
        }
    }
    let digit_start = digit_start?;

    let mut digit_end = digit_start;
    for (i, ch) in tail[digit_start..].char_indices() {
        if !ch.is_ascii_digit() {
            break;
        }
        digit_end = digit_start + i + ch.len_utf8();
    }
    if digit_end <= digit_start {
        return None;
    }
    let zone: u16 = tail[digit_start..digit_end].parse().ok()?;

    let mut hemisphere_north = true;
    for ch in tail[digit_end..].chars().take(3) {
        if ch == 'N' {
            hemisphere_north = true;
            break;
        }
        if ch == 'S' {
            hemisphere_north = false;
            break;
        }
    }

    Some((zone, hemisphere_north))
}

fn looks_like_lon_lat_bounds(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> bool {
    min_x >= -180.0 && max_x <= 180.0 && min_y >= -90.0 && max_y <= 90.0
}

fn feature_bounds(raw_features: &[RawFeature]) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found_any = false;

    for feature in raw_features {
        for part in &feature.parts {
            for (x, y) in part {
                if *x < min_x {
                    min_x = *x;
                }
                if *x > max_x {
                    max_x = *x;
                }
                if *y < min_y {
                    min_y = *y;
                }
                if *y > max_y {
                    max_y = *y;
                }
                found_any = true;
            }
        }
    }

    if found_any {
        Some((min_x, min_y, max_x, max_y))
    } else {
        None
    }
}

fn build_feature_configs_and_parts(
    road_network: &RoadNetwork,
    width_field: &Option<String>,
    default_road_width: f64,
    search_radius_override: Option<f64>,
    grid_res: f64,
    strategy: Strategy,
    _taper_mode: TaperMode,
    height: f64,
    _margin: f64,
    cross_defaults: &CrossSectionParams,
    stats: &mut ResolutionStats,
) -> Result<(Vec<FeatureConfig>, Vec<FlattenedRoadPart>), Error> {
    let mut configs: Vec<FeatureConfig> = Vec::with_capacity(road_network.raw_features.len());
    let mut parts: Vec<FlattenedRoadPart> = vec![];

    for (feature_idx, raw_feature) in road_network.raw_features.iter().enumerate() {
        let (width, width_source) = resolve_feature_width(&raw_feature.attrs, width_field, default_road_width);
        if width_source == "width_field" || width_source == "width_auto_field" {
            stats.width_from_field_count += 1;
        } else if width_source == "heuristic" {
            stats.width_from_heuristic_count += 1;
        } else {
            stats.width_from_default_count += 1;
        }

        let resolved_search_radius =
            search_radius_override.unwrap_or_else(|| (width * 2.0).max(default_road_width));
        let search_radius_cells = ((resolved_search_radius / grid_res).ceil() as isize).max(1);

        let mut cross_params = cross_defaults.clone();
        apply_cross_section_overrides(&raw_feature.attrs, &mut cross_params);

        let is_unpaved = is_unpaved_feature(&raw_feature.attrs);
        let has_cross_overrides = raw_feature.attrs.has_any_key(&[
            "crown_width_m",
            "shoulder_width_m",
            "shoulder_slope",
            "backslope_angle_deg",
        ]);

        let mut cross_height = height;
        let mut cross_section_mode = CrossSectionMode::Direct;

        if strategy == Strategy::CrossSection {
            if !cross_params.is_valid() {
                cross_section_mode = CrossSectionMode::FallbackProfileRelative;
                stats.cross_fallback_count += 1;
                cross_params = cross_defaults.clone();
            } else if is_unpaved && !has_cross_overrides {
                cross_params = CrossSectionParams::conservative_unpaved();
                cross_height = height.min(CONSERVATIVE_CROSS_MAX_HEIGHT);
                cross_section_mode = CrossSectionMode::ConservativeUnpaved;
                stats.cross_conservative_count += 1;
            } else {
                stats.cross_direct_count += 1;
            }

            if !cross_params.is_valid() {
                cross_section_mode = CrossSectionMode::FallbackProfileRelative;
            }
        }

        let influence_radius = match strategy {
            Strategy::CrossSection => match cross_section_mode {
                CrossSectionMode::FallbackProfileRelative => (width * 0.5).max(grid_res),
                _ => cross_section_extent(&cross_params, cross_height, width).max(grid_res),
            },
            _ => (width * 0.5).max(grid_res),
        };

        configs.push(FeatureConfig {
            search_radius_cells,
            influence_radius,
            cross_section_mode,
            cross_params,
            cross_height,
        });

        for part in &raw_feature.parts {
            parts.push(FlattenedRoadPart {
                feature_idx,
                points: part.clone(),
            });
        }
    }

    if configs.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No valid road features were parsed from roads input.",
        ));
    }

    Ok((configs, parts))
}

fn resolve_feature_width(
    attrs: &FeatureAttributes,
    width_field: &Option<String>,
    default_road_width: f64,
) -> (f64, String) {
    if let Some(field_name) = width_field {
        if let Some(width) = attrs.get_number(field_name) {
            if width > 0.0 {
                return (width, "width_field".to_string());
            }
        }
    }

    if let Some(width) = attrs.get_number_from_candidates(&[
        "road_width",
        "width",
        "width_m",
        "roadwidth",
        "rd_width",
    ]) {
        if width > 0.0 {
            return (width, "width_auto_field".to_string());
        }
    }

    if let Some(width) = width_from_heuristic(attrs) {
        if width > 0.0 {
            return (width, "heuristic".to_string());
        }
    }

    (default_road_width.max(0.1), "default".to_string())
}

fn width_from_heuristic(attrs: &FeatureAttributes) -> Option<f64> {
    let class_text = attrs
        .get_text_from_candidates(&[
            "road_class",
            "class",
            "fclass",
            "type",
            "highway",
            "surface",
            "design",
            "condition",
        ])
        .unwrap_or_else(|| "".to_string())
        .to_lowercase();

    if class_text.is_empty() {
        return None;
    }

    if class_text.contains("motorway")
        || class_text.contains("trunk")
        || class_text.contains("primary")
        || class_text.contains("paved")
        || class_text.contains("asphalt")
        || class_text.contains("concrete")
    {
        return Some(8.0);
    }

    if class_text.contains("secondary")
        || class_text.contains("tertiary")
        || class_text.contains("collector")
        || class_text.contains("gravel")
        || class_text.contains("forest")
    {
        return Some(6.0);
    }

    if class_text.contains("dirt")
        || class_text.contains("track")
        || class_text.contains("trail")
        || class_text.contains("unpaved")
        || class_text.contains("inslope")
    {
        return Some(4.0);
    }

    None
}

fn is_unpaved_feature(attrs: &FeatureAttributes) -> bool {
    let text = attrs
        .get_text_from_candidates(&[
            "surface",
            "road_class",
            "class",
            "fclass",
            "type",
            "condition",
            "design",
        ])
        .unwrap_or_else(|| "".to_string())
        .to_lowercase();

    text.contains("dirt")
        || text.contains("gravel")
        || text.contains("track")
        || text.contains("trail")
        || text.contains("unpaved")
}

fn apply_cross_section_overrides(attrs: &FeatureAttributes, params: &mut CrossSectionParams) {
    if let Some(value) = attrs.get_number("crown_width_m") {
        params.crown_width = value;
    }
    if let Some(value) = attrs.get_number("shoulder_width_m") {
        params.shoulder_width = value;
    }
    if let Some(value) = attrs.get_number("shoulder_slope") {
        params.shoulder_slope = value;
    }
    if let Some(value) = attrs.get_number("backslope_angle_deg") {
        params.backslope_angle = value;
    }
}

fn cross_section_extent(params: &CrossSectionParams, height: f64, width: f64) -> f64 {
    let half_crown = params.crown_width * 0.5;
    let shoulder_outer = half_crown + params.shoulder_width;
    let shoulder_raise = (height - params.shoulder_slope * params.shoulder_width).max(0.0);
    let tan_back = params.backslope_angle.to_radians().tan();

    let profile_extent = if tan_back > 0.0 {
        shoulder_outer + shoulder_raise / tan_back
    } else {
        shoulder_outer
    };

    profile_extent.max(width * 0.5)
}

fn taper_weight(taper_mode: TaperMode, distance: f64, influence_radius: f64) -> f64 {
    if influence_radius <= 0.0 {
        return 0.0;
    }

    let normalized = (distance / influence_radius).clamp(0.0, 1.0);
    match taper_mode {
        TaperMode::None => 1.0,
        TaperMode::Linear => (1.0 - normalized).max(0.0),
        TaperMode::Cosine => 0.5 * (1.0 + (std::f64::consts::PI * normalized).cos()),
    }
}

fn local_max_within_radius(
    dem: &Raster,
    row: isize,
    col: isize,
    radius_cells: isize,
    nodata: f64,
    grid_res: f64,
) -> f64 {
    let mut local_max = dem.get_value(row, col);
    let mut found_any = false;

    let radius = radius_cells.max(1);
    let max_dist = radius as f64 * grid_res;

    for r in (row - radius)..=(row + radius) {
        for c in (col - radius)..=(col + radius) {
            let z = dem.get_value(r, c);
            if z == nodata {
                continue;
            }

            let dr = (r - row) as f64 * grid_res;
            let dc = (c - col) as f64 * grid_res;
            if (dr * dr + dc * dc).sqrt() <= max_dist {
                if !found_any {
                    local_max = z;
                    found_any = true;
                } else if z > local_max {
                    local_max = z;
                }
            }
        }
    }

    if found_any {
        local_max
    } else {
        dem.get_value(row, col)
    }
}

fn cross_section_raise(feature: &FeatureConfig, distance: f64) -> Option<f64> {
    let params = &feature.cross_params;
    if !params.is_valid() {
        return None;
    }

    let half_crown = params.crown_width * 0.5;
    let shoulder_outer = half_crown + params.shoulder_width;
    let tan_back = params.backslope_angle.to_radians().tan();
    if tan_back <= 0.0 {
        return None;
    }

    let shoulder_raise = (feature.cross_height - params.shoulder_slope * params.shoulder_width).max(0.0);

    if distance <= half_crown {
        return Some(feature.cross_height.max(0.0));
    }

    if distance <= shoulder_outer {
        let d = distance - half_crown;
        return Some((feature.cross_height - params.shoulder_slope * d).max(0.0));
    }

    let d = distance - shoulder_outer;
    Some((shoulder_raise - tan_back * d).max(0.0))
}

fn progress_percent(index: usize, total: usize) -> usize {
    if total <= 1 {
        100
    } else {
        (100.0_f64 * index as f64 / (total - 1) as f64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_taper_weight_bounds() {
        let w0 = taper_weight(TaperMode::Cosine, 0.0, 10.0);
        let w1 = taper_weight(TaperMode::Cosine, 10.0, 10.0);
        assert!((w0 - 1.0).abs() < 1e-9);
        assert!(w1.abs() < 1e-9);
    }

    #[test]
    fn test_width_heuristic_unpaved() {
        let mut attrs = FeatureAttributes::new();
        attrs.insert("surface", "Gravel".to_string());
        let width = width_from_heuristic(&attrs).unwrap();
        assert_eq!(width, 6.0);
    }

    #[test]
    fn test_cross_section_raise_non_negative() {
        let feature = FeatureConfig {
            search_radius_cells: 3,
            influence_radius: 8.0,
            cross_section_mode: CrossSectionMode::Direct,
            cross_params: CrossSectionParams::defaults(),
            cross_height: 2.0,
        };

        let raise = cross_section_raise(&feature, 100.0).unwrap();
        assert!(raise >= 0.0);
    }

    #[test]
    fn test_progress_percent_handles_singleton_total() {
        assert_eq!(progress_percent(0, 1), 100);
        assert_eq!(progress_percent(0, 5), 0);
        assert_eq!(progress_percent(4, 5), 100);
    }

    #[test]
    fn test_parse_last_epsg_code_prefers_most_specific_code() {
        let wkt = "PROJCS[\"WGS 84 / UTM zone 11N\",GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",4326]],AUTHORITY[\"EPSG\",32611]]";
        assert_eq!(parse_last_epsg_code(wkt), Some(32611));
    }

    #[test]
    fn test_parse_last_epsg_code_ignores_unit_codes() {
        let wkt = "PROJCS[\"WGS 84 / UTM zone 11N\",GEOGCS[\"WGS 84\",AUTHORITY[\"EPSG\",4326]],AUTHORITY[\"EPSG\",32611],UNIT[\"metre\",1,AUTHORITY[\"EPSG\",9001]]]";
        assert_eq!(parse_last_epsg_code(wkt), Some(32611));
    }

    #[test]
    fn test_infer_epsg_from_projection_text_nad83_utm() {
        let prj = "PROJCS[\"NAD_1983_UTM_Zone_10N\",GEOGCS[\"GCS_North_American_1983\"]]";
        assert_eq!(infer_epsg_from_projection_text(prj), Some(26910));
    }

    #[test]
    fn test_infer_geojson_source_epsg_crs84() {
        let root = serde_json::json!({
            "type": "FeatureCollection",
            "crs": {
                "type": "name",
                "properties": { "name": "urn:ogc:def:crs:OGC:1.3:CRS84" }
            },
            "features": []
        });
        assert_eq!(infer_geojson_source_epsg(&root), Some(4326));
    }

    #[test]
    fn test_lon_lat_bounds_detection() {
        assert!(looks_like_lon_lat_bounds(-123.0, 45.0, -122.0, 46.0));
        assert!(!looks_like_lon_lat_bounds(540000.0, 5020000.0, 541000.0, 5021000.0));
    }
}

#[cfg(test)]
#[path = "raise_roads_integration_tests.rs"]
mod raise_roads_integration_tests;
