---
id: SR-202611210022442313144108504
title: "QSL-2248 first independent code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2248; fsl-analyze/src/analyze/items.rs, fsl-analyze/src/model_check/hyper.rs, fsl-analyze/src/model_check/hyper/lift.rs, fsl-analyze/src/model_check/hyper/reductions.rs, fsl-analyze/src/model_check/hyper/single.rs, fsl-analyze/tests/it/hyper_reductions.rs, fsl-analyze/tests/it/hyper_swap.rs, fsl-analyze/tests/it/hyper_world.rs, fsl-eval/src/hyper/copy_swap.rs, fsl-eval/src/hyper/product.rs, fsl-replay/src/model_check/graph_hyper.rs, fsl-replay/src/model_check/hyper_outcome.rs, fsl-replay/src/model_check/hyper_settle.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: references
---

## Summary

Frozen head: 7bcc12aea0bc013bde867125cd7aa4f16635dbb1. Base: bd669ebc46820921e04413e124f51605798c5b6c. Ticket: QSL-2248. No source, tracked reviews, Cargo, push or merge actions were performed. QSL-2128 request integration remains separate. No applicable AssuranceProfile was found. No spec/plan files changed; spec-review and its sub-analyses do not apply.

## Verdict

FAIL — Finite HP-2 silently ignores selected state constraints and returns HoldsClosed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Finite HP-2 silently ignores selected state constraints and returns HoldsClosed. | fsl-analyze/src/model_check/hyper/reductions.rs:93 |

The newly widened Universal admission at reductions.rs:93-95 accepts EventPositionFalseExtension as well as infinite-trace Universal clauses. check_hyper admits the named constraint, then Run::decide at hyper.rs:814-817 immediately returns decide_closed. closed.rs:55-167 never consults reductions or constraint_value and returns HoldsClosed. Existing hyper_closed.rs:57-64 establishes that this profile has HyperForm::Universal. Concrete case: an event-position HP-2 bounded clause whose body always holds, with a declared Zero constraint violated by a reachable step, still enumerates all executions and returns HoldsClosed, rather than enforcing or explicitly refusing the selection. Key symmetry also bypasses its product canonicalisation/reporting on this route. Patch the admission/dispatch boundary and finite selected-reduction behavior together, preserving no-selection finite semantics; add the finite Zero regression. If finite reduction execution is deferred, explicitly return Unsupported before expansion rather than silently admitting it; document the residual eligibility gap.

## Scope

