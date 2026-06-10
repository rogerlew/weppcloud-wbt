# Adversarial Pre-Submission Review

Review target: `paper/paper.md` in `/workdir/weppcloud-wbt`  
Review date: 2026-06-09 America/Los_Angeles  
Reviewer stance: adversarial author-side review against the supplied JOSS-style checklist.  
Important assumption: a tagged software release and Zenodo DOI will be minted after the items below are otherwise in order. The current absence of a release archive is therefore tracked as a planned final-submission step, not a defect by itself.

## Bottom Line

Follow-up status: the actionable evidence/documentation issues identified below have been addressed after this review, except for the intentionally deferred release/Zenodo DOI step. Do not submit until that final archive step is complete.

1. Resolved: `docs/release-build-install.md` and `AGENTS.md` now use the package id `whitebox-tools-app`.
2. Resolved: `paper/claims-test-matrix.md` now reflects current `IterativeFirstOrderLinkPrune` coverage.
3. Resolved: `RemoveShortStreams --max_junctions` now has a direct regression test and advertised tool metadata.
4. Resolved: the paper now identifies `WhiteboxToolsTopazEmulator` as living in the companion WEPPpy codebase, and the claims matrix cross-references the WEPPpy adapter tests.
5. Pending by plan: there is no current release or Zenodo DOI for this repository. Per author instruction this is expected, but it remains a hard final-submission prerequisite.
6. Pending by plan: the paper cites `wepppy` via the WEPPpy Zenodo concept DOI, but does not yet cite a `weppcloud-wbt` archive DOI. Add this after release minting.

## Evidence Collected

Local commands run from `/workdir/weppcloud-wbt` unless noted:

| Check | Result |
|---|---|
| `git status --short` before review artifact | Clean |
| Public repository URL check | `https://github.com/rogerlew/weppcloud-wbt` is reachable and public; GitHub shows repo files, MIT license, tests, and no releases yet |
| `git shortlog -sne --all` | `83 Roger Lew <rogerlew@gmail.com>` in this clone |
| `cargo check -p whitebox-tools-app` | Passes, warnings only |
| `cargo test -p whitebox-tools-app remove_short_streams_integration -- --nocapture` | Passes: `1 passed` |
| `python -m py_compile whitebox_tools.py WBT/whitebox_tools.py` | Passes |
| `python -m pytest -q tests/test_ifolp_wrapper_smoke.py` | Passes: `1 passed, 2 subtests passed` |
| `cargo test --workspace --lib --tests --bins` | Passes; notable groups include `whitebox-tools-app` 125 tests, `whitebox_common` 39 tests, and `whitebox-raster` VRT/geotiff tests 42 tests |
| `pandoc paper.md --citeproc -o /tmp/weppcloud-wbt-paper.html` from `paper/` | Passes |
| `pandoc paper.md -t plain | wc -w` from `paper/` | 1527 words |
| `cargo fmt --check` | Blocked by pre-existing rustfmt/trailing-whitespace failures in unrelated files; targeted `rustfmt` was run on the changed Rust files |

## General Checks

### Repository

Status: pass, with final-release caveat.

The source repository URL in the paper is reachable and public. GitHub lists the project as `rogerlew/weppcloud-wbt`, forked from `jblindsay/whitebox-tools`, with Rust/Python source, tests, documentation, and a license file.

Adversarial caveat: GitHub currently reports no releases. This is acceptable only under the stated assumption that a release and Zenodo DOI will be minted before submission. Do not submit the paper with only the moving `master` branch as the software archive.

### License

Status: pass.

The repository contains `LICENSE.txt` with the MIT License text. MIT is OSI-approved. `WBT/LICENSE.txt` is also tracked.

Resolved: `LICENSE.txt` now says "code amendments".

### Contribution and Authorship

Status: pass, with a likely reviewer question.

The local commit history in this clone attributes all visible commits to Roger Lew, and the recent changelog records substantial WEPPcloud-specific functionality: VRT support, HillslopesTopaz, FVSlope, RaiseRoads, IFOLP, UnnestBasins updates, wrappers, and tests. A single-author paper is defensible for the fork-specific work.

