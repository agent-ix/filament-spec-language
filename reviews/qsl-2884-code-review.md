---
id: SR-967176
title: "QSL-2884 PR 3590 code review: several racing given delays in the digital workload checker"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@99104cefc512ce65d96a0f37916a967f5b5748f4; fsl-replay/src/probability_certificate/document/timed.rs, fsl-analyze/tests/it/probability_from_source.rs"
review_set: subset
---

## Summary

Ticket: QSL-2884. PR 3590 removes the `racers.len() > 1` refusal in the
checker's `digital_workload_choices` and races every given delay together:
the product of each racer's window-conditioned support, the least draw wins,
the identities tied at it ordered by the workload's weights. It un-ignores
`pta_from_source` and re-points the five `probabilistic_on_each_*` ignores at
QSL-2979. The rust-review lane is folded in.

## Verdict

CONDITIONAL. The checker re-derives what the producer emits rather than
accepting it. Checked against `workload_moves` and `free_delays_moves` in
`fsl-analyze/src/exact_probabilistic/digital.rs`:

- Conditioning, normalisation and exactness match. Both sides use the same
  window filter, divide each weight by the in-window total, drop a racer with
  zero in-window mass, and multiply exact `Rational` shares. So `P(g)` sums to 1.
- Least draw and tie set match. `tied_given` filters the draws equal to the
  minimum. In the mixed path, ties span the given and the free identities.
- Tie ordering matches. `weighted_orderings` with `operation_shares` mirrors
  the producer's `each_ordering`, including the per-operation count division.
  A weight of 0 is refused before this point by `admit_workload`.
- Truncation at the horizon matches: `race_states` clamps the index to the
  last trajectory state, as `race_edges` does.
- Racer and row order differ (window order here, `schedule_key` order in the
  producer). This is harmless: graph verification and witness-choice
  derivation do not depend on order.
- The ignore reasons are accurate. QSL-2979 exists and is In Progress.
- rustfmt (edition 2021) is clean and there is no foreign reformat.

The open findings are a missing discriminating test and an unmetered product
build.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new several-given-racer branch has no discriminating checker-level test. `pta_from_source` races two point supports `{1 ms: 1}` under equal `Even` weights, so a checker that ignored workload weights on a tie, or mis-weighted unequal supports, would still be accepted. No test covers unequal weights, 3+ racers, given-plus-free with two given racers, or rejection of a tampered certificate in this branch. | fsl-replay/src/probability_certificate/document/timed.rs:3463-3506, fsl-analyze/tests/it/probability_from_source.rs:437 |
| FND-002 | medium | The joint product `joint` is materialised in full (prod of support sizes, each entry a `Vec<u64>` of length k plus a `Rational`) with only `poll(cancel)` and no meter charge or limit check. `charge_work` runs only afterwards, once per joint entry. Other checker enumerations check the edge limit before they build (the FreeDelays function listing), so here an adversarial certificate can force unbounded allocation before any resource limit trips. | fsl-replay/src/probability_certificate/document/timed.rs:3469-3481 |
| FND-003 | low | Stale doc comments. The header still describes "the race of the one identity whose delay a `discrete` distribution gives", and the FreeDelays comment says "a row is one drawn delay". Both are now a drawn vector over several given racers. | fsl-replay/src/probability_certificate/document/timed.rs:3324-3326, fsl-replay/src/probability_certificate/document/timed.rs:3508-3510 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | Fix commit 3685ed9e deletes `assert_eq!(built.kind, ProductKind::Dtmc);` from `discrete_delays_under_a_workload_are_a_chain` without explanation. That assertion is the only check of FR-204-AC-3's "is a DTMC over digital timed states" clause, so the AC is now partly untested. The PR does not change the producer, so either the assertion still passes (an unjustified weakening) or it already failed at the base (an AC-3 defect hidden instead of ticketed). | fsl-analyze/tests/it/digital_clocks.rs:547-553 |
| FND-005 | low | Three supplementary tests in timed_product_certificate.rs (`three_tied_given_racers_resolve_by_unequal_weights`, `the_tie_weights_change_the_checked_value`, `a_certificate_of_other_tie_weights_is_refused`) carry `#[trace("TC-3204", "FR-204-AC-18")]` but test a model and numbers other than AC-18's `Trio`/`Skew`. This inflates AC-18's matrix coverage; they should be untagged or tagged to a criterion they assert. | fsl-analyze/tests/it/timed_product_certificate.rs:1741-1793 |
| FND-006 | low | The forged-certificate negatives in timed_product_certificate.rs assert only `!accepted(..)`, which any `Err` passes (`Unsupported`, `Malformed`, `Limit`), and `a_certificate_of_other_tie_weights_is_refused` discards the rule with `let _ = rejected_item_rule(..)`. A forged document refused for the wrong reason still passes. Assert `InequalityFails`, as the digital_clocks.rs helper does. | fsl-analyze/tests/it/timed_product_certificate.rs:1784-1793, fsl-analyze/tests/it/timed_product_certificate.rs:1825 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Tests added for unequal weights, 3 racers, two given plus one free, a forged other-weights certificate and AC-18/19/20 (digital_clocks.rs, timed_product_certificate.rs). Values recomputed by hand: 121/160, 53/96, 5/8, 11/40. |
| FND-002 | fixed | The product size is computed with u64 saturating_mul before `joint` is built and refused above `limits.edges` with `Limit{Edges}`. It saturates rather than wraps, so an overflowing product is refused for any edge limit below u64::MAX. |
| FND-003 | fixed | Both doc comments now describe several given racers and a drawn vector per row. |
