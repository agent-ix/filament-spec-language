---
id: SR-202611210022442313144108543
title: "code-review review of QSL-2248 finite HP-2 reductions"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2248; PR3366; finite HP-2 code, tests and FR-174"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: references
---

## Summary

Code/Rust review of finite HP-2 execution and typed settlement found two scoped correctness defects in constraint admission and transition-budget units.

## Verdict

**CONDITIONAL** — Fix both scoped findings before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Finite constraints are admitted only after unconstrained component enumeration and complete orbit construction, so excluded paths or orbits can exhaust budgets or fail before the required cut is recorded. | fsl-analyze/src/model_check/hyper/closed.rs:184-212,252-275,345-410; closed/reduction.rs:161-168 |
| FND-002 | medium | MaxTransitions is debited for tuple bookkeeping and permutation-generator applications instead of actual joint steps or moves, changing the caller's budget units and stopping otherwise sufficient reduced runs. | fsl-analyze/src/model_check/hyper/closed/reduction.rs:71,89,114,130,193-215 |

## Coverage

The review is frozen at `2c971d31c9802b90320c3817663d5f86c1bf87a0`. No Cargo was run by this reviewer. Source qualification was captured at `974d08be2152969d5cc04b5e58470c6f5a032e01`; the final delta is only FR-174 Markdown and a Rust doc comment.

FND-001 follows directly from the call order: `closed_executions` generates complete unconstrained paths at closed.rs:188, and `closed_quotient` stores and limits their orbits at reduction.rs:161, before `closed_constraint` is reached at closed.rs:252. A concrete static regression uses SECURE_CONSTRAINED, one key v, one initial snapshot (h=1,l=1), horizon one, Zero selected, and max_states=1. Zero rejects the initial component, so no successor may be explored, but two unconstrained i=0/1 paths exceed the component limit first. An analogous matched later cut must not evaluate contracts or match steps strictly after that cut; a horizon-three zero-preserving prefix can also fit while its unconstrained eight completions exceed a smaller state allowance. Preserve entering-match-before-cut order and keep excluded prefixes out of admitted orbit/state accounting. These scenarios were reasoned from source, not executed.

FND-002 has an independent static regression using uniform_snapshots([v]) and horizon one: four component paths form sixteen concrete pairs. An ordinary run fits max_transitions=16; Low-selected copy quotient additionally charges the copy-generator BFS and raw enumeration, consuming 32 bookkeeping units before admission although it has no more than those sixteen real joint moves. The zero-budget test cannot distinguish these units. Keep joint-move counters distinct from algorithm work.

The fix must retain a real resource bound and cancellation for orbit work: do not merely make the BFS free or debit removed orbit members as canonical stored states. Existing `model_check/quotient.rs::Analyzer::charge` uses the run's `filament_exact::Meter`, `ChargePoint::GraphExpand`, and a typed meter stop; Hyper Run::meter_stop already maps this to EvaluationMeter with its real bound. Confirm the applicable meter scope under existing FR-184/FR-255 accounting before reusing it. ADR-021 SYM-7/RU-2 also provides the existing nonfactorial canonicalization rationale; FR-174's exact least-orbit requirement still controls this hyper quotient. No new public limit is proposed.

The real-path keys, typed reduced scope, no-selection HoldsClosed, preservation precedence, least representative ordering, replay, terminal false extension and retained infinite-certificate negative were inspected. No vendoring, alternate wire schema, test-only production branch, panic-based caller refusal, or copied upstream vocabulary was introduced.

## Verification and scope

Actual focused evidence: check exit 0; one combined runtime 28 passed / 0 failed / 0 ignored / 2643 filtered (1.66s). Logs: /private/tmp/qsl2248-finite-c4.log and /private/tmp/qsl2248-finite-t1.log. The diagonal 432-to-46 oracle, exact boundary count three and concrete deterministic replay assert genuine behavior, but do not cover the two scoped defects.

Both Clippy attempts exit 101 on unchanged restore.rs at 111, 246 and 407; analyzer/replay lint was not reached. Base and head restore blob are both 78661e07452a6743c3f183b8ebca5dc0abd8bc74. The unchanged sample_fingerprint warning has matching blob 7a8c6dd7f8cc44e6d99796084f23bef43b15cc40. These are documented baseline checks, not a lint PASS.

Direct rustfmt with skip_children=true on all eleven changed Rust files, git diff --check, and explicit FR-174 Quire validation exit 0. No applicable AssuranceProfile document exists; repository mentions of that type are prose in old reviews.

Applicable methods: code-review with Rust lane; gap-analysis; spec-review/base. The only spec edit is a current-behavior paragraph with no changed normative statement, criterion, relationship or domain object, so EARS/integrity/dependency/object/security lenses are not selected. Plan completion: not assessed.

## Reviewed units

```yaml
{
  "scope": [
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
