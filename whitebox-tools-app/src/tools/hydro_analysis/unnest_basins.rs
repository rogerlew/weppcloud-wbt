/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. John Lindsay
Created: 27/04/2018
Last Modified: 18/10/2019
License: MIT
*/

use crate::tools::*;
use std::env;
use std::f64;
use std::fs::File;
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::path;
use whitebox_common::structures::Array2D;
use whitebox_raster::*;
use whitebox_vector::*;

const LOW_OUTLET_ID: i32 = i32::MIN;
const NODATA_OUTLET_ID: i32 = -1;

fn calculate_nesting_order(
    flow_dir: &Array2D<i8>,
    outlet_points: &Array2D<isize>,
    outlet_rows: &[isize],
    outlet_columns: &[isize],
    dx: &[isize; 8],
    dy: &[isize; 8],
) -> (Vec<usize>, usize) {
    let mut nesting_order = vec![0usize; outlet_rows.len()];
    let mut max_nesting_order = 1usize;
    let mut flag: bool;
    let mut cur_order: usize;
    let (mut x, mut y): (isize, isize);
    let mut dir: i8;
    let mut outlet: usize;
    let num_outlets = outlet_rows.len() - 1;

    for record_num in 0..num_outlets {
        outlet = record_num + 1;
        cur_order = 1;
        if nesting_order[outlet] < cur_order {
            nesting_order[outlet] = cur_order;
            flag = false;
            y = outlet_rows[outlet];
            x = outlet_columns[outlet];
            while !flag {
                dir = flow_dir.get_value(y, x);
                if dir >= 0 {
                    x += dx[dir as usize];
                    y += dy[dir as usize];
                    if outlet_points.get_value(y, x) > 0 {
                        outlet = outlet_points.get_value(y, x) as usize;
                        cur_order += 1;
                        if nesting_order[outlet] < cur_order {
                            nesting_order[outlet] = cur_order;
                            if cur_order > max_nesting_order {
                                max_nesting_order = cur_order;
                            }
                        } else {
                            flag = true;
                        }
                    }
                } else {
                    flag = true;
                }
            }
        }
    }

    (nesting_order, max_nesting_order)
}

fn calculate_parent_outlet(
    flow_dir: &Array2D<i8>,
    outlet_points: &Array2D<isize>,
    outlet_rows: &[isize],
    outlet_columns: &[isize],
    dx: &[isize; 8],
    dy: &[isize; 8],
) -> Vec<usize> {
    let mut parent_outlet = vec![0usize; outlet_rows.len()];
    let mut flag: bool;
    let mut dir: i8;
    let (mut x, mut y): (isize, isize);
    let num_outlets = outlet_rows.len() - 1;

    for outlet in 1..num_outlets + 1 {
        flag = false;
        y = outlet_rows[outlet];
        x = outlet_columns[outlet];
        while !flag {
            dir = flow_dir.get_value(y, x);
            if dir >= 0 {
                x += dx[dir as usize];
                y += dy[dir as usize];
                let downstream_outlet = outlet_points.get_value(y, x);
                if downstream_outlet > 0 {
                    let downstream_outlet = downstream_outlet as usize;
                    if downstream_outlet != outlet {
                        parent_outlet[outlet] = downstream_outlet;
                    }
                    flag = true;
                }
            } else {
                flag = true;
            }
        }
    }

    parent_outlet
}

fn calculate_child_outlets(parent_outlet: &[usize]) -> Vec<Vec<usize>> {
    let mut child_outlets = vec![Vec::<usize>::new(); parent_outlet.len()];
    let num_outlets = parent_outlet.len() - 1;

    for outlet in 1..num_outlets + 1 {
        if parent_outlet[outlet] > 0 {
            child_outlets[parent_outlet[outlet]].push(outlet);
        }
    }
    for outlet in 1..num_outlets + 1 {
        child_outlets[outlet].sort_unstable();
    }

    child_outlets
}

