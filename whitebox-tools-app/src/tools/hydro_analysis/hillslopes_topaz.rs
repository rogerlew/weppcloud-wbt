/*
This tool implements Garbrecht & Martz TOPAZ-style channel & hillslope IDs for a single watershed.
Authors: Dr. Roger Lew
Created: 09/06/2025
*/

use whitebox_raster::*;
use whitebox_common::structures::Array2D;
use whitebox_vector::*;
use crate::tools::*;
use std::env;
use std::f64;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path;
use std::collections::VecDeque;
use geojson::{GeoJson, Geometry, Value};

/// Represents a channel link segment
struct Link {
    id: i32,
    topaz_id: i32,
    pourpoint: (isize, isize),    // Downstream pourpoint coordinates
    ds: (isize, isize),     // Downstream end coordinates
    us: (isize, isize),     // Upstream end coordinates
    inflow0_id: i32,        // Link index of first inflow
    inflow1_id: i32,        // Link index of second inflow
    length_m: f64,          // Channel length in meters
    drop_m: f64,            // Elevation drop along channel
    order: u8,              // Stream order
    is_headwater: bool,     // True for headwater links
    is_outlet: bool,        // True for outlet link
    path: Vec<(isize, isize)>, // Cells in the channel path from bottom to top
}

impl Link {
    fn new() -> Link {
        Link {
            id: -1,
            topaz_id: 0,
            jnt: (-1, -1),
            ds: (-1, -1),
            us: (-1, -1),
            inflow0_id: -1,
            inflow1_id: -1,
            length_m: 0.0,
            drop_m: 0.0,
            order: 0,
            is_headwater: false,
            is_outlet: false,
            path: Vec::new(),
        }
    }
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
    
    /// Calculate flow vector between two points
    fn flow_vector(from: (isize, isize), to: (isize, isize)) -> (f64, f64) {
        let dx = (to.1 - from.1) as f64;
        let dy = (from.0 - to.0) as f64; // Inverted Y-axis
        let magnitude = (dx * dx + dy * dy).sqrt();
        if magnitude > 0.0 {
            (dx / magnitude, dy / magnitude)
        } else {
            (0.0, 0.0)
        }
    }
    
    /// Calculate angle between two vectors
    fn angle_between(v1: (f64, f64), v2: (f64, f64)) -> f64 {
        let dot = v1.0 * v2.0 + v1.1 * v2.1;
        let det = v1.0 * v2.1 - v1.1 * v2.0;
        det.atan2(dot).to_degrees()
    }
    
