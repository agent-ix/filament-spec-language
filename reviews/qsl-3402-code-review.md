---
id: SR-967182
title: "QSL-3402 code review of parameter presence on the parameter type axis (filament-spec-language PR 4004)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@a7a5e0be8bd68f13f03c8bff23483747c91f39f6; git diff origin/main...HEAD (fsl-semantics model/conformance.rs, model/index.rs, tests/it/model_conformance.rs, tests/it/model_conformance_boundary.rs)"
review_set: subset
date: 2026-11-26
---

## Summary

Ticket: QSL-3402. PR filament-spec-language#4004, frozen head
a7a5e0be8bd68f13f03c8bff23483747c91f39f6. Rust lane (rust-review) folded in.
Owning requirements read at the pinned filament-specification revision
691ac5a: FSpec FR-151 (conformance table, AC-4, AC-21), FSpec TC-196 (R02,
R03), FSpec TC-2054 step 5, FSpec FR-154 "Presence and option", FSpec FR-322
step 4 option row, FSpec value-accounting `conformance.axis` row, FSpec
native-diagnostics `ill_typed` row; FSL FR-082 (AC-1, AC-5).

Which side was wrong: the code. FSpec FR-151's operation axes are arity,
parameter type, parameter multiplicity, result type, result multiplicity,
effect, precondition, postcondition; there is no presence axis, and TC-196
R02 prices one parameter at eight charges. The separate `parameter-presence`
charge was the defect; removing it restores the three red tests without
touching their expected values.

Verdict points the lead asked about:

- (a) Charging the option spelling like the `optional` spelling. Not stated
  outright. The value-accounting row says the parameter type axis charges
  `f(x)` for "the redefined parameter i's type", `f(x)` being the type
  derivation facts the conformance walk scans, "or one for a non-object type
  decided by the FR-149 equality-conversion table". FR-151's parameter type
  row makes that type the FR-322 step 4 type, `Option<X>` for both spellings,
  and FR-154 "Presence and option" says a reader derives no difference
  between the spellings. Equal charging follows. Whether `f(Option<X>)` is
  `f(X)` (the facts the walk scans, which the code now uses) or one (a
  non-object type) is not settled by the spec (FND-005). On main the
  `optional` spelling already charged `f(X)`, so the PR changes only the
  option-type spelling, and makes it agree with the other one.
- (b) Two distinct option value types over the same `X` as one type. This
  follows from the spec. FR-322 step 4's option row gives `Option<T>` as "the
  `composite_type`/`option` node over the node of `T`" and says the
  spellings share one node. FR-322-AC-298 has `OptInt`-typed and
  `optional Integer` fields deriving the same node. FR-154 forbids an option
  value type from declaring supertypes, so it has no nominal conformance of
  its own to lose. The FR-151 sentence "equal shapes of distinct declarations
  never make one narrower" is about declared supertype narrowing and does not
  reach structural option nodes.
- (c) The receiver-relative cause index. The code (unchanged `i + 1`) and the
  new test (`index: 2` for `secondApprover`) follow FR-151's "the receiver is
  parameter 0" and TC-196 R03, which gives `index 1` for the only parameter
  `p1`. FR-151-AC-21 and TC-2054 step 5 say "at position 1" for
  `secondApprover`, which is zero-based over `params`, the convention of
  FR-154's `Operation.params[i]` ("zero-based position `i`") and the
  FR-151-AC-20 call positions. Both readings name the same parameter, but the
  spec never states how `position` maps to the cause's parameter index
  (FND-006).

The new test fails on the old code for the intended reasons. Its committed
form has every assertion live: the three probe runs at 75044abe00 (main,
before the fix) failed at the committed line numbers. Line 1965: the
`optional`/`optional` pair cost 12 units, not 10, from two extra presence
charges. Line 1964: `optional Staff` against `required OptStaff` was refused
twice with `variance-parameter`. Line 1976: `required`/`optional` was
admitted. On head the test passes in the coder's `model_conformance` run (79
passed).

Mirrors and dependents: `check_operation_redefinition` is the only
implementation and is called only from `check/model_conformance.rs`, for both
the source and package routes. Searching for `variance-parameter`,
`VarianceParameter` and `ConformanceAxis` outside the semantics crate found
only the root `forms_package_contracts` test, which ran. The reviewer added
one run on this head for redefinition consumers the coder's filters did not
select (`model_unit_inheritance`, which holds an FR-082-AC-1 binder,
`model_dispatch`, `dispatch_linking`, `operation_clauses`): 47 passed,
1 ignored, 0 failed.

Rust lane: `ModelIndex::record` mirrors the existing `field` lookup;
`parameter_type` borrows without cloning; there are no new panics, `unwrap`
calls or integer conversions; `implements` is imported bare from
`ix_trace_rs`. Clippy with `-D warnings` exited 0 on this head.

Main interaction: origin/main has 3 commits past the head's base (#3979,
#4003, #4002), touching only fsl-analyze tests, fsl-replay, fsl-eval
simulation, reviews and FSL spec. None of them touches fsl-semantics. The
merge-check rule still asks for a head that contains current origin/main.