fn calculate_hierarchy_levels(parent_outlet: &[usize]) -> Vec<usize> {
    // Hierarchy level: 0 for root outlets (no downstream outlet), increasing upstream.
    let mut hierarchy_level = vec![0usize; parent_outlet.len()];
    let mut unresolved = vec![true; parent_outlet.len()];
    unresolved[0] = false;
    let num_outlets = parent_outlet.len() - 1;

    for outlet in 1..num_outlets + 1 {
        let mut chain = Vec::<usize>::new();
        let mut current = outlet;
        while current > 0 && unresolved[current] {
            chain.push(current);
            current = parent_outlet[current];
        }

        let mut level = if current > 0 {
            hierarchy_level[current] + 1
        } else {
            0
        };
        while let Some(node) = chain.pop() {
            hierarchy_level[node] = level;
            unresolved[node] = false;
            level += 1;
        }
    }

    hierarchy_level
}

fn write_hierarchy_sidecar(
    output_file: &str,
    parent_outlet: &[usize],
    child_outlets: &[Vec<usize>],
    nesting_order: &[usize],
    hierarchy_level: &[usize],
    outlet_rows: &[isize],
    outlet_columns: &[isize],
) -> Result<(), Error> {
    let output_path = path::Path::new(output_file);
    let output_stem = output_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Output file name is invalid."))?;
    let hierarchy_file = output_path.with_file_name(format!("{}_hierarchy.csv", output_stem));
    let hierarchy_f = File::create(&hierarchy_file)?;
    let mut hierarchy_writer = BufWriter::new(hierarchy_f);
    writeln!(
        hierarchy_writer,
        "outlet_id,parent_outlet_id,child_count,child_ids,nesting_order,hierarchy_level,is_root,row,column"
    )?;
    for outlet in 1..parent_outlet.len() {
        let child_ids = child_outlets[outlet]
            .iter()
            .map(|child_id| child_id.to_string())
            .collect::<Vec<String>>()
            .join(";");
        writeln!(
            hierarchy_writer,
            "{},{},{},\"{}\",{},{},{},{},{}",
            outlet,
            parent_outlet[outlet],
            child_outlets[outlet].len(),
            child_ids,
            nesting_order[outlet],
            hierarchy_level[outlet],
            parent_outlet[outlet] == 0,
            outlet_rows[outlet],
            outlet_columns[outlet]
        )?;
    }
    hierarchy_writer.flush()?;

    Ok(())
}

fn delineate_to_seeded_outlets(
    mut seeded_outlets: Array2D<i32>,
    flow_dir: &Array2D<i8>,
    dx: &[isize; 8],
    dy: &[isize; 8],
) -> Array2D<i32> {
    let rows = flow_dir.rows;
    let columns = flow_dir.columns;
    let mut flag: bool;
    let (mut x, mut y): (isize, isize);
    let mut dir: i8;
    let mut outlet_id: i32;
    let mut z: i32;

    for row in 0..rows {
        for col in 0..columns {
            if flow_dir.get_value(row, col) == -2 {
                seeded_outlets.set_value(row, col, NODATA_OUTLET_ID);
            }
            if seeded_outlets.get_value(row, col) == LOW_OUTLET_ID {
                flag = false;
                x = col;
                y = row;
                outlet_id = NODATA_OUTLET_ID;
                while !flag {
                    dir = flow_dir.get_value(y, x);
                    if dir >= 0 {
                        x += dx[dir as usize];
                        y += dy[dir as usize];
                        z = seeded_outlets.get_value(y, x);
                        if z != LOW_OUTLET_ID {
                            outlet_id = z;
                            flag = true;
                        }
                    } else {
                        flag = true;
                    }
                }

                flag = false;
                x = col;
                y = row;
                seeded_outlets.set_value(y, x, outlet_id);
                while !flag {
                    dir = flow_dir.get_value(y, x);
                    if dir >= 0 {
                        x += dx[dir as usize];
                        y += dy[dir as usize];
                        if seeded_outlets.get_value(y, x) != LOW_OUTLET_ID {
                            flag = true;
                        }
                    } else {
                        flag = true;
                    }
                    seeded_outlets.set_value(y, x, outlet_id);
                }
            }
        }
    }

    seeded_outlets
}

fn build_order_ancestor_lookup(
    parent_outlet: &[usize],
    nesting_order: &[usize],
    max_nesting_order: usize,
) -> Vec<Vec<usize>> {
    let num_outlets = parent_outlet.len() - 1;
    let mut order_lookup = vec![vec![0usize; num_outlets + 1]; max_nesting_order + 1];

    for order in 1..max_nesting_order + 1 {
        for outlet in 1..num_outlets + 1 {
            let mut current = outlet;
            while current > 0 {
                if nesting_order[current] == order {
                    order_lookup[order][outlet] = current;
                    break;
                }
                current = parent_outlet[current];
            }
        }
    }

    order_lookup
}

