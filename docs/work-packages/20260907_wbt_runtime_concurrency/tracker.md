# Tracker

- [x] Assess current shared-settings behavior and agree default semantics.
- [x] Implement and test runtime override and persistent/runtime separation.
- [x] Wire default and batch worker Compose configuration and ADR.
- [x] Validate rebuilt CLI, container wrapper, isolation, and Compose resolution.
- [x] Close documentation and evidence.

Decision: runtime environment overrides persisted settings for execution only.
Unset standalone WBT remains automatic absent settings; Compose supplies 12.

## Verification and Outcome

Closed with all gates passing; see artifacts/validation.json. Compose defaults
to 12, while standalone unset WBT preserves settings/default automatic behavior.
Runtime overrides are excluded from CLI persistence. No production deployment.

The container probe uses fail_on_unresolved=True, as required by WEPPpy's
conditioning contract, and the existing flat-edge fixture resolves all pits.
