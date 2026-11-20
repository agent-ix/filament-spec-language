---
id: SR-1497
title: "Code and Rust review of QSL-2091 completion boundaries"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2091; fsl-analyze/tests/it/refinement_certificate.rs and completion_boundaries.rs; reduction.rs and completion_boundaries.rs; exact_route.rs and completion_boundaries.rs; .agent/receipts/qsl-2091/scope.md and validation.md"
review_set: subset
---

## Summary

Ticket: QSL-2091. Reviewed frozen source `4b911c94db93985df0b8a0ddfbf03c8d88cbd381`: eight new active behavior functions, one strict owner-pending liveness case, three parent registrations, and the scope/validation receipts. One medium defect affects the reduction fixture's admission contract.

## Verdict

**PASS** — FND-001 is fixed at `d84a5f48b4226f68b12d3741dbe7748db35c3421`; see Dispositions. No new finding arose from the confined fixture fix. This is the scoped review result, not a whole-ticket completion or final-gate claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | New reduction cases select invariant declarations as constraints, so they assert forbidden admission and misground QSL-2713's liveness evidence. | fsl-analyze/tests/it/reduction/completion_boundaries.rs:8; fsl-analyze/tests/it/reduction.rs:58; fsl-analyze/tests/it/reduction_world.rs:278; spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:125 |

## Finding detail

Every new reduction case calls the inherited `count()`. Its `COUNT` source declares `invariant Low` and `invariant Started`, and `World::new_with_mode` incorporates that body unchanged. The cases then select those names as `reductions.constraint` and demand successful exploration. FR-158's Resolution rule explicitly refuses an at-current invariant selected as a constraint. Thus these additions presently pass by depending on forbidden admission and would fail when that admission is corrected.

Give only these new cases a fixture with actual `constraint` declarations; preserve the inherited fixture and tests. If that current syntax/checked form is unavailable, retain the required oracle as strict pending under its implementation owner rather than treating invariant selection as valid. The strict liveness case also selects the inherited invariant through `count()`. Its original `ReducedLiveness` result therefore does not establish a defect for a normatively admitted constraint request. Correct the QSL-2713 evidence/scope as part of this fix and give its desired oracle actual `constraint` declarations. Do not rerun the original invalid fixture or continue calling it a confirmed ConstraintReached-contract defect.

## Coverage

Plan completion: not assessed.

This is the assigned diff's semantic/trace and stub review, not a whole-repository assurance claim. The saved baseline and in-lane execution receipts are accepted. No matrix, tests, builds, lint retries, full-repository profile/history discovery, or Done-ticket audits ran. The lead owns the single final affected-component gate after fixes. Broader historical QSL-2091 families remain unclaimed; accepted QSL-2082/2086/2087 cases were not duplicated. QSL-2713 remains one ignored, uncounted strict oracle, not proof of liveness success.

Examined implementation seams are the existing production `check_simulation_certificate`, actual model-check request/result, and actual EN-5 provider route. No production code, dependency, configuration, gate, external fixture, or runtime bypass changes.

## Assurance Context

No AssuranceProfile was selected in this bounded assignment. Whole-repository profile selection and broader assurance evidence were not performed; none is claimed. No new exception is introduced.

## Dispositions

Round 1, confined fix review at `d84a5f48b4226f68b12d3741dbe7748db35c3421`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Only the new child now constructs actual native `constraint Low` and `constraint Started` through `World::new`; all four child cases use that fixture. Inherited `count`, `COUNT` and parents are unchanged. The author corrected QSL-2713's evidence and scope, explicitly retracting the original invariant-based run; comment f55e315c and the new actual-constraint receipt ground the remaining owner-pending liveness oracle. |

After excerpt from the corrected child fixture:

```rust
         constraint Low using v on M::Counter at current { self.v <= 1 }\n\
         constraint Started using v on M::Counter at current { self.v >= 1 }\n",
```

The trusted author receipt reports three changed active qualified cases passed individually and the actual-constraint liveness oracle was executed once, returning `ReducedLiveness`. No original invalid fixture or unaffected passing test was rerun. The latter is one strict ignored owner-pending test, excluded from coverage. Eight new active cases remain accepted; final component gate is the lead's next step. This disposition inspected only the fix diff and receipt, with no tests, builds, lint, matrix, owner-record recheck, Done audit or new whole review.

## Examined units

- `FR-149-AC-1` (examined), spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:

  "ADR-020 §8's `CasRefinesCounter` without its `ensure` row: the engine's certificate holds one position per stored product state, and the checker returns `Accepted`."

- `FR-149-AC-4` (examined), spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:

  "The AC-1 certificate checked with `max_transitions` 1 returns `Stopped(LimitReached{MaxTransitions, 1})`. Checking the AC-1 certificate twice gives equal results."

- `FR-149-AC-6` (examined), spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:

  "In a certificate for a model whose chain of enabled operations ends in a terminal state (`CasRefinesCounter` without its `ensure` row, with `peek` and the second thread's `begin` given preconditions that are never enabled, so the states form a chain that ends in a terminal state), the terminal state's position lists one edge whose `transition` is `None` and which returns to the position;"

- `FR-149-AC-7` (examined), spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:

  "A certificate whose `successors` list is one shorter than `positions` is rejected `Malformed` at the shorter length; one whose `initial` omits a concrete initial state, at `len(initial)`; one with a position that only an edge listed at a higher-indexed position reaches, at that position; each before any recomputation."

- `FR-158-AC-1` (examined), spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:

  "The constrained run stores 20 states and expands 8: the 8 with every counter at 0 or 1, and 12 boundary states with one counter at 2. `always holds(x.v <= 1)` returns `Violated` at the boundary state `(2,0,0)` with prefix `bump(a), bump(a)`, the unconstrained run's prefix."

- `FR-158-AC-2` (examined), spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:

  "`always holds(x.v <= 2)` returns `ConstraintReached{boundary_states: 12}` (the unconstrained run returns `Violated` at `(3,0,0)`). `eventually holds(x.v = 3)` under the empty fairness set returns `ConstraintReached{12}`: no boundary state is stutter-extended, so no lasso is formed (the unconstrained run returns `Holds`)."

- `FR-158-AC-3` (examined), spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:

  "A constraint that evaluates `Undefined` at a reached state settles the item `Violated` with cause `UndefinedEvaluation` and a replaying counterexample ending at that state, and that state is not a boundary state. A request naming an absent constraint refuses `invalid_runtime_input`/`invalid-value` naming it, with no state expanded."

- `FR-158` (examined), spec/functional/FR-158-cut-the-search-with-a-state-constraint.md:

  "- **Resolution.** The pre-check SHALL resolve the request's constraint name\n  to a checked `constraint` clause of the subject's package; a name that\n  resolves to none, or to a state clause of another kind (an `at current`\n  invariant included), SHALL refuse `invalid_runtime_input`/`invalid-value`,\n  naming it, before any expansion."

- `FR-195-AC-1` (examined), spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:

  "§15.6's `Terminates` and `FairTerminates` check as `Reach{AtLeast(99/100), Eventually(p.done)}` under `Every`, the second with fairness `[strong flip]`; their obligation identities differ."

- `FR-195-AC-3` (examined), spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md:

  "`exact_only` is true for each claim of AC-1 and AC-2 that checks, for §15.5's `MeanTime` at `3 ms` and for `P95` with no confidence parameters, and false for `P95` under `Steady` with its confidence parameters."