/// In some applications it is necessary to relate a measured variable for a group of
/// hydrometric stations (e.g. characteristics of flow timing and duration or water
/// chemistry) to some characteristics of each outlet's catchment (e.g. mean slope,
/// area of wetlands, etc.). When the group of outlets are nested, i.e. some stations
/// are located downstream of others, then performing a watershed operation will
/// result in inappropriate watershed delineation. In particular, the delineated
/// watersheds of each nested outlet will not include the catchment areas of upstream
/// outlets. This creates a serious problem for this type of application.
///
/// The Unnest Basin tool can be used to perform a watershedding operation based on a
/// group of specified pour points, i.e. outlets or target cells, such that each
/// complete watershed is delineated. The user must specify the name of a flow pointer
/// (flow direction) raster, a pour point raster, and the name of the output rasters.
/// Multiple numbered outputs will be created, one for each nesting level. Pour point,
/// or target, cells are denoted in the input pour-point image as any non-zero,
/// non-NoData value. A hierarchy sidecar CSV is also written to
/// `<output_stem>_hierarchy.csv`, containing outlet parent/child relationships and
/// nesting metadata. The flow pointer raster should be generated using the D8 algorithm.
pub struct UnnestBasins {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl UnnestBasins {
    pub fn new() -> UnnestBasins {
        // public constructor
        let name = "UnnestBasins".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Extract whole watersheds for a set of outlet points.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input D8 pointer raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Pour Points (Outlet) File".to_owned(),
            flags: vec!["--pour_pts".to_owned()],
            description: "Input vector pour points (outlet) file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr='d8pntr.tif' --pour_pts='pour_pts.shp' -o='output.tif'", short_exe, name).replace("*", &sep);

