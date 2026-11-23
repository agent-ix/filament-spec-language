---
id: SR-599009
title: "Gap analysis — QSL-2910 PR #3559 caller limits at analyze, monitor and replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; git diff origin/main...91a4787ba, FR-255-AC-24, TC-721 step 11"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: references
---

## Summary

Ticket: QSL-2910 (QSL-2912 pointer). Planless gap analysis scoped to the PR diff. Before this PR no test carried FR-255-AC-24; after it, six unit tests in `fsl-command/src/command/limits.rs` carry `#[trace("TC-721", "FR-255-AC-24")]`. The implementation covers the replay list (all but `s1.diagnostics`, which is not a `Setting` row) and the monitor and analyze source/package/normalization/observation/identity/I2/dependency/intake/accounting settings, plus `monitor.*`, `replay.input_bytes` and analyze `emit.output_bytes`. It does not cover the rest of the analyze and monitor lists. The coder's partial-coverage list is accurate and complete: no further omitted setting was found.

Plan completion: not assessed

## Verdict

**FAIL** — FR-255-AC-24 is tagged as backed while the enumerated analyze and monitor sets are not implemented.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-255-AC-24 is tagged by six tests but its analyze and monitor lists are only partly accepted | fsl-command/src/command/limits.rs:169-236,529-563 |

FND-001: settings FR-255 assigns that the PR refuses. Absent from the `Setting` table (so no entry point can accept them): `s1.diagnostics`, `exact_prob.*` (9), `statistical.*` (4), `zone_check.*` (4), `schedule.max_demand_points`, `hybrid.max_solver_steps`/`max_solver_time`, `smt_encoding.*` (6), `smt.max_solver_steps`/`max_solver_time`, `certificate_check.*` (4 applicable), `simulation.max_steps`; `plan_search.*` is also absent but is not assigned to any of the three operations. Present in the table but not exposed: analyze `admission.*` (3), `library.*` (4), all ten `model_check.*`, `expansion.max_candidates`; monitor `admission.*`, `library.*`, and with routed items `model_check.*`, `expansion.max_candidates`, `emit.output_bytes`. None is cheap: admission meters are built with `PopulationAdmissionLimits::default()` inside the spine (`fsl-replay/src/spine/call.rs:1843`), library limits are not threaded into the analyze or monitor compile, model-check and expansion bounds come from item parameters rather than `AnalyzeLimits`, and each absent family needs new `Setting` rows plus engine-limit mapping. The safe direction holds (a missing setting is refused before work), so a partial merge is acceptable only if the AC-24 tag is withdrawn or narrowed and the remaining settings are recorded as remaining work on QSL-2910.

## Coverage

- Matrix: `quire matrix --scope . --format json` and `quoin matrix --repo . --json` both failed to load the module registry (quire 0.52.1, engine 0.62.2; `~/.ix/filament/modules/spec-artifacts-process/manifest.yaml` v0.2.0: `traceability.trace_tags: unknown field legacy`). Fallback: grep index of `#[trace]` tags for FR-255-AC-24 at HEAD and origin/main, declared as such.
- Reverse gap: the new public items (`CommandLimits`, `LimitedOperation`, `SettingUnavailable`, `MonitorPreparationLimits`, `PreparationBound`, `PreparationRefusal`, `replay_with_limits`) are owned by FR-255-AC-24 and the FR-255 setting-table rows for `monitor.*`. `MonitorPreparationLimits`/`PreparationRefusal` have no FSL production consumer (see code-review SR-599008 FND-003).
- Stubs: none found in the diff; no `todo!`/`unimplemented!`.
- Semantic review: skipped (not requested).
- Plan completion: not assessed.
