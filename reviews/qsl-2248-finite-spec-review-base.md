---
id: SR-202611210022442313144108545
title: "spec-review/base review of QSL-2248 finite HP-2 reductions"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2248; PR3366; finite HP-2 code, tests and FR-174"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: references
---

## Summary

Base review of the FR-174 current-behavior paragraph: the edit introduces no new eligibility, criterion, object, relationship or wire requirement.

## Verdict

**PASS** — Specification edit only; source acceptance is decided by the Code/Rust review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

The paragraph accurately states the intended finite route and preserves no-selection HoldsClosed, match-before-cut ordering, concrete replay and unchanged refusal precedence. Normative eligibility and AC-7 remain unchanged; no new requirement is authored or approved by this review. Implementation acceptance remains conditional under the separate Code/Rust artifact's concrete admission/budget findings.

ID, relationship, criterion and link structure were checked; explicit FR-174 Quire validation exits 0. No applicable AssuranceProfile requires an enforced workflow; direct base review applies. Optional lenses were omitted because no normative requirement, structure, domain object or relationship changed. No hash/pin/file-tracking requirement was added. Existing global coverage gaps are recorded in the separate gap artifact, not hidden by this specification-readiness PASS.

## Reviewed units

```yaml
{
  "scope": [
    {
      "id": "FR-174",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "**Current system behaviour.** EN-1 executes selected key symmetry and state\nconstraints on infinite-trace HP-2 products and event-position HP-2 closed\nexecution tuples. The finite route retains one least concrete tuple per\norbit under the diagonal action of the object parameters' stabiliser and\nthe eligible copy group. Entering matches precede constraint checks; cut\npredecessor prefixes are recorded separately and cannot become terminal\ntraces or proofs. Finite refutations carry actual selected closed paths\nand replay under FR-173. A row that reads \"no\" still takes preceden",
      "excerpt_part": "1/2"
    },
    {
      "id": "FR-174",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "ce and\nsettles `ReductionNotPreserving`. With no selection, the finite route\ncontinues to decide the closed scope with `HoldsClosed`.",
      "excerpt_part": "2/2"
    },
    {
      "id": "FR-173-AC-6",
      "path": "spec/functional/FR-173-classify-hyper-clauses-into-forms.md",
      "role": "examined",
      "excerpt": "Under the event-position profile, analyzed through `native-analyze/1` with EN-1, never `not-yet-implemented`. Fixtures: `fsl-analyze`'s two Secure vault models `V` and `N` (`v: V::Vault`, `w: N::Vault`), each with the one initial state `h = 1`, `l = 0`, whose `step(i)` sets `l = i` for `i` in `{0, 1}` and which have no terminal state, and FR-178-AC-1's busy vault. (HP-2) `forall trace a of V forall trace b of N { always [0, 3] holds(v.l @ a = w.l @ b) }` settles `refuted`: its counterexample holds one trace for `a` and one for `b`, neither with a loop, each of four position",
      "excerpt_part": "1/3"
    },
    {
      "id": "FR-173-AC-6",
      "path": "spec/functional/FR-173-classify-hyper-clauses-into-forms.md",
      "role": "examined",
      "excerpt": "s, 0 to 3, and `fsl_replay::replay` reads each as a closed trace under its own `temporal_bindings` row and reproduces the refutation; the same clause with body `always [0, 3] holds(v.h @ a = w.h @ b)` settles `proved`. (HP-3) `forall trace a of V exists trace b of N { always [0, 3] holds(v.l @ a = w.l @ b) }` settles `proved`, and with `!=` in place of `=` settles `refuted` at position 0 with a universal trace that has no loop. (HP-6) FR-178-AC-1's `align skip { V::Vault::mix }` clause over the busy vault from its idle initial states, with body `always [0, 2] holds(v.l @ a ",
      "excerpt_part": "2/3"
    },
    {
      "id": "FR-173-AC-6",
      "path": "spec/functional/FR-173-classify-hyper-clauses-into-forms.md",
      "role": "examined",
      "excerpt": "= v.l @ b)`, settles `proved`, and FR-178-AC-2's leaky vault started mid-operation with the same body settles `refuted` with a projected counterexample of two closed traces, no loop, that replays. (HP-5) `exists trace b of V { eventually [0, 2] holds(v.l @ b = 1) }` settles `proved` with one closed witness of three positions; `exists trace b of V { eventually [0, 2] holds(v.h @ b = 0) }` settles `refuted` with a trap at the initial state. FR-640-AC-12's `Saves` settles `proved` or `refuted` by the HP-2 construction above, never `FormNotDecided`.",
      "excerpt_part": "3/3"
    },
    {
      "id": "FR-174-AC-4",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "With partial-order reduction selected, `NonInterference` settles `ReductionNotPreserving` before any expansion; with a state constraint selected, `Opaque` settles `ReductionNotPreserving`; with key symmetry selected over `SavesPower`'s two subjects, it settles `ReductionNotPreserving`.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-174-AC-7",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "Over the secure vault with the two vaults `v` and `w`, `NonInterference` (whole fairness) with the key symmetry of `vaults` `[[v, w]]` selected returns `HoldsReduced` whose `reductions` are `Symmetry{..}` with orbit `[v, w]` then `CopySwap{[[a, b]]}`, and stores fewer product states than the same run with no selection, which also holds; over the leaky vault the same selection returns `Violated` with a counterexample that replays and names no reduction. With the state constraint `Low` (`self.l <= 1`, met by every reachable state) selected, the secure `NonInterference` return",
      "excerpt_part": "1/3"
    },
    {
      "id": "FR-174-AC-7",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "s `HoldsReduced{[CopySwap{[[a, b]]}]}`; with `Zero` (`self.l = 0`) selected it returns `ConstraintReached` with its boundary states; with key symmetry and `Low` both selected it returns `HoldsReduced` naming `Symmetry` then `CopySwap`. The constraint is eligible on every HP-2 item: with `Low` selected, `SavesPower` over two subjects and `NonInterference` with `fair each` constraints each run under the constraint, settle neither V-8 nor `ReductionNotPreserving`, and give the verdict of the same run with no selection. With key symmetry selected over one subject, `Opaque` (HP-",
      "excerpt_part": "2/3"
    },
    {
      "id": "FR-174-AC-7",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "3, ineligible) settles V-8 `Unsupported` naming the symmetry before any expansion. With key symmetry selected and `fair each` constraints on `NonInterference`, it settles `ReductionNotPreserving`. With no selection, `NonInterference` over the secure vault holds with the reduction `CopySwap`. With partial-order reduction and key symmetry both selected on `Opaque`, it settles `ReductionNotPreserving` naming partial-order reduction.",
      "excerpt_part": "3/3"
    },
    {
      "id": "FR-174",
      "path": "spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md",
      "role": "examined",
      "excerpt": "EN-1 SHALL check the selected constraint on every\n  component state of every product state it creates and SHALL store no\n  successor any of whose components violates it, recording each such\n  product state's predecessor as a boundary state (ADR-021 SC-2)."
    },
    {
      "id": "FR-184",
      "path": "spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md",
      "role": "examined",
      "excerpt": "For hyper items, `max_states` SHALL count product states,\n  `max_transitions` joint steps or moves, and `max_automaton_states`\n  automaton states, as FR-176 and FR-178 state."
    }
  ]
}
```

## Disposition pass 1

The FR-174 paragraph and the original finding-free specification review are unchanged. **PASS for the specification text only** remains; this is not implementation acceptance. Code/Rust now has a newly recorded open FND-003. No additional spec analysis, matrix, Cargo or tracker operation was performed.

Frozen fix head: `5a4c944e70fd9dc28dc88b40eef76e24d1b59002`. Actual root-supplied and raw-log-verified focused integration compile exits 0; combined named run exits 0 with **34 passed, 0 failed, 0 ignored, 2800 filtered**, runtime 2.25 seconds. All six new adversarial names appear as `ok`. Logs: `/private/tmp/qsl2248-round1-5a4c94-check.log` and `/private/tmp/qsl2248-round1-5a4c94-combined.log`.

This is the same original reviewer's bounded disposition round one, not a new full review or matrix. No Cargo, Titan, tracker, relay or source edits were run by this reviewer. The incoming receipt allocation/fail-fast source and Cargo.toml/Cargo.lock are unchanged relative to receipt base `2fa667`; read-only `git diff --quiet` exits 0. Worktree is clean. Root centrally publishes the prepared private comments and verifies read-back.

The original scoped FND-001/FND-002 repairs are real and covered by the new six passing cases. However, the new FND-003 introduced by the rewrite remains open. **PR3366 is not merge-ready**, despite the passing 34-case run.

## Final bounded disposition (round 2)

**PASS for the unchanged FR-174 specification text** remains. The original placeholder is not a real finding and receives no disposition row. Round two changes private execution/order machinery and its tests; it introduces no new requirement, eligibility, wire schema, public limit, pin or dependency. No additional spec analysis or full audit ran. The associated Code/Rust FND-003 is now fixed; the repository-wide gap verdict remains FAIL with scoped-only accepted baseline findings.

Frozen final head: `9fa544b166edf5e0011d9cfb5e0ee3bedd043c07`. Root's captured combined run exits 0: **37 passed, 0 failed, 0 ignored, 2800 filtered**, runtime 3.16 seconds. Log: `/private/tmp/qsl2248-round2-stream-combined.log`. All original 34 controls and the three round-two guards appear as `ok`.

The real pre-fix control at `797f6cad889d3e7b963fee3ba720f30c9496abef` executed and failed exactly as reported: 0 passed / 1 failed, `MaxTransitions=2`, zero product states and two moves before any judgment. Log: `/private/tmp/qsl2248-round2-797f6c-red.log`. This turns the prior static finding into a recorded adverse execution; it is not a hypothetical failure. The parent also supplied the focused original-regression pass on the production fix commit `ec27d18bc9`; the final 37-case log independently retains that pass on the frozen final head.

Bounded source assessment: `Choices` shares component alternatives by Arc and splits compact ranges into at most two pending boxes, rather than materializing their Cartesian product. Admission and joint-move charges happen only when a singleton is popped. A real completed tuple invokes `visit_closed_tuple` immediately; only an actually traversed least member is judged, and its canonical key is judged once. Ordinary lower bounds are component-major; selected bounds minimize each compact box over full group actions, including actions that fix the current prefix but distinguish future successors. Key/action/box work still charges GraphExpand and observes cancellation.

FND-001/FND-002 repairs remain intact: initial constraints precede expansion, entering matches precede successor cuts, excluded successors never queue, cut predecessors and completed members remain separate, complete orbit members alone use max_states, and actual admitted moves alone use max_transitions. Actual replay, terminal false extension, the 432-to-46 orbit oracle, boundary three, exact16/short15 move units, WorkUnits8, real in-flight cancellation and the infinite certificate negative are retained in the passing group.

This is the same original reviewer's **final disposition round 2 of maximum 2**, not a fresh full review or matrix. No Cargo, Titan, tracker, relay or source edits were run by this reviewer. Read-only direct rustfmt on the four round-two Rust files and git diff --check exit 0. HEAD is clean and unchanged; Cargo.toml/Cargo.lock and incoming receipt allocation/fail-fast paths remain identical to receipt base `2fa667`. The root owns subsequent central workspace verification, metadata custody, private publication/read-back and merge.

## Delivery hold

Root/arch currently holds finite delivery while the separately owned main emit/source-map regression repair runs. This bounded review PASS does not lift that hold: keep 9fa544b166edf5e0011d9cfb5e0ee3bedd043c07 frozen, run no further finite Cargo, and perform no finite merge until root releases the sequence. The unchanged-head object-free G-153 probe also actually passed 1/0/0 (1093 filtered), log /private/tmp/qsl2248-round2-9fa544-object-free.log.
