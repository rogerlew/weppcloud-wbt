use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::sync::OnceLock;
// use std::path;

/// A structure to hold environment settings. Backed by settings.json file in same directory
#[derive(Serialize, Deserialize, Debug, Clone)]
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

/// Immutable effective options for one tool process (including its worker threads).
static RUNTIME_CONFIG: OnceLock<Configs> = OnceLock::new();
pub const RUNTIME_CONFIG_ENV: &str = "WBT_RUNTIME_CONFIG_JSON";

pub fn set_runtime_config(configs: Configs) -> Result<(), Error> {
    RUNTIME_CONFIG.set(configs).map_err(|_| {
        Error::new(
            ErrorKind::AlreadyExists,
            "Tool runtime configuration is already initialized",
        )
    })
}

/// Child plugins inherit the same options, never another job's saved settings.
pub fn get_configs() -> Result<Configs, Error> {
    let mut configs = if let Some(configs) = RUNTIME_CONFIG.get() {
        configs.clone()
    } else {
        match std::env::var(RUNTIME_CONFIG_ENV) {
            Ok(value) => {
                serde_json::from_str(&value).map_err(|e| Error::new(ErrorKind::InvalidData, e))?
            }
            Err(std::env::VarError::NotPresent) => get_persisted_configs()?,
            Err(e) => return Err(Error::new(ErrorKind::InvalidInput, e)),
        }
    };
    if let Some(max_procs) = max_procs_override()? {
        configs.max_procs = max_procs;
    }
    Ok(configs)
}

fn config_path() -> Result<std::path::PathBuf, Error> {
    let mut directory = std::env::current_exe()?;
    directory.pop();
    if directory.ends_with("plugins") {
        directory.pop();
    }
    if directory.ends_with("whitebox_tools") || directory.ends_with("whitebox_tools.exe") {
        directory.pop();
    }
    Ok(directory.join("settings.json"))
}

/// Read only disk configuration for callers that may subsequently save it.
pub fn get_persisted_configs() -> Result<Configs, Error> {
    match fs::read_to_string(config_path()?) {
        Ok(contents) => {
            serde_json::from_str(&contents).map_err(|e| Error::new(ErrorKind::InvalidData, e))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Configs::new()),
        Err(e) => Err(e),
    }
}

// Linux extended attributes include POSIX ACLs and security labels. Copy them
// through open descriptors so atomic replacement preserves effective access.
#[cfg(target_os = "linux")]
fn copy_extended_attributes(source: &File, target: &File) -> Result<(), Error> {
    use std::ffi::{c_char, c_int, c_void, CString};
    use std::os::fd::AsRawFd;
    extern "C" {
        fn flistxattr(fd: c_int, names: *mut c_char, size: usize) -> isize;
        fn fgetxattr(fd: c_int, name: *const c_char, value: *mut c_void, size: usize) -> isize;
        fn fsetxattr(
            fd: c_int,
            name: *const c_char,
            value: *const c_void,
            size: usize,
            flags: c_int,
        ) -> c_int;
        fn fremovexattr(fd: c_int, name: *const c_char) -> c_int;
    }
    fn names(file: &File) -> Result<Vec<CString>, Error> {
        // SAFETY: live descriptor; null buffer explicitly requests required size.
        let size = unsafe { flistxattr(file.as_raw_fd(), std::ptr::null_mut(), 0) };
        if size < 0 {
            return Err(Error::last_os_error());
        }
        if size == 0 {
            return Ok(Vec::new());
        }
        let mut buffer = vec![0u8; size as usize];
        // SAFETY: writable buffer length equals supplied size; descriptor remains live.
        let read =
            unsafe { flistxattr(file.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len()) };
        if read < 0 {
            return Err(Error::last_os_error());
        }
        buffer.truncate(read as usize);
        buffer
            .split(|value| *value == 0)
            .filter(|name| !name.is_empty())
            .map(|name| CString::new(name).map_err(|e| Error::new(ErrorKind::InvalidData, e)))
            .collect()
    }
    let source_names = names(source)?;
    for name in names(target)? {
        if !source_names.contains(&name) {
            // SAFETY: valid descriptor and nul-terminated attribute name.
            if unsafe { fremovexattr(target.as_raw_fd(), name.as_ptr()) } < 0 {
                return Err(Error::last_os_error());
            }
        }
    }
    for name in source_names {
        // SAFETY: live descriptor and name; null buffer requests attribute size.
        let size = unsafe { fgetxattr(source.as_raw_fd(), name.as_ptr(), std::ptr::null_mut(), 0) };
        if size < 0 {
            return Err(Error::last_os_error());
        }
        let mut value = vec![0u8; size as usize];
        // SAFETY: allocated buffer is valid for the supplied length.
        let read = unsafe {
            fgetxattr(
                source.as_raw_fd(),
                name.as_ptr(),
                value.as_mut_ptr().cast(),
                value.len(),
            )
        };
        if read < 0 {
            return Err(Error::last_os_error());
        }
        if read as usize > value.len() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Configuration attribute changed during capture",
            ));
        }
        // SAFETY: valid target/name and initialized bytes returned by fgetxattr.
        if unsafe {
            fsetxattr(
                target.as_raw_fd(),
                name.as_ptr(),
                value.as_ptr().cast(),
                read as usize,
                0,
            )
        } < 0
        {
            return Err(Error::last_os_error());
        }
    }
    Ok(())
}

