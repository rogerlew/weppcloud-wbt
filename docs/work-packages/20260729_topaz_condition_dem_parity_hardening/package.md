# TopazConditionDem parity hardening

**Status**: Closed (2026-07-30 01:14 UTC)

**Timezone**: UTC

## Overview

This package turns empirical `TopazConditionDem` parity into a repeatable
golden-hash gate and expands evidence across obstruction modes and NoData /
waterbody geometry. It also establishes the repository-local work-package
workflow used for future multi-step changes.

## Objectives

- Verify canonical TOPAZ FILDEP and RELIEF content, not only final outlet
  behavior or GeoTIFF metadata.
- Exercise obstruction widths 0, 1, and 2 against original TOPAZ.
- Add deterministic synthetic irregular-NoData coverage.
- Add an NLCD-2019-derived waterbody mask over burned-out-harmonic and verify
  exact TOPAZ parity.
- Leave a runnable parity harness and checksummed evidence in the repository.

## Scope

### Included

- Repository work-package scaffolding and tracking.
- Additive stage-output support needed by the parity harness.
- Golden manifests and automated comparison tooling.
- Synthetic and production-derived fixtures.
- Rust, Python-wrapper, algorithm, fixture, and validation documentation.

### Explicitly out of scope

- WEPPpy integration or production deployment.
- Changes to public conditioning defaults or TOPAZ numerical rules.
- Vendoring TOPAZ executables or generated oracle rasters.
- Treating NLCD as authoritative hydrography outside this test purpose.

## Implementation fidelity and evidence

- **Fidelity target**: faithful extraction
- **Authoritative source**: `/workdir/topaz/src/dednm.f90` at
  `116607fc1185800ca78e387454ef1ccd3ffd73b4`
- **Acceptance evidence**: generated-output and fixture evidence
- **Cutover proof**: not applicable; WEPPpy integration is out of scope

## Success criteria

- [x] The harness regenerates Rust FILDEP and RELIEF arrays and verifies their
  canonical scaled-integer hashes.
- [x] Widths 0, 1, and 2 match TOPAZ on a production DEM.
- [x] Synthetic irregular NoData and edge-connected NoData match TOPAZ.
- [x] The NLCD-derived burned-out-harmonic water mask contains water cells and
  matches TOPAZ.
- [x] Repeat-run determinism and all repository Rust tests pass.
- [x] Work-package and user/developer documentation are complete.

## Security and parameterization gates

- **Security impact**: none
- **Dedicated security review**: no
- **Rationale**: offline raster fixtures, local CLI code, and documentation;
  no auth, secret, route, queue, deployment, or external-write surface changes
- **Parameterization change present**: no
- **ADR required**: no

## Dependencies

- Committed production DEM fixtures under
  `test_fixtures/topaz_condition_dem/`.
- Read-only TOPAZ source and executable under `/workdir/topaz`.
- Read-only NLCD retrieval through the configured WMesque service.
- GDAL/NumPy development tooling for oracle conversion and hashing.

## Deliverables

- `tools/validate_topaz_condition_dem_parity.py`
- `test_fixtures/topaz_condition_dem/parity_manifest.json`
- Synthetic and NLCD-derived masked DEM fixtures
- `docs/work-packages/20260729_topaz_condition_dem_parity_hardening/artifacts/validation.md`
- Updated algorithm, fixture, wrapper, changelog, and validation documentation

## References

- `docs/topaz_condition_dem_algorithm.md`
- `prompts/artifacts/topaz_condition_dem_validation.md`
- `prompts/TOPAZ_CONDITION_DEM_EXECPLAN.md`
- `test_fixtures/topaz_condition_dem/additional_parity.json`
