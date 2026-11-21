---
id: SR-202611210022442313144108430
title: "gap analysis of PR 2957 acceptance criteria against tagged tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; PR #2957 head abd6935cc8c29737ed1c8be4618543a6b35ec101; FR-898, FR-899, FR-900, FR-488-AC-18; QSL-2239 and QSL-2243 acceptance criteria"
review_set: subset
---

## Summary

Ticket: QSL-2239 (PR record) and QSL-2243. `quoin matrix --json` reports every FR-898, FR-899 and FR-900 criterion `tagged`, with no run evidence (no tests were run, by order). Plan completion: not assessed. All tags are present, but several oracles are too weak to catch the defect they claim to guard.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-898-AC-2 test checks `text.contains(at)` on the unit text, which always holds; no refusal span is asserted, and cause kinds (`invalid-value`, `wrong-export`, `type-mismatch`) are not asserted; "Q naming an object type" is exercised with a tuple | fsl-semantics/tests/it/hyper_clauses.rs:2049-2105 |
| FND-002 | medium | FR-899-AC-6/7/8 integration tests assert only the verdict (and name for AC-7): no stem `[tick, tick]` / loop `[tick]`, no replay `Reproduced`, no edited `2->1` variant (AC-6), no replay refusal of `HiddenUnfair{fire1}` or the "without fire2 holds" variant (AC-7), and no hand-built counterexample refusal (AC-8); the replay naming check is untested | fsl-analyze/tests/it/refinement_liveness.rs:346-404 |
| FND-003 | medium | No test covers a timed abstract side with a hidden field, the case where the engine skips abstract clock guards (code-review FND-001), and no test has a fair SCC holding both a fair and an unfair candidate sub-loop (code-review FND-002) | fsl-analyze/tests/it/timed_refinement.rs; fsl-analyze/tests/it/timed_refinement_liveness.rs |
| FND-004 | low | `a_weakly_enabled_everywhere_label_does_not_subsume_a_fair_label` is tagged FR-899-AC-6, but it tests antichain label order, not AC-6's fixture | fsl-analyze/src/model_check/refine.rs:3679 |
| FND-005 | low | The FR-898 conflicting-identity request test asserts only the code and kind; it does not assert that the refusal names the trace as FR-898 requires | fsl-analyze/tests/it/finite_hyper_request.rs:191-222 |
| FND-006 | low | `the_first_refusal_in_precedence_order_is_reported` stays tagged FR-488-AC-18 after the alias-alone case it was written for was deleted, so the AC's alias clause has no binder | fsl-package/src/emit/tests/hyper_clause.rs:1145-1163 |

## Scope and evidence

QSL-2239 ACs map to: hyper_clauses.rs (T-081 accept/reject, at fsl-semantics/tests rather than the in-file unit test the ticket names), fsl-eval/tests/it/finite_hyper.rs (holds and violating tuple), fsl-replay/tests/it/hyper_finite_replay.rs plus finite_hyper_request.rs (replay). QSL-2243 ACs map to refinement_liveness.rs (hidden holds and refuted; strong refuted, weak holds), timed_refinement.rs (AC-2/3/4/5 refutations with traces), timed_refinement_certificate.rs (certificate accepted, an altered one rejected) and refinement_certificate.rs. Each is present. The gaps above are oracle strength.

## Verdict

Coverage is tagged throughout. Three medium oracle gaps leave the AC-2 spans, the AC-6/7/8 replay and naming, and the timed hidden-field cases unverified.
