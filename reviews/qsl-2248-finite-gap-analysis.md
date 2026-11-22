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

## Dispositions

### Round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Root explicitly accepts the original whole-repository untagged-criterion baseline for this scoped delivery only; no new matrix run or global PASS is claimed. |
| FND-002 | accepted-no-change | Root explicitly accepts the original ignored-only baseline for this scoped delivery only; actual 34-case qualification is separate and does not discharge the global findings. |
| FND-003 | accepted-no-change | Root explicitly accepts the incomplete whole-repository reverse audit as existing baseline, scoped only. This bounded disposition does not repeat or complete that audit. |
| FND-004 | fixed | The six new adversarial cases execute initial-cut/no-expansion, later excluded failure, excluded-completion state allowance, nonzero joint-move units, work-meter exhaustion and real in-flight cancellation. Captured combined qualification is34PASS0FAIL0IGNORE. New Code FND003 separately requires its own regression before merge. |

Frozen fix head: `5a4c944e70fd9dc28dc88b40eef76e24d1b59002`. Actual root-supplied and raw-log-verified focused integration compile exits 0; combined named run exits 0 with **34 passed, 0 failed, 0 ignored, 2800 filtered**, runtime 2.25 seconds. All six new adversarial names appear as `ok`. Logs: `/private/tmp/qsl2248-round1-5a4c94-check.log` and `/private/tmp/qsl2248-round1-5a4c94-combined.log`.

This is the same original reviewer's bounded disposition round one, not a new full review or matrix. No Cargo, Titan, tracker, relay or source edits were run by this reviewer. The incoming receipt allocation/fail-fast source and Cargo.toml/Cargo.lock are unchanged relative to receipt base `2fa667`; read-only `git diff --quiet` exits 0. Worktree is clean. Root centrally publishes the prepared private comments and verifies read-back.

The original scoped FND-001/FND-002 repairs are real and covered by the new six passing cases. However, the new FND-003 introduced by the rewrite remains open. **PR3366 is not merge-ready**, despite the passing 34-case run.

## Latest disposition verdict

**FAIL** remains the repository-wide gap verdict. Baseline FND-001–003 are accepted-no-change for scoped delivery only; scoped FND-004 is fixed. The new Code/Rust FND-003 still blocks this PR and requires a real bounded-refutation test.

## Final bounded disposition (round 2)

No new gap finding or additional disposition row is required: original global FND-001–003 retain their explicit scoped-only accepted-no-change outcomes, and original scoped FND-004 remains fixed. The three additional real round-two tests now exercise the new Code/Rust FND-003's adverse boundary and ordering guards; that finding is fixed in its owning artifact.

Frozen final head: `9fa544b166edf5e0011d9cfb5e0ee3bedd043c07`. Root's captured combined run exits 0: **37 passed, 0 failed, 0 ignored, 2800 filtered**, runtime 3.16 seconds. Log: `/private/tmp/qsl2248-round2-stream-combined.log`. All original 34 controls and the three round-two guards appear as `ok`.

The real pre-fix control at `797f6cad889d3e7b963fee3ba720f30c9496abef` executed and failed exactly as reported: 0 passed / 1 failed, `MaxTransitions=2`, zero product states and two moves before any judgment. Log: `/private/tmp/qsl2248-round2-797f6c-red.log`. This turns the prior static finding into a recorded adverse execution; it is not a hypothetical failure. The parent also supplied the focused original-regression pass on the production fix commit `ec27d18bc9`; the final 37-case log independently retains that pass on the frozen final head.

Bounded source assessment: `Choices` shares component alternatives by Arc and splits compact ranges into at most two pending boxes, rather than materializing their Cartesian product. Admission and joint-move charges happen only when a singleton is popped. A real completed tuple invokes `visit_closed_tuple` immediately; only an actually traversed least member is judged, and its canonical key is judged once. Ordinary lower bounds are component-major; selected bounds minimize each compact box over full group actions, including actions that fix the current prefix but distinguish future successors. Key/action/box work still charges GraphExpand and observes cancellation.

FND-001/FND-002 repairs remain intact: initial constraints precede expansion, entering matches precede successor cuts, excluded successors never queue, cut predecessors and completed members remain separate, complete orbit members alone use max_states, and actual admitted moves alone use max_transitions. Actual replay, terminal false extension, the 432-to-46 orbit oracle, boundary three, exact16/short15 move units, WorkUnits8, real in-flight cancellation and the infinite certificate negative are retained in the passing group.

This is the same original reviewer's **final disposition round 2 of maximum 2**, not a fresh full review or matrix. No Cargo, Titan, tracker, relay or source edits were run by this reviewer. Read-only direct rustfmt on the four round-two Rust files and git diff --check exit 0. HEAD is clean and unchanged; Cargo.toml/Cargo.lock and incoming receipt allocation/fail-fast paths remain identical to receipt base `2fa667`. The root owns subsequent central workspace verification, metadata custody, private publication/read-back and merge.

## Final gap verdict

**FAIL** remains the repository-wide assurance verdict. The original whole-repository counts and incomplete reverse audit are not recomputed, silently discharged or promoted to PASS. Scoped delivery has no remaining local gap finding after this bounded final disposition.

## Delivery hold

Root/arch currently holds finite delivery while the separately owned main emit/source-map regression repair runs. This bounded review PASS does not lift that hold: keep 9fa544b166edf5e0011d9cfb5e0ee3bedd043c07 frozen, run no further finite Cargo, and perform no finite merge until root releases the sequence. The unchanged-head object-free G-153 probe also actually passed 1/0/0 (1093 filtered), log /private/tmp/qsl2248-round2-9fa544-object-free.log.