    /// Flood fill operation for headwater hillslopes
    fn flood_fill(
        start: (isize, isize), 
        value: f64, 
        output: &mut Raster, 
        watershed: &Raster,
        d8_pntr: &Raster,
        pntr_nodata: f64,
        pntr_matches: &[usize; 129]
    ) {
        let mut stack = vec![start];
        let rows = watershed.configs.rows as isize;
        let columns = watershed.configs.columns as isize;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        
        while let Some((row, col)) = stack.pop() {
            // Only process if in watershed and not already labeled
            if watershed[(row, col)] == 1.0 && output[(row, col)] == 0.0 {
                output[(row, col)] = value;
                
                // Add neighbors that flow into this cell
                for n in 0..8 {
                    let row_n = row + dy[n];
                    let col_n = col + dx[n];
                    
                    if row_n >= 0 && row_n < rows && col_n >= 0 && col_n < columns {
                        let pntr_val = d8_pntr[(row_n, col_n)];
                        if pntr_val != pntr_nodata {
                            let dir = pntr_val as usize;
                            if dir < 129 && pntr_matches[dir] == n {
                                stack.push((row_n, col_n));
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Residual fill for remaining hillslope cells
    fn residual_fill(
        output: &mut Raster,
        d8_pntr: &Raster,
        watershed: &Raster,
        pntr_nodata: f64,
        pntr_matches: &[usize; 129]
    ) {
        let rows = output.configs.rows as isize;
        let columns = output.configs.columns as isize;

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        
        for row in 0..rows {
            for col in 0..columns {
                if watershed[(row, col)] == 1.0 && output[(row, col)] == 0.0 {
                    let mut flag = false;
                    let (mut x, mut y) = (col, row);
                    let mut outlet_id = 0f64;
                    
                    // Trace flow path to find labeled cell
                    while !flag {
                        let dir_val = d8_pntr[(y, x)];
                        if dir_val != pntr_nodata {
                            let dir = dir_val as usize;
                            if dir < 129 {
                                let c = pntr_matches[dir];
                                y += dy[c];
                                x += dx[c];
                                
                                if output[(y, x)] > 0.0 {
                                    outlet_id = output[(y, x)];
                                    flag = true;
                                }
                            } else {
                                flag = true; // Invalid direction
                            }
                        } else {
                            flag = true; // Nodata
                        }
                    }
                    
                    // Back-fill the path
                    flag = false;
                    let (mut x2, mut y2) = (col, row);
                    output[(y2, x2)] = outlet_id;
                    
                    while !flag {
                        let dir_val = d8_pntr[(y2, x2)];
                        if dir_val != pntr_nodata {
                            let dir = dir_val as usize;
                            if dir < 129 {
                                let c = pntr_matches[dir];
                                y2 += dy[c];
                                x2 += dx[c];
                                
                                if output[(y2, x2)] > 0.0 {
                                    flag = true;
                                } else {
                                    output[(y2, x2)] = outlet_id;
                                }
                            } else {
                                flag = true;
                            }
                        } else {
                            flag = true;
                        }
                    }
                }
            }
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
            println!("Reading grids.");
        }

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
        
        if verbose {
            println!("Checking grid alignment.");
        }

        // Validate grid alignment
        if !rasters_share_geometry(&[&dem, &d8_pntr, &streams, &watershed, &chnjnt]) {
            return Err(Error::new(ErrorKind::InvalidInput, "Input rasters must share geometry"));
        }
        
        if verbose {
            println!("Checking channel junction map for 3 or more inflows.");
        }

        // Validate chnjnt values
        for row in 0..chnjnt.configs.rows as isize {
            for col in 0..chnjnt.configs.columns as isize {
                let val = chnjnt.get_value(row, col);
                if val != chnjnt.configs.nodata && val >= 3.0 {
                    return Err(Error::new(ErrorKind::InvalidInput, "chnjnt values must be 0, 1, or 2"));
                }
            }
        }

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;
        let pntr_nodata = d8_pntr.configs.nodata;
        let streams_nodata = streams.configs.nodata;
        let cellsize_x = dem.configs.resolution_x;
        let cellsize_y = dem.configs.resolution_y;
        let diag_cellsize = (cellsize_x * cellsize_x + cellsize_y * cellsize_y).sqrt();

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [usize; 129] = [0usize; 129];
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

        // Initialize output raster (u32 with nodata=0)
        // whitebox_raster::Raster treats data as f64 then writes to the config.data_type
        if verbose {
            println!("Initializing output raster.");
        }
        let mut output_config = dem.configs.clone();
        output_config.data_type = DataType::U32;
        let mut subwta = Raster::initialize_using_config(&subwta_file, &output_config);

        // Phase 1: Build channel tree (BFS from outlet)
        let mut links = Vec::<Link>::new();
        let mut queue = VecDeque::new();
        let mut visited: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut link_id_grid = Array2D::new(rows, columns, -1i32, -1i32)?;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        
        // Start at pour point
        queue.push_back(pour_point);
        visited[pour_point] = 1;
        let mut link_id = 0;

        while let Some(mut current) = queue.pop_front() {
            // Create new link
            let mut link = Link::new();
            link.id = link_id;
            link.ds = current;
            link.path.push(current);
            link.is_outlet = links.is_empty();
            
            // Trace upstream until junction or headwater
            loop {
                // Mark cell as part of this link
                link_id_grid[current] = link_id;
                
                // Check if we've reached a junction or headwater
                let jnt_val = chnjnt.get_value(current.0, current.1);
                if jnt_val == 0.0 || jnt_val == 2.0 {
                    link.us = current;
                    link.is_headwater = jnt_val == 0.0;
                    link.pourpoint = current;
                    break;
                }
                
                // Move upstream
                let mut found = false;
                let pntr_val = d8_pntr.get_value(current.0, current.1);
                if pntr_val != pntr_nodata {
                    let dir = pntr_val as usize;
                    if dir < 129 {
                        let c = pntr_matches[dir];
                        let row_n = current.0 + dy[c];
                        let col_n = current.1 + dx[c];
                        
                        if row_n >= 0 && row_n < rows && col_n >= 0 && col_n < columns {
                            // Only follow if not visited and in watershed
                            if visited[(row_n, col_n)] == 0 && watershed[(row_n, col_n)] == 1.0 {
                                current = (row_n, col_n);
                                visited[(row_n, col_n)] = 1;
                                link.path.push(current);
                                found = true;
                            }
                        }
                    }
                }
                
                if !found {
                    link.us = current;
                    link.is_headwater = true; // Premature end
                    break;
                }
            }
            
            // Add upstream inflows to queue (if junction)
            if chnjnt.get_value(link.us.0, link.us.1) == 2.0 {
                for n in 0..8 {
                    let row_n = link.us.0 + dy[n];
                    let col_n = link.us.1 + dx[n];
                    
                    if row_n >= 0 && row_n < rows && col_n >= 0 && col_n < columns {
                        // Check if flows into junction
                        if d8_pntr.get_value(row_n, col_n) != pntr_nodata {
                            let dir_val = d8_pntr.get_value(row_n, col_n) as usize;
                            if dir_val < 129 {
                                let dir = pntr_matches[dir_val];
                                let target_row = row_n + dy[dir];
                                let target_col = col_n + dx[dir];
                                
                                if target_row == link.us.0 && target_col == link.us.1 {
                                    if watershed[(row_n, col_n)] == 1.0 && visited[(row_n, col_n)] == 0 {
                                        queue.push_back((row_n, col_n));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            links.push(link);
            link_id += 1;
        }

        // Phase 2: Calculate link lengths and drops
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
            let ds_elev = dem.get_value(link.ds.0, link.ds.1);
            let us_elev = dem.get_value(link.us.0, link.us.1);
            if ds_elev != nodata && us_elev != nodata {
                link.drop_m = us_elev - ds_elev;
            }
            
            // Set stream order if provided
            link.order = order.get_value(link.ds.0, link.ds.1) as u8;
        }
        
        // Phase 3: Assign TOPAZ IDs (bottom-up traversal)// Phase 3: Assign TOPAZ IDs (bottom-up traversal)
        let mut next_id = 24; // Starting TOPAZ ID
        let mut queue = VecDeque::new();
        queue.push_back(0); // Start with outlet link

        while let Some(link_idx) = queue.pop_front() {
            // 1. Update current link first
            {
                let link = &mut links[link_idx];
                link.topaz_id = next_id;
                next_id += 10;
            } // Mutable borrow ends here
            
            // 2. Prepare data for children without holding references
            let mut inflows = Vec::new();
            for n in 0..8 {
                let row_n = links[link_idx].us.0 + dy[n];
                let col_n = links[link_idx].us.1 + dx[n];
                
                if row_n >= 0 && row_n < rows && col_n >= 0 && col_n < columns {
                    let inflow_id = link_id_grid[(row_n, col_n)];
                    if inflow_id >= 0 && inflow_id != links[link_idx].id {
                        inflows.push(inflow_id as usize);
                    }
                }
            }
            
            // 3. Process children if any exist
            if !inflows.is_empty() {
                if inflows.len() == 2 {
                    // Calculate parent vector once
                    let parent_vec = if links[link_idx].path.len() > 1 {
                        let last = links[link_idx].path.len() - 1;
                        HillslopesTopaz::flow_vector(links[link_idx].path[last - 1], links[link_idx].path[last])
                    } else {
                        let dir_val = d8_pntr.get_value(links[link_idx].us.0, links[link_idx].us.1) as usize;
                        if dir_val < 129 {
                            let c = pntr_matches[dir_val];
                            (dx[c] as f64, dy[c] as f64)
                        } else {
                            (0.0, 0.0)
                        }
                    };
                    
                    // Calculate child vectors without references
                    let mut child_angles = Vec::new();
                    for &inflow_id in &inflows {
                        let child_vec = if links[inflow_id].path.len() > 1 {
                            HillslopesTopaz::flow_vector(links[inflow_id].path[0], links[inflow_id].path[1])
                        } else {
                            let dir_val = d8_pntr.get_value(links[inflow_id].ds.0, links[inflow_id].ds.1) as usize;
                            if dir_val < 129 {
                                let c = pntr_matches[dir_val];
                                (dx[c] as f64, dy[c] as f64)
                            } else {
                                (0.0, 0.0)
                            }
                        };
                        
                        let angle = HillslopesTopaz::angle_between(parent_vec, child_vec);
                        child_angles.push((inflow_id, angle));
                    }
                    
                    // Order by angle
                    child_angles.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
                    let left_id = child_angles[0].0;
                    let right_id = child_angles[1].0;
                    
                    // Update children using indices directly
                    links[left_id].topaz_id = next_id;
                    links[right_id].topaz_id = next_id + 10;
                    
                    // Update current link's inflow references
                    {
                        let link = &mut links[link_idx];
                        link.inflow0_id = left_id as i32;
                        link.inflow1_id = right_id as i32;
                    }
                    
                    // Add to queue
                    queue.push_back(left_id);
                    queue.push_back(right_id);
                    next_id += 20;
                } else {
                    // Single inflow
                    let inflow_id = inflows[0];
                    links[inflow_id].topaz_id = next_id;
                    
                    {
                        let link = &mut links[link_idx];
                        link.inflow0_id = inflow_id as i32;
                    }
                    
                    queue.push_back(inflow_id);
                    next_id += 10;
                }
            }
        }

        // Phase 4: Stamp channels in output raster
        for link in &links {
            for &(row, col) in &link.path {
                if watershed[(row, col)] == 1.0 {
                    subwta[(row, col)] = link.topaz_id as f64;
                }
            }
        }
        
        // Phase 5: Headwater hillslopes (ID-3)
        for link in &links {
            if link.is_headwater {
                let value = (link.topaz_id - 3) as f64;
                HillslopesTopaz::flood_fill(
                    link.us, 
                    value, 
                    &mut subwta, 
                    &watershed,
                    &d8_pntr,
                    pntr_nodata,
                    &pntr_matches
                );
            }
        }
        
        // Phase 6: Side hillslopes (along channel buffers)
        let dx_f64 = [1.0, 1.0, 1.0, 0.0, -1.0, -1.0, -1.0, 0.0];
        let dy_f64 = [-1.0, 0.0, 1.0, 1.0, 1.0, 0.0, -1.0, -1.0];
        
        for link in &links {
            for i in 0..link.path.len() {
                let (row, col) = link.path[i];
                let topaz_id = link.topaz_id as u32;
                
                // Determine downstream direction
                let ds_vec = if i > 0 {
                    // Flow from previous cell to current
                    let prev = link.path[i - 1];
                    HillslopesTopaz::flow_vector(prev, (row, col))
                } else if link.path.len() > 1 {
                    // First cell - flow to next
                    let next = link.path[1];
                    HillslopesTopaz::flow_vector((row, col), next)
                } else {
                    // Single-cell link - use flow direction
                    let dir_val = d8_pntr.get_value(row, col) as usize;
                    if dir_val < 129 {
                        let c = pntr_matches[dir_val];
                        (dx_f64[c], dy_f64[c])
                    } else {
                        (0.0, 0.0)
                    }
                };
                
                // Check neighbors
                for n in 0..8 {
                    let row_n = row + dy[n] as isize;
                    let col_n = col + dx[n] as isize;
                    
                    if row_n >= 0 && row_n < rows && col_n >= 0 && col_n < columns {
                        if watershed[(row_n, col_n)] == 1.0 && 
                           subwta.get_value(row_n, col_n) == 0.0 {
                            // Calculate angle between downstream vector and neighbor vector
                            let neighbor_vec = (dx_f64[n], dy_f64[n]);
                            let mut angle = HillslopesTopaz::angle_between(ds_vec, neighbor_vec);
                            
                            // Normalize to 0-360
                            if angle < 0.0 {
                                angle += 360.0;
                            }
                            
                            // Assign left (ID-2) or right (ID-1)
                            let value = if angle < 180.0 {
                                (topaz_id - 2) as f64
                            } else {
                                (topaz_id - 1) as f64
                            };
                            
                            subwta[(row_n, col_n)] = value;
                        }
                    }
                }
            }
        }
        
        // Phase 7: Residual fill for remaining hillslope cells
        HillslopesTopaz::residual_fill(
            &mut subwta,
            &d8_pntr,
            &watershed,
            pntr_nodata,
            &pntr_matches
        );
        
        // Phase 8: Write outputs
        // Write subwta raster
        subwta.add_metadata_entry(format!("Created by whitebox_tools' {}", self.get_tool_name()));
        subwta.write()?;
        
        // Write netw.tsv
        let mut tsv = File::create(netw_file)?;
        writeln!(tsv, "id\ttopaz_id\tjnt_row\tjnt_col\tds_row\tds_col\tus_row\tus_col\tinflow0_id\tinflow1_id\tlength_m\tdrop_m\torder\tis_headwater\tis_outlet")?;
        
        for link in &links {
            writeln!(tsv, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{:.2}\t{}\t{}\t{}",
                link.id,
                link.topaz_id,
                link.pourpoint.0, link.pourpoint.1,
                link.ds.0, link.ds.1,
                link.us.0, link.us.1,
                link.inflow0_id, link.inflow1_id,
                link.length_m,
                link.drop_m,
                link.order,
                link.is_headwater as u8,
                link.is_outlet as u8
            )?;
        }

        if verbose {
            println!("TOPAZ hillslope identification complete");
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