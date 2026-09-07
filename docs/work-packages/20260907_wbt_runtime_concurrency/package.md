# WBT runtime concurrency

Status: closed, 2026-09-07 UTC. Security impact: low; process-local environment configuration.

Provide a positive-integer `WBT_MAX_PROCS` override without persisting it into
settings.json. Unset preserves persisted max_procs, default -1 (automatic).
WEPPpy default and batch worker services explicitly default the environment value
to 12. No project/NoDb, admission, or queue changes. The operator selected this
option on 2026-09-07; WEPPpy ADR-0051 records provenance and default semantics.

Acceptance: strict validation; settings not contaminated by runtime overrides;
concurrent subprocess limits remain independent; exact fixture parity; worker
Compose models resolve 12 or an explicit operator override. Validate the existing
Python wrapper in a container using an isolated rebuilt binary before any rollout.
Production deployment is outside scope.

Validation passed: 147 app tests, 42 common tests, 7 common doctests; rebuilt CLI
precedence/isolation/error/parity checks; 10 Compose cases across five models;
forest worker-container wrapper with UID 1000/GID 993 and 12 effective workers.
The persisted limit remained 4 and conditioning diagnostics matched reference.
See artifacts/cli.json, container.json, compose.json, and validation.json.
