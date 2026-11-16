---
id: SR-2026111612172400000008
title: "QSL-2323 follow-up gap-analysis for PR 2790"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2323; PR 2790; fsl-analyze/tests/it (12 changed test/support files)"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-495", type: references }
---

## Summary

Ticket: QSL-2323. PR: filament-spec-language#2790. Diff-scoped gap analysis found no new defect in the 31-case runtime-undefinedness fixture correction. This separate follow-up preserves the completed PR 2787 artifacts SR-13627 and SR-13628.

## Verdict

PASS for the requested diff scope. The five replay/mode-propagation and 23 hyper cases remain parked. This is not ticket completion or aggregate suite acceptance.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Frozen review head: 1f9d16dad128f238163bc3b612684b3be05cd5dd. Compared git diff origin/main...HEAD in the sole QSL-2323 worktree. All 12 changed files are fsl-analyze/tests/it Rust test/support files; no production, spec, dependencies, CI, assertions, ignore markers, or gate thresholds changed. No applicable AssuranceProfile found.

Runtime fixture analysis: value_validity's function-body packages now use Kernel for runtime undefinedness. General counter, probabilistic, timed and units constructors retain Linked defaults; explicit mode variants forward the requested mode. reduction::count carries an undefined Odd helper and uses Kernel; reduction_world switches only for the known undefined-mod clause. config_over and ticker_with include their runtime-undefined helper under Kernel, also when a control case does not call it. These are runtime model/evaluator fixture controls, not tests of positive Linked admission. Existing Linked refusal and bounded-nonzero admission coverage in check_mode.rs and division_law_selection.rs remains unchanged.

The zero_mod, odd_value and mod_defined wrappers preserve the input domains and zero-divisor points. Checked calls return Boolean inside declared clauses; the arithmetic obligation lives in the function body. The unchanged state_clause.rs, temporal_clause/condition.rs and probabilistic/claim.rs use normal Definedness, while family.rs passes the mode to function-body Definedness. The edit therefore does not grant Kernel admission to an unproved operator directly inside a declared clause. Existing custom ModelLetters seams stay in place; this review does not claim these become evaluator tests. Exact undefined causes, state/position ranking, BadState rejection and original-model Markov refusal assertions remain.

Scope is the PR's 31 previously failing cases: value_validity 10; reduction 8; example_units 7; bounded_fair_certificate 1; closure_certificate 1; cyclic_mdp_certificate 3; dense_route 1. Helper callers were inspected for unintended mode changes; their presence is not an expansion of the claimed 31-case fix. Five replay/mode-propagation cases and 23 hyper cases are explicitly parked; no hyper source or assertion is changed. The parked work remains necessary for the ticket and no aggregate suite or ticket-completion claim is made.

No Cargo command ran in this review, as dispatched. PR-reported module evidence is value_validity 28/28; reduction 52 passing with two replay cases excluded; example_units 14/14; bounded_fair_certificate 4/4; closure_certificate 21/21; cyclic_mdp_certificate 5/5; dense_route 6/6, plus fmt and diff checks. The lead additionally reports value_validity 28/28 with FSPEC_DIR and QSPEC_DIR set to /Users/peter/dev/filament-specification; without them, four environment panics occur. These are attributed execution reports, not reviewer reruns. Reviewer git diff --check origin/main...HEAD passed.

Reconciliation: quoin matrix 0.29.4, quire 0.38.3 (engine 0.52.1), queried at this head. Whole-repository background: tagged 3826; tagged-by-ignored-test 381; untagged 1143; method-without-symbol 102; retired 13. No run evidence is stored for the selected criteria. Of the 27 local criteria read, 24 are tagged, FR-158-AC-4 and FR-160-AC-5 are untagged, and FR-201-AC-50 is absent from the computed matrix. These pre-existing conditions are not introduced by this test-fixture diff. FR-160-AC-5's test documents that it checks the run/prefix, not the complete replay obligation. FR-201-AC-50 is present in source after a blank line separating it from the table header; the existing test tag is reported untracked. Its undefined-cause binding was manually inspected, not counted as computed coverage. quire coverage reports 967 diagnostics and 4587 untracked symbols repository-wide. Its exit is 0 with inline-data-schema and duplicate process-archetype warnings. This is a scoped no-new-gap conclusion, not a whole-repository assurance PASS.

Plan completion: not assessed. Semantic review was limited to the requested runtime-undefinedness and declared-clause boundaries. No broader semantic review, plan review, or spec-review was run. Reverse-gap/stub review found no new production behavior, stubs, duplicated production semantics, or test-oracle bypass. Partial criterion excerpts below limit context length and do not assert full coverage of every clause.


Parked replay/mode-propagation cases: reduction::an_authored_clauses_letter_is_read_before_a_state_constraint; reduction::undefined_absent_and_initially_false_constraints_and_bounds; clause_item::an_unspelled_counterexample_fails_its_item_and_not_the_document; clause_item::a_claim_that_takes_a_modulus_by_zero_settles_refuted_with_a_replayed_undefined_counterexample; cyclic_reward_certificate::predicate_undefined_only_after_target_is_not_evaluated. The 23 parked hyper cases comprise hyper_tuple 2, hyper_witness 8, hyper 2, hyper_lowering 2, hyper_projected 4, hyper_replay 5. No parked test was treated as passing or disabled by this PR.

## Examined units