## Verdict

MERGEABLE after the lead decides FND-001. There is no high finding. The
verdict change is right on every case the spec states. FND-001 is a gap in
the cause payload, not a wrong verdict. FND-005 and FND-006 are spec
ambiguities to raise as spec-gap tickets.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A presence mismatch is now reported on the parameter type axis as `VarianceParameter { declared, redefined }`, which carries the two `typeRef`s (`Staff`, `Staff`) and not the effective types `Option<Staff>` and `Staff`. FSpec native-diagnostics and FR-272 require a conformance cause to keep "the expected and actual effective type". The cause of an `optional Staff`/`required Staff` refusal therefore reads like a failed `Staff`-to-`Staff` conformance, and only the free-text `detail` says that optionality differs. The parameter-presence axis this PR removes had the same gap. | fsl-semantics/src/model/conformance.rs:427-458 |
| FND-002 | low | The test's 10-unit assertion cannot tell the new type-axis charge subject (`f(X)` for the option spelling) from the old one (`f(OptStaff)`), because `Staff` has no supertypes and both cost one. The PR's stated pricing change ("the type axis now prices the conformance walk on X for both spellings") is untested. A `Staff` with one supertype would separate the two. | fsl-semantics/tests/it/model_conformance_boundary.rs:1954-1966 |
| FND-003 | low | The edited `operation_parameter_presence_is_kept_under_redefinition` is retagged `FSpec-FR-151-AC-21`, but it covers two of AC-21's four pairs over the bundle-h fixture rather than TC-2054's, and it asserts with `any(...)`. It is partial evidence for AC-21, not a test that completes it. The new boundary test completes AC-21, and the old `FSpec-FR-151-AC-20` tag was wrong too. | fsl-semantics/tests/it/model_conformance.rs:465-527 |
| FND-004 | low | The PR's criteria list names `FSpec-FR-151-AC-11` (the effect axis work units). The diff neither changes that axis nor tags it, so the traceability list and the tags disagree. | fsl-semantics/src/model/conformance.rs:352-357 |
| FND-005 | low | Spec ambiguity: the FSpec value-accounting `conformance.axis` row prices the parameter type axis at `f(x)` of the redefined parameter's type: the facts the conformance walk scans, or one for a non-object type. It does not say whether `Option<X>` costs `f(X)` or one, so the PR's `f(X)` for both spellings is consistent with the spec but not stated by it. | fsl-semantics/src/model/conformance.rs:412-415 |
| FND-006 | low | Spec ambiguity: FSpec FR-151-AC-21 and TC-2054 step 5 refuse "at position 1" (zero-based over `params`), while FR-151 makes the receiver parameter 0 and TC-196 R03 reports `index 1` for the first parameter. The spec does not state how the position maps to the cause's parameter index. The code and test use the receiver-relative index 2 for `secondApprover`. | fsl-semantics/tests/it/model_conformance_boundary.rs:1985-1994 |

## Dispositions

Round 1, head 28acf82a0360e67f9fbf72936591f64f550f360f, which contains
origin/main 070824b9fa. The fix commits are 5172d3ba68 and 7e9a883ded; the
other commits are merges of main. The changed test lists itself, but
`EffectiveParameterType` does not exist before the fix, so the test cannot
compile against the unfixed code. Reading the old behavior instead, the test
still fails there for the intended reasons:

- The admitted pairs would cost 14 units, not 12: 12 plus one presence charge
  per parameter.
- `optional Staff` against `required OptStaff` would be refused.
- `required`/`optional` would be admitted.

These are the same failures an earlier form of the test showed at
75044abe00. The new assertions add only the exact cause and the supertype
pricing.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `VarianceParameter` now carries `EffectiveParameterType { optional, value_type }` for both sides: `declared` is the redefined (expected) side and `redefined` the redefining (actual) side. The detail text reads `expected Option<Staff> to conform to Staff`. The payload has no serialization (only the tag is exposed), and the workspace check compiles every user of it. |
| FND-002 | fixed | `Staff` now has the supertype `Person`, so `f(Staff)` is 2 and the admitted pairs assert 12 units. A walk over the option type would cost 11, so the assertion separates the two subjects. |
| FND-003 | fixed | The `FSpec-FR-151-AC-21` tag is removed from `operation_parameter_presence_is_kept_under_redefinition`. The boundary test alone completes AC-21. |
| FND-004 | fixed | The PR criteria list is now FSpec-FR-151-AC-4, FSpec-FR-151-AC-21, FR-082-AC-1 and FR-082-AC-5, which matches the tags. |
| FND-005 | deferred | Spec gap filed as STD-741 (pricing of a conformance axis over `Option<X>`). The code's choice, `f(X)`, is consistent with the spec, so it does not block. |
| FND-006 | deferred | Spec gap filed as STD-742 (`variance-parameter` position against the receiver-relative index). The code follows TC-196 R03, so it does not block. |
