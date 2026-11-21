---
id: SR-202611210022442313144108544
title: "gap-analysis review of QSL-2248 finite HP-2 reductions"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2248; PR3366; finite HP-2 code, tests and FR-174"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: references
---

## Summary

Repository-driven traceability audit retains global baseline gaps and finds missing adversarial finite-admission/budget-unit coverage.

## Verdict

**FAIL** — Repository-wide traceability and reverse auditing remain incomplete; scoped regressions also require tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The whole-repository matrix reports 1144 untagged criteria; repository-wide source traceability is not established. | spec/functional/FR-001-read-exact-source.md:acceptance table |
| FND-002 | high | The whole-repository matrix reports 77 criteria bound only by ignored tests; ordinary runs provide no executed evidence for those rows. | spec/functional/FR-056-admit-domain-package-model-declarations.md:acceptance table |
| FND-003 | high | The repository reverse audit remains incomplete: 16735 public-surface candidates and 318 stub-pattern candidates were inventoried but not exhaustively mapped and classified; extraction diagnostics and unmatched tags also remain globally unresolved. | repository source surface:public-surface.txt; coverage.json |
| FND-004 | medium | The finite constraint tests use roomy state budgets and the only transition-budget test uses zero, so they do not detect work beyond a cut or distinguish joint moves from quotient-bookkeeping charges. | fsl-analyze/tests/it/hyper_reductions.rs:347-419,484-589 |

## Coverage

Plan completion: not assessed.

Reconciliation: quoin 0.36.7 matrix exit 1: native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size. Recorded once; approved static Quire fallback exit 0. No run evidence store was read.

Frozen-head static matrix rollup: tagged 4882; untagged 1144; ignored-only 77; method-without-symbol 106; retired 13. These are this branch's computed rows, not copied counts from another branch. FR-173-AC-6 and FR-174-AC-7 are tagged; tags do not prove semantic completeness. The coverage report's separate totals report 6172 criteria; its reference-table totals and this matrix's row census have different populations and are not converted to a coverage percentage.

Whole-tree inventory: 16735 public-surface candidates and 318 stub-pattern candidates. Scoped new surfaces map to FR-173/174/182/184. No new scoped stub or unspecified public behavior was found; the full reverse mapping and classification are incomplete. Coverage reports 22800 diagnostics, 5833 untracked-symbol records and 6663 unmatched-tag records, whose reasons remain globally unclassified. Rust census: 13691 candidates, 13121 tagged, 12601 bound. These extraction records are not all asserted to be defects.

Scoped semantic inspection was explicitly requested by the parent. It identifies the two live Code/Rust defects and FND-004's missing regressions. Global baseline findings remain FAIL and may be accepted only for scoped delivery, never promoted to a repository PASS.

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
    },
    {
      "id": "FR-001-AC-7",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`. From initial `a` `3ff0000000000000` (1.0) it reaches exactly the states 1.0 and `bff0000000000000` (-1.0), with one candidate per application. From initial `a` `0000000000000000` (+0) it reaches exactly +0 and `8000000000000000` (-0), two states with distinct keys, and each application has two candidates, because both zeros are `numericEqual` to the solved value (FR-120-AC-34). A s",
      "excerpt_part": "1/2"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "imulation run applying `flip()` twice from 1.0 ends at 1.0 with no flag.",
      "excerpt_part": "2/2"
    }
  ]
}
```
