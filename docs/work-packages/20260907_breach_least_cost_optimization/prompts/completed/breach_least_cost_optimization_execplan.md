# Optimize least-cost breaching while preserving results

This living ExecPlan follows repository AGENTS.md and docs/work-packages/README.md.
Keep Progress, Surprises & Discoveries, Decision Log, and Outcomes & Retrospective
current. All commands below run in `/workdir/weppcloud-wbt` on forest.

## Purpose / Big Picture

Make valid-tabletop conditioning finish in under 600 seconds with 12 physical
cores, retaining exact hydrologic behavior. The 2,015 by 2,053 DEM has 5 m cells.
Its production call uses `--dist=600 --min_dist --fail_on_unresolved` and no fill.
A manually interrupted wepp1 run exceeded 794 seconds at about 14% progress.
This is a faithful implementation optimization, and closure requires generated
output evidence from the rebuilt binary, not just unit tests or scaffolding.

## Progress

- [x] (2026-09-07 22:01Z) Preserve production evidence and acquire DEM on forest.
- [x] Preserve reference binary before implementation changes.
- [x] (2026-09-07 22:35Z) Build baseline/parity harness; confirm legacy ordering differences.
- [x] Implement ordered parallel searches, exact flat reuse, and scratch generation tags.
- [x] Verify 30 exact parity cases at 1 and 12 cores; 147 Rust tests pass.
- [x] Measure final conditioning at 384.386 seconds on 12 physical cores, zero unresolved pits.
- [x] (2026-09-07 22:43Z) Full channel repeat succeeded in 420.566 s, with exact relief parity.
- [x] Run repository gates and close documentation; all acceptance gates passed.

## Surprises & Discoveries

Legacy pit discovery uses threads, but the expensive breach loop was serial. The
fixture has 400,026 candidate pits, approximately 395,000 at elevation zero.
Threading alone exceeded 600 seconds; exact translated-search reuse was needed.
Optional filling also exposed an ownership race fixed by releasing each worker
raster reference before its completion message.

Pit discovery uses threads, but the legacy expensive breach loop was serial. Each solved
pit lowers output elevations and later searches read those changes. Sorting pits
by elevation alone preserves nondeterministic row-arrival order for tied heights.

## Decision Log

2026-09-07: retain the existing cost arithmetic, neighbor order, heap ordering,
path limits, and failure contract. First characterize ordering and baseline
results. Consider concurrent read-only searches with ordered, conflict-validated
commits; never apply stale search results. No new dependencies are planned.

2026-09-07: prototype exact translation reuse for no-write searches reaching one
outer edge through uniform terrain. Validate the entire translated read rectangle
against current raster values before reuse, including its edge band; reject
nonuniform rectangles and corners. This is memoization of identical searches,
not an approximate elevation test. Keep ordered conflict checking.

## Outcomes & Retrospective

Final conditioning completed successfully in 384.386 seconds on 12 physical
cores (forest CPUs 12-23), resolving all 400,026 pits. Thirty parity cases and
147 Rust tests pass. The isolated full channel call completed in 420.566 seconds
from 22:36:04 UTC to 22:43:04 UTC, with ten generated artifacts checksummed.
The performance goal is met with 179.434 seconds of margin. No production
deployment or RQ timeout change was performed. Exact flat-terrain reuse was
necessary: threading-only prototypes exceeded the target.

## Context and Orientation

The tool lives in whitebox-tools-app/src/tools/hydro_analysis/
breach_depressions_least_cost.rs. Initial pit raising reads an immutable raster,
then a serial low-to-high pit loop performs heap searches and lowers paths in the
output raster. Diagnostics are emitted by conditioning_diagnostics.rs. Settings
are read beside the binary; isolate benchmark binaries and settings under
`target/breach-benchmark/` so tests do not modify installed WBT settings.

The new fixture is test_fixtures/breach_least_cost/valid-tabletop/dem.tif. Existing
smaller DEMs are in test_fixtures/topaz_condition_dem/. The baseline release binary
was copied to target/breach-benchmark/reference/whitebox_tools before edits; its
identity is recorded in tracker.md. Raw outputs stay under target, durable JSON
summaries belong in this package's artifacts directory.

## Milestones and Concrete Steps

First create a Python standard-library subprocess harness with existing GDAL/NumPy
for exact raster comparison. Record binary/input hashes, arguments, affinity,
wall time, status, normalized diagnostics, and array/mask/spatial hashes. Run
smaller real DEMs and synthetic ties, interacting pits, NoData edges, distance and
cost limits, and filling/failure cases. Preserve reference outputs before edits.

Next extract or optimize the search without changing its arithmetic and decisions.
Use existing threading primitives if parallel work is useful. If searches run
against shared snapshots, track every raster read (including the initial pit
check) and recompute a result whenever an earlier committed path modified a read
cell. Commit in the same pit order. Reuse per-worker scratch storage and bound
worker count by existing max_procs. Verify against a serial reference; changes
in tied-pit ordering require explicit rationale and an ADR if needed.

Build using `cargo build --release -p whitebox-tools-app --locked`. Copy the rebuilt
binary into an isolated benchmark directory with settings max_procs=12 and bind
to 12 distinct physical cores using taskset. Measure complete CLI elapsed time on
the large fixture, preserving raw logs and summaries. A failed unresolved-pit
result must match reference behavior and cannot be claimed as successful channel
completion. If successful, exercise downstream channel construction with the
same CSA=4 ha, MCL=40 m, and ifolp pruning.

Finally run `cargo check -p whitebox-tools-app`, `cargo test -p whitebox-tools-app`,
and `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py`. Update CHANGELOG,
user/developer notes, package tracker, and PROJECT_TRACKER. Move this plan to
prompts/completed only when parity and measured performance acceptance pass.

## Validation and Acceptance

Require exact raster values and masks, matching georeferencing and diagnostic
counts, consistent machine-readable failures, deterministic 1/12-core execution,
and no more than 12 compute workers. Preserve reference failure semantics and
all existing CLI/wrapper defaults. Require less than 600 seconds complete CLI
wall time on valid-tabletop with its original settings, reporting host/core
identity and repeated timing evidence. Do not weaken the search to meet timing.

## Idempotence and Recovery

Use unique benchmark output directories; never overwrite reference outputs or
production artifacts. Stop only verified benchmark PIDs if interrupted. Keep the
reference binary and source revision identity for re-comparison. No deployment,
branch switching, or RQ changes are part of this work.

## Artifacts and Interfaces

Use existing Rust standard-library threads, BinaryHeap, Raster, and Array2D.
Keep both Python bindings unchanged unless a necessary public interface change is
explicitly documented. Store reproducible commands and generated JSON evidence
under artifacts, excluding large binaries and scratch outputs from git.

Revision note (2026-09-07 22:43 UTC): closed after final rebuilt-binary parity,
384.386-second standalone conditioning, and 420.566-second full channel execution.
See artifacts/results.md for commands, fidelity limits, and evidence locations.
