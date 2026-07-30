# Work packages

Work packages are self-contained records for multi-step or high-risk work in
this repository. Name package directories `YYYYMMDD_slug` using the Pacific
start date and use UTC timestamps inside package documents.

Each package contains:

    package.md
    tracker.md
    prompts/active/
    prompts/completed/
    artifacts/

`package.md` defines scope, security triage, fidelity target, and measurable
exit criteria. `tracker.md` is the living task board, decision log, risk log,
and verification record. Active ExecPlans live under `prompts/active/`; move
them to `prompts/completed/` with an outcome section when the package closes.
Durable generated evidence belongs under `artifacts/`.

For every active package:

1. Link it from `/PROJECT_TRACKER.md`.
2. Point `/AGENTS.md` at the active ExecPlan.
3. Keep the ExecPlan's `Progress`, `Surprises & Discoveries`, `Decision Log`,
   and `Outcomes & Retrospective` sections current.
4. Update `package.md`, `tracker.md`, `PROJECT_TRACKER.md`, and the prompt
   lifecycle before handoff.
5. Record security impact as `none`, `low`, or `high`. A high-impact package
   requires a dedicated security review artifact.
6. Record whether defaults, formulas, thresholds, conversions, or fallback
   rules change. Such parameterization changes require an ADR before closure.

An ExecPlan must be self-contained for a contributor with only the repository
and plan. It must state purpose, context, milestones, concrete commands,
acceptance criteria, recovery behavior, dependencies, and durable artifacts.
Passing unit tests alone is insufficient when the package promises empirical
parity; checksummed generated-output evidence is required.
