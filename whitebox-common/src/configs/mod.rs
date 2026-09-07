use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
// use std::path;

/// A structure to hold environment settings. Backed by settings.json file in same directory
#[derive(Serialize, Deserialize, Debug)]
pub struct Configs {
    pub verbose_mode: bool,
    pub working_directory: String,
    pub compress_rasters: bool,
    pub max_procs: isize,
}

impl Configs {
    pub fn new() -> Configs {
        Configs {
            verbose_mode: true,
            working_directory: String::new(),
            compress_rasters: true,
            max_procs: -1,
        }
    }
}

/// Read effective tool configuration without persisting the environment override.
pub fn get_configs() -> std::result::Result<Configs, Error> {
    let mut configs = get_persisted_configs()?;
    if let Some(max_procs) = max_procs_override()? {
        configs.max_procs = max_procs;
    }
    Ok(configs)
}

/// Read only disk configuration for callers that may subsequently save it.
pub fn get_persisted_configs() -> std::result::Result<Configs, Error> {
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    if exe_path.ends_with("plugins") {
        exe_path.pop();
    }
    if exe_path.ends_with("whitebox_tools") || exe_path.ends_with("whitebox_tools.exe") {
        exe_path.pop();
    }
    let config_file = exe_path.join("settings.json");
    let config_file = config_file
        .to_str()
        .unwrap_or("No configs path found.")
        .to_string();

    let configs: Configs = match fs::read_to_string(config_file) {
        Ok(contents) => {
            serde_json::from_str(&contents).expect("Failed to parse config_file.json file.")
        }
        Err(_) => Configs::new(),
    };
    Ok(configs)
}

pub fn save_configs<'a>(configs: &Configs) -> std::result::Result<(), Error> {
    let configs_json =
        serde_json::to_string_pretty(&configs).expect("Error converting Configs object to JSON.");
    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();
    if exe_path.ends_with("plugins") {
        exe_path.pop();
    }
    if exe_path.ends_with("whitebox_tools") || exe_path.ends_with("whitebox_tools.exe") {
        exe_path.pop();
    }
    let config_file = exe_path.join("settings.json");
    let config_file = config_file
        .to_str()
        .unwrap_or("No configs path found.")
        .to_string();
    match File::create(config_file) {
        Ok(mut file) => {
            match file.write_all(configs_json.as_bytes()) {
                Ok(()) => {} // do nothing
                Err(_e) => {
                    eprintln!("Error writing to output settings.json file, likely do to a permissions problem. Settings will not be updated.");
                }
            };
        }
        Err(_e) => {
            eprintln!("Could not create output settings.json file. WBT is likely installed somewhere without write permission.")
        }
    };

    Ok(())
}

/// Return a process-local concurrency override, rejecting invalid configuration.
pub fn max_procs_override() -> Result<Option<isize>, Error> {
    match std::env::var("WBT_MAX_PROCS") {
        Ok(value) => parse_max_procs_override(Some(&value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(Error::new(
            ErrorKind::InvalidInput,
            "WBT_MAX_PROCS must be a positive integer",
        )),
    }
}

fn parse_max_procs_override(value: Option<&str>) -> Result<Option<isize>, Error> {
    match value {
        None => Ok(None),
        Some(value) => match value.parse::<isize>() {
            Ok(limit) if limit > 0 => Ok(Some(limit)),
            _ => Err(Error::new(
                ErrorKind::InvalidInput,
                "WBT_MAX_PROCS must be a positive integer",
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_override_preserves_automatic_default() {
        assert_eq!(parse_max_procs_override(None).unwrap(), None);
        assert_eq!(Configs::new().max_procs, -1);
    }

    #[test]
    fn positive_override_is_accepted() {
        assert_eq!(parse_max_procs_override(Some("12")).unwrap(), Some(12));
        assert_eq!(parse_max_procs_override(Some("1")).unwrap(), Some(1));
    }

    #[test]
    fn invalid_overrides_fail_explicitly() {
        for value in [
            "",
            "0",
            "-1",
            "-12",
            "abc",
            "1.5",
            " 12",
            "999999999999999999999999",
        ] {
            let error = parse_max_procs_override(Some(value)).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::InvalidInput);
            assert_eq!(
                error.to_string(),
                "WBT_MAX_PROCS must be a positive integer"
            );
        }
    }
}