Adversarial caveat: because this is a fork of WhiteboxTools with substantial upstream code, a reviewer may ask why John Lindsay is acknowledged rather than included as an author. The paper and README already frame Lindsay/WhiteboxTools as upstream foundation and preserve MIT licensing; that is probably adequate, but be prepared to answer that the paper contribution is the WEPPcloud/TOPAZ fork layer, not original authorship of all WhiteboxTools.

## Functionality

### Installation

Status: pass.

The README's source build command is valid:

```bash
cargo build --release -p whitebox-tools-app
```

The linked release/install runbook also now uses the same package id:

```bash
cargo build -p whitebox-tools-app --release
```

The original review found stale `whitebox_tools` package-id references. Those have been corrected; `whitebox_tools` remains only the binary name.

Recommended reviewer commands:

- `cargo check -p whitebox-tools-app`
- `cargo test --workspace --lib --tests --bins`

### Functionality

Status: mostly pass.

The repository has substantial committed test coverage and the CI-style local gate passed. The strongest evidence is the combination of:

- `paper/claims-test-matrix.md`
- `whitebox-tools-app/src/tools/*/*_integration_tests.rs`
- `whitebox-raster/tests/{vrt_parser.rs,vrt_integration.rs,geotiff_window.rs}`
- `tests/test_ifolp_wrapper_smoke.py`
- `.github/workflows/ci-tests.yml`

Adversarial caveats:

- `WhiteboxToolsTopazEmulator` evidence is still in `/workdir/wepppy/tests/topo/test_terrain_processor_wbt_integration.py`, not in this repository. The paper and claims matrix now make that boundary explicit.
- Structured Python error propagation still needs a direct Python-layer regression test if the paper leans harder on `raise_on_error` behavior.

### Performance

Status: pass for the paper, caution for README/docs.

The paper does not make numeric runtime speedup claims. It says VRT support and clipping reduce unnecessary format conversions/full-raster reads, which is a design/IO claim supported by VRT implementation tests and `docs/vrt-support/vrt-performance-notes.md`.

Adversarial caveat: `docs/vrt-support/vrt-performance-notes.md` includes a single warm-cache performance comparison, not a benchmark suite. Do not add quantified performance claims to the paper unless they are backed by repeatable benchmark commands and current outputs.

## Documentation

### Statement of Need

Status: pass.

The paper clearly states that the software solves watershed terrain/network preprocessing needs for WEPPcloud and WEPP workflows, not erosion simulation itself. Target users are explicitly named: hydrologists, erosion-model developers, post-fire risk analysts, watershed scientists, and WEPP-based decision-support maintainers.

### Installation Instructions

Status: pass.

README lists Rust/rustup, clone, build, and tests. `CONTRIBUTING.md` lists Rust stable and Python 3.11+. The automated package story is reasonable for a Rust workspace.

The linked release/install runbook has been corrected to use the package id `whitebox-tools-app`.

### Example Usage

Status: pass.

The end-user docs under `docs/*.ENDUSER.md` include command examples and workflow steps for the WEPPcloud-specific tools:

- `HillslopesTopaz`
- `FindOutlet`
- `FVSlope`
- `RaiseRoads`
- `Watershed` GeoJSON support
- `UnnestBasins`
- `StreamJunctionIdentifier`
- `PruneStrahlerStreamOrder`
- `IterativeFirstOrderLinkPrune`
- `RemoveShortStreams`
- `ClipRasterToRaster`

### Functionality Documentation

Status: pass.

The core functionality is documented at three levels: README summary, end-user guides, and tool/spec docs. `DEVELOPING_TOOLS.md` covers contributor-facing tool conventions. `paper/claims-test-matrix.md` is a strong reviewer-facing asset and now reflects current IFOLP and `RemoveShortStreams` coverage.

### Automated Tests

Status: pass.

Automated tests exist and the local CI-equivalent gate passed. The test suite covers raster IO/VRT behavior, tool integrations, parser contracts, wrapper smoke behavior, and selected WEPPcloud adapter paths.

