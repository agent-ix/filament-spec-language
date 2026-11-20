---
id: SR-1480
title: "QSL-2082 scoped requirement and evidence review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2082; fsl-analyze/tests/it/{exact_probabilistic_item.rs,hyper.rs,hyper_settle.rs,reduction.rs,reduction_differential.rs,hyper_witness.rs}"
review_set: subset
---

## Summary

Reviewed the four new substantive analysis/replay tests and two existing HP-4 trace additions at the frozen PR3159 head. Reconciliation: manual exact current criterion-to-changed-assertion review only, as explicitly assigned. Computed matrix and repository-wide reverse-gap/stub audit intentionally excluded by the dispatch; this is not an engine-backed repository assurance PASS. Scoped reverse ownership and hollow-evidence checks found no introduced unowned behavior or stub assertions.

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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The settlement fixture used a bare possibly-zero modulo in a Linked match, so S3 refused before its Undefined and incomplete-match assertions executed. | fsl-analyze/tests/it/hyper_settle.rs:224 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | The fixture now calls the existing Kernel-checked q from its Linked match; actual runtime assertions and both incomplete controls remain intact. |

## Confined disposition evidence

Confined disposition at reviewed head 599a5a36f69ffe15ec918ab078e65b5db642cb1b, implementation commit 5cae698add0ff5512848e2b220bef96fa719946c. Inspected only the corrected fixture, its documentation/import and the documented pub(crate) visibility of the existing KERNEL_MOD_FUNCTION constant. The Linked match now calls q(value(...)) through World::new_with_kernel_function, matching FR-177-AC-5 and preserving recorded checking mode. All runtime Undefined/refutation/cause/named-variable assertions, FR-182-AC-8 binding and both legitimate incomplete-match controls remain unchanged. No fake model, duplicated fixture, S3 weakening or ignored case was introduced. No further findings.

Accepted coder receipts without repetition: original affected-module gate 83 passed, 1 failed, 2 ignored; corrected qualified test ran once and passed 1/0. Together the affected scope has 84 accepted passes, 0 unresolved failures, 2 existing ignored cases. The original failure remains recorded in reviews/qsl-2082-validation.json; this is not a whole-gate rerun or a claim that the original gate passed. Reviewer ran no tests, builds, lint, matrix or other-module checks. PASS retained for the scoped change; QSL-2082 remains partial under its existing owners.

```yaml
{
  "scope": [
    {
      "id": "FR-182-AC-8",
      "path": "spec/functional/FR-182-settle-a-hyper-verdict.md",
      "role": "examined",
      "excerpt": "FR-177-AC-5's outcome settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation` naming `a` and `b`, after its replay; FR-177-AC-6's and FR-178-AC-6's outcomes settle `inconclusive`, `MatchUndetermined`."
    },
    {
      "id": "FR-177-AC-5",
      "path": "spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md",
      "role": "context_only",
      "excerpt": "`function q using v(x: Integer): Integer pure { 1 mod x }` is checked under `CheckMode::Kernel` and the clause calls it (the clause is checked Linked and would refuse a bare `mod` as `unproved-nonzero`, FR-495-AC-3, FR-952)"
    }
  ],
  "findings": [
    {
      "fnd": "FND-002",
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "test-intent",
      "artifact_id": "FR-182-AC-8",
      "related_ids": [
        "FR-177-AC-5",
        "FR-495-AC-3",
        "FR-952"
      ],
      "path": "fsl-analyze/tests/it/hyper_settle.rs",
      "lines": "224-236",
      "excerpt": "FR-177-AC-5's outcome settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation` naming `a` and `b`, after its replay; FR-177-AC-6's and FR-178-AC-6's outcomes settle `inconclusive`, `MatchUndetermined`.",
      "finding": "The settlement fixture used a bare possibly-zero modulo in a Linked match, so S3 refused before its Undefined and incomplete-match assertions executed."
    }
  ],
  "dispositions": [
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "let divided = \" and (if present(b.step.Vault::step.i) then q(value(b.step.Vault::step.i)) \\\n                   else 0) = 0\";\n    let world = World::new_with_kernel_function(\n        Vault::Secure,\n        false,\n        &STARTS,\n        &opaque(divided),\n        KERNEL_MOD_FUNCTION,\n    );"
    }
  ],
  "bindings": [
    {
      "test_id": "an_undefined_tuple_refutes_and_an_undetermined_match_is_inconclusive",
      "ac_id": "FR-182-AC-8",
      "trace": "correct"
    }
  ]
}
```
