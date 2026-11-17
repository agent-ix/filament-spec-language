---
id: SR-2026111610120002
title: "Gap analysis of PR 2743 FSL analysis reach"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; FR-623, FR-625, FR-626, FR-627, FR-628, FR-632 ACs against fsl-analyze/tests/it at 39b65c91fc938a547ce59cf47de935203db19e10"
review_set: subset
evaluated_revision: "39b65c91fc938a547ce59cf47de935203db19e10"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-623
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-626
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-627
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-628
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-632
    type: reviews
---

## Summary

Tickets: QSL-2122, QSL-2123, QSL-2129, QSL-2124, QSL-2125. This is a manual check of ACs against tests. Each AC id of FR-623, FR-625 to FR-628 and FR-632 was searched for in `#[trace]` tags under `*/tests/*`, and the tagged tests were read for what they assert. Quoin gap-analysis was not run, because the brief limits this review to focused cargo. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-628-AC-1's reachable half is tagged on `a_possible_claim_proves_with_a_replayed_witness_path_and_refutes`, which refutes `possible (value > 3)` in place of the AC's `value = 4`; the substitution is not noted in the test while the AC text stands | fsl-analyze/tests/it/analyze_state_graph_cli.rs:24-43 |

## Verdict

Coverage holds for the scope this PR claims. Every AC in scope has an un-ignored test that can fail, except the ones split out by the brief:

- FR-625-AC-4: QSL-2301 and W6.
- FR-628-AC-1's `value = 4` half and FR-632-AC-2 and AC-3: QSL-2296.

The checks per AC:

- FR-623-AC-1 to AC-5: `analyze_reduction_cli.rs`, `reduction_proptest.rs` and `reduction_differential.rs`. These cover agreement under every selection, fewer states under symmetry, declared equal to explicit, partial-order not preserving on MLTL, and the typed refusals.
- FR-625-AC-1, AC-2, AC-3 and AC-5: `analyze_cli.rs`, `model_check_oracle.rs` and `analyze_coverage_cli.rs`. These were un-ignored.
- FR-626-AC-1 to AC-3: deadlock with terminal states, the stuck-state counterexample that replays, untimed time-lock as inapplicable, and timed time-lock proved and refuted.
- FR-627-AC-1 to AC-4: `analyze_liveness_cli.rs`, `weak_fairness_decides_liveness`, `weak_and_strong_fairness_separate` and the finite-replay refusal.
- FR-628-AC-2 and AC-3: `analyze_state_graph_cli.rs` and `always_possible_refutes_and_witnesses_repeat`. AC-3's "three sampled witnesses" is QSL-2296 item 7.
- FR-632-AC-1: `zone_items_prove_with_certificates_and_refute_with_exact_delays` and `an_mitl_claim_proves_certified_and_refutes_with_exact_delays`. Assertion weaknesses are in the code-review finding FND-010.

No production code in the diff lacks an owning requirement. The new modules cite FR-623, FR-626, FR-628 and FR-632, and the fsl-route and fsl-replay additions cite FSpec FR-390 and FR-393.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |

Round 2 (head 697dec508): the test comment now says `value > 3` stands in for the AC's `value = 4` (QSL-2296). The new tests tagged FR-626-AC-1 and FR-632-AC-1/AC-3 are covered in the code-review artifact (FND-013, FND-015, FND-008).
