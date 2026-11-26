---
id: SR-967175026302197768900474181941935084871
title: "Code review — QSL-3381 trim proof rows to the stopped graph"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-3381; PR #4015 head 69dd2dec71; fsl-replay/src/probability_certificate/document.rs, document/mdp.rs, document/partial.rs, tools/fsl-probability-check/tests/it/value_bounds.rs"
review_set: subset
---

## Summary

Ticket: QSL-3381. `construct_document` now drops certificate rows outside the rebuilt checker graph for every `ProbForm::Reach` and `ProbForm::ExpectedReward` claim, and the `keep_graph_values` special case on the partial-lower path is removed. The rust-review lane is folded into this file.

The old condition was `event.is_some() || reward.is_some() || cyclic_dtmc`. `subject_graph_with_identity` sets `state_event` only for `until`/`always`, sets `reward` only for `ExpectedReward`, and admits `cyclic_dtmc` only for Reach and ExpectedReward. So the new condition matches the old one except that it adds acyclic `eventually`, under both a workload and every scheduler. The bounded `Probability { Formula }` form stays unfiltered, as before. Quantile, fraction, steps and MeanTime are `Unsupported` in this builder and never reach the filter.

Missing rows: the checker (`check_document_with_control`) rebuilds its graph with the same `subject_graph_with_identity`, the same `document.cyclic_dtmc` and the same fairness flag. The producer filter keys on that same graph, so it can only drop rows for states the checker never visits. `certificate_values` rejects any row outside the graph. The partial-lower `explored_subject` route is reached only when the whole graph hits `max_states`, and then `construct_document` has already failed, so the filter never runs there.

False claims: the checker is unchanged. A dropped row is one the checker would refuse anyway, so a document cannot gain acceptance this way. The new negative test confirms that a threshold miss, a changed value and a row for the state past the target are still rejected.

Callers of the removed `keep_graph_values`: it had one call site (partial.rs), gated on `!cyclic_dtmc && Workload && eventually`. That route now goes through the filtered `construct_document`, and `ranking` keys equal the graph keys there, so the result is the same. A fair `eventually` is admitted only under every scheduler (`fairness::admit`), so the gate never reached the fair-MDP route. The cyclic and fair MDP `construct` routes get graph-keyed `Complete` values from `rows` → `certificate_values`. The existing test `a_partial_lower_over_an_acyclic_eventual_names_only_the_stopped_graph` still covers the removed path.

## Verdict

**PASS with low findings.** The PR is mergeable. No soundness or coverage defect was found. The three findings are a doc typo, formatting in the new test code, and the PR's related-code list not naming every route and how its result was obtained. The `#[implements("FR-201-AC-11", "FR-1042-AC-25")]` on `construct_document` is correct: it builds the `reachability/v1` exact document (FR-201-AC-11) and the stopped-graph rows that the AC-25 bounds documents rest on. `quire trace` reads both tags as `implements`. `use ix_trace_rs::implements;` is already imported in document.rs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Rewrapped doc comment of `partial_lower_probability_proof_document` lost the space after a sentence: `(FR-1042 "Document").Over that part` | fsl-replay/src/probability_certificate/document/partial.rs:93 |
| FND-002 | low | New test code is not rustfmt-clean: two blank lines after `past_target_key`, and the nested `proof(&acyclic(..).replace(..))` call is laid out differently from rustfmt's output; CI runs `cargo fmt --all -- --check` | tools/fsl-probability-check/tests/it/value_bounds.rs:1534-1559 |
| FND-003 | low | PR's related-code list (QSL-2846 rule 2) does not name the route of the "cycle past the target" probe (cyclic DTMC vs cyclic MDP), does not say whether the fraction, steps and quantile probes were also run on the unfixed head, and does not list the fair-MDP and partial-lower (`max_states`) routes that the removed `keep_graph_values` sat beside, with their results | PR #4015 body "Related code checked" |

## Examined units and bindings

```yaml
clean: false
scope:
  - {id: FR-201-AC-11, path: spec/functional/FR-201-check-a-probability-certificate.md, role: examined, excerpt: "Minimum AtLeast, maximum AtMost and resolved-workload exact proofs pass the `reachability/v1` profile checker at an attained threshold; a threshold strictly beyond the certified initial value fails."}
  - {id: FR-201-AC-10, path: spec/functional/FR-201-check-a-probability-certificate.md, role: examined, excerpt: "Altering a value fails equality; removing or breaking positive-value rank progress fails ranking even when equalities hold."}
  - {id: FR-1042-AC-25, path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md, role: examined, excerpt: "Over the acyclic chain under a workload in which the undelivered `s` moves `1/2` each to delivered `g` and to the stopped terminal `f` (value `1/2`), `probability >= rational(2,5) [eventually holds(m.delivered)]` takes the unbounded `reachability/v1` base: `lower` `s` `7/16`, rank 1, and counterpart `upper` `s` `9/16` are accepted; ... Where `max_states` stops exploration past a target but admits the graph that stops there, EN-5's `lower` keeps that base, names that graph's states alone and settles `exploration: Partial` with no counterpart."}
bindings:
  - {test_id: value_bounds::an_exact_proof_past_a_target_is_accepted, ac_id: FR-201-AC-11, trace: correct}
  - {test_id: value_bounds::an_exact_proof_past_a_target_is_accepted, ac_id: FR-1042-AC-25, trace: correct}
  - {test_id: value_bounds::a_false_claim_past_a_target_is_still_rejected, ac_id: FR-201-AC-10, trace: correct}
  - {test_id: value_bounds::a_false_claim_past_a_target_is_still_rejected, ac_id: FR-201-AC-11, trace: correct}
```