### Community Guidelines

Status: pass, with minor polish recommended.

`CONTRIBUTING.md` explains contribution and issue-reporting expectations. `SUPPORT.md` explains best-effort support, GitHub issue use, sensitive issue contact, governance, and long-term availability.

Minor issue: add direct issue tracker URL(s) to `CONTRIBUTING.md` and `SUPPORT.md` so reviewers do not need to infer the channel.

## Software Paper

### Summary

Status: pass.

The Summary is clear for a non-specialist technical audience. It explains WEPP, WEPPcloud, and the role of terrain preprocessing before listing the fork-specific tools.

### Statement of Need

Status: pass.

The need is clear and scoped: modernize TOPAZ-style preprocessing for WEPPcloud/WEPP workflows while preserving compatibility with hydrologic terrain concepts and WEPP consumer constraints.

### State of the Field

Status: pass, with two adversarial caveats.

The paper compares against WhiteboxTools, TauDEM, GRASS GIS, ArcGIS hydrology tools, and generic GIS packages. It explains why those tools do not directly emit the TOPAZ-style/WEPPcloud-specific artifacts required here.

Caveats:

- The claim that upstream WhiteboxTools has had "no updates since February 2025" should be verified immediately before submission or softened to avoid a date-sensitive statement. GitHub does mark the upstream repository as legacy, but dates can be challenged.
- The paper says WhiteboxTools provides a "performant raster-processing framework." This is general enough, but avoid turning it into an untested comparative performance claim.

### Quality of Writing

Status: pass.

The manuscript is well structured and within the likely JOSS word budget at 1522 words by `pandoc -t plain | wc -w`. The prose is clear and mostly reviewer-ready.

Minor polish:

- Resolved: the `FVSlope` wording now says "The fork's `FVSlope` implementation..." rather than "The modified `FVSlope` tool..."
- The legacy `Hillslopes` defect paragraph is valuable but long. It is defensible because it distinguishes the contribution, but it is the densest section for non-specialists.

### References

Status: pass, with final DOI caveat.

All citation keys used in `paper.md` appear in `paper.bib`, and `pandoc --citeproc` succeeds when run from `paper/`. The figures referenced by the paper exist.

Final-submission caveat:

- Add the eventual `weppcloud-wbt` software archive DOI after release/Zenodo minting.
- Keep the current `wepppy` concept DOI if intentional, but ensure the paper distinguishes WEPPpy/WEPPcloud from the software under review.

## Required Fixes Before Submission

1. Resolved: package id references in `docs/release-build-install.md` and `AGENTS.md` now use `whitebox-tools-app`.
2. Resolved: `paper/claims-test-matrix.md` reflects current IFOLP and `max_junctions` coverage.
3. Resolved: `RemoveShortStreams --max_junctions` has direct integration coverage.
4. Resolved: the paper and claims matrix identify `WhiteboxToolsTopazEmulator` evidence as companion WEPPpy coverage.
5. Resolved: `LICENSE.txt` typo fixed (`ammendments` -> `amendments`).
6. Pending by plan: immediately before submission, mint the release archive and Zenodo DOI, then add the citation/DOI to the paper metadata or references as required by the journal.

## Recommended Non-Blocking Polish

1. Add direct GitHub issue links to `CONTRIBUTING.md` and `SUPPORT.md`.
2. Consider adding `CITATION.cff` after Zenodo DOI minting.
3. Refresh `docs/vrt-support/vrt-support-acceptance-test-report.md`; it still says VRT integration tests are `3/3`, while the current suite has 14 VRT integration tests.
4. Re-check the upstream WhiteboxTools legacy/no-update statement immediately before submission.
5. If using the JOSS draft action as the final build authority, verify the GitHub Action artifact after the release branch/tag is ready.

## Submission Readiness Judgment

Current state: evidence fixes complete; final archive DOI still pending by plan.

After the planned release/Zenodo DOI is complete, the repository should satisfy the checklist. The strongest remaining reviewer risk is no longer missing functionality evidence; it is ensuring the final archived release, DOI, paper references, and GitHub state all match.