        UnnestBasins {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for UnnestBasins {
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
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
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
        let mut d8_file = String::new();
        let mut pourpts_file = String::new();
        let mut output_file = String::new();
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
            if flag_val == "-d8_pntr" {
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
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
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

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !pourpts_file.contains(&sep) && !pourpts_file.contains("/") {
            pourpts_file = format!("{}{}", working_directory, pourpts_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        let start = Instant::now();

        if verbose {
            println!("Reading data...")
        };

        let pntr = Raster::new(&d8_file, "r")?;

        let pourpts = Shapefile::read(&pourpts_file)?;

        // make sure the input vector file is of points type
        if pourpts.header.shape_type.base_shape_type() != ShapeType::Point {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input vector data must be of point base shape type.",
            ));
        }

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = -32768f64; //pour_pts.configs.nodata;
        let pntr_nodata = pntr.configs.nodata;

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut flow_dir: Array2D<i8> = Array2D::new(rows, columns, -2, -2)?;
        let mut outlet_points: Array2D<isize> = Array2D::new(rows, columns, 0, 0)?;
        let mut outlet_rows = vec![0isize; pourpts.num_records + 1];
        let mut outlet_columns = vec![0isize; pourpts.num_records + 1];
        let mut nesting_order = vec![0usize; pourpts.num_records + 1];
        let mut outlet: usize;

        for record_num in 0..pourpts.num_records {
            let record = pourpts.get_record(record_num);
            outlet = record_num + 1;
            let row = pntr.get_row_from_y(record.points[0].y);
            let col = pntr.get_column_from_x(record.points[0].x);
            outlet_points.set_value(row, col, outlet as isize);
            outlet_rows[outlet] = row;
            outlet_columns[outlet] = col;

            if verbose {
                progress = (100.0_f64 * outlet as f64 / pourpts.num_records as f64) as usize;
                if progress != old_progress {
                    println!("Locating pour points: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        // Create a mapping from the pointer values to cells offsets.
        // This may seem wasteful, using only 8 of 129 values in the array,
        // but the mapping method is far faster than calculating z.ln() / ln(2.0).
        // It's also a good way of allowing for different point styles.
        let mut pntr_matches: [i8; 129] = [0i8; 129];
        if !esri_style {
            // This maps Whitebox-style D8 pointer values
            // onto the cell offsets in dx and dy.
            pntr_matches[1] = 0i8;
            pntr_matches[2] = 1i8;
            pntr_matches[4] = 2i8;
            pntr_matches[8] = 3i8;
            pntr_matches[16] = 4i8;
            pntr_matches[32] = 5i8;
            pntr_matches[64] = 6i8;
            pntr_matches[128] = 7i8;
        } else {
            // This maps Esri-style D8 pointer values
            // onto the cell offsets in dx and dy.
            pntr_matches[1] = 1i8;
            pntr_matches[2] = 2i8;
            pntr_matches[4] = 3i8;
            pntr_matches[8] = 4i8;
            pntr_matches[16] = 5i8;
            pntr_matches[32] = 6i8;
            pntr_matches[64] = 7i8;
            pntr_matches[128] = 0i8;
        }

        let mut z: f64;
        for row in 0..rows {
            for col in 0..columns {
                z = pntr.get_value(row, col);
                if z != pntr_nodata {
                    if z > 0.0 {
                        flow_dir.set_value(row, col, pntr_matches[z as usize]);
                    } else {
                        flow_dir.set_value(row, col, -1i8);
                    }
                }
            }
            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Initializing: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let (nesting_order_calc, max_nesting_order) = calculate_nesting_order(
            &flow_dir,
            &outlet_points,
            &outlet_rows,
            &outlet_columns,
            &dx,
            &dy,
        );
        nesting_order = nesting_order_calc;
        if verbose {
            println!("Calculating outlet nesting order: 100%");
        }

        let parent_outlet = calculate_parent_outlet(
            &flow_dir,
            &outlet_points,
            &outlet_rows,
            &outlet_columns,
            &dx,
            &dy,
        );
        let child_outlets = calculate_child_outlets(&parent_outlet);
        let hierarchy_level = calculate_hierarchy_levels(&parent_outlet);

        write_hierarchy_sidecar(
            &output_file,
            &parent_outlet,
            &child_outlets,
            &nesting_order,
            &hierarchy_level,
            &outlet_rows,
            &outlet_columns,
        )?;
        if verbose {
            let output_path = path::Path::new(&output_file);
            let output_stem = output_path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or_else(|| {
                    Error::new(ErrorKind::InvalidInput, "Output file name is invalid.")
                })?;
            let hierarchy_file =
                output_path.with_file_name(format!("{}_hierarchy.csv", output_stem));
            println!("Hierarchy sidecar written: {}", hierarchy_file.display());
        }

        // `outlet_points` can be freed before large-cell assignment to reduce peak memory.
        drop(outlet_points);

        let mut all_outlet_seeds: Array2D<i32> =
            Array2D::new(rows, columns, LOW_OUTLET_ID, LOW_OUTLET_ID)?;
        for outlet in 1..pourpts.num_records + 1 {
            all_outlet_seeds.set_value(outlet_rows[outlet], outlet_columns[outlet], outlet as i32);
        }
        let base_assignment = delineate_to_seeded_outlets(all_outlet_seeds, &flow_dir, &dx, &dy);
        let order_lookup =
            build_order_ancestor_lookup(&parent_outlet, &nesting_order, max_nesting_order);

        for order in 1..max_nesting_order + 1 {
            let start2 = Instant::now();
            // there will be an output file for each nesting order
            let pos_of_dot = output_file.rfind('.').unwrap_or(0);
            let ext = &output_file[pos_of_dot..];
            let output_file_order = output_file.replace(ext, &format!("_{}{}", order, ext));

            let mut output = Raster::initialize_using_file(&output_file_order, &pntr);
            output.configs.nodata = nodata;
            output.configs.data_type = DataType::I16;
            output.configs.photometric_interp = PhotometricInterpretation::Categorical;
            output.configs.palette = "qual.pal".to_string();
            output.reinitialize_values(nodata);
            for row in 0..rows {
                for col in 0..columns {
                    if flow_dir.get_value(row, col) == -2 {
                        continue;
                    }
                    let base_outlet_id = base_assignment.get_value(row, col);
                    if base_outlet_id > 0 {
                        let mapped_outlet_id = order_lookup[order][base_outlet_id as usize];
                        if mapped_outlet_id > 0 {
                            output.set_value(row, col, mapped_outlet_id as f64);
                        }
                    }
                }
                if verbose {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!(
                            "Progress (Loop {} of {}): {}%",
                            order, max_nesting_order, progress
                        );
                        old_progress = progress;
                    }
                }
            }

            let elapsed_time2 = get_formatted_elapsed_time(start2);
            output.add_metadata_entry(format!(
                "Created by whitebox_tools\' {} tool",
                self.get_tool_name()
            ));
            output.add_metadata_entry(format!("D8 pointer file: {}", d8_file));
            output.add_metadata_entry(format!("Pour-points file: {}", pourpts_file));
            output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time2));

            if verbose {
                println!("Saving data for nesting order {}...", order)
            };
            let _ = match output.write() {
                Ok(_) => {
                    if verbose {
                        println!("Output file written")
                    }
                }
                Err(e) => return Err(e),
            };
        }

        let elapsed_time = get_formatted_elapsed_time(start);

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (including I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_seed_array(
        rows: isize,
        columns: isize,
        outlet_rows: &[isize],
        outlet_columns: &[isize],
        allowed_orders: &[usize],
        nesting_order: &[usize],
    ) -> Array2D<i32> {
        let mut seeds = Array2D::new(rows, columns, LOW_OUTLET_ID, LOW_OUTLET_ID).unwrap();
        for outlet in 1..outlet_rows.len() {
            if allowed_orders.contains(&nesting_order[outlet]) {
                seeds.set_value(outlet_rows[outlet], outlet_columns[outlet], outlet as i32);
            }
        }
        seeds
    }

    fn map_base_assignment_to_order(
        base_assignment: &Array2D<i32>,
        order_lookup: &[Vec<usize>],
        order: usize,
    ) -> Array2D<i32> {
        let mut mapped = Array2D::new(
            base_assignment.rows,
            base_assignment.columns,
            NODATA_OUTLET_ID,
            NODATA_OUTLET_ID,
        )
        .unwrap();
        for row in 0..base_assignment.rows {
            for col in 0..base_assignment.columns {
                let base_id = base_assignment.get_value(row, col);
                if base_id > 0 {
                    let order_id = order_lookup[order][base_id as usize];
                    if order_id > 0 {
                        mapped.set_value(row, col, order_id as i32);
                    }
                }
            }
        }
        mapped
    }

    fn assert_arrays_equal(a: &Array2D<i32>, b: &Array2D<i32>) {
        assert_eq!(a.rows, b.rows);
        assert_eq!(a.columns, b.columns);
        for row in 0..a.rows {
            for col in 0..a.columns {
                assert_eq!(
                    a.get_value(row, col),
                    b.get_value(row, col),
                    "Mismatch at row {}, col {}",
                    row,
                    col
                );
            }
        }
    }

    #[test]
    fn optimized_mapping_matches_legacy_tracing_for_linear_network() {
        let rows = 1isize;
        let columns = 7isize;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        let mut flow_dir = Array2D::new(rows, columns, -2i8, -2i8).unwrap();
        for col in 0..6 {
            flow_dir.set_value(0, col, 1i8); // flow east
        }
        flow_dir.set_value(0, 6, -1i8); // sink

        let mut outlet_points = Array2D::new(rows, columns, 0isize, 0isize).unwrap();
        outlet_points.set_value(0, 1, 1);
        outlet_points.set_value(0, 3, 2);
        outlet_points.set_value(0, 5, 3);
        let outlet_rows = vec![0isize, 0, 0, 0];
        let outlet_columns = vec![0isize, 1, 3, 5];

        let (nesting_order, max_order) = calculate_nesting_order(
            &flow_dir,
            &outlet_points,
            &outlet_rows,
            &outlet_columns,
            &dx,
            &dy,
        );
        let parent_outlet = calculate_parent_outlet(
            &flow_dir,
            &outlet_points,
            &outlet_rows,
            &outlet_columns,
            &dx,
            &dy,
        );
        let order_lookup = build_order_ancestor_lookup(&parent_outlet, &nesting_order, max_order);

        let all_seeds = make_seed_array(
            rows,
            columns,
            &outlet_rows,
            &outlet_columns,
            &[1usize, 2usize, 3usize],
            &nesting_order,
        );
        let base_assignment = delineate_to_seeded_outlets(all_seeds, &flow_dir, &dx, &dy);

        for order in 1..max_order + 1 {
            let order_seeds = make_seed_array(
                rows,
                columns,
                &outlet_rows,
                &outlet_columns,
                &[order],
                &nesting_order,
            );
            let legacy = delineate_to_seeded_outlets(order_seeds, &flow_dir, &dx, &dy);
            let optimized = map_base_assignment_to_order(&base_assignment, &order_lookup, order);
            assert_arrays_equal(&legacy, &optimized);
        }
    }

    #[test]
    fn optimized_mapping_matches_legacy_tracing_for_branched_network() {
        let rows = 3isize;
        let columns = 5isize;
        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        let mut flow_dir = Array2D::new(rows, columns, -2i8, -2i8).unwrap();
        // main stem
        flow_dir.set_value(1, 0, 1i8);
        flow_dir.set_value(1, 1, 1i8);
        flow_dir.set_value(1, 2, 1i8);
        flow_dir.set_value(1, 3, 1i8);
        flow_dir.set_value(1, 4, -1i8);
        // north branch feeding row 1
        flow_dir.set_value(0, 0, 1i8);
        flow_dir.set_value(0, 1, 3i8);
        flow_dir.set_value(0, 2, 3i8);
        flow_dir.set_value(0, 3, 3i8);
        // south branch feeding row 1
        flow_dir.set_value(2, 0, 1i8);
        flow_dir.set_value(2, 1, 1i8);
        flow_dir.set_value(2, 2, 7i8);
        flow_dir.set_value(2, 3, 7i8);
        flow_dir.set_value(2, 4, 7i8);

        let mut outlet_points = Array2D::new(rows, columns, 0isize, 0isize).unwrap();
        outlet_points.set_value(1, 1, 1);
        outlet_points.set_value(1, 3, 2);
        let outlet_rows = vec![0isize, 1, 1];
        let outlet_columns = vec![0isize, 1, 3];

        let (nesting_order, max_order) = calculate_nesting_order(
            &flow_dir,
            &outlet_points,
            &outlet_rows,
            &outlet_columns,
            &dx,
            &dy,
        );
        let parent_outlet = calculate_parent_outlet(
            &flow_dir,
            &outlet_points,
            &outlet_rows,
            &outlet_columns,
            &dx,
            &dy,
        );
        let order_lookup = build_order_ancestor_lookup(&parent_outlet, &nesting_order, max_order);

        let all_seeds = make_seed_array(
            rows,
            columns,
            &outlet_rows,
            &outlet_columns,
            &[1usize, 2usize],
            &nesting_order,
        );
        let base_assignment = delineate_to_seeded_outlets(all_seeds, &flow_dir, &dx, &dy);

        for order in 1..max_order + 1 {
            let order_seeds = make_seed_array(
                rows,
                columns,
                &outlet_rows,
                &outlet_columns,
                &[order],
                &nesting_order,
            );
            let legacy = delineate_to_seeded_outlets(order_seeds, &flow_dir, &dx, &dy);
            let optimized = map_base_assignment_to_order(&base_assignment, &order_lookup, order);
            assert_arrays_equal(&legacy, &optimized);
        }
    }

    #[test]
    fn hierarchy_sidecar_contains_expected_header_and_rows() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output_file = format!("/tmp/unnest_basins_test_{}.tif", unique);
        let sidecar_file = format!("/tmp/unnest_basins_test_{}_hierarchy.csv", unique);

        let parent_outlet = vec![0usize, 2, 3, 0];
        let child_outlets = calculate_child_outlets(&parent_outlet);
        let nesting_order = vec![0usize, 1, 2, 3];
        let hierarchy_level = calculate_hierarchy_levels(&parent_outlet);
        let outlet_rows = vec![0isize, 10, 20, 30];
        let outlet_columns = vec![0isize, 11, 21, 31];

        write_hierarchy_sidecar(
            &output_file,
            &parent_outlet,
            &child_outlets,
            &nesting_order,
            &hierarchy_level,
            &outlet_rows,
            &outlet_columns,
        )
        .unwrap();

        let text = fs::read_to_string(&sidecar_file).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines[0],
            "outlet_id,parent_outlet_id,child_count,child_ids,nesting_order,hierarchy_level,is_root,row,column"
        );
        assert_eq!(lines.len(), 4); // header + 3 outlets

        let _ = fs::remove_file(&sidecar_file);
    }
}
