---
id: SR-1479
title: "QSL-2082 Rust test review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2082; fsl-analyze/tests/it/{exact_probabilistic_item.rs,hyper.rs,hyper_settle.rs,reduction.rs,reduction_differential.rs}"
review_set: subset
---

## Summary

Reviewed the four new substantive analysis/replay tests and two existing HP-4 trace additions at the frozen PR3159 head. Code-review included the Rust lane, rust-style testing/implementation checks, duplication, integrity and assertion-to-contract review. No defects found in the assigned diff.

## Verdict

PASS for the assigned diff and asserted criterion portions; no new defect found. This does not close QSL-2082 or certify entire criteria/repository coverage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed frozen head 8dea5c6691059b7df58b93ef8c29f5bf7e524fe7, only fsl-analyze/tests/it/{exact_probabilistic_item.rs,hyper.rs,hyper_settle.rs,reduction.rs,reduction_differential.rs}. Existing helpers inspected only to trace the new assertions into actual analyze, settlement and subject replay. Four author-reported focused tests passed; accepted without repetition. No runtime checks run by reviewer. Final affected-module gate remains for the coder. Scoped Clippy is blocked by the accepted four unchanged library findings; manually reviewed changed Rust. No production API, dependency, CI, compatibility or vendoring changes. No applicable assurance profile identified in the assigned context.

The hyper test covers the truth/reduction distinction and concrete leaky witness, supplementing accepted existing FR-176 evidence; it does not newly establish all state/SCC counts or the exact ADR witness shape. The reduction tests assert whole concrete counterexample equality and real refuted/decisive-counterexample settlement/replay; they do not claim all FR-161 permutation/idempotence arms. The new EN-5 test covers only the certified EN-5 portion of FR-290-AC-1, with exact 24/25 and a stricter-bound refutation. Existing HP-4 tests directly assert source classification, unsupported causes and routed disposition without an engine record. Partial criterion bindings are supplements, not full acceptance claims.

QSL-2082 stays partial: existing QSL-998/QSL-1056, QSL-2695 emitted-content identity, and the combined FR-290 four-record/C09 acceptance remain pending. Withdrawn package tests are absent and not evidence. Existing Done 1028/986/988/1047/1053 accepted without audit. No proof/Kani gate, fake certificate, package-name/content-identity substitution, matrix or baseline rerun.

Plan completion: not assessed

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-160-AC-4",
      "path": "spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md",
      "role": "examined",
      "excerpt": "ADR-021 §7.1's `fair weak whole` refutation from the symmetry run settles `refuted`, `decisive-counterexample`, after replay, and its record names no reduction, and its counterexample, the three `upd(b)` steps from `(0,0,0)`, equals the unreduced run's counterexample over every member."
    },
    {
      "id": "FR-161-AC-4",
      "path": "spec/functional/FR-161-concretise-a-reduced-counterexample.md",
      "role": "examined",
      "excerpt": "FR-156-AC-3's partial-order counterexample and FR-158-AC-1's constrained counterexample are emitted unchanged and replay. No concretised counterexample in AC-1 to AC-4 holds a canonical state, a permutation or a reduction name. Each one is a `TemporalCounterexample` whose every member equals the corresponding member of an unreduced run's counterexample. Concretising AC-1 twice gives equal counterexamples over every member."
    },
    {
      "id": "FR-173-AC-2",
      "path": "spec/functional/FR-173-classify-hyper-clauses-into-forms.md",
      "role": "examined",
      "excerpt": "`Other`, each naming its rule: `exists trace b of V forall trace a of V`; `Opaque` with body `always eventually holds(v.l @ a = v.l @ b)`; `Opaque` with `fair { weak V::Vault::step }` on `b`; §13's clause with an `exists` variable. Each settles V-8 `unsupported-requested-capability` in negotiation with no engine run."
    },
    {
      "id": "FR-176-AC-1",
      "path": "spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md",
      "role": "examined",
      "excerpt": "ADR-023 §8.1, no reduction: the leaky product has 10 reachable states and two accepting SCCs, and returns `Violated` with a `Lockstep` counterexample of the §8.1 table's shape, `a` from `(0, 0)`, `b` from `(1, 0)`, joint steps `(step(0), step(0))`, loop entry 1. The secure product has 8 states and returns `Holds{Exhaustive}`."
    },
    {
      "id": "FR-290-AC-1",
      "path": "spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md",
      "role": "examined",
      "excerpt": "The FR-281-AC-1 record's certification is `certified`, naming the EN-5 checker;"
    }
  ],
  "bindings": [
    {
      "test_id": "exact_threshold_equality_is_certified_but_a_stricter_threshold_is_refuted",
      "ac_id": "FR-290-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "an_unreachable_asymmetric_disjunct_keeps_truth_but_changes_proof_reduction",
      "ac_id": "FR-176-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "an_other_clause_settles_unsupported_at_negotiation_with_no_run",
      "ac_id": "FR-173-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "routed_hp4_hyper_item_settles_unsupported_requested_capability",
      "ac_id": "FR-173-AC-2",
      "trace": "correct"
    },
    {
      "test_id": "concrete_constraint_and_partial_order_refutations_keep_the_unreduced_witness",
      "ac_id": "FR-161-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "fair_whole_symmetry_preserves_every_counterexample_member_and_replays",
      "ac_id": "FR-160-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "fair_whole_symmetry_preserves_every_counterexample_member_and_replays",
      "ac_id": "FR-161-AC-4",
      "trace": "correct"
    }
  ]
}
```
