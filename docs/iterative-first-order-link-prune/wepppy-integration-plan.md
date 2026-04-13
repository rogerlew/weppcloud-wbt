# WEPPpy Integration Plan: Iterative First-Order Link Prune

## Objective

Replace WEPPpy's current two-step delineation path in the WBT TOPAZ emulator:
1. `ExtractStreams` (CSA threshold)
2. `RemoveShortStreams` (MCL threshold)

with a single call to:
- `IterativeFirstOrderLinkPrune` (`iterative_first_order_link_prune`)

while keeping production rollout and rollback operationally safe.

## Target Outcome

1. `wbt_topaz_emulator` uses one WBT tool for source-area qualification + iterative first-order-link pruning.
2. Downstream artifacts remain valid (`netful`, `chnjnt`, netw/subwta-derived products).
3. Cutover is deterministic, tested, and reversible.

## Scope

In scope:
- New WBT tool implementation and registration.
- Python wrappers in both wrapper files.
- WEPPpy emulator refactor to single-tool call.
- Explicit treatment of other `remove_short_streams` consumers.
- Validation, rollout, rollback runbook.

Out of scope:
- DEM conditioning changes.
- Non-WBT TOPAZ backend changes.

## Current Dependency Inventory (Must-Acknowledge)

`remove_short_streams` is currently used in at least two WEPPpy paths:
1. Emulator path:
- `/workdir/wepppy/wepppy/topo/wbt/wbt_topaz_emulator.py`
2. Culvert batch path:
- `/workdir/wepppy/wepppy/rq/culvert_rq.py`

Implication:
- Do not remove/deprecate `remove_short_streams` until culvert path is migrated or intentionally pinned to legacy behavior.

## Proposed WBT API Contract

New wrapper method:
- `iterative_first_order_link_prune(
    d8_pntr,
    upstream_area,
    output,
    csa,
    mscl,
    esri_pntr=False,
    threshold_code_raster=None,
    threshold_table=None,
    epsilon=1e-5,
    fail_if_only_channel_pruned=True,
    callback=None
  )`

Migration note:
- Tool default remains `true` for parity.
- During rollout, WEPPpy emulator may explicitly pass `fail_if_only_channel_pruned=False` as a temporary operational exception.
- Remove this exception only after explicit operational acceptance.

## Work Plan

## Phase 0: Compatibility and Rollback Design

Tasks:
1. Define compatible version pairs:
- Pair A: legacy WEPPpy + legacy WBT
- Pair B: refactored WEPPpy + new WBT
2. Document atomic deploy/rollback rule:
- never roll back only one side of the pair.
3. Record exact rollback pins for each pair:
- WEPPpy commit SHA,
- WBT artifact identifier (tag/build hash),
- wrapper version marker.
4. Confirm packaging paths for wrapper availability in distributed WBT artifacts.

Acceptance gate:
- version matrix documented in release notes/runbook with exact pins.

## Phase 1: WBT Tool Implementation

Repository: `/workdir/weppcloud-wbt`

Tasks:
1. Add tool source:
- `whitebox-tools-app/src/tools/stream_network_analysis/iterative_first_order_link_prune.rs`
2. Register module/export/dispatch:
- `whitebox-tools-app/src/tools/stream_network_analysis/mod.rs`
- `whitebox-tools-app/src/tools/mod.rs`
3. Add parameter metadata/help text.
4. Add tests for:
- adjacent short-link iterative pruning,
- single-incoming-link receiver pruning,
- terminal-head half-cell behavior,
- only-channel guard behavior,
- deterministic tie-by-encounter behavior,
- optional threshold-code behavior,
- ESRI pointer mode,
- invalid pointer failure contract,
- cycle-detection failure contract,
- no-network failure contract.

Acceptance gate:
- `cargo check -p whitebox_tools`
- `cargo test -p whitebox_tools`

## Phase 2: Python Wrapper Updates (WBT Repo)

Tasks:
1. Add wrapper method in:
- `whitebox_tools.py`
- `WBT/whitebox_tools.py`
2. Keep `remove_short_streams` wrapper and underlying Rust tool available in Pair B while culvert remains legacy.
3. Validate packaged artifact includes the new tool and wrappers.

Acceptance gate:
- `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py`
- packaged WBT smoke run invokes `iterative_first_order_link_prune` successfully.

## Phase 3: WEPPpy Emulator Refactor

