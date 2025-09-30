# Developing New WhiteboxTools Commands

This note is for contributors extending WhiteBoxTools with additional tools. It
covers the expected file locations, argument parsing pattern, output/metadata
structure, and stylistic conventions so that new code matches the surrounding
WhiteboxTools ecosystem.

## 1. Project Layout and Naming

- **Tool source files** live under
  `whitebox-tools-app/src/tools/<toolbox>/<tool_name>.rs`. Match the existing
  folder hierarchy (e.g. `stream_network_analysis`, `hydro_analysis`,
  `gis_analysis`). Tool filenames use `snake_case`.
- Each toolbox directory exposes a `mod.rs`. To register your tool:
  1. `mod <tool_name>;` near the top of the toolbox `mod.rs`.
  2. `pub use self::<tool_name>::ToolStructName;` in the exports section.
- Global dispatch occurs in `whitebox-tools-app/src/tools/mod.rs`:
  - Append the CamelCase name to the `tool_names` list in `ToolManager::new`.
  - Add a match arm in `ToolManager::get_tool` mapping the lowercased command
    (`"mynewtool"`) to `Box::new(toolbox::ToolStructName::new())`.
- **Python bindings** live in `whitebox_tools.py` (and the mirrored
  `WBT/whitebox_tools.py`). Add a wrapper method that collects arguments and
  calls `self.run_tool('<snake_case_command>', args, callback)`.

## 2. Argument Parsing Template

Every Rust tool follows the same `run` method structure:

1. **Declarations**: define mutable `String` holders for filenames and scalars
   for each configurable parameter (booleans default to `false`, numeric values
   often default to `0.0`/`-1`).
2. **Argument loop**:
   ```rust
   for i in 0..args.len() {
       let mut arg = args[i].replace("\"", "");
       arg = arg.replace("\'", "");
       let cmd = arg.split("=");
       let vec = cmd.collect::<Vec<&str>>();
       let keyval = vec.len() > 1;
       let flag_val = vec[0].to_lowercase().replace("--", "-");
       if flag_val == "-input" { ... }
   }
   ```
   - Support both `--flag=value` and `--flag value` forms by testing `keyval`.
   - Normalise flags to lowercase with single `-`.
   - Accept boolean flags when the user omits the explicit value (`--flag`).
3. **Validation**: error early if required inputs remain empty:
   ```rust
   if input_file.is_empty() {
       return Err(Error::new(ErrorKind::InvalidInput,
           "Input file not specified."));
   }
   ```
4. **Path resolution**: prepend `working_directory` when the provided path is
   relative (use `path::MAIN_SEPARATOR` test as in existing tools).

## 3. Output and Metadata Pattern

- Initialise the result raster using `Raster::initialize_using_file` so that
  georeferencing is inherited from the input.
- After the processing loop completes:
  ```rust
  let elapsed_time = get_formatted_elapsed_time(start);
  output.add_metadata_entry(format!(
      "Created by whitebox_tools' {} tool",
      self.get_tool_name()
  ));
  output.add_metadata_entry(format!("Input file: {}", input_file));
  // Add extra metadata describing parameters as needed.
  output.add_metadata_entry(format!(
      "Elapsed Time (excluding I/O): {}",
      elapsed_time
  ));
  output.write()?;
  ```
- Always wrap writes in `match`/`?` so IO errors propagate to the caller.

## 4. Style Guide

- **Imports**: group standard library `use` statements first, followed by
  `crate::tools::*` and then external crates (`whitebox_raster`,
  `whitebox_common`, etc.). Keep alphabetised blocks when practical.
- **Doc comments** (`///`) should summarise the tool, note shared WhiteboxTools
  expectations (e.g., D8 pointers must come from `D8Pointer`), and list
  references/related tools.
- **Progress reporting**: reuse the standard progress pattern with `old_progress`
  checks to avoid noisy output, and honour the `verbose` flag.
- **Formatting**: use `rustfmt` defaults (already enforced in this repository).
  Prefer `snake_case` for variables and keep variable scope tight.
- **Error messages**: match the tone in existing tools—concise, actionable
  strings via `Error::new(ErrorKind::InvalidInput, "message")`.
- **Booleans**: prefer descriptive flag names (`use_zero_background` over `flag`)
  and initialise them with `false` unless a tool requires a different default.
- **Metadata**: include any non-obvious parameter choices (units, thresholds)
  in the output metadata so downstream workflows can trace how rasters were
  produced.

Following these conventions ensures that new functionality integrates cleanly
with the command-line interface, Python bindings, and downstream consumers such
as WEPPcloud.

## 5. Prompt Template for CODEX

When you want CODEX to scaffold a new tool, provide a concise specification that
covers the toolbox, intent, parameters, and expected outputs. The following
template works well:

```
You are in the WhiteboxTools fork (WEPPcloud variant).

Goal: Create a new tool in the `<toolbox>` toolbox named `<CamelCaseName>`.

Inputs:
- list each flag (`--name`, type, required/optional, default).

Behaviour:
- explain the core algorithm / processing steps.
- specify any assumptions (e.g. expects D8 pointer, no ESRI support needed).

Outputs:
- describe raster/vector outputs, metadata additions, and how to treat
  background/no-data.

Extras:
- mention if a Python wrapper is required.
- highlight test data or validation steps if relevant.

Please update the necessary module registrations and ensure the code follows the
style in `DEVELOPING_TOOLS.md`.
```

Replace the placeholders with your tool’s details and paste the prompt into the
chat to have CODEX generate the implementation skeleton.
