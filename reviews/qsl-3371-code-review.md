---
id: SR-2026112517540001
title: "Code review — filament-spec-language QSL-3371 value bounds over an acyclic eventual base"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/probability_certificate/document.rs, fsl-replay/src/probability_certificate/document/partial.rs, fsl-replay/src/probability_certificate/document/mdp.rs, tools/fsl-probability-check/tests/it/bounded.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs, tools/fsl-probability-check/tests/it/value_bounds.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-1042", type: references }
---

## Summary

Ticket: QSL-3371. PR agent-ix/filament-spec-language#3950 at head `bc3b621fc52f64ca1e81828b1af5f47cf767abfd`, diff `origin/main...HEAD` (two commits). Soundness review of the independent probability certificate checker (`fsl-replay` `check_document_bytes`), with the rust-review lane folded in. The four newer `origin/main` commits (QSL-3311, QSL-3338, QSL-3347 group, QSL-3348) touch no `probability_certificate` file and no `fsl-probability-check` test.

No soundness defect was found. The checker admits `lower`/`upper` over the unbounded `reachability/v1` base only when the document is not `cyclic_dtmc`, the objective is `Workload` and the claim is `eventually`; `subject_graph_with_identity` still runs `topological_order` for every non-reward, non-cyclic graph, so the base is checked acyclic. On an acyclic graph the inequalities alone bound the value by induction on height; the added progress-ranking check is stricter than soundness needs.

## Verdict

**CONDITIONAL** — two low findings, neither a soundness hole.

Refusal evidence, by reading `document.rs` 2210–2572 and `probability_certificate.rs` 2383–2440 and confirmed by the passing tests below:

- Wrong `lower` value: `x(s) <= Q(x,s)` at each named state; terminals must equal 1/0 exactly; unnamed successors count 0 (`certificate_values` partial fill); `InequalityFails`, then `ThresholdMissed` at the initial key for the claim side.
- Wrong `upper` value: `certificate_values(.., allow_partial_lower=false)` requires every graph state; `Q(y,s) <= y(s)` is a pre-fixed point, sound on any graph.
- Missing or forged ranking: a named positive non-terminal state with no rank is `RankMissing`, a missing member is `Malformed` at `ranking`, and a flat or non-decreasing rank is `RankBroken` (progress needs a positive-probability successor of positive value and strictly smaller rank). A forged rank cannot raise a value, since acyclicity plus the inequalities already bound it.
- States outside the stopped graph: any `values` key not in the rebuilt graph is `Malformed` at `values`. The explored (max_states) path is not open to this base: `explored_subject` passes `cyclic_dtmc=false` while `partial::cyclic_dtmc` gives `Some(true)` for `eventually`, so `subject_graph_with_identity` returns `Unsupported` and the original `States` limit error is returned.
- MDP / every scheduler: `claim_objective` gives `Minimum`/`Maximum`, so `acyclic_eventual` is false and `bound_base` false: `Unsupported`. The `CyclicMdp`/`FairMdp` dispatches are unchanged.
- Fair: a non-empty fairness set is refused at line 2406 (`Unsupported`); `FairMdp` dispatch is unchanged; `fairness::admit` admits fair `eventually` only under `every scheduler`, so the producer's `Workload` guard never sees a fair claim.
- Workload states with more than one choice are refused `Unsupported` in `check_document_equalities`.

Producer/checker mirror: `fsl-analyze` `analyze/exact.rs` 2121–2160 calls `partial_lower_probability_proof_document`, writes the `lower` and self-checks it with the same checker (`check_probability_certificate_document_as`, Claim role). `tools/fsl-probability-check` source and package routes both end in `check_recompiled` -> `check_probability_certificate_document_as` -> `check_document_bytes`. The producer change (`name_lower_rows`) only drops rows; it cannot widen what the checker accepts.

The replaced `certificate_kinds.rs` measure case was unsound. In `link`, `send_a` is the only enabled action: 1/3 back to the undelivered state, 1/3 delivered, 1/3 stopped. Pr(delivered within 4 steps) = 1/3 + 1/9 + 1/27 + 1/81 = 40/81 < 1/2, the same under every scheduler. FSpec FR-411 lets a prefix prove only what is true, so a `Proved` for `>= 0.5` at `max_states` 1 would be wrong. No AC requires that case: FR-201-AC-51 lists other certificate kinds, and FR-1042-AC-11 lists the chain, `shifted`, reward and timed partial proofs. The lost EN-5 partial-measure-proof coverage is the coder's known separate defect.

Tests run at this head (name-filtered, `--skip on_each`, CARGO lock C, memory checked):

- `fsl-probability-check --test it -- value_bounds:: certificate_kinds:: reward:: cyclic state_predicate:: bounded:: timed_product roundtrip`: 93 passed, 2 failed. `value_bounds_certificates_are_accepted` fails at `certificate_kinds.rs:353` (`shifted` quantile 0.4, `Incomplete(ExactProbMaxStates, 10)`, QSL-3352). `an_unbounded_fair_eventual_certificate_is_accepted` fails with `ValueOutsideType` (QSL-3370, an untouched test).
- `fsl-replay --test it -- probability_certificate::`: 6 passed. `fsl-replay --lib -- probability_certificate:: --skip instances`: 69 passed.
- `fsl-analyze --test it -- value_bounds partial_product past_the_rational_budget interval_document backend_provider_schema external_always_path_set`: 19 passed.

The new tests can fail. On main, `bound_base` is false for this base, so the `lower` would be `Unsupported`. The `flat` case fails if the `|| acyclic_eventual` progress term is removed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | A `lower` naming a subset of a stopped graph that fits `max_states` (now reachable for the acyclic eventual base) accepts `ranking` rows for states it does not name; FR-1042 "Document" requires ranking rows to cover exactly the named states the table requires. The extra-row check runs only when `explored`. Not a soundness hole: an unnamed state's value is 0 and never shows progress. | fsl-replay/src/probability_certificate/document.rs:2434 |
| FND-002 | low | The refusals the PR states ("every-scheduler acyclic eventual bounds stay refused") and the fair refusal for this base have no test: no test in the run set offers a `lower`/`upper` over an acyclic `eventually` under `every scheduler`. Deleting the `objective == ProbabilityObjective::Workload` term would fail no test. | fsl-replay/src/probability_certificate/document.rs:2398-2400 |

## Coverage

- Rust lane: no `unwrap`/panic added to production code; `name_lower_rows` is `pub(super)`, takes no locks and does no I/O; `BTreeSet` borrows are scoped. No integer conversions added.
- Semantic soundness was checked against FR-1042 "Bases", "Document" and "Conditions" (workload row), FR-201, FSpec FR-411 "Evidence on a partial product" and FSpec FR-412 `lower`, as of filament-specification origin/main `53fe180bdc`.