```yaml
{
  "scope": [
    {
      "id": "FR-174-AC-1",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "context_only",
      "excerpt": "ADR-023 §1's `NonInterference` has copy group `[[a, b]]`. The same clause with `v.l @ a <= v.l @ b` as its body, with `fair { weak V::Vault::step }` on `a` only, or with `SavesPower`'s two aliases has an empty copy group. A body written `v.l @ b = v.l @ a` and one written `v.l @ a = v.l @ b` are both swappable."
    },
    {
      "id": "FR-174-AC-2",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "context_only",
      "excerpt": "With copy-swap, §8.1's leaky product stores 7 states and the secure product 6, against 10 and 8 unreduced; the leaky verdict is `refuted` and the secure verdict `proved` in both runs, the secure proof with basis `Reduced` naming `CopySwap{[[a, b]]}`."
    },
    {
      "id": "FR-174-AC-3",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "context_only",
      "excerpt": "The leaky refutation under copy-swap is a `HyperCounterexample` with traces in quantifier order and no canonical state, and it replays by FR-183. Running the request twice gives byte-equal counterexamples."
    },
    {
      "id": "FR-174-AC-4",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "With partial-order reduction selected, `NonInterference` settles `ReductionNotPreserving` before any expansion; with a state constraint selected, `Opaque` settles `ReductionNotPreserving`; with key symmetry selected over `SavesPower`'s two subjects, it settles `ReductionNotPreserving`."
    },
    {
      "id": "FR-174-AC-5",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "context_only",
      "excerpt": "A clause with the body `always holds(v.l @ a = v.l @ b)`, the `match` conjunct `a.step.op = b.step.op` and `fair weak V::Vault::reset` on both `a` and `b`, over a vault whose `step` flips `l` and whose `reset` is enabled at `l = 1` and keeps it, from `a = (0, 0)` and `b = (0, 1)`, has the copy group `[[a, b]]` and is stored whole: 4 product states, the unreduced count, no reduction named, and `Violated` with a counterexample that replays fair. With the fairness on `a` and `b` and a second class `c`, `d` without any, the outcome names `CopySwap{[[c, d]]}` alone."
    },
    {
      "id": "FR-174-AC-6",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "context_only",
      "excerpt": "The clause with the body `always holds(v.l @ a = 0) and always holds(v.l @ b = 0)` over the secure vault has the copy group `[[a, b]]` and is stored whole with no reduction named, since its swap exchanges two acceptance sets. The body `eventually holds(v.l @ a = 1) and eventually holds(v.l @ b = 1)` over a vault whose `step` toggles `l`, from the initial states `(0, 0)` and `(0, 1)`, has a negation with no acceptance set, which its swap fixes, and is stored as 5 states against the unreduced 8. A clause with one class whose swap passes and one whose exchange fails is stored whole with"
    },
    {
      "id": "FR-174-AC-7",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "The constraint is eligible on every HP-2 item: with `Low` selected, `SavesPower` over two subjects and `NonInterference` with `fair each` constraints each run under the constraint, settle neither V-8 nor `ReductionNotPreserving`, and give the verdict of the same run with no selection."
    },
    {
      "id": "FR-173-AC-6",
      "path": "spec/functional/FR-173-classify-hyper-clauses-into-forms.md",
      "role": "context_only",
      "excerpt": "Under the event-position profile, analyzed through `native-analyze/1` with EN-1, never `not-yet-implemented`."
    }
  ],
  "bindings": [
    {
      "test_id": "a_selected_nonpreserving_reduction_is_refused_before_expansion",
      "ac_id": "FR-174-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "a_reduction_whose_row_reads_no_settles_before_any_expansion",
      "ac_id": "FR-174-AC-4",
      "trace": "correct"
    },
    {
      "test_id": "a_key_symmetry_keeps_the_verdict_with_fewer_states",
      "ac_id": "FR-174-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "a_state_constraint_reduces_and_holds_reduced",
      "ac_id": "FR-174-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "a_key_symmetry_on_an_ineligible_form_settles_unsupported",
      "ac_id": "FR-174-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "a_state_constraint_is_eligible_on_every_hp2_item",
      "ac_id": "FR-174-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "a_symmetry_over_a_key_outside_the_universe_is_refused",
      "ac_id": "FR-174-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "initial_states_not_closed_under_the_symmetry_settle_symmetry_broken",
      "ac_id": "FR-174-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "a_key_symmetry_with_a_nontrivial_stabiliser_keeps_the_verdict",
      "ac_id": "FR-174-AC-7",
      "trace": "correct"
    }
  ]
}
```

## Coverage

Reconciliation: quoin matrix 0.36.7 and quire matrix 0.52.0 (engine 0.62.2). Whole-tree static rollup: tagged 4767; untagged 1131; tagged-by-ignored-test 92; method-without-symbol 104; retired 13. Those repository-wide gaps are context outside this diff, not requests for unrelated fixes. FR-174 extracted scope: 7 tagged criteria. No run evidence bound in the evidence matrix. quire coverage reports no untracked symbols in the two changed hyper test files; its symbol-id-collision diagnostics occur in other files and were retained in the command output. Reverse-gap/stub inspection of all 13 changed files found FR-174 ownership and no stubs, copied external implementation, or weakened CI gates. Semantic inspection was explicitly requested in the dispatch. Plan completion: not assessed.

## Validation

Root-provided logs read: 8 hyper_reductions passed; hyper_swap 21 passed, 1 failed; same named undefined-contract test fails on clean base before the engine with Unproved(Nonzero). This baseline failure is not attributed to the PR. Root reports clippy blocked by 616 unchanged fsl-semantics lints and changed-file formatting has main-identical items.rs output; not claimed clean. git diff --check against the stated base passed. Rebase retained mod closed, the finite dispatch, ClosedUndefined mapping, and HoldsClosed outcome/stats arm. The interaction of retained finite dispatch with widened reduction admission is the reported finding.
