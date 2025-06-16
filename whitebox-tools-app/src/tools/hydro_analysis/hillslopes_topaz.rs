/*
This tool implements Garbrecht & Martz TOPAZ-style channel & hillslope IDs for a single watershed.
Authors: Dr. Roger Lew
Created: 09/06/2025
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use whitebox_common::algorithms::calculate_rotation_degrees;
use whitebox_vector::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::fs::File;
use std::io::{self, Error, ErrorKind, Write};
use std::path;
use std::collections::VecDeque;
use geojson::{GeoJson, Geometry, Value};

/// This tool will identify the hillslopes associated with a user-specified stream network for a single catchment. Hillslopes
/// include the catchment areas draining to the left and right sides of each stream link in the network as well
/// as the catchment areas draining to all channel heads. `Hillslopes` are conceptually similar to `Subbasins`,
/// except that sub-basins do not distinguish between the right-bank and left-bank catchment areas of stream links.
/// The `Subbasins` tool simply assigns a unique identifier to each stream link in a stream network. Each hillslope
/// output by this tool is assigned a unique, positive identifier  value. All grid cells in the output raster that
/// coincide with a stream cell are assigned a non-zero idenitifier. 
///
/// The tool implements the TOPAZ-style channel and hillslope IDs with channels ending with "4" starting with 24, and hillslopes 
/// ending with 1 ("top"), 2 ("left"), or 3 ("right"). 
///
/// The user must specify the name of a flow pointer
/// (flow direction) raster (`--d8_pntr`), a streams raster (`--streams`), and the output raster (`--output`).
/// The flow pointer and streams rasters should be generated using the `D8Pointer` algorithm. This will require
/// a depressionless DEM, processed using either the `BreachDepressions` or `FillDepressions` tool.
///
/// By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools.
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// NoData values in the input flow pointer raster are assigned NoData values in the output image.
///
/// # See Also
/// `Hillslopes`, `StreamLinkIdentifier`, `Watershed`, `Subbasins`, `D8Pointer`, `BreachDepressions`, `FillDepressions`

/// Represents a channel link segment
struct Link {
    id: i32,
    topaz_id: i32,
    ds: (isize, isize),     // Downstream end coordinates
    us: (isize, isize),     // Upstream end coordinates
    inflow0_id: i32,        // Link index of first inflow
    inflow1_id: i32,        // Link index of second inflow
    inflow2_id: i32,        // Link index of third inflow
    length_m: f64,          // Channel length in meters
    ds_z: f64,              // Elevation at downstream end
    us_z: f64,              // Elevation at upstream end
    drop_m: f64,            // Elevation drop along channel
    order: u8,              // Stream order
    areaup: f64,            // Area upstream of the link in square meters
    is_headwater: bool,     // True for headwater links
    is_outlet: bool,        // True for outlet link
    path: Vec<(isize, isize)>, // Cells in the channel path from top to bottom
}

impl Link {
    fn new() -> Link {
        Link {
            id: -1,
            topaz_id: 0,
            ds: (-1, -1),
            us: (-1, -1),
            inflow0_id: -1,
            inflow1_id: -1,
            inflow2_id: -1,
            length_m: 0.0,
            ds_z: f64::NAN,
            us_z: f64::NAN,
            drop_m: f64::NAN,
            order: 0,
            areaup: 0.0,
            is_headwater: false,
            is_outlet: false,
            path: Vec::new(),
        }
    }
}

fn write_links_to_tsv(links: &[Link], file_path: &str) -> io::Result<()> {
    let mut file = File::create(file_path)?;
    
    // Write header
    writeln!(
        &mut file,
        "id\ttopaz_id\tds_x\tds_y\tus_x\tus_y\tinflow0_id\tinflow1_id\tinflow2_id\tlength_m\tds_z\tus_z\tdrop_m\torder\tareaup\tis_headwater\tis_outlet"
    )?;
    
    // Write each link
    for link in links {
        writeln!(
            &mut file,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}t{:.3}\t{}\t{}\t{}",
            link.id,
            link.topaz_id,
            link.ds.0,
            link.ds.1,
            link.us.0,
            link.us.1,
            link.inflow0_id,
            link.inflow1_id,
            link.inflow2_id,
            link.length_m,
            link.ds_z,
            link.us_z,
            link.drop_m,
            link.order,
            link.areaup,
            link.is_headwater,
            link.is_outlet
        )?;
    }
    
    Ok(())
}

pub struct HillslopesTopaz {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl HillslopesTopaz {
    pub fn new() -> HillslopesTopaz {
        let name = "HillslopesTopaz".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Implements TOPAZ-style channel & hillslope IDs for a single watershed".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["--dem".to_owned()],
            description: "Input filled or breached DEM raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input raster D8 pointer file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file (1=stream, 0=non-stream).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });


        parameters.push(ToolParameter {
            name: "Input Pour Points (Outlet) File".to_owned(),
            flags: vec!["--pour_pts".to_owned()],
            description: "Input pour points (outlet) file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::RasterAndVector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Watershed Mask File".to_owned(),
            flags: vec!["--watershed".to_owned()],
            description: "Input watershed mask raster file (1=inside, 0=outside).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Channel Junctions File".to_owned(),
            flags: vec!["--chnjnt".to_owned()],
            description: "Input channel junctions raster file (0=headwater, 1=mid-link, 2=junction).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Stream Order File (Optional)".to_owned(),
            flags: vec!["--order".to_owned()],
            description: "Input stream order raster file (optional but recommended).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output TOPAZ IDs File".to_owned(),
            flags: vec!["--subwta".to_owned()],
            description: "Output raster file for TOPAZ identifiers.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Network Table File".to_owned(),
            flags: vec!["--netw".to_owned()],
            description: "Output TSV file for channel network table.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Text),
            default_value: None,
            optional: false,
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=dem.tif --d8_pntr=d8.tif --streams=streams.tif --pour_pts=outlet.shp --watershed=basin.tif --chnjnt=junctions.tif --order=order.tif --subwta=subwta.tif --netw=netw.tsv", short_exe, name).replace("*", &sep);

        HillslopesTopaz {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
    
    /// Locate pour point from vector or raster input
    fn locate_pour_point(&self, pourpts_file: &str, pntr: &Raster) -> Result<(isize, isize), Error> {
        let mut pour_point = (-1, -1);
        let mut count = 0;
        
        if pourpts_file.to_lowercase().ends_with(".shp") {
            let pourpts = Shapefile::read(pourpts_file)?;
            if pourpts.header.shape_type.base_shape_type() != ShapeType::Point {
                return Err(Error::new(ErrorKind::InvalidInput, "Pour points must be point type"));
            }
            
            for i in 0..pourpts.num_records {
                let record = pourpts.get_record(i);
                let row = pntr.get_row_from_y(record.points[0].y);
                let col = pntr.get_column_from_x(record.points[0].x);
                pour_point = (row, col);
                count += 1;
            }
        } 
        else if pourpts_file.to_lowercase().ends_with(".geojson") || 
                pourpts_file.to_lowercase().ends_with(".json") {
            let geojson_str = std::fs::read_to_string(pourpts_file)?;
            let gj: GeoJson = geojson_str.parse().map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
            
            if let GeoJson::FeatureCollection(fc) = gj {
                for feature in fc.features {
                    if let Some(Geometry { value, .. }) = feature.geometry {
                        match value {
                            Value::Point(pt) => {
                                let (x, y) = (pt[0], pt[1]);
                                let row = pntr.get_row_from_y(y);
                                let col = pntr.get_column_from_x(x);
                                pour_point = (row, col);
                                count += 1;
                            }
                            Value::MultiPoint(pts) => {
                                for pt in pts {
                                    let (x, y) = (pt[0], pt[1]);
                                    let row = pntr.get_row_from_y(y);
                                    let col = pntr.get_column_from_x(x);
                                    pour_point = (row, col);
                                    count += 1;
                                }
                            }
                            _ => continue,
                        }
                    }
                }
            }
        } 
        else { // Raster
            let pourpts = Raster::new(pourpts_file, "r")?;
            if pourpts.configs.rows != pntr.configs.rows || pourpts.configs.columns != pntr.configs.columns {
                return Err(Error::new(ErrorKind::InvalidInput, "Pour points raster must match DEM dimensions"));
            }
            
            for row in 0..pntr.configs.rows as isize {
                for col in 0..pntr.configs.columns as isize {
                    if pourpts.get_value(row, col) > 0.0 && pourpts.get_value(row, col) != pourpts.configs.nodata {
                        pour_point = (row, col);
                        count += 1;
                    }
                }
            }
        }
        
        if count == 0 {
            Err(Error::new(ErrorKind::InvalidInput, "No pour points found"))
        } else if count > 1 {
            Err(Error::new(ErrorKind::InvalidInput, "Exactly one pour point required"))
        } else {
            Ok(pour_point)
        }
    }
}

impl WhiteboxTool for HillslopesTopaz {
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
        _working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {


        let start0 = Instant::now();

        // Parse command line arguments
        let mut dem_file = String::new();
        let mut d8_file = String::new();
        let mut streams_file = String::new();
        let mut pourpts_file = String::new();
        let mut watershed_file = String::new();
        let mut chnjnt_file = String::new();
        let mut order_file = String::new();
        let mut subwta_file = String::new();
        let mut netw_file = String::new();
        let mut esri_style = false;
        
        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-dem" {
                dem_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-d8_pntr" {
                d8_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-pour_pts" {
                pourpts_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-watershed" {
                watershed_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-chnjnt" {
                chnjnt_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-order" {
                order_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-subwta" {
                subwta_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-netw" {
                netw_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-esri_pntr" || flag_val == "-esri_style" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    esri_style = true;
                }
            }
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28); 
            // 28 = length of the 'Powered by' by statement.
            println!("{}", "*".repeat(welcome_len));
            println!("* Welcome to {} {}*", tool_name, " ".repeat(welcome_len - 15 - tool_name.len()));
            println!("* Powered by WhiteboxTools {}*", " ".repeat(welcome_len - 28));
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        if verbose {
            println!("Reading data...")
        };

        // Add working directory to file paths
        if verbose {
            println!("Reading {} file.", dem_file);
        }
        let dem = Raster::new(&dem_file, "r")?;

        if verbose {
            println!("Reading {} file.", d8_file);
        }   
        let d8_pntr = Raster::new(&d8_file, "r")?;

        if verbose {
            println!("Reading {} file.", streams_file);
        }
        let streams = Raster::new(&streams_file, "r")?;
        if verbose {
            println!("Reading {} file.", watershed_file);
        }
        let watershed = Raster::new(&watershed_file, "r")?;
        if verbose {
            println!("Reading {} file.", chnjnt_file);
        }
        let chnjnt = Raster::new(&chnjnt_file, "r")?;
        if verbose {
            println!("Reading {} file.", order_file);
        }
        let order = Raster::new(&order_file, "r")?;
        
        let start = Instant::now();

        if verbose {
            println!("Checking grid alignment.");
        }


        // Validate grid alignment
        if !rasters_share_geometry(&[&dem, &d8_pntr, &streams, &watershed, &chnjnt]) {
            return Err(Error::new(ErrorKind::InvalidInput, "Input rasters must share geometry"));
        }
        
        // Validate chnjnt values
        if verbose {
            println!("Checking channel junction map for 3 or more inflows.");
        }
        for row in 0..chnjnt.configs.rows as isize {
            for col in 0..chnjnt.configs.columns as isize {
                let val = chnjnt.get_value(row, col);

                // Limit the number of inflows to 3 or fewer
                // this is a requiremnent for WEPP watershed model
                if val != chnjnt.configs.nodata && val > 3.0 {
                    return Err(Error::new(ErrorKind::InvalidInput, "chnjnt values must be 0, 1, 2, or 3"));
                }
            }
        }

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let _nodata = dem.configs.nodata;
        let streams_nodata = streams.configs.nodata;
        let cellsize_x = dem.configs.resolution_x;
        let cellsize_y = dem.configs.resolution_y;
        let diag_cellsize = (cellsize_x * cellsize_x + cellsize_y * cellsize_y).sqrt();

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        
        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [usize; 129] = [8usize; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in dx and dy.
            pntr_matches[1] = 0;
            pntr_matches[2] = 1;
            pntr_matches[4] = 2;
            pntr_matches[8] = 3;
            pntr_matches[16] = 4;
            pntr_matches[32] = 5;
            pntr_matches[64] = 6;
            pntr_matches[128] = 7;
        } else {
            // This maps Esri-style D8 pointer values
            // onto the cell offsets in dx and dy.
            pntr_matches[1] = 1;
            pntr_matches[2] = 2;
            pntr_matches[4] = 3;
            pntr_matches[8] = 4;
            pntr_matches[16] = 5;
            pntr_matches[32] = 6;
            pntr_matches[64] = 7;
            pntr_matches[128] = 0;
        }

        // validate d8_pntr values
        // this avoids having to check after every direction check and reduces cyclomatic complexity
        if verbose {
            println!("Checking D8 pointer map for valid values.");
        }
        for row in 0..d8_pntr.configs.rows as isize {
            for col in 0..d8_pntr.configs.columns as isize {
                if watershed.get_value(row, col) == watershed.configs.nodata {
                    continue; // Skip cells outside watershed
                }
                let val = d8_pntr.get_value(row, col) as usize;
                if pntr_matches[val] >= 8 {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Invalid D8 pointer value {} at ({}, {})", val, row, col),
                    ));
                }
            }
        }

        // Locate pour point
        if verbose {
            println!("Locating pour point.");
        }
        let pour_point = self.locate_pour_point(&pourpts_file, &dem)?;
        if streams.get_value(pour_point.0, pour_point.1) <= 0.0 || 
           streams.get_value(pour_point.0, pour_point.1) == streams_nodata {
            return Err(Error::new(ErrorKind::InvalidInput, "Pour point must be on a stream cell"));
        }
        if watershed.get_value(pour_point.0, pour_point.1) <= 0.0 {
            return Err(Error::new(ErrorKind::InvalidInput, "Pour point must be within watershed"));
        }

        // Initialize output raster
        // whitebox_raster::Raster treats data as f64 then writes to the config.data_type
        if verbose {
            println!("Initializing output raster.");
        }
        
        let mut subwta = Raster::initialize_using_file(&subwta_file, &d8_pntr);
        subwta.configs.data_type = DataType::F32;
        subwta.configs.palette = "qual.plt".to_string();
        subwta.configs.photometric_interp = PhotometricInterpretation::Categorical;
        let low_value = f64::MIN;
        subwta.configs.nodata = low_value;
        subwta.reinitialize_values(low_value);

        if verbose {
            let elapsed = start0.elapsed();
            println!("Phase 0: Initialization including input data read in {:.2?}.", elapsed);
        }

        // Phase 1: Build links
        let start1 = Instant::now();

        let mut links = Vec::<Link>::new();

        // 1.1 Identify headwaters
        if verbose {
            println!("Finding headwaters.");
        }
        let mut headwaters = Vec::new();
        for row in 0..rows {
            for col in 0..columns {
                if chnjnt[(row, col)] == 0.0 && watershed[(row, col)] == 1.0 {
                    headwaters.push((row, col));
                }
            }
        }

        if verbose {
            println!("Found {} headwaters.", headwaters.len());
        }

        let mut link_id_grid = Array2D::new(rows, columns, -1i32, -1i32)?;

        // Walk down headwaters to identify links.
        if verbose {
            println!("Walk down headwaters to identify links.");
        }
        for hw in headwaters {
            // Skip if this headwater is already part of a link
            if link_id_grid[hw] != -1 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Headwater cell is already part of a link",
                ));
            }
            
            let mut link = Link::new();
            link.id = links.len() as i32; // Assign current link ID
            link.us = hw;  // set the upstream location
            link.is_headwater = true;
            
            let mut current = hw;
            loop {
                // push current cell to the link path
                link.path.push(current);

                // There are two break conditions:
                // 1. If we reach a cell that is already part of a link (link_id_grid[current] != -1)
                // 2. If we reach the pour point

                // Check if we're joining an existing link
                if link_id_grid[current] != -1 {

                    // validate it is a junction
                    if chnjnt[current] < 2.0 {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Current cell is not recognized as a junction",
                        ));
                    }
                    link.ds = current;
                    links.push(link);
                    break;
                }

                // Mark cell as part of this link
                // we would have broken out of the loop if if current was already part of a link
                link_id_grid[current] = link.id;
                
                // Check if we've reached the outlet
                if current == pour_point {
                    link.ds = current;
                    link.is_outlet = true;
                    links.push(link);
                    break;
                }
                
                // Check if we've reached a junction
                if current != hw && chnjnt[current] >= 2.0 {
                    link.ds = current;
                    links.push(link);
                    
                    // we now this hasn't been visited. create a new link and continue walking downstream.
                    // we would have returned if link_id_grid[current] != -1
                    link = Link::new();
                    link.id = links.len() as i32; // Assign current link ID
                    link.us = current; // set the upstream location
                    link.path.push(current);
                    link.is_headwater = false; // this is not a headwater link
                }
                
                // Move downstream
                let pntr_val = d8_pntr.get_value(current.0, current.1);
                let dir = pntr_val as usize;
                let c = pntr_matches[dir];
                let row_n = current.0 + dy[c];
                let col_n = current.1 + dx[c];
                
                // Check bounds
                if row_n < 0 || row_n >= rows || col_n < 0 || col_n >= columns {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Pointer direction leads outside raster bounds",
                    ));
                }
                
                // Check if next cell is in watershed
                if watershed[(row_n, col_n)] != 1.0 {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Pointer direction leads outside watershed",
                    ));
                }
                
                current = (row_n, col_n);                
            }
        }

        if verbose {
            let elapsed = start1.elapsed();
            println!("Phase 1: Identified {} links in {:.2?}.", links.len(), elapsed);
        }

        // Phase 2: Now that we have all links, establish their relationships
        let start2 = Instant::now();
        for i in 0..links.len() {
            if links[i].is_headwater {
                links[i].inflow0_id = -1;
                links[i].inflow1_id = -1;
                links[i].inflow2_id = -1;
                continue;
            }

            let us_end = links[i].us;
            
            // Find links that flow into this one
            let mut inflows = Vec::new();
            for j in 0..links.len() {
                if links[j].ds == us_end {
                    inflows.push(links[j].id);
                }

                if inflows.len() > 3 {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "Link has more than 3 inflows",
                    ));
                }
            }
            
            // Assign inflow IDs (up to 2)
            if inflows.len() > 0 {
                links[i].inflow0_id = inflows[0];
            }
            if inflows.len() > 1 {
                links[i].inflow1_id = inflows[1];
            }
            if inflows.len() > 2 {
                links[i].inflow2_id = inflows[2];
            }
        }

        // Calculate link lengths and drops
        for link in &mut links {

            // Calculate length
            let mut length = 0.0;
            for i in 1..link.path.len() {
                let (r1, c1) = link.path[i - 1];
                let (r2, c2) = link.path[i];
                
                if r1 == r2 || c1 == c2 {
                    length += if r1 == r2 { cellsize_x } else { cellsize_y };
                } else {
                    length += diag_cellsize;
                }
            }
            link.length_m = length;
            
            // Calculate elevation drop
            link.ds_z = dem.get_value(link.ds.0, link.ds.1);
            link.us_z = dem.get_value(link.us.0, link.us.1);
            link.drop_m = link.us_z - link.ds_z;
            
            // Set stream order if provided
            link.order = order.get_value(link.ds.0, link.ds.1) as u8;
        }

        if verbose {
            let elapsed = start2.elapsed();
            println!("Phase 2: Established link relationships in {:.2?}.", elapsed);
        }
        
        // Phase 3: Assign TOPAZ IDs (bottom-up traversal)
        let start3 = Instant::now();
        if verbose {
            println!("Assigning TOPAZ IDs to links.");
        }
        let mut next_id = 24; // Starting TOPAZ ID
                              // channel ids always end with 4 staring with 24

        let mut outlet_idx: i32 = -1; // Index of the outlet link
        for i in 1..links.len() {
            if links[i].is_outlet {
                outlet_idx = i as i32;
                links[i].topaz_id = next_id;
                next_id += 10;
                break;
            }
        }

        if outlet_idx == -1 {
            return Err(Error::new(ErrorKind::InvalidInput, "No outlet link found"));
        }

        // We walk up the channel network using a breadth-firest queue
        let mut queue = VecDeque::new();
        queue.push_back(outlet_idx as usize); // Start with outlet link

        while let Some(link_idx) = queue.pop_front() {
            // If this is a headwater link, skip to next iteration
            if links[link_idx].is_headwater {
                continue;
            }

            if links[link_idx].inflow0_id == -1 || links[link_idx].inflow1_id == -1 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Link does not have two inflows",
                ));
            }

            // the link ids and the indexes are the same
            // because the ids are assigned as links.len()
            let inflow0_id = links[link_idx].inflow0_id as usize;
            let inflow1_id = links[link_idx].inflow1_id as usize;
            
            let inflow0_angle = calculate_rotation_degrees(
                links[link_idx].ds.0 as f64, -links[link_idx].ds.1 as f64,     // a
                links[link_idx].us.0 as f64, -links[link_idx].us.1 as f64,     // o
                links[inflow0_id].us.0 as f64, -links[inflow0_id].us.1 as f64, // b
            );

            let inflow1_angle = calculate_rotation_degrees(
                links[link_idx].ds.0 as f64, -links[link_idx].ds.1 as f64,     // a
                links[link_idx].us.0 as f64, -links[link_idx].us.1 as f64,     // o
                links[inflow1_id].us.0 as f64, -links[inflow1_id].us.1 as f64, // b
            );

            // no third inflow
            if links[link_idx].inflow2_id == -1
            {
                // determien clockwise rotations of the inflows.
                // The lesser is numbered first
                // queue pops from the front, push the index in the
                // clockwise order of the inflows
                if inflow0_angle < inflow1_angle {
                    links[inflow0_id].topaz_id = next_id;
                    queue.push_back(inflow0_id as usize);
                    next_id += 10;  // channels are enumerated by 10s
                    links[inflow1_id].topaz_id = next_id;
                    queue.push_back(inflow1_id as usize);
                    next_id += 10;
                } else {
                    links[inflow1_id].topaz_id = next_id;
                    queue.push_back(inflow1_id as usize);
                    next_id += 10;
                    links[inflow0_id].topaz_id = next_id;
                    queue.push_back(inflow0_id as usize);
                    next_id += 10;
                }
            } else {
                // handle thrid inflow
                // aiming for maintainability here over succinctness
                let inflow2_id = links[link_idx].inflow2_id as usize;
                
                let inflow2_angle = calculate_rotation_degrees(
                    links[link_idx].ds.0 as f64, -links[link_idx].ds.1 as f64,     // a
                    links[link_idx].us.0 as f64, -links[link_idx].us.1 as f64,     // o
                    links[inflow2_id].us.0 as f64, -links[inflow2_id].us.1 as f64, // b
                );

                // Determine clockwise rotations of the inflows.
                // order them smallest to largest
                let mut inflows = vec![
                    (inflow0_id, inflow0_angle),
                    (inflow1_id, inflow1_angle),
                    (inflow2_id, inflow2_angle),
                ];
                inflows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

                // Assign TOPAZ IDs in clockwise order
                links[inflows[0].0].topaz_id = next_id;
                queue.push_back(inflows[0].0);
                next_id += 10; // channels are enumerated by 10s
                links[inflows[1].0].topaz_id = next_id;
                queue.push_back(inflows[1].0);
                next_id += 10;
                links[inflows[2].0].topaz_id = next_id;
                queue.push_back(inflows[2].0);
                next_id += 10;
            }
        }

        if verbose {
            let elapsed = start3.elapsed();
            println!("Phase 3: Assigned TOPAZ IDs in {:.2?}.", elapsed);
        }

        // Phase 4: Stamp channel topaz_ids in output raster
        let start4 = Instant::now();
        if verbose {
            println!("Stamping channels in output raster.");
        }

        for link in &links {
            let topaz_id = link.topaz_id as f64;
            if topaz_id <= 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Invalid TOPAZ ID {} for link {}", topaz_id, link.id),
                ));
            }

            for &(row, col) in &link.path {
                if (row, col) == link.ds && !link.is_outlet {
                    // pour points should not be stamped unless it is the outlet
                    continue;
                }
                subwta.set_value(row, col, topaz_id as f64);
            }
        }

        if verbose {
            let elapsed = start4.elapsed();
            println!("Phase 4: Stamped channels in output raster in {:.2?}.", elapsed);
        }
        
        // Phase 5: flood fill hillslope values
        let start5 = Instant::now();

        if verbose {
            println!("Flood filling hillslope values.");
        }
        for row in 0..rows {
            for col in 0..columns {
                // check if not in watershed
                if watershed[(row, col)] != 1.0 {
                    continue; 
                }
                
                // check if already labeled
                if subwta.get_value(row, col) != low_value {
                    continue;
                }
                
                // flood fill from this cell
                let mut current = (row, col);
                let mut found_topaz_id = 0.0;
                while found_topaz_id == 0.0 {
                    let dir_val = d8_pntr.get_value(current.0, current.1);
                    let dir = dir_val as usize;
                    let c = pntr_matches[dir];
                    let row_n = current.0 + dy[c];
                    let col_n = current.1 + dx[c];
                    
                    // Check bounds
                    if row_n < 0 || row_n >= rows || col_n < 0 || col_n >= columns {
                        break; // Out of bounds
                    }
                    
                    // Check if next cell is in watershed
                    if watershed[(row_n, col_n)] != 1.0 {
                        break; // left the watershed
                    }
                    
                    // Check for hillslope cell (ID-3)
                    if subwta[(row_n, col_n)] != low_value {
                        if subwta[(row_n, col_n)] <= 0.0 {
                            return Err(Error::new(
                                ErrorKind::InvalidInput,
                                format!("Invalid hillslope ID {} at ({}, {})", subwta[(row_n, col_n)], row_n, col_n),
                            ));
                        }

                        // found a hillslope cell
                        if subwta[(row_n, col_n)] % 10.0 <= 3.0 {
                            found_topaz_id = subwta[(row_n, col_n)];

                        // we know this is a channel cell is it a headwater pour point
                        } else if chnjnt[(row_n, col_n)] == 0.0 {
                            found_topaz_id = subwta[(row_n, col_n)] - 3.0;

                        // we know this is a channel cell that isn't a headwater pour point
                        } else { 
                            let topaz_id = subwta[(row_n, col_n)];

                            let dir_val = d8_pntr.get_value(row_n, col_n);
                            let dir = dir_val as usize;
                            let cn = pntr_matches[dir];

                            // direction of flow into channel
                            let vx = dx[c] as f64;
                            let vy = dy[c] as f64;

                            // direction of flow down channel
                            let ux = dx[cn] as f64;
                            let uy = dy[cn] as f64;

                            // Calculate cross product to determine side of flow
                            let cross = ux * vy - uy * vx;

                            if cross > 0.0 {
                                // test cell is on the “left” side of the flow
                                // ends with 2
                                found_topaz_id = topaz_id - 2.0;
                            } else if cross < 0.0 {
                                // test cell is on the “right” side of the flow
                                // ends with 3
                                found_topaz_id = topaz_id - 1.0;
                            } else {
                                // the hillslope drains in the same direction as the channel cell. 
                                // The cross product is ambiguous and can't be used to determine the side of the flow.
                                // So we need to look at the flow direciton of the upstream channel to determine the side of the hillslope
                                for i in 0..8 {
                                    let row_nn = row_n + dy[i];
                                    let col_nn = col_n + dx[i];
                                    if row_nn < 0 || row_nn >= rows || col_nn < 0 || col_nn >= columns {
                                        continue; // out of bounds
                                    }
                                    let dir_val = d8_pntr.get_value(row_nn, col_nn);
                                    let dir = dir_val as usize;
                                    let c_up = pntr_matches[dir];

                                    let up_chn_candidate_row = row_nn + dy[c_up];
                                    let up_chn_candidate_col = col_nn + dx[c_up];
                                    
                                    if up_chn_candidate_row == row_n && 
                                    up_chn_candidate_col == col_n && 
                                    chnjnt.get_value(row_nn, col_nn) > 0.0 {
                                                    
                                        // direction of the flow down channel from the upstream channel cell
                                        let ux = dx[c_up] as f64;
                                        let uy = dy[c_up] as f64;

                                        // Calculate cross product to determine side of flow
                                        let cross = ux * vy - uy * vx;

                                        if cross > 0.0 {
                                            // test cell is on the “left” side of the flow
                                            found_topaz_id = topaz_id - 2.0;
                                        } else if cross < 0.0 {
                                            // test cell is on the “right” side of the flow
                                            found_topaz_id = topaz_id - 1.0;
                                        }
                                        break;
                                    }
                                }
                            }
                        } 
                    }
                    current = (row_n, col_n);
                }

                // If we reached a hillslope cell, walk back down and assign found_topaz_id value
                if found_topaz_id != 0.0 {
                    let mut backtrack = (row, col);
                    while backtrack != current {
                        if subwta[(backtrack.0, backtrack.1)] == low_value {
                            subwta.set_value(backtrack.0, backtrack.1, found_topaz_id);
                        }
                        let dir_val = d8_pntr.get_value(backtrack.0, backtrack.1);
                        let dir = dir_val as usize;
                        let c = pntr_matches[dir];
                        backtrack = (backtrack.0 + dy[c], backtrack.1 + dx[c]);
                    }
                }
            }
        }

        if verbose {
            let elapsed = start5.elapsed();
            println!("Phase 5: Flood filled hillslope values in {:.2?}.", elapsed);
        }


        // Phase 6: Calculate up area for each link
        let start6 = Instant::now();
        if verbose {
            println!("Calculating area for each link.");
        }

        for link in &mut links {
            let topaz_id = link.topaz_id;

            let mut count = 0;
            for i in 1..3 {
                let hill_id = topaz_id as f64 - i as f64;
                // find number of cells in subwta with hill_id
                for row in 0..rows {
                    for col in 0..columns {
                        if subwta[(row, col)] == hill_id {
                            count += 1;
                        }
                    }
                }
            }
            link.areaup = count as f64 * cellsize_x * cellsize_y; // area in m2
        }

        if verbose {
            let elapsed = start6.elapsed();
            println!("Phase 6: Calculated area for each link in {:.2?}.", elapsed);
        }


        // Write netw.tsv
        let start6 = Instant::now();
        if verbose {
            println!("Writing network links to {}.", netw_file);
        }
        write_links_to_tsv(&links, &netw_file)?;

        let elapsed_time = get_formatted_elapsed_time(start);
        subwta.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        subwta.add_metadata_entry(format!("D8 pointer file: {}", d8_file));
        subwta.add_metadata_entry(format!("Pour-points file: {}", pourpts_file));
        subwta.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match subwta.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };

        if verbose {
            let elapsed = start6.elapsed();
            println!("Phase 6: Write files {:.2?}.", elapsed);

            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

/// Check if all rasters share the same geometry
fn rasters_share_geometry(rasters: &[&Raster]) -> bool {
    if rasters.is_empty() {
        return true;
    }
    
    let base = &rasters[0].configs;
    for raster in rasters.iter().skip(1) {
        if raster.configs.rows != base.rows ||
           raster.configs.columns != base.columns ||
           raster.configs.north != base.north ||
           raster.configs.south != base.south ||
           raster.configs.east != base.east ||
           raster.configs.west != base.west ||
           raster.configs.resolution_x != base.resolution_x ||
           raster.configs.resolution_y != base.resolution_y {
            return false;
        }
    }
    true
}