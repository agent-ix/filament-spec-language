---
id: SR-9137
title: "Gap analysis: QSL-2946 caller limits batch (PR 3597)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-255-AC-24, FR-255-AC-25 at 4c7671e65f; Ticket: QSL-2946"
review_set: subset
evaluated_revision: "4c7671e65f"
---

## Summary

Ticket: QSL-2946. FR-255-AC-24 and FR-255-AC-25 are tagged by three tests that cover simulation.max_steps (both routes), schedule.max_demand_points (both routes) and exact_prob.max_states (source route only). The statistical, smt and max_rank settings, the other eight exact_prob settings, the temporal and timed-sampling conflict refusals, the monitor conflict refusal and the package-route probabilistic case have no test.

## Verdict

FIX FIRST. Two acceptance criteria are tagged as covered but are only partly exercised, and two accepted settings have no consumer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-255-AC-25 requires the silent-item case on the source route and the package route alike; the probabilistic test runs the source route only | fsl-analyze/tests/it/package_probabilistic_parity.rs |
| FND-002 | medium | FR-255-AC-24 (each accepted setting changes only its bound) has no test for statistical.max_samples, max_draws, max_cycle_steps, max_sample_steps or eight of nine exact_prob settings | fsl-analyze/src/analyze/request.rs |
| FND-003 | medium | The item/caller duplicate refusal is untested for a temporal item (limits.max_solver_steps, max_solver_time), a statistical limits member, timed_sampling.max_sample_steps, and on the monitor routed-items path | fsl-analyze/src/analyze.rs |
| FND-004 | high | smt.max_solver_steps and smt.max_solver_time are accepted by analyze and monitor but no engine reads them, so FR-255-AC-24 cannot hold for them | fsl-analyze/src/analyze.rs:686 |
| FND-005 | low | certificate_check.max_rank is accepted and carried with no consumer and no test; the FR text names it as carried, so only the missing test is a gap | fsl-analyze/src/analyze.rs:686 |

## Coverage

Read the diff of origin/main...4c7671e65f. Counted tagged tests by name; counted consumers by searching for engines.get calls (exact_prob.*, statistical.*, simulation.max_steps and schedule.max_demand_points have one; smt.* and certificate_check.max_rank have none).

