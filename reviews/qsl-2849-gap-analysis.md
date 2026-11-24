---
id: SR-3639002
title: "gap-analysis of PR 3639 (QSL-2849, QSL-2969)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR 3639 head bc7fa1b398; FR-126-AC-25, FR-126-AC-26, FR-203-AC-16, FR-247-AC-5, FR-264-AC-9 and the code they cover"
review_set: subset
---

## Summary

Ticket: QSL-2849. Planless gap analysis scoped to the PR. `quire matrix --scope . --format
json` (quire 0.52.0) reports all five new criteria as `tagged`. FR-264-AC-6 keeps five
tagged tests after three tests were re-tagged to AC-9. Production code with no owning
requirement: none. Stubs: none.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-126-AC-26 names check_model and check_refinement, but all three tagged tests call only the admit_initial helper; reverting check.rs or refine.rs to key the whole set leaves every tagged test green | fsl-analyze/src/model_check/initial.rs:50-86 |
| FND-002 | medium | FR-203-AC-16's settlement (Incomplete(LimitReached{CertificateMaxWorkingBytes, bound, certificate_check.max_working_bytes})) has no test at any of its five mapping sites, and the reward, stationary-distribution and bias dense_system call sites are untested | fsl-analyze/src/analyze/exact.rs:302-324 |

## Coverage

- Criteria examined: 5 new (all tagged), plus FR-264-AC-6, FR-126-AC-23 and FR-247-AC-4 as context.
- Semantic review: not run as a separate fan-out; test-can-fail checks were done by reading the code (see SR-3639001 verdict).
- Plan completion: not assessed

## Verdict

CONDITIONAL. Two medium coverage gaps: the end-to-end wiring that the ACs name is not
exercised.

## Dispositions

Round 1, reviewed head 1946b4382f.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Integration tests run check_model, check_refinement and the state-graph explorer under max_states 0 and count keyed states; the tests fail if any entry stops calling admit_initial |
| FND-002 | fixed | every_dense_call_site_checks_the_ceiling covers all four dense_system sites; a_dense_solve_stop_settles_as_the_limit_or_the_cancellation covers the unbounded_failure mapping, which is the reach/until/always/reward/fair-eventual path |

Round 2 reviewed `1ac4da2a725290e8986f265eeb45bf73917e7d0d`. The PR requires another fix round.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | still-open | The timed deadline, expected-elapsed and both long-run limit settlements now have request-level checks, and the untimed and timed finite-horizon callers share bounded_solve_failure. Cancellation coverage still stops short of the finite-horizon FhError::Cancelled arm (exact.rs:355) and the independent workload UnboundedError::Cancelled arm (exact/long_run.rs:182). The pivot oracle stops at solve_linear and the settlement unit calls unbounded_failure directly. None of the four new request cases injects cancellation into the dense solve. A regression mapping either remaining arm to a fault would escape these tests. |

Round 3 reviewed `bb649d2006b756bcbd7358a4fbd1474760ab5c86`. All previously open findings in this artifact are fixed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | The timed-deadline and workload long-run integrations now cancel only at a Bareiss pivot poll. They settle the actual engine report and check the exact Cancelled cause, so the outer analyze dispatcher cannot mask an incorrect adapter mapping. Both passed on this head. The timed/expected-elapsed and both long-run byte-limit tests also passed. |

Round 4 reviewed `b05ca318826032f4fb39bf61918f9e3993f51c72` against `608e201178a28f96a0b4e738114ce7227cb66c7b`. BLOCKED by the target handoff coverage gap. Earlier dense cancellation and initial wiring coverage findings remain closed.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | still-open | The FR-644-AC-16 command test exercises only a malformed target, a missing caller setting and a setting supplied without a target. The two other AC-16 binders test local decoding and setting visibility. Dropping the accepted target and capacity leaves all three green. Exercise a valid target with an activation bound through the FSL command-to-driver boundary. The AC-17 rejection test also misses the required exact diagnostic path and message. |

Round 5 reviewed `b0078a167be4aa35c8fce8d1ef8bfcbec4094c6e` against `dccaf11c3d00f51738761f26cb72ef3124415e02`. BLOCKED by the incomplete exact diagnostic assertion. The positive driver handoff now has coverage.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | still-open | Positive finite and timed command-to-driver callback tests now exercise the accepted target and activation capacity. The AC-17 portion still checks only the “invalid MonitorTarget” substring and details.path at cli_contract/monitor.rs:310–319. The actual emitted message still violates the exact string; all of those assertions stay green. |

Round 6 reviewed `2dc5f4817dfad4059ac12990eff97e0176311dba` against `16c724b092132cce8283860c9f67fd327f3db064`. PASS. The monitor handoff, exact diagnostic and earlier gap findings are closed. The dense tests detect the new adapter regressions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | The command test directly compares the full diagnostic message and details.path. The two malformed raw cases check that a driver callback never ran; positive finite and timed handoff tests remain. |

Round 7 reviewed `e77768d387d01eee1ed13483835e60b2ce249000` against `ca74568372bdda843b1808c6fee4e3c5c56a8ee7`. PASS. The earlier gap findings remain closed. The EN-5 suite includes an independent expected-elapsed pivot-cancellation regression.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The coder dropped the analyze-level test on the claim that certificate storage always trips max_working_bytes first. That is false: until/always/long-run runs charge no storage before the solve, and eventual/reward runs charge only linear key bytes against a quadratic dense system. The three inline FhError/long_run MaxWorkingBytes and Cancelled arms remain untested, and a discriminating test is reachable | fsl-analyze/src/analyze/exact.rs:527-532 |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The FR-644-AC-16 command test exercises only a malformed target, a missing caller setting and a setting supplied without a target. The two other AC-16 binders test local decoding and setting visibility. Dropping the accepted target and capacity leaves all three green. Exercise a valid target with an activation bound through the FSL command-to-driver boundary. The AC-17 rejection test also misses the required exact diagnostic path and message. | fsl-command/tests/it/cli_contract/monitor.rs:290-297 |