```yaml
{
  "scope": [
    {
      "id": "FR-201-AC-24",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "context_only",
      "excerpt": "In a transient SCC, a goes to b with probability 1, b to a with 1/2, target g with 1/4 and closed failure f with 1/4. Values a=b=1/2, g=1, f=0 and ranks a=2,b=1,g=f=0 accept; both SCC members having condensation rank 1 rejects RankBroken. A fully authored workload at s choosing equally between deterministic return-to-s and a distribution g:1/3,f:2/3 resolves to s:1/2,g:1/6,f:1/3 and accepts exact value 1/3, rank 1 on either workload bound at equality; 2/5 AtLeast and 1/4 AtMost reject."
    },
    {
      "id": "FR-201-AC-39",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "context_only",
      "excerpt": "A live state s has actions a with probabilities (self,target,fail)=(1/2,1/5,3/10) and b with (1/2,2/5,1/10). With target value 1 and false-terminal fail value 0, the new profile accepts minimum 2/5 under AtLeast 2/5 and maximum 4/5 under AtMost 4/5, with rank(s)=1 and boundary ranks 0. Minimum AtLeast 1/2 and maximum AtMost 3/4 reject ThresholdMissed. Changing an otherwise matching actual action probability, omitting a source action from the extremum or replacing separate actions by their averaged distribution cannot verify the old vector. Separately let S choose A=(1/2 fail,1/2 T) o"
    },
    {
      "id": "FR-201-AC-42",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "Actual pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic exhaustion during original/stopped admission, raw/internal/full keys, all-action and draw reconstruction, min/max sums, zero-support and progress checks retain their real typed cause without an accepted truth. Undefined predicates/workload or source refusals are not coerced to zero or dropped actions. Fairness, workload selection, rewards, timed/general temporal, pending multi-post, multiple-initial/binding, non-exact or refutation requests issue no new-profile proo"
    },
    {
      "id": "FR-201-AC-50",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "Actual Requested/Deadline cancellation and byte/state/edge/work/storage/representation/arithmetic exhaustion during checked fairness/source declaration intake, structural identity traversal/encoding, canonical writing, raw/full-key admission and independent graph/arithmetic/rank checking retain typed causes without accepted truth. Earlier source refusals and unsupported/undefined evaluation remain distinct. External transition/private-reader and ordinary interpreter allocation boundaries are not claimed qualified."
    },
    {
      "id": "FR-162-AC-1",
      "path": "spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md",
      "role": "examined",
      "excerpt": "`ModelSystem` offers all three reductions through `offered_reductions` and returns `Some` from all three hooks for ADR-021 §7.1's annotated subject. A test `TransitionSystem` whose `offered_reductions` is `NONE` settles a request selecting symmetry, one selecting partial-order reduction, and one selecting a constraint each V-8 in the pre-check. FR-120's `explore_model` over §7.1's annotated subject stores 27 states and records findings for each, as it does without the annotation."
    },
    {
      "id": "FR-162-AC-4",
      "path": "spec/functional/FR-162-offer-reductions-through-transition-system-hooks.md",
      "role": "examined",
      "excerpt": "`max_states` 5 over ADR-021 §7.3's constrained run stops `Stopped(ResourceExhausted, MaxStates)` with boundary states counted toward the 5. Two runs of AC-3 retain equal edge lists."
    },
    {
      "id": "FR-126-AC-11",
      "path": "spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md",
      "role": "examined",
      "excerpt": "Over the example subject with both versions 0, `always holds(6 mod (2 - c.versionNumber) >= 0)` returns `Violated` bound to `a` with the prefix `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)` and `kind: UndefinedEvaluation{where: position 2, cause: division-by-zero}`, since both bindings are undefined at depth 2 and `a` is the earlier. With initial versions `a` 0 and `b` 1, it returns `Violated` bound to `b` with the prefix `(0,1) -upd(b)-> (0,2)` and `where: position 1`."
    },
    {
      "id": "FR-126-AC-12",
      "path": "spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md",
      "role": "examined",
      "excerpt": "Over the example subject with both versions 0, `eventually (holds(c.versionNumber = 0) and holds(6 mod (2 - c.versionNumber) >= 0))`, whose negation automaton has no move at position 0, returns `Violated` bound to `a` with `kind: UndefinedEvaluation{where: position 2, cause: division-by-zero}` and the prefix `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`, found by the model-only pass; the run's statistics report no product state for the dead positions. With `max_states` 1 it returns `Stopped{ResourceExhausted, {MaxStates, 1}}`, the pass's second state exhausting the budget."
    },
    {
      "id": "FR-126-AC-14",
      "path": "spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md",
      "role": "examined",
      "excerpt": "With initial versions `a` 0 and `b` 1, the AC-12 claim returns `Violated` bound to `b` with the prefix `(0,1) -upd(b)-> (0,2)` and `where: position 1`, since `b`'s product reaches the undefined letter at depth 1 and `a`'s model-only pass at depth 2."
    },
    {
      "id": "FR-126-AC-15",
      "path": "spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md",
      "role": "examined",
      "excerpt": "With initial versions `a` 0 and `b` 1 and the AC-12 claim, whose first phase ends in `a` with a finding at depth 2, `max_states` 6 returns `Stopped{ResourceExhausted, {MaxStates, 6}}`, since binding `b` stops before it completes depth 1; `max_states` 1,000 returns the AC-14 finding for `b`; with both versions 0, `max_states` 1,000 returns the AC-12 finding for `a`, the tie going to the earlier binding."
    },
    {
      "id": "FR-160-AC-1",
      "path": "spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md",
      "role": "examined",
      "excerpt": "Each new map row settles as the table states: ADR-021 §7.1's `fair weak each` instance settles `proved`, `closed-scope`, `Proved{Reduced{[Symmetry{groups: [[a, b, c]], orbit: [a, b, c], stabiliser: [[b, c]]}]}, Uncertified}`, success, with three records, one per key of the orbit; §7.2's instance settles `Proved{Reduced{[PartialOrder{BreadthFirstRevisit}]}, Uncertified}`; FR-159-AC-1's TP-2 request settles `inconclusive`, `unsettled`, `ReductionNotPreserving{PartialOrder, TP-2}`; FR-151-AC-3's broken initial state settles `SymmetryBroken{config_history, InitialStatesNotClosed{0, (a b)"
    },
    {
      "id": "FR-160-AC-2",
      "path": "spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md",
      "role": "examined",
      "excerpt": "Over ADR-021 §7.3's subject, `always holds(x.v <= 3)` with constraint `Wide` (`self.v <= 3`) reaches no boundary state and settles `Proved{Exhaustive, Uncertified}`, with `Wide` named in the method. `always holds(x.v <= 2)` with constraint `Low` and `max_depth` 2 completes depth 2, reaches boundary states at depth 2 and finds no violation; it settles `ConstraintReached`, not `BoundReached`."
    },
    {
      "id": "FR-160-AC-5",
      "path": "spec/functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md",
      "role": "examined",
      "excerpt": "Over ADR-021 §7.1's subject with the symmetry declaration, `always holds(6 mod (2 - c.versionNumber) >= 0)` for `c = a` settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation` with `cause` `division-by-zero` at a state where `a.versionNumber = 2`, after its concretised counterexample replays; its record names no reduction, and its prefix has the length of the unreduced run's, two steps."
    },
    {
      "id": "FR-495-AC-3",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "`function d1 using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a div b }`, the same with `rem` and `mod`, `function d2 using v(a: P, b: P): P pure { a / b }` with `type P = Decimal[-1000, 1000; 0, 2; nearest-even];`, `function d3 using v(): Boolean pure { 1 cm / 0 cm = 1 cm / 0 cm }` (with TC-187's units) and `function d4 using v(): P pure { decimal(1, 0) / decimal(0, 3) }` each refuse `undefined_expression`/`unproved-nonzero` at the operator under `CheckMode::Linked`; `function d5 using v(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }` and `if b != 0 then a div b else 0"
    },
    {
      "id": "FR-495-AC-11",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "Over the ConfigVersion package (FR-103-AC-1), checked through `PackageDeclarations::check` (not `CheckedGraph::check_clause_expression`), `invariant NoGuard using v on Config::ConfigVersion at current { deref(value(self.parent)).versionNumber < self.versionNumber }` refuses `undefined_expression`/`unproved-presence` at the `value` under `CheckMode::Linked` and under `CheckMode::Kernel`, one refusal each; the guarded `present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber` (FR-104-AC-1's `ParentOrder`) checks under both."
    },
    {
      "id": "FR-495-AC-17",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "Under `CheckMode::Linked`, from source with no law header: `function m0 using v(a: Int[0, 100], b: Int[0, 12]): Int[0, 11] pure { a mod b }` refuses `undefined_expression`/`unproved-nonzero` at the `mod`, as its only refusal; `function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks and the checked `mod` node has type `Int[0, 11]`; `function m2 using v(a: Integer, b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks (the remainder is bounded by the divisor alone); `function m3 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 10] pure { a mod b }` refuses `"
    },
    {
      "id": "FR-495-AC-18",
      "path": "spec/functional/FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-fsl-source.md",
      "role": "examined",
      "excerpt": "Under `CheckMode::Linked` and a law header for each of the three laws (FR-670): `function r1 using v(a: Int[-100, 100], b: Int[1, 12]): Int[-11, 11] pure { a rem b }` checks, its `rem` node typed `Int[0, 11]` under Euclidean and floor and `Int[-11, 11]` under truncating; the same with result `Int[0, 11]` checks under Euclidean and floor and refuses `unproved-range` at the `rem` under truncating; `function q1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 100] pure { a div b }` checks with its `div` node typed `Int[0, 100]`; and `q1` and `r1` with `b: Int[0, 12]` each refuse `undefine"
    },
    {
      "id": "FR-198-AC-5",
      "path": "spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md",
      "role": "context_only",
      "excerpt": "A `Mec` model whose live state has the actions `a_stay`, back to itself with probability 1, and `b_exit`, to `goal` or `fail` with `1/2` each: over every scheduler the maximum of `Pr(eventually holds(goal))` is `1/2`, and the witness policy takes `b_exit` at the live state, though `a_stay` comes first in canonical order and attains the same iterate value. `probability <= 0.25` over the same event settles `refuted` on that policy, and its evidence replays."
    },
    {
      "id": "FR-158-AC-1",
      "path": "spec/functional/FR-158-cut-the-search-with-a-state-constraint.md",
      "role": "examined",
      "excerpt": "The constrained run stores 20 states and expands 8: the 8 with every counter at 0 or 1, and 12 boundary states with one counter at 2. `always holds(x.v <= 1)` returns `Violated` at the boundary state `(2,0,0)` with prefix `bump(a), bump(a)`, the unconstrained run's prefix."
    },
    {
      "id": "FR-158-AC-3",
      "path": "spec/functional/FR-158-cut-the-search-with-a-state-constraint.md",
      "role": "context_only",
      "excerpt": "A constraint that evaluates `Undefined` at a reached state settles the item `Violated` with cause `UndefinedEvaluation` and a replaying counterexample ending at that state, and that state is not a boundary state. A request naming an absent constraint refuses `invalid_runtime_input`/`invalid-value` naming it, with no state expanded. A constraint false at the initial state makes it a boundary state, and the run stores one state. With no universe for `counters`, the constrained run still returns `RequiresBound`."
    },
    {
      "id": "FR-158-AC-4",
      "path": "spec/functional/FR-158-cut-the-search-with-a-state-constraint.md",
      "role": "examined",
      "excerpt": "The constrained and unconstrained requests for one claim have equal obligation identities and different request identities, no result of one settles the other, and the constrained run's terminal record names `Low` in its method. Repeating AC-1's run gives equal outcomes and equal boundary counts."
    },
    {
      "id": "FR-235-AC-5",
      "path": "spec/functional/FR-235-settle-a-timed-verdict-as-a-terminal-record.md",
      "role": "examined",
      "excerpt": "FR-239-AC-5's outcome settles `refuted`, `decisive-counterexample`, category violation, cause `UndefinedEvaluation{where, cause: division-by-zero}` with `where` at position 2, time stamp 2, after FR-237's replay reproduces it; the time-locked variant's `Undecided(NoAdmittedBehaviour)` settles `inconclusive`, `NoAdmittedBehaviour`."
    },
    {
      "id": "FR-338-AC-3",
      "path": "spec/functional/FR-338-check-an-en-1-closure-certificate.md",
      "role": "examined",
      "excerpt": "Over the `Counter` subject with `pre CanInc { 6 mod (2 - self.value) >= 0 }`, a certificate holding values 0 to 2 for `always holds(true)` is rejected `BadState` at value 2, whose expansion records `ContractUndetermined` for the undefined `pre CanInc`."
    },
    {
      "id": "FSpec-FR-483-AC-3",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-483-settle-backend-items-at-the-check-arm.md",
      "role": "context_only",
      "excerpt": "A `violation` settles `refuted` with one counterexample of the item kind's type that the native replay reproduces; a candidate the replay does not reproduce settles `inconclusive` with `replay-parity` and records no counterexample; an undefined goal also carries `undefined-evaluation{where, cause}`."
    },
    {
      "id": "FSpec-FR-483-AC-4",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-483-settle-backend-items-at-the-check-arm.md",
      "role": "context_only",
      "excerpt": "An engine `unknown` settles `inconclusive` `solver-inconclusive`; a rejected certificate settles `inconclusive` `certificate-rejected{rule, at}`; reaching `max_cases` settles `incomplete` `limit-reached` naming `max_cases`, its value and the setting `value_check.max_cases` or `contract_check.max_cases`; the provider's own time limit settles `incomplete` `timed-out` and the caller's deadline `incomplete` `cancelled{source: deadline}`; the item's `check.limits.max_cases` replaces the descriptor default, and a zero or non-count value, or `check` on another kind, is refused by the wire s"
    },
    {
      "id": "FSpec-FR-483-AC-6",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-483-settle-backend-items-at-the-check-arm.md",
      "role": "context_only",
      "excerpt": "A bounded item's `proved` states the claim over the item's bounded domain only; an `unbounded` item reaches the arm only on a candidate that advertises `unbounded`, and a `bounded`-only candidate gets `requires-bound` or `unsupported`/`unbounded-extent` from negotiation, never a result."
    },
    {
      "id": "FSpec-FR-483-AC-7",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-483-settle-backend-items-at-the-check-arm.md",
      "role": "context_only",
      "excerpt": "A registry with no descriptor that advertises `value-validity` has an empty candidate set for a `value-validity` item and settles it `unsupported`, `unsupported_projection`/`unsupported-requested-capability`; the registry snapshot of a production source holds a descriptor for a kind only when an arm for it is linked, so a kind with no arm has no descriptor and no candidate."
    },
    {
      "id": "FSpec-FR-484-AC-3",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-484-check-value-validity-items.md",
      "role": "context_only",
      "excerpt": "A clause with a `case` over a union of three members settles `proved` only after every member and payload domain is covered; a violation under one member gives a counterexample whose union value names that member and payload."
    },
    {
      "id": "FSpec-FR-484-AC-4",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-484-check-value-validity-items.md",
      "role": "context_only",
      "excerpt": "A function body with two occurrences of `a / b` is two items; the occurrence under `if b != 0` has the premise `b != 0` and settles `proved`, and the unguarded occurrence settles `refuted` with `undefined-evaluation` at its own occurrence with `b = 0`. The same holds for an integer `10 mod a` over `a: Int[0, 9]` at `a = 0` (VV-08). Both are delivered only by a package whose checker records no nonzero obligation for integer division, remainder and modulus; FSL FR-495 is the example host, where a `CheckMode::Linked` check refuses them and a `CheckMode::Kernel` check admits them."
    },
    {
      "id": "FSpec-FR-484-AC-6",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-484-check-value-validity-items.md",
      "role": "context_only",
      "excerpt": "A scalar operation that is the operand of a range narrowing to `Int[0, 5]`, over `a: Int[0, 3]`, is checked against `Int[0, 5]`, not its own result type, and settles `proved`; an operand that leaves the narrowing's range at an assignment settles `refuted` with `undefined-evaluation` there, though a host that proves narrowing ranges at admission, as FSL's checker does under `CheckMode::Linked`, never delivers such a package."
    },
    {
      "id": "FSpec-FR-484-AC-7",
      "path": "/Users/peter/dev/filament-specification/spec/functional/ir-proof/FR-484-check-value-validity-items.md",
      "role": "context_only",
      "excerpt": "An undefined subexpression of a clause settles `refuted` with `undefined-evaluation{where, cause}` whose `where.locus` is that subexpression; a path condition no assignment satisfies settles `inconclusive` with `kani_vacuous_proof`."
    },
    {
      "id": "fsl-analyze/tests/it/bounded_fair_certificate.rs",
      "path": "fsl-analyze/tests/it/bounded_fair_certificate.rs",
      "role": "examined",
      "excerpt": "let undefined_source = format!(\n        \"function zero_mod using vc(n: Int[0, 2]): Boolean pure {{ 1 mod n = 0 }}\\n{}\",\n        source.replace(\"holds(m.delivered)\", \"holds(zero_mod(m.attempts))\")\n    );\n    let undefined = World::new_with_mode(&prob_world::link(&undefined_source), CheckMode::Kernel);\n    let system = ModelSystem::new(undefined.subject().simulation).unwrap();\n    let claim = undefined.claim(\"Likely\");\n    let item = probabilistic_claim_identity(claim).unwrap();\n    assert_eq!(\n        item,\n        probabilistic_claim_identity(valid.claim(\"Likely\")).unwrap()\n    );\n  "
    },
    {
      "id": "fsl-analyze/tests/it/closure_certificate.rs",
      "path": "fsl-analyze/tests/it/closure_certificate.rs",
      "role": "examined",
      "excerpt": "fn world_with_mode(body: &str, start: &[(&'static str, i64)], check_mode: CheckMode) -> World {\n    let document = document(false);\n    let package = crate::counter::package_with_mode(&document, body, check_mode);\n    let mut documents = Documents::new(document);\n    let initial = vec![documents.start(\"s0\", start)];\n    World {\n        body: body.to_owned(),\n        package,\n        documents,\n        initial,\n        keys: start.iter().map(|(key, _)| *key).collect(),\n        check_mode,\n    }\n}\n\n/// A TP-1 clause with its letters, declared in a package of its own: the\n/// clause is "
    },
    {
      "id": "fsl-analyze/tests/it/counter.rs",
      "path": "fsl-analyze/tests/it/counter.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn package_with_mode(document: &[u8], body: &str, mode: CheckMode) -> CheckedPackage {\n    let graph = declarations(document, body)\n        .check(mode, CheckingLimits::default())\n        .unwrap_or_else(|refusals| panic!(\"{body}: the unit checks: {refusals:?}\"));\n    CheckedPackage::link(graph)\n}\n\n/// [`package`] with the temporal profile `filament.temporal.complete/v1`, which\n/// admits a `time` member (FR-230).\npub(crate) fn timed_package(document: &[u8], body: &str) -> CheckedPackage {\n    let text = unit_text(body).replace(\n        \"profile t0 = \\\"filament.temporal.in"
    },
    {
      "id": "fsl-analyze/tests/it/cyclic_mdp_certificate.rs",
      "path": "fsl-analyze/tests/it/cyclic_mdp_certificate.rs",
      "role": "examined",
      "excerpt": "let has_zero_mod = target.contains(\"1 mod 0 = 0\");\n    let target = target.replace(\"1 mod 0 = 0\", \"zero_mod(0)\");\n    let zero_mod = if has_zero_mod {\n        \"function zero_mod using vc(n: Int[0, 0]): Boolean pure { 1 mod n = 0 }\\n\"\n    } else {\n        \"\"\n    };\n    Model {\n        package, object:\"Item\", population:\"items\", key:\"x\",\n        integers:vec![(\"Phase\",0,3),(\"Outcome\",0,2)],\n        fields:vec![(\"phase\",format!(\"ix://{package}/Phase\"))],\n        operations:vec![\n            Operation {name:\"a\",parameters:vec![(\"outcome\",format!(\"ix://{package}/Outcome\"))],modifies:vec!["
    },
    {
      "id": "fsl-analyze/tests/it/example_units.rs",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "let formula = formula.replace(\n        \"6 mod (2 - c.versionNumber) >= 0\",\n        \"mod_defined(c.versionNumber)\",\n    );\n    let clause = clause(OVER, fairness, &formula);\n    let function =\n        \"function mod_defined using v(n: Int[0, 2]): Boolean pure { 6 mod (2 - n) >= 0 }\\n\";\n    let text = if shipped {\n        let header = \"profile v = \\\"filament.model.complete/v1\\\";\\n\";\n        let shipped = shipped_unit_text();\n        assert!(shipped.contains(header), \"the shipped unit's profile line\");\n        let shipped = shipped.replace(\n            header,\n            &format!(\"{head"
    },
    {
      "id": "fsl-analyze/tests/it/prob_world.rs",
      "path": "fsl-analyze/tests/it/prob_world.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn new_with_mode(model: &Model, mode: CheckMode) -> Self {\n        Self::try_build_with_mode(\n            model,\n            &[model.key],\n            &[model.key],\n            &[&model.initial],\n            false,\n            true,\n            mode,\n        )\n        .unwrap_or_else(|refusals| panic!(\"the unit checks: {refusals:#?}\"))\n    }\n\n    /// `model`'s S3 check result, exposed for acceptance tests of pre-sampling\n    /// refusals.\n    pub(crate) fn try_new(model: &Model) -> Result<Self, Vec<CheckRefusal>> {\n        Self::try_build(\n            model,\n            &["
    },
    {
      "id": "fsl-analyze/tests/it/reduction.rs",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn count(clauses: &[Clause<'_>]) -> World {\n    World::new_with_mode(\n        shape(false, 3),\n        COUNT,\n        clauses,\n        &[start([0, 0, 0])],\n        &KEYS,\n        CheckMode::Kernel,\n    )\n}\n\npub(crate) fn infinite(formula: &str) -> Clause<'_> {\n    Clause {\n        profile: INFINITE,\n        formula,\n    }\n}\n\nfn keys(keys: &[&str]) -> Vec<String> {\n    keys.iter().map(|key| (*key).to_owned()).collect()\n}\n\nfn class(population: &filament_checked::key::DeclarationKey, members: &[&str]) -> SymmetryClass {\n    SymmetryClass {\n        population: population.clone"
    },
    {
      "id": "fsl-analyze/tests/it/reduction_world.rs",
      "path": "fsl-analyze/tests/it/reduction_world.rs",
      "role": "examined",
      "excerpt": "let mode = if clauses\n            .iter()\n            .any(|clause| clause.formula.contains(\"6 mod (2 - x.v)\"))\n        {\n            CheckMode::Kernel\n        } else {\n            CheckMode::Linked\n        };\n        Self::new_with_mode(shape, body, clauses, starts, keys, mode)\n    }\n\n    /// [`Self::new`] checked under `mode`, for runtime undefinedness fixtures.\n    pub(crate) fn new_with_mode(\n        shape: Shape,\n        body: &str,\n        clauses: &[Clause<'_>],\n        starts: &[Vec<(&str, Json)>],\n        keys: &[&str],\n        mode: CheckMode,\n    ) -> Self {\n        let do"
    },
    {
      "id": "fsl-analyze/tests/it/timed_world.rs",
      "path": "fsl-analyze/tests/it/timed_world.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn world_with_mode(\n    document: &[u8],\n    profiles: &str,\n    body: &str,\n    mode: CheckMode,\n) -> World {\n    world_built_in_with_protocol_selection(\n        PACKAGE,\n        &[document],\n        profiles,\n        body,\n        &[\"c\"],\n        &[],\n        &json!({}),\n        true,\n        mode,\n    )\n}\n\n/// [`world`] whose `c` also holds the fields `extra`.\npub(crate) fn world_holding(document: &[u8], profiles: &str, body: &str, extra: &Json) -> World {\n    world_built(document, profiles, body, &[\"c\"], &[], extra)\n}\n\n/// [`world`] over the universe `keys`, each objec"
    },
    {
      "id": "fsl-analyze/tests/it/units.rs",
      "path": "fsl-analyze/tests/it/units.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn checked_with_mode(\n    domain: &Domain,\n    text: &str,\n    mode: CheckMode,\n) -> (CheckedPackage, Vec<CheckedTemporalClause>) {\n    let packages = package_input([domain.document.as_slice()]).unwrap();\n    let parsed = fsl_cst::parse(\n        SourceIdentity::new(\"agent-ix\", \"units\"),\n        \"units.native\",\n        text.as_bytes(),\n        fsl_cst::Limits::default(),\n    )\n    .expect(\"S1 parses the unit\");\n    assert!(parsed.is_admissible(), \"S1: {:?}\", parsed.diagnostics());\n    let built = fsl_forms::build_unit(&parsed).expect(\"S2 builds the unit\");\n    let models = "
    },
    {
      "id": "fsl-analyze/tests/it/value_validity.rs",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn package_with_source(declarations: &str) -> (CheckedPackage, Vec<u8>) {\n    // Value-validity cases exercise undefinedness in the complete-V1\n    // evaluator. Kernel leaves the Nonzero obligation for that runtime path;\n    // Linked refusal is covered by the checker acceptance tests.\n    package_with_source_mode(declarations, CheckMode::Kernel)\n}\n\nfn package_of_mode(declarations: &str, mode: CheckMode) -> CheckedPackage {\n    package_with_source_mode(declarations, mode).0\n}\n\nfn package_with_source_mode(declarations: &str, mode: CheckMode) -> (CheckedPackage, Vec<u8>) {\n"
    },
    {
      "id": "fsl-analyze/tests/it/zone_search.rs",
      "path": "fsl-analyze/tests/it/zone_search.rs",
      "role": "examined",
      "excerpt": "pub(crate) fn ticker_with(locked: bool, body: &str) -> World {\n    let when = if locked { \"true\" } else { \"self.phase < 2\" };\n    let body = body.replace(\"6 mod (2 - c.phase) >= 0\", \"mod_defined(c.phase)\");\n    crate::timed_world::world_with_mode(\n        &document(&[\"x\"], &[(\"step\", &[\"phase\", \"x\"])]),\n        TIMED,\n        &format!(\n            \"{DENSE}\\\n             function mod_defined using v(n: Int[0, 3]): Boolean pure {{ 6 mod (2 - n) >= 0 }}\\n\\\n             pre Step using v on M::Call::step {{ self.phase < 2 and self.x >= 1 ms }}\\n\\\n             post Stepped using v on M::Ca"
    },
    {
      "id": "FR-281-AC-14",
      "path": "spec/functional/FR-281-analyze-claims-with-in-process-engines.md",
      "role": "examined",
      "excerpt": "With a test provider registered under `fsl.test-smt` advertising (`value-validity`, `unbounded`) beside the value-validity arm, the item `function q using v(a: Integer, b: Integer): Integer pure { a div b }` settled `Supported(fsl.test-smt)` whose provider reports `refuted` with the counterexample `a` = 2^100, `b` = 0 settles `refuted`, category violation, naming `fsl.test-smt`, and the item's `ItemCounterexample` holds that counterexample at those integers; the replay of the counterexample reproduces the `division-by-zero` violation at the `div`. The item `if b != 0 then a div b els"
    },
    {
      "id": "FR-281-AC-15",
      "path": "spec/functional/FR-281-analyze-claims-with-in-process-engines.md",
      "role": "examined",
      "excerpt": "With the AC-14 provider reporting `refuted` with `a` = 7, `b` = 2 (an assignment at which the claim holds), with `b` = 0 and a missing second operand, or with a Boolean in place of an `Integer`, the item settles `inconclusive` with cause `replay_parity` and no counterexample, never `refuted`. With the provider reporting `unknown` the item settles `inconclusive` with cause `solver-inconclusive`."
    },
    {
      "id": "FR-281-AC-16",
      "path": "spec/functional/FR-281-analyze-claims-with-in-process-engines.md",
      "role": "examined",
      "excerpt": "With the AC-14 provider registered beside the value-validity arm, the arm's item over the AC-14 claim settled `Supported(value-checker)` settles as it does with no provider registered: `unsupported` with `unsupported_projection`/`unbounded-extent`, naming `value-checker`, and the provider is not called for it; the same record results with no provider registered."
    },
    {
      "id": "FR-281-AC-19",
      "path": "spec/functional/FR-281-analyze-claims-with-in-process-engines.md",
      "role": "examined",
      "excerpt": "With the AC-14 `refuted` provider whose descriptor publishes `max_cases` 1, the item settles `incomplete` with the `ValueCheckMaxCases` limit cause and no counterexample. With the run's `Cancel` cancelled before `analyze`, the provider item settles `incomplete` with `cancelled` and the provider is not called; with a provider that cancels the run during its check and reports the AC-14 counterexample, the item settles `incomplete` with `cancelled`, never `refuted`."
    },
    {
      "id": "value_validity::a_provider_item_stopped_by_the_case_budget_or_cancellation_is_incomplete",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn a_provider_item_stopped_by_the_case_budget_or_cancellation_is_incomplete() {\n    let package = package_of(UNGUARDED);\n    let key = only_scalar_key(&package, \"q\");\n    let at = vec![integer(1_i128 << 100), integer(0)];\n    let refuted = ProviderVerdict::Refuted {\n        counterexample: at.clone(),\n    };\n\n    let (engines, _) = engines_under(\n        refuted.clone(),\n        smt_descriptor_of(Mode::Unbounded, fsl_route::ProviderOrigin::Linked, Some(1)),\n    );\n    let record = provider_run(&engines, &package, &key);\n    assert!(\n        matches!(\n            terminal(&record),\n  "
    },
    {
      "id": "value_validity::registering_a_provider_leaves_the_bounded_arms_unbounded_item_requires_bound",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn registering_a_provider_leaves_the_bounded_arms_unbounded_item_requires_bound() {\n    let package = package_of(UNGUARDED);\n    let key = only_scalar_key(&package, \"q\");\n    let (with_provider, calls) = engines_with_provider(ProviderVerdict::Proved);\n    let settled = |engines: &Engines| {\n        let record = run_in(engines, &package, &key, \"q\", Ask::search());\n        let Settlement::Disposed(disposed) = &record.settlement else {\n            panic!(\"a disposition, not {:?}\", record.settlement)\n        };\n        assert!(record.value_check.is_none());\n        disposed.clone()\n    }"
    },
    {
      "id": "value_validity::an_undefined_subexpression_is_located_at_its_own_occurrence",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn an_undefined_subexpression_is_located_at_its_own_occurrence() {\n    let package = package_of(\n        \"function subject using v(a: Int[-1, 1], b: Int[-1, 1]): Boolean pure \\\n         { (a div b) = (a div b) }\",\n    );\n    let key = key_of(\n        &package,\n        \"subject\",\n        &serde_json::json!({\"role\": \"claim\", \"ordinal\": 0}),\n    );\n    let record = run(&package, &key, \"subject\", None);\n    assert!(matches!(terminal(&record), TerminalValue::Refuted));\n    let found = record.value_check.as_ref().unwrap();\n    let at = found.reproduced.violation().undefined.as_ref().unwrap"
    },
    {
      "id": "value_validity::an_undefinedness_inside_a_callee_is_located_in_the_callees_body",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn an_undefinedness_inside_a_callee_is_located_in_the_callees_body() {\n    let package = package_of(\n        \"function inverse using v(n: Integer): Integer pure { 10 div n }\\n\\\n         function subject using v(x: Int[-1, 1]): Boolean pure { inverse(x) >= 0 - 10 }\",\n    );\n    let key = key_of(\n        &package,\n        \"subject\",\n        &serde_json::json!({\"role\": \"claim\", \"ordinal\": 0}),\n    );\n    let record = run(&package, &key, \"subject\", None);\n    assert!(\n        matches!(terminal(&record), TerminalValue::Refuted),\n        \"{:?}\",\n        record.settlement\n    );\n    let fou"
    },
    {
      "id": "value_validity::a_location_holds_one_candidate_expression_occurrence",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn a_location_holds_one_candidate_expression_occurrence() {\n    let packages = [\n        package_of(\n            \"function inverse using v(n: Integer): Integer pure { 10 div n }\\n\\\n             function subject using v(x: Int[-1, 1]): Boolean pure { inverse(x) >= 0 - 10 }\",\n        ),\n        package_of(\n            \"function subject using v(a: Int[-2, 2], b: Int[-2, 2]): Integer pure \\\n             { (if b != 0 then a div b else a) + a div b }\",\n        ),\n        package_of(\n            \"function subject using v(a: Int[-2, 2], b: Int[-2, 2]): Boolean pure \\\n             { b != 0 an"
    },
    {
      "id": "value_validity::a_provider_counterexample_that_does_not_replay_is_not_refuted",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn a_provider_counterexample_that_does_not_replay_is_not_refuted() {\n    let package = package_of(UNGUARDED);\n    let key = only_scalar_key(&package, \"q\");\n    for counterexample in [\n        vec![integer(7), integer(2)],\n        vec![integer(0)],\n        vec![Value::Boolean(true), integer(0)],\n    ] {\n        let (engines, calls) = engines_with_provider(ProviderVerdict::Refuted {\n            counterexample: counterexample.clone(),\n        });\n        let record = provider_run(&engines, &package, &key);\n        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);\n        a"
    },
    {
      "id": "value_validity::a_provider_counterexample_that_replays_settles_refuted_and_a_proof_settles_proved",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn a_provider_counterexample_that_replays_settles_refuted_and_a_proof_settles_proved() {\n    let package = package_of(UNGUARDED);\n    let key = only_scalar_key(&package, \"q\");\n    let at = vec![integer(1_i128 << 100), integer(0)];\n    let (engines, calls) = engines_with_provider(ProviderVerdict::Refuted {\n        counterexample: at.clone(),\n    });\n    let record = provider_run(&engines, &package, &key);\n    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);\n    let terminal = record.terminal().expect(\"a terminal record\");\n    assert_eq!(terminal.value(), &TerminalValue:"
    },
    {
      "id": "value_validity::a_replay_confirms_the_violation_it_was_asked_about_and_no_other",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn a_replay_confirms_the_violation_it_was_asked_about_and_no_other() {\n    let package = package_of(\n        \"function subject using v(a: Int[-1, 1], b: Int[-1, 1]): Boolean pure \\\n         { (a div b) = (a div b) }\",\n    );\n    let key = key_of(\n        &package,\n        \"subject\",\n        &serde_json::json!({\"role\": \"claim\", \"ordinal\": 0}),\n    );\n    let target = ValueTarget::select(&package, \"subject\", &key).unwrap();\n    let at = [integer(1), integer(0)];\n    let Verdict::Violated(actual) = target.evaluate(&at, &Cancel::new()).unwrap() else {\n        panic!(\"undefined at b = 0\")"
    },
    {
      "id": "value_validity::scalar_operation_vectors_use_occurrence_reach_and_undefinedness",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn scalar_operation_vectors_use_occurrence_reach_and_undefinedness() {\n    let cases: Vec<Json> = supported()\n        .into_iter()\n        .filter(|c| shape(c) == Shape::Scalar)\n        .collect();\n    let ids: Vec<&str> = cases.iter().map(id_of).collect();\n    assert_eq!(ids, [\"VV-04\", \"VV-05\", \"VV-06\", \"VV-08\", \"VV-10\", \"VV-18\"]);\n    for case in cases {\n        let id = id_of(&case);\n        let model = &case[\"model\"];\n        let package = package_of(&declarations_of(model));\n        let key = marked_scalar_key(&package, model);\n        let record = run(&package, &key, \"subject\","
    },
    {
      "id": "value_validity::the_clause_vectors_settle_as_the_vectors_state",
      "path": "fsl-analyze/tests/it/value_validity.rs",
      "role": "examined",
      "excerpt": "fn the_clause_vectors_settle_as_the_vectors_state() {\n    let cases: Vec<Json> = supported()\n        .into_iter()\n        .filter(|c| shape(c) == Shape::Clause)\n        .collect();\n    assert!(cases.len() >= 9);\n    for case in cases {\n        let id = id_of(&case);\n        let model = &case[\"model\"];\n        let package = package_of(&declarations_of(model));\n        let key = key_of(&package, \"subject\", &case[\"item\"][\"occurrence\"]);\n        assert_eq!(\n            key.origin().role().as_str(),\n            \"claim\",\n            \"{id}: the vector's own role\"\n        );\n        let engi"
    },
    {
      "id": "reduction::a_constraint_reports_real_violations_and_never_proves_past_its_boundary",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn a_constraint_reports_real_violations_and_never_proves_past_its_boundary() {\n    let world = count(&[\n        infinite(\"always holds(x.v <= 1)\"),\n        infinite(\"always holds(x.v <= 2)\"),\n    ]);\n    let low = || vec![atom(|v| Some(v <= 1))];\n    let two = || vec![atom(|v| Some(v <= 2))];\n    for reductions in [constraint(\"Low\"), Reductions::default()] {\n        let outcome = run(&world, Some(0), low(), reductions);\n        assert_eq!(path(&world, &outcome), [(0, 0, 0), (1, 0, 0), (2, 0, 0)]);\n        assert_eq!(receivers(&world, &outcome), [\"a\", \"a\"]);\n        assert_eq!(violati"
    },
    {
      "id": "reduction::a_constraint_only_proof_is_exhaustive_and_constraint_reached_wins_over_the_horizon",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn a_constraint_only_proof_is_exhaustive_and_constraint_reached_wins_over_the_horizon() {\n    let world = count(&[\n        infinite(\"always holds(x.v <= 3)\"),\n        infinite(\"always holds(x.v <= 2)\"),\n    ]);\n    let outcome = run(\n        &world,\n        Some(0),\n        vec![atom(|v| Some(v <= 3))],\n        constraint(\"Wide\"),\n    );\n    assert_eq!(\n        settle(&world, &outcome).value,\n        TerminalValue::Proved {\n            basis: ProofBasis::Exhaustive,\n            certification: Certification::Uncertified,\n        }\n    );\n    let horizon = world\n        .run(\n         "
    },
    {
      "id": "reduction::a_constrained_run_is_deterministic",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn a_constrained_run_is_deterministic() {\n    let world = count(&[infinite(\"always holds(x.v <= 2)\")]);\n    let first = run(\n        &world,\n        Some(0),\n        vec![atom(|v| Some(v <= 2))],\n        constraint(\"Low\"),\n    );\n    let second = run(\n        &world,\n        Some(0),\n        vec![atom(|v| Some(v <= 2))],\n        constraint(\"Low\"),\n    );\n    assert_eq!(first, second);\n}\n\n/// FR-159-AC-1: over the annotated §7.2-style counters, `always[0,2]\n/// holds(x.v = 0)` (TP-2) and `always (holds(x.v = 0) implies\n/// eventually[0,2] holds(x.v = 1))` (TP-3 with an interval) with\n"
    },
    {
      "id": "reduction::a_reduction_the_system_does_not_offer_settles_unsupported",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn a_reduction_the_system_does_not_offer_settles_unsupported() {\n    assert_eq!(Bare.offered_reductions(), OfferedReductions::NONE);\n    assert_eq!(\n        unoffered(&partial_order(), &Bare),\n        Some(ReductionKind::PartialOrder)\n    );\n    assert_eq!(\n        unoffered(&symmetry(&[&[\"a\", \"b\"]]), &Bare),\n        Some(ReductionKind::Symmetry)\n    );\n    assert_eq!(\n        unoffered(&constraint(\"Low\"), &Bare),\n        Some(ReductionKind::StateConstraint)\n    );\n    let world = count(&[infinite(\"always holds(x.v <= 3)\")]);\n    let system = world.system();\n    assert_eq!(unoffered("
    },
    {
      "id": "reduction::an_undefined_evaluation_on_a_reduced_run_refutes_with_a_concrete_prefix",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn an_undefined_evaluation_on_a_reduced_run_refutes_with_a_concrete_prefix() {\n    let world = cycle(\n        true,\n        &[[0, 0, 0]],\n        &[infinite(\"always holds(6 mod (2 - x.v) >= 0)\")],\n    );\n    let undefined = || vec![atom(|v| (v != 2).then(|| 6 % (2 - v) >= 0))];\n    let reduced = run(&world, Some(0), undefined(), symmetry(&[&[\"a\", \"b\", \"c\"]]));\n    let unreduced = run(&world, Some(0), undefined(), Reductions::default());\n    for outcome in [&reduced, &unreduced] {\n        let counterexample = violation(outcome);\n        let CounterexampleKind::UndefinedEvaluation(eval"
    },
    {
      "id": "reduction::boundary_states_count_toward_max_states",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn boundary_states_count_toward_max_states() {\n    let world = count(&[infinite(\"always holds(x.v <= 2)\")]);\n    let limits = ModelCheckLimits {\n        limits: Limits {\n            max_states: 5,\n            ..Limits::default()\n        },\n        ..ModelCheckLimits::default()\n    };\n    let outcome = world\n        .run(\n            Some(0),\n            vec![atom(|v| Some(v <= 2))],\n            constraint(\"Low\"),\n            None,\n            limits,\n        )\n        .expect(\"checked\");\n    assert_eq!(\n        outcome,\n        ModelCheckOutcome::Stopped {\n            cause: Incomple"
    },
    {
      "id": "reduction::model_hooks_answer_and_simulation_stays_unreduced",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn model_hooks_answer_and_simulation_stays_unreduced() {\n    let world = count(&[]);\n    let system = world.system();\n    let offered = system.offered_reductions();\n    assert!(offered.symmetry && offered.partial_order && offered.constraint);\n    let state = &system.initial()[0];\n    let successors = system.successors(state).expect(\"the state expands\");\n    for (transition, _) in &successors.successors {\n        assert!(matches!(\n            TransitionSystem::footprint(&system, transition),\n            Some(Ok(_))\n        ));\n    }\n    assert!(system.canonical(state, &group(&[&[\"a\", "
    },
    {
      "id": "reduction::reduced_outcomes_settle_through_the_one_map",
      "path": "fsl-analyze/tests/it/reduction.rs",
      "role": "examined",
      "excerpt": "fn reduced_outcomes_settle_through_the_one_map() {\n    let world = cycle(true, &[[0, 0, 0]], &[infinite(\"always holds(x.v <= 2)\")]);\n    let bounded = cycle(\n        true,\n        &[[0, 0, 0]],\n        &[Clause {\n            profile: BOUNDED,\n            formula: \"always[0,2] holds(x.v = 0)\",\n        }],\n    );\n    let outcome = run(\n        &world,\n        Some(0),\n        vec![atom(|v| Some(v <= 2))],\n        symmetry(&[&[\"a\", \"b\", \"c\"]]),\n    );\n    let record = settle(&world, &outcome);\n    assert_eq!(\n        record.value,\n        TerminalValue::Proved {\n            basis: Proof"
    },
    {
      "id": "example_units::a_later_binding_wins_with_a_strictly_shallower_finding",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "fn a_later_binding_wins_with_a_strictly_shallower_finding() {\n    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 1));\n    let outcome = with_limits(&world, &dead_at_zero(&world), ModelCheckLimits::default());\n    let counterexample = violated(&outcome);\n    assert_eq!(bound(counterexample), \"b\");\n    assert_eq!(\n        counterexample\n            .prefix\n            .iter()\n            .map(receiver)\n            .collect::<Vec<_>>(),\n        ['b']\n    );\n    assert_eq!(undefined_at(counterexample).0, 1);\n}\n\n/// FR-126-AC-15: with `a` at 0 and `b` at 1, `a`'s first phase ends"
    },
    {
      "id": "example_units::a_stopped_binding_does_not_return_a_finding_its_own_pass_may_beat",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "fn a_stopped_binding_does_not_return_a_finding_its_own_pass_may_beat() {\n    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 0));\n    let atoms = fsl_eval::automaton::translate(&world.clauses[0], 1 << 20)\n        .expect(\"translates\")\n        .atoms()\n        .to_vec();\n    let plain = Joint {\n        world: &world,\n        atoms: atoms.clone(),\n        read: hidden_pass,\n        on_trigger: Box::new(|| {}),\n    };\n    let outcome = with_limits(&world, &plain, ModelCheckLimits::default());\n    let counterexample = violated(&outcome);\n    assert_eq!(bound(counterexample), \"b\")"
    },
    {
      "id": "example_units::a_later_binding_that_stops_on_any_cause_answers_with_its_stop",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "fn a_later_binding_that_stops_on_any_cause_answers_with_its_stop() {\n    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 1));\n    let inner = dead_at_zero(&world);\n\n    let mut limits = ModelCheckLimits::default();\n    limits.limits.max_transitions = 4;\n    let outcome = with_limits(&world, &inner, limits);\n    assert!(\n        matches!(\n            outcome,\n            Ok(ModelCheckOutcome::Stopped {\n                limit: Some(ReachedLimit {\n                    limit: ModelCheckLimit::MaxTransitions,\n                    value: 4\n                }),\n                ..\n      "
    },
    {
      "id": "example_units::a_safety_claim_is_bound_to_the_binding_undefined_at_the_shallowest_depth",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "fn a_safety_claim_is_bound_to_the_binding_undefined_at_the_shallowest_depth() {\n    let formula = \"always holds(6 mod (2 - c.versionNumber) >= 0)\";\n    let both = config_over(false, EACH, formula, (0, 0));\n    let outcome = with_limits(\n        &both,\n        &Versions::new(&both, vec![mod_is_defined]),\n        ModelCheckLimits::default(),\n    );\n    let counterexample = violated(&outcome);\n    assert_eq!(bound(counterexample), \"a\");\n    assert_eq!(\n        counterexample\n            .prefix\n            .iter()\n            .map(receiver)\n            .collect::<Vec<_>>(),\n        ['a'"
    },
    {
      "id": "example_units::the_model_only_pass_finds_the_undefined_letter_a_dead_automaton_leaves",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "fn the_model_only_pass_finds_the_undefined_letter_a_dead_automaton_leaves() {\n    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 0));\n    let letters = dead_at_zero(&world);\n    let outcome = with_limits(&world, &letters, ModelCheckLimits::default());\n    let Ok(ModelCheckOutcome::Violated {\n        counterexample,\n        stats,\n    }) = &outcome\n    else {\n        panic!(\"expected a violation, got {outcome:?}\");\n    };\n    assert_eq!(bound(counterexample), \"a\");\n    assert_eq!(undefined_at(counterexample).0, 2);\n    assert_eq!(\n        counterexample\n            .prefix\n  "
    },
    {
      "id": "example_units::a_binding_that_stops_with_a_finding_does_not_hide_a_shallower_one",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "fn a_binding_that_stops_with_a_finding_does_not_hide_a_shallower_one() {\n    const CLAIM: &str = \"always eventually holds(6 mod (2 - c.versionNumber) >= 0)\";\n    for (versions, enough, position) in [((0, 1), 8, 1), ((0, 2), 6, 0)] {\n        let world = config_over(false, WHOLE, CLAIM, versions);\n        let letters = Versions::new(&world, vec![mod_is_defined]);\n        for max_states in 1..=enough {\n            let mut limits = ModelCheckLimits::default();\n            limits.limits.max_states = max_states;\n            let outcome = with_limits(&world, &letters, limits);\n            i"
    },
    {
      "id": "example_units::a_later_binding_is_explored_through_one_less_than_the_finding",
      "path": "fsl-analyze/tests/it/example_units.rs",
      "role": "examined",
      "excerpt": "fn a_later_binding_is_explored_through_one_less_than_the_finding() {\n    let world = config_over(false, WHOLE, DEAD_AT_ZERO, (0, 1));\n    let letters = dead_at_zero(&world);\n    let mut limits = ModelCheckLimits::default();\n    let stopped_in = |limits: ModelCheckLimits| match with_limits(&world, &letters, limits) {\n        Ok(ModelCheckOutcome::Stopped {\n            cause: IncompleteCause::ResourceExhausted,\n            limit:\n                Some(ReachedLimit {\n                    limit: ModelCheckLimit::MaxStates,\n                    value,\n                }),\n            stats,\n "
    },
    {
      "id": "bounded_fair_certificate::in_horizon_undefined_preserves_the_actual_owner_predicate_cause",
      "path": "fsl-analyze/tests/it/bounded_fair_certificate.rs",
      "role": "examined",
      "excerpt": "fn in_horizon_undefined_preserves_the_actual_owner_predicate_cause() {\n    use fsl_eval::simulation::model::PredicateReading;\n    use fsl_eval::simulation::TransitionSystem;\n    use fsl_semantics::check::{Event, ProbForm};\n    let source = CLAIM\n        .replace(\">= 0.95\", \"<= 0.5\")\n        .replace(\"eventually[0,2]\", \"eventually[0,0]\");\n    let valid = World::new(&prob_world::link(&source));\n    let bytes = produce(&valid, \"Likely\");\n    let undefined_source = format!(\n        \"function zero_mod using vc(n: Int[0, 2]): Boolean pure {{ 1 mod n = 0 }}\\n{}\",\n        source.replace(\"hol"
    },
    {
      "id": "closure_certificate::an_undetermined_contract_or_a_failing_atom_is_a_bad_state",
      "path": "fsl-analyze/tests/it/closure_certificate.rs",
      "role": "examined",
      "excerpt": "fn an_undetermined_contract_or_a_failing_atom_is_a_bad_state() {\n    let function =\n        \"function odd_value using v(n: Int[0, 3]): Boolean pure { 6 mod (2 - n) >= 0 }\\n\";\n    let guarded = format!(\n        \"{function}{}\",\n        GUARDED.replace(\"6 mod (2 - self.value) >= 0\", \"odd_value(self.value)\")\n    );\n    let world = world_with_mode(&guarded, &[(\"c\", 0)], CheckMode::Kernel);\n    let reachable = world.reachable();\n    assert_eq!(reachable.iter().map(|(_, values)| values[0]).max(), Some(2));\n    let truth = world.invariant(\"always holds(true)\", |_| Some(true));\n    let transl"
    },
    {
      "id": "cyclic_mdp_certificate::first_hit_candidate_does_not_evaluate_query_after_target_and_original_model_still_admits",
      "path": "fsl-analyze/tests/it/cyclic_mdp_certificate.rs",
      "role": "examined",
      "excerpt": "fn first_hit_candidate_does_not_evaluate_query_after_target_and_original_model_still_admits() {\n    for op in [\">=\", \"<=\"] {\n        let world = kernel_world(op, \"if x.phase = 3 then (1 mod 0 = 0) else x.phase = 1\");\n        let bytes = produce(&world, \"Likely\");\n        let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();\n        assert!(document.is_cyclic_mdp());\n        assert_eq!(\n            document.document().values.len(),\n            3,\n            \"post-target state is absent\"\n        );\n        let system = ModelSystem::new(world.subjec"
    },
    {
      "id": "cyclic_mdp_certificate::undefined_query_after_target_does_not_skip_original_markov_admission",
      "path": "fsl-analyze/tests/it/cyclic_mdp_certificate.rs",
      "role": "examined",
      "excerpt": "fn undefined_query_after_target_does_not_skip_original_markov_admission() {\n    let target = \"if x.phase = 3 then (1 mod 0 = 0) else x.phase = 1\";\n    let valid = kernel_world(\">=\", target);\n    let bytes = produce(&valid, \"Likely\");\n    let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();\n    let certificate = fsl_replay::ExactProbabilityCertificate {\n        values: document.document().values.clone(),\n    };\n    let mut invalid = model(\">=\", target);\n    invalid.body = invalid.body.replace(\n        \"self.phase - (if pre(self.phase) = 1 then 3 e"
    },
    {
      "id": "cyclic_mdp_certificate::pre_target_undefined_retains_its_actual_cause_in_new_and_old_adapters",
      "path": "fsl-analyze/tests/it/cyclic_mdp_certificate.rs",
      "role": "examined",
      "excerpt": "fn pre_target_undefined_retains_its_actual_cause_in_new_and_old_adapters() {\n    let valid = world(\">=\", \"x.phase = 1\");\n    let bytes = produce(&valid, \"Likely\");\n    let document = read_probability_proof_document(&bytes, limits(), &Cancel::new()).unwrap();\n    let certificate = fsl_replay::ExactProbabilityCertificate {\n        values: document.document().values.clone(),\n    };\n    for dtmc in [false, true] {\n        let mut specification = model(\">=\", \"if x.phase = 0 then (1 mod 0 = 0) else x.phase = 1\");\n        if dtmc {\n            specification.body = specification.body.replace"
    },
    {
      "id": "dense_route::an_undefined_timed_evaluation_settles_refuted_through_analyze",
      "path": "fsl-analyze/tests/it/dense_route.rs",
      "role": "examined",
      "excerpt": "fn an_undefined_timed_evaluation_settles_refuted_through_analyze() {\n    use filament_semantic_value::catalog::Category;\n    use fsl_replay::{CounterexampleKind, TerminalValue};\n    let world = crate::zone_search::ticker(false);\n    let mut record = analyzed(&world);\n    let node = &world.package.graph().temporal_clauses()[0];\n    record.claim = Some(fsl_foundation::source::provenance::OccurrenceKey::new(\n        node.identity(),\n        node.claim().clone(),\n    ));\n    let terminal = record.terminal().expect(\"a terminal record\");\n    assert_eq!(*terminal.value(), TerminalValue::Ref"
    }
  ],
  "bindings": [
    {
      "test_id": "in_horizon_undefined_preserves_the_actual_owner_predicate_cause",
      "ac_id": "FR-201-AC-50",
      "trace": "correct"
    },
    {
      "test_id": "an_undetermined_contract_or_a_failing_atom_is_a_bad_state",
      "ac_id": "FR-338-AC-3",
      "trace": "correct"
    },
    {
      "test_id": "first_hit_candidate_does_not_evaluate_query_after_target_and_original_model_still_admits",
      "ac_id": "FR-201-AC-42",
      "trace": "correct"
    },
    {
      "test_id": "undefined_query_after_target_does_not_skip_original_markov_admission",
      "ac_id": "FR-201-AC-42",
      "trace": "correct"
    },
    {
      "test_id": "pre_target_undefined_retains_its_actual_cause_in_new_and_old_adapters",
      "ac_id": "FR-201-AC-42",
      "trace": "correct"
    },
    {
      "test_id": "a_safety_claim_is_bound_to_the_binding_undefined_at_the_shallowest_depth",
      "ac_id": "FR-126-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "the_model_only_pass_finds_the_undefined_letter_a_dead_automaton_leaves",
      "ac_id": "FR-126-AC-12",
      "trace": "correct"
    },
    {
      "test_id": "a_later_binding_wins_with_a_strictly_shallower_finding",
      "ac_id": "FR-126-AC-14",
      "trace": "correct"
    },
    {
      "test_id": "a_later_binding_is_explored_through_one_less_than_the_finding",
      "ac_id": "FR-126-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "a_provider_counterexample_that_replays_settles_refuted_and_a_proof_settles_proved",
      "ac_id": "FR-281-AC-14",
      "trace": "correct"
    },
    {
      "test_id": "a_provider_counterexample_that_does_not_replay_is_not_refuted",
      "ac_id": "FR-281-AC-15",
      "trace": "correct"
    },
    {
      "test_id": "registering_a_provider_leaves_the_bounded_arms_unbounded_item_requires_bound",
      "ac_id": "FR-281-AC-16",
      "trace": "correct"
    },
    {
      "test_id": "a_provider_item_stopped_by_the_case_budget_or_cancellation_is_incomplete",
      "ac_id": "FR-281-AC-19",
      "trace": "correct"
    },
    {
      "test_id": "an_undefined_timed_evaluation_settles_refuted_through_analyze",
      "ac_id": "FR-235-AC-5",
      "trace": "correct"
    },
    {
      "test_id": "a_constraint_reports_real_violations_and_never_proves_past_its_boundary",
      "ac_id": "FR-158-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "a_reduction_the_system_does_not_offer_settles_unsupported",
      "ac_id": "FR-162-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "model_hooks_answer_and_simulation_stays_unreduced",
      "ac_id": "FR-162-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "boundary_states_count_toward_max_states",
      "ac_id": "FR-162-AC-4",
      "trace": "correct"
    }
  ]
}
```

## Dispositions

No new finding needs disposition. Prior PR 2787's law-identity finding remains fixed in its original artifact; this review does not reopen it.

