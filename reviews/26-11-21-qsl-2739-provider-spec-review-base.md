---
id: SR-202611211854271
title: "base spec review of FSL PR3373 QSL-2739 provider contract alignment (FR-1044, FR-507, FR-530, TC-1330)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; PR3373 780a3d7b890510be4a4e8c83e0b585df46d1c42d over deccdc2a50c1632c8c7dbda448995e162ce7f76c; FR-1044, FR-507, FR-530, TC-1330; upstream FSpec #792 5423fcf4829b2b9d927ecae95305fcc0c1e06aa6 FR-331, FR-491, FR-492, FR-489, FR-290, FR-348, TC-271, TC-302"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: references
---

## Summary

Ticket: QSL-2739. This is the base review of draft PR #3373, frozen at 780a3d7b89 on base deccdc2a50. No rebase was done, and main has not changed the four files since the base. Upstream FSpec #792 (5423fcf482) was read from git objects only. The superseded assume-guarantee-local-rule audit and the prep note were used as inventory, not as authority.

The PR aligns FSL with #792 on the main points:
- the AG analyze route
- derived `assumptions` as request identity, with empty sets under the other three rules
- caller `assumptions` refused as an unknown member
- no context-free run of an assumed clause
- the producer-only `provider-answer-unconfirmed` guard and the refusal of that cause from providers
- typed `unsupported` for cycles under all four rules, before any write

The base findings are an impossible lattice-profile fixture and a false claim in an acceptance criterion. Sibling method artifacts carry the other findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1044-AC-7 (`test/system-lattice` selecting profile `fixed-point-lattice`), FR-530-AC-5 ("a profile that declares FR-176's lattice") and TC-1330 step 4 need a fixture that cannot exist. FSpec FR-489 "Cycles" says no requirement defines how a profile declares the lattice, "so none can be selected", and FSpec withdrew FR-489-AC-5 for exactly this reason. In FSL, no source unit can select the profile. It is reachable only through the test-only hook `with_composition_lattice` (fsl-semantics/src/check/check.rs:724, called only from tests/it/composition_planner.rs:169), which belongs to the superseded lattice design. Fix: withdraw FR-1044-AC-7 and FR-530-AC-5 with their ids kept, as FSpec did, and drop the profile clause from TC-1330 step 4, since AC-6 and AC-4 already state the answer. Record that the code PR deletes `composition_lattice`/`with_composition_lattice` rather than keeping them to drive a "profile changes nothing" test. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:400; spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md:177; spec/test-cases/TC-1330-native-composition-carries-and-checks-premises.md:31 |
| FND-002 | medium | FR-1044-AC-11 says the requests of `SysAG` and `SysAGRely` "differ by `assumptions` alone". That is false. `SysAGRely` declares one more `rely` member, so its `premises` has a third `rely` slot (`[cg Bounded, cg Capped, rely]` against `[cg Bounded, cg Capped]`), and its declaration node also differs. As written, the criterion cannot pass, or would pass only by observing the wrong thing. Fix: drop "alone". State instead that the two differ in `assumptions` (`[[], []]` vs `[[], [0]]`) and that a mutation of one request's `assumptions` alone is a different request identity (FSpec FR-331-AC-64, FR-491-AC-9). | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:404 |

## Verdict

**Changes required.** Across the method set there are 2 high, 4 medium and 9 low findings (one of the low is an info-only upstream note). Each one goes back to the same FSL spec author inside PR #3373. Clean and examined:
- **Single-route claim.** FR-1044 Decisions "same routed item" and FR-530 "two entries stay separate" agree with FSpec FR-491 "Derived item and slots" and FR-290 rows 154–156.
- **FR-530 honesty.** FR-530 adds no native TL-9 or fixed-point algorithm and no fake provider proof.
- **`SysRely` re-derivation.** Under `interface`, `SysRely` now derives empty sets and `Capped` is `proved`, as FR-491 "Assumption binding" requires. The assumed fixtures move to `SysAGRely`.
- **`SysAbsent` is buildable.** FR-491 static check 3 constrains only `C::G`, and an undeclared `H` contributes no index.
- **External items stay open.** AC-10 package parity and the text-mode driver are still open and external.
- **No false completion.** There is no false full-capability completion claim: "Remaining work: QSL-2739" is kept.

## Scope and evidence

This is a spec-only Markdown PR. Proposed tests were neither built nor run, and no coverage is claimed green. No Rust review (there is no Rust) and no gap matrix (there is no implementation claim). `quire validate` (quire 0.52.0, engine 0.62.2) on the four changed files exited 0 with no output. `gh pr view 3373` reports head 780a3d7b890510be4a4e8c83e0b585df46d1c42d, draft, OPEN.

- FR-1044: examined, all sections, AC-1..AC-14.
- FR-530: examined, diff and the sections it touches, AC-1..AC-10.
- FR-507: examined, row at line 105 and the family-read bullet at lines 132–138.
- TC-1330: examined, steps 3–8 and expected results.
- FSpec FR-331 Items 604–630, core-guard 913–946, counterexamples 1060–1075, AC-63..AC-66: context_only.
- FSpec FR-491, FR-492, FR-489, FR-290 rows 150–160 and 255–258, FR-348 Behavior: context_only.
- fsl-analyze/src/analyze/document.rs:3807-3925, document/composition_item.rs:40-60,380-460, provider.rs:315-360: context_only (bounded code-seam check).

## Dispositions

Round 1 (final disposition), fix commits 8692503282 and 5a4b61b1925aa3365c1773fe3fb8945784f99af5 over 780a3d7b89, same base deccdc2a50; verified against the committed text at 5a4b61b192, not the author's report. quire validate exit 0 on FR-1044, FR-530, FR-507, TC-1330.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-1044-AC-7 and FR-530-AC-5 are withdrawn with their ids kept, citing FSpec FR-489 (no selectable lattice profile; its AC-5 was withdrawn). The test/system-lattice fixture is gone, TC-1330 step 4 no longer has a profile, and FR-1044 Behavior says no source unit can select one. No fake profile or grammar was added. The code PR untags or deletes the stale FR-530-AC-5 tests (tests/it/composition_planner.rs:336, fsl-replay/src/planner/composition_tests.rs:437) and removes with_composition_lattice. |
| FND-002 | fixed | AC-11 no longer says 'alone'. It cites FSpec FR-331-AC-64 for equal-except-assumptions identity. |