Repository: `/workdir/wepppy`

Tasks:
1. Update:
- `/workdir/wepppy/wepppy/topo/wbt/wbt_topaz_emulator.py`
2. Replace two-step `extract_streams + remove_short_streams` sequence with one `iterative_first_order_link_prune` call.
3. Map inputs:
- `csa -> csa`
- `mcl -> mscl`
- `flovec -> d8_pntr`
- `floaccum -> upstream_area`
- `netful -> output`
4. Update resource/provenance docs for artifact flow (`netful0` contract changes):
- `/workdir/wepppy/wepppy/topo/wbt/wbt_documentation.py`
- explicitly replace current `netful0` and `remove_short_streams` tool-description entries.

Acceptance gate:
- command-concrete emulator integration gate passes:
  `wctl run-pytest tests/topo/test_terrain_processor_wbt_integration.py`
- provenance/resources documentation updated and reviewed.

## Phase 4: Non-Emulator Consumer Disposition (`culvert_rq`)

Choose one explicit strategy before removing/deprecating `remove_short_streams`:
1. Migrate culvert path to new tool by providing/deriving `upstream_area`, or
2. Keep culvert path on `remove_short_streams` and mark as intentionally legacy.

Required file for decision execution:
- `/workdir/wepppy/wepppy/rq/culvert_rq.py`

Acceptance gate:
- culvert tests pass under chosen strategy.

## Phase 5: Regression Validation

Datasets:
1. Existing representative watershed fixtures.
2. At least one known adjacent/chained tributary case.
3. VRT-mode runs (`flovec`/`netful` in `.vrt` workflows) to protect NoDb paths.

Checks:
1. `netful` exists, has non-zero stream-cell count for expected-positive fixtures, and preserves expected outlet reachability.
2. `chnjnt` consistency.
3. downstream hillslope/channel outputs match expected count bounds and required schema fields.
4. deterministic output across repeated runs.
5. culvert pipeline behavior under chosen Phase 4 strategy.
6. negative-case error-contract tests pass (invalid pointer, cycle, no-network).

Acceptance gate examples:
- WEPPpy emulator tests.
- `/workdir/wepppy/tests/culverts/test_culvert_batch_rq.py`.
- packaged-WBT smoke invoking new tool.
- suggested concrete commands:
  - `wctl run-pytest tests/topo/test_terrain_processor_wbt_integration.py`
  - `wctl run-pytest tests/culverts/test_culvert_batch_rq.py`

## Phase 6: Rollout

1. Build/publish new WBT artifact.
2. Deploy compatible WEPPpy commit that calls new tool.
3. Promote only as Pair B (atomic pair promotion).
4. Run representative end-to-end suite in staging-like environment.
5. Promote to production after verification.

## Rollback Plan (Executable)

Rollback must be pair-wise:
1. Roll back WEPPpy and WBT together to Pair A.
2. Do not roll back WBT alone after WEPPpy has switched call sites.
3. Verify recovery with emulator and culvert smoke runs.

Rollback trigger examples:
- topology regressions breaking downstream jobs,
- unexpected hard-fail rate increase,
- nondeterministic outputs,
- VRT-mode regression.

## Risks and Mitigations

Risk: behavior drift in corner cases.
Mitigation:
- parity fixture suite + deterministic rules.

Risk: hidden `remove_short_streams` dependencies.
Mitigation:
- explicit Phase 4 consumer disposition before deprecation.

Risk: rollback mismatch.
Mitigation:
- compatible version-pair runbook and atomic deploy/rollback.

Risk: hard-fail behavior shock.
Mitigation:
- temporary WEPPpy call-site exception sets `fail_if_only_channel_pruned=false`; phase-gate before removing exception.

Risk: VRT regressions.
Mitigation:
- mandatory VRT-mode validation in Phase 5.

## Deliverables Checklist

1. New WBT tool + tests.
2. Wrapper methods in both Python wrapper files.
3. WEPPpy emulator refactor.
4. WEPPpy artifact/provenance doc updates.
5. Culvert consumer disposition completed.
6. Regression report (including VRT and culvert results).
7. Version-pair rollout/rollback runbook.

## Execution Order

1. Phase 0 compatibility design.
2. WBT tool + tests.
3. WBT wrappers + packaged smoke.
4. WEPPpy emulator refactor + docs update.
5. Culvert consumer disposition.
6. Regression validation.
7. Pair-wise rollout.