/// Explicit administrative writes publish complete JSON; tool runs never call this.
pub fn save_configs(configs: &Configs) -> Result<(), Error> {
    let configured_path = config_path()?;
    // Preserve existing symlink targets instead of replacing the configuration link.
    let destination = match fs::symlink_metadata(&configured_path) {
        Ok(_) => fs::canonicalize(&configured_path)?,
        Err(e) if e.kind() == ErrorKind::NotFound => configured_path,
        Err(e) => return Err(e),
    };
    let bytes =
        serde_json::to_vec_pretty(configs).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    let temporary = destination.with_file_name(format!(
        ".settings.{}.{}.tmp",
        std::process::id(),
        rand::random::<u64>()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600); // Never expose configuration bytes through a public temporary.
    }
    let mut file = options.open(&temporary)?;
    let mut creation_probe = None;
    let result = (|| {
        file.write_all(&bytes)?;
        match File::open(&destination) {
            Ok(source) => {
                let metadata = source.metadata()?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    let created = file.metadata()?;
                    if created.uid() != metadata.uid() || created.gid() != metadata.gid() {
                        std::os::unix::fs::chown(
                            &temporary,
                            Some(metadata.uid()),
                            Some(metadata.gid()),
                        )?;
                    }
                }
                file.set_permissions(metadata.permissions())?;
                #[cfg(target_os = "linux")]
                copy_extended_attributes(&source, &file)?;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                // An empty probe captures normal umask/default-ACL creation semantics
                // without ever putting configuration bytes in a public temporary.
                let probe_path = temporary.with_extension("permissions");
                let probe = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&probe_path)?;
                creation_probe = Some(probe_path);
                file.set_permissions(probe.metadata()?.permissions())?;
                #[cfg(target_os = "linux")]
                copy_extended_attributes(&probe, &file)?;
            }
            Err(e) => return Err(e),
        }
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &destination)?;
        #[cfg(unix)]
        File::open(destination.parent().unwrap())?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        // Best-effort cleanup must not replace the original publication error.
        let _ = fs::remove_file(&temporary);
    }
    if let Some(probe_path) = creation_probe {
        return result.and(fs::remove_file(probe_path));
    }
    result
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
    fn runtime_snapshot_is_immutable_and_initialized_once() {
        let mut initial = Configs::new();
        initial.working_directory = "invocation-owned".to_string();
        initial.compress_rasters = false;
        set_runtime_config(initial).unwrap();
        let mut copy = get_configs().unwrap();
        copy.compress_rasters = true;
        assert!(!get_configs().unwrap().compress_rasters);
        assert_eq!(get_configs().unwrap().working_directory, "invocation-owned");
        assert_eq!(
            set_runtime_config(copy).unwrap_err().kind(),
            ErrorKind::AlreadyExists
        );
        assert!(!get_configs().unwrap().compress_rasters);
    }

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
