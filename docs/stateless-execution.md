# Stateless tool execution

Operator decision, 2026-11-09: web and batch workers must be able to run tools
concurrently without mutating shared installation configuration or process cwd.

A CLI invocation with `--run` reads installation defaults, applies explicit
options, and uses one immutable process-local configuration throughout tool and
raster execution. It never writes `settings.json`. Existing WBT_MAX_PROCS remains
a process-local override. Algorithms, numerical defaults and file formats stay
unchanged. Installation settings are optional static defaults, not job state.
Malformed settings fail explicitly; only an absent settings file selects defaults.

Python wrapper setters change only that instance. Its subprocess receives those
options and its own cwd. The parent process cwd never changes. Defaults come from
the wrapper installation, never an unrelated caller cwd. Separate instances can
run concurrently; sharing one mutable instance between threads is not promised.

Explicit CLI configuration commands without `--run` retain persistence. Writes
publish complete JSON by atomic replacement and report failures. Such commands
are administrative operations, not a transactional concurrent editing API.

This replaces the shared mutable settings pattern rather than adding per-job
binary copies, global model locks, or retries that hide corrupt JSON. All three bindings (root, WBT and pip package), bundled plugins and all Rust
configuration consumers must obey this contract. Validate
with real parallel outputs and immutable settings/cwd evidence before rollout.


## Compatibility and deployment

Python setters return success after changing only instance state. To persist
installation defaults explicitly, invoke the CLI without `--run`, for example
`whitebox_tools --max_procs=4`. Administrative edits are not concurrent
read-modify-write transactions. On Linux, atomic publication preserves the
existing owner/group, mode, ACLs and extended attributes, and follows existing
settings symlinks. Inability to preserve metadata fails before replacement.
Directory fsync failure after replacement can report failure after publication;
inspect the destination before repeating an administrative command.

Bundled plugins receive the effective options in `WBT_RUNTIME_CONFIG_JSON`.
Rebuild/install all bundled plugins with the dispatcher; old external plugin
binaries that read only settings.json are not verified for this protocol and
must not be assumed compatible. Do not enable them without equivalent execution
acceptance. Ordinary users need read access to optional installation defaults,
not permission to write the installation. Settings absence selects the existing
compiled defaults; malformed or unreadable settings fails explicitly.

The pip wrapper uses the existing subprocess containment implementation so tool
failures are not reported as success and children are reaped. No runtime mutates
parent cwd or persists thread-limit environment overrides. These Linux runtime
claims require the executable and Python tests in `tests/test_stateless_execution.py`
and `tests/test_whitebox_tools_process_containment.py`; other platforms retain
separate validation requirements. No scientific parameter default changed.
