# Runtime concurrency configuration

Set `WBT_MAX_PROCS=12` in the process environment to limit WBT tools using the
common concurrency configuration. The value must be a positive integer; empty,
zero, negative, non-integer, overflowing, and non-Unicode values fail explicitly.

Precedence for execution: WBT_MAX_PROCS, then settings.json max_procs, then -1
(automatic CPU selection). The actual tool may use fewer threads because of CPU
availability or workload size. Verbose CLI execution reports the runtime override;
least-cost breaching also reports its resolved search-worker count.

The runtime override is not saved. The legacy CLI --max_procs and Python
set_max_procs remain persistent operations; when both a CLI value and environment
value are supplied, the CLI value is saved and the environment value controls
execution. The CLI uses get_persisted_configs for settings changes; tools use
get_configs for effective configuration. New save callers must follow this split.

The Python wrapper already copies parent environment into each child. For limits
that differ between simultaneous calls, supply separate child environments rather
than mutating os.environ or shared settings.json. No wrapper signature changes
are required for worker-scoped configuration.

WEPPpy default and batch Compose workers explicitly default WBT_MAX_PROCS to 12.
This requires the updated binary; older WBT versions do not implement the variable.
No queue concurrency or model/project settings are changed.
