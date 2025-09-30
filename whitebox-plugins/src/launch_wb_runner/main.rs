/*
Authors:  Dr. John Lindsay
Created: 14/11/2022
Last Modified: 14/11/2022
License: MIT
*/
use std::{
    env,
    io::{Error, ErrorKind},
    path,
    path::Path,
    process::Command,
    // thread,
};

/// This tool can be used to launch the Whitebox Runner application from within other Whitebox front-ends.
/// The purpose of this tool is to make the Whitebox Runner more accessible from other Whitebox front-ends.
/// However, note that you can also launch the Whitebox Runner simply by double-clicking on the executable
/// file (`whitebox_runner.exe` on Windows, `whitebox_tools` on other systems) located within your WBT
/// directory, containing your Whitebox installation.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args[1].trim() == "run" {
        match run(&args) {
            Ok(_) => {}
            Err(e) => panic!("{:?}", e),
        }
    }

    if args.len() <= 1 || args[1].trim() == "help" {
        // print help
        help();
    }

    if args[1].trim() == "version" {
        // print version information
        version();
    }
}

fn help() {
    let mut ext = "";
    if cfg!(target_os = "windows") {
        ext = ".exe";
    }

    let exe_name = &format!("launch_wb_runner{}", ext);
    let sep: String = path::MAIN_SEPARATOR.to_string();
    let s = r#"
    launch_wb_runner Help

    This tool is used launch the Whitebox Runner application.

    The following commands are recognized:
    help       Prints help information.
    run        Runs the tool.
    version    Prints the tool version information.

    The following flags can be used with the 'run' command:
    --clear_state  Boolean flag determines whether to clear previous app state.
    
    Input/output file names can be fully qualified, or can rely on the working directory contained in 
    the WhiteboxTools settings.json file.

     "#
    .replace("*", &sep)
    .replace("EXE_NAME", exe_name);
    println!("{}", s);
}

fn version() {
    const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    println!(
        "launch_wb_runner v{} by Dr. John B. Lindsay (c) 2022.",
        VERSION.unwrap_or("Unknown version")
    );
}

// fn get_tool_name() -> String {
//     String::from("LaunchWbRunner") // This should be camel case and is a reference to the tool name.
// }

fn run(args: &Vec<String>) -> Result<(), std::io::Error> {
    // read the arguments
    let mut clear_app_state: bool = false;

    if args.len() <= 1 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Tool run with too few parameters.",
        ));
    }
    for i in 0..args.len() {
        let mut arg = args[i].replace("\"", "");
        arg = arg.replace("\'", "");
        let cmd = arg.split("="); // in case an equals sign was used
        let vec = cmd.collect::<Vec<&str>>();
        // let mut keyval = false;
        // if vec.len() > 1 {
        //     keyval = true;
        // }
        let flag_val = vec[0].to_lowercase().replace("--", "-");
        if flag_val == "-clear_app_state" {
            if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                clear_app_state = true;
            }
        }
    }

    // see if you can find the runner app in the WBT directory.
    // First, check the path of the WbRunner executable.
    let mut dir = env::current_exe().unwrap_or(Path::new("").to_path_buf());
    dir.pop(); // tool name popped
    dir.pop(); // plugins directory popped

    let exe = dir.join(&format!("whitebox_runner{}", env::consts::EXE_SUFFIX));

    // check that it exists.
    if exe.exists() {
        if !clear_app_state {
            // thread::spawn(move || {
            let _output = Command::new(exe.to_str().unwrap_or(""))
                .output()
                .expect("Failed to execute process");
            // });
        } else {
            // thread::spawn(move || {
            let _output = Command::new(&exe)
                .args(["clear_state"])
                .output()
                .expect("Failed to execute process");
            // });
        }
    } else {
        println!("The Whitebox Runner app does not appear to be located within the WBT folder.");
    }

    Ok(())
}
