# Wire process-local WBT concurrency

This living ExecPlan follows AGENTS.md and docs/work-packages/README.md.

## Purpose

Give WEPPpy workers a 12-thread WBT budget without shared-settings races. Preserve
standalone behavior when WBT_MAX_PROCS is absent. This is faithful configuration
wiring, not an algorithm or queue change; rebuilt-binary evidence is required.

## Progress

- [x] Establish semantics and locate configuration consumers.
- [x] Implement common runtime override and persistent-only CLI settings reads.
- [x] Configure WEPPpy default/batch workers and document ADR-0051.
- [x] Validate subprocess isolation, errors, parity, and container inheritance.
- [x] Close trackers and evidence.

## Surprises & Discoveries

CLI --max_procs persists settings; tools reread settings independently. The
runtime overlay must be excluded from the main CLI configuration-save path.

## Decision Log

2026-09-07: accept only positive integer environment overrides. Unset retains
settings/default -1. Compose defaults to 12. Do not add project settings or change
job concurrency; operator explicitly excluded the latter concern.

## Outcomes & Retrospective

Completed: runtime overrides do not persist, concurrent 1/12-worker CLI runs
produce identical outputs, invalid limits fail before writes, and all Compose
models supply 12 or the operator override. The forest worker container inherited
12 through the unchanged wrapper while preserving its saved value of 4.
147 app tests, 42 common tests, and 7 doctests pass. Production was not deployed.

## Context and Plan of Work

In /workdir/weppcloud-wbt, whitebox-common/src/configs/mod.rs supplies tool config;
whitebox-tools-app/src/main.rs persists CLI settings. Split persisted reading
from effective runtime reading. Validate environment without mutating it, and
log the requested override for verbose tool execution. Keep Python bindings
unchanged: _build_process_env already copies the parent environment.

In /workdir/wepppy, add WBT_MAX_PROCS interpolation default 12 to rq-worker and
rq-worker-batch in dev, dev.hpc, prod, and prod.worker Compose files. Existing
wepp1 overrides merge environment keys. Document semantics in infrastructure
notes and ADR-0051 before edits. Do not modify production services in this task.

## Concrete Steps and Validation

Run cargo test -p whitebox_common, cargo test -p whitebox-tools-app, cargo check
-p whitebox-tools-app, and cargo build --release -p whitebox-tools-app with --locked.
Use an isolated target/wbt-runtime binary directory and copied fixture; run tools
with different environment limits and verify logs, exact raster/diagnostic parity,
and unchanged persisted max_procs even when other CLI settings are saved. Invalid
values must return errors before tool output or settings changes. Check absent
environment and a persisted CLI max_procs still work. Run the existing Python
wrapper in the forest rq-worker container with child-specific environments and
check identity, inherited limit, and generated output. Parse Compose models using
wctl and report only concurrency values, never full resolved environments.

## Recovery and Interfaces

Keep benchmark binaries and generated rasters under ignored target directories.
No deployment, branch changes, new dependencies, or per-job concurrency file writes.
Recover by rerunning into fresh output paths. Preserve previous optimization work
and its closed package. Keep standalone CLI persistent --max_procs compatible.

Revision note: closed on 2026-09-07 UTC after CLI, Compose, container, and Rust
validation. Generated evidence and reproducible probes are in artifacts/.
