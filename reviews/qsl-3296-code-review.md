---
id: SR-5590831
title: "Code review of filament-spec-language PR 3898 (QSL-3296): three red-main fixes"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@020c4bdf1c8fcfde796e7008a6d50cca2c636f43; PR #3898 diff against merge base addba69c91; fsl-eval/src/simulation/model.rs, fsl-semantics/src/check/check.rs, fsl-semantics/tests/it/refinement_class_catalog.rs, fsl-analyze/tests/it/model_instances.rs"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-383
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-1040
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-125
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-084
  - type: references
    target: ix://agent-ix/filament-spec-language/FR-640
---
# Code review of filament-spec-language PR #3898

## Summary

Ticket: QSL-3296. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (4 files, +87/-31) at head 020c4bdf1c.
`origin/main` has moved past the base (addba69c91) by 8 commits, none of which
touches the four changed files; `git merge-tree --write-tree HEAD origin/main`
is clean and GitHub reports MERGEABLE. No tests were rerun by the reviewer
(owner rule); the coder's logs in
`worktrees/QSL-handoff-2026-11-24/gates-lead-conf/QSL-3296-*` were read.

**Fix 1, refinement class catalog test.** The old test asserted that a
contributing clause holding a post-test `repeat` has no keyed catalog
(`class_catalog_keys().is_none()`, `!v2_emittable()`), built with
`with_unwritten_refinements()/with_unwritten_protocols()`. FR-383 now states
that S4 writes "a post-test `repeat`" and "a protocol is never written without
a part of its graph" (no `not-yet-implemented` refusal), and 4e464cb957 (#3070)
emits post-test repeat controls; the base log shows the old test failing at
line 288 (`class_catalog_keys().is_none()`). The old expectation is wrong
under FR-383. The replacement runs the production `check_with` path, asserts
the full concrete class list as eight hand-written literals, asserts the keyed
catalog spells the same list, asserts `Local::Try` and `Local::E` are keyed by
the control nodes declared at hand-written paths
`Local/run/Main/Try` and `Local/run/Main/Again/Body/E` with
`semantic_form() == "control"`, and asserts `v2_emittable()`. It is strictly
stronger than the old test and its literals do not come from the code under
test. Its trace tag is the only problem (FND-001).

**Fix 2, `(ClockReference, ClockReference)` in `Typer::ordering`.** FR-1040
"`<` and the other orderings over two place ids refuse
`ill_typed`/`operator-ineligible`"; FR-1040-AC-6 tests `<`. `Typer::ordering`
serves every `OrderingOperator` and the new arm does not match on the
operator, so `<`, `<=`, `>`, `>=` all refuse `operator-ineligible`. A mixed
clock-reference/other operand still falls to `type-mismatch`, as for every
other kind. Equality is a separate path (`Typer::equality`), untouched; the
`clock_places::` suite (FR-1040-AC-7 `a = b` over `Reference<Clock>`,
AC-10 equality and the cross-kind `eq` refusal) passed on the change
(84 passed, `QSL-3296-fix-semantics.log`).

**Fix 3, admission keyed on `(UniverseId, key)`.** FR-084 "Conflicting identity"
and FR-084-AC-4 define the conflict as "the same universe and object identity
but different most-specific types"; FR-125 says "Two member records with one
universe and key and different types ... SHALL refuse". The new key matches
both. Checked against what must still be refused: an `Animal` and a `Dog` under
`c` (one hierarchy, one universe) in one or two populations or initial states
are still refused (FR-125-AC-13; `model_instancing_checks_bounds_initials_and_cross_snapshot_identity_first`
passes); same-type records are still admitted; a source-declared object type
reaches this loop only after `admit_initial` resolved its type through the
same `views` that `state_reference` reads, so `object_reference` cannot newly
fault there. Downstream, I looked for anything still keyed by object key alone
across universes: `instances`/`excluded_keys` key by text but only over the
populations overlapping one `over` type (one universe per FR-125);
`ClauseLetters::bound` additionally requires the row's type to conform to the
parameter; reference-literal resolution (`resolve_literal`) keys on
(type, identity) and requires conformance; symmetry renaming
(`Renaming::reference`) checks the universe; state keys are per population and
type (`encode.rs` references map keyed by type, then key). No path was found
where two now-admitted equal keys of different universes collide. The
two-model hyper test (`analyze_family_cli::a_hyper_clause_over_two_models_analyzes_and_replays_per_trace`,
FR-640-AC-12) now analyzes and replays per trace. The new assertion in
`model_instances.rs` would fail without the fix (the old check refused key `c`
held as `Animal` and as `Other`).

**Red-test comparison.** `QSL-3296-fix-analyze.log` and
`QSL-3296-baseline22-analyze-without-eval-fix.log` fail the same 22 tests, and
the extracted panic locations and left/right values are identical line for
line. The baseline run kept the `check.rs` change and the rewritten
fsl-semantics test; the test rewrite cannot reach fsl-analyze, and none of the
five failing test files uses a clock reference (`ref(`/`Reference<Clock>`), so
the ordering arm cannot cause them. The one fsl-eval failure
(`precondition_clauses_stop_at_the_first_non_true_clause_in_name_order`,
`left: [] right: [("Aaa", None)]`) is identical with and without the
model.rs change. Not investigated further, as instructed.

Rust lane: no `unwrap`/panic in production code added; the new `?` on
`object_reference` propagates an `AdmissionFailure` like the surrounding
code; the clone of `UniverseId` per object is a bounded cost over admitted
initial objects. Workspace check on 020c4bdf1c exit 0 with no warning in a
changed file.

## Verdict

PASS with two low findings. All three fixes are right against their owning
requirements, and the main risk (a downstream key-only collision) was not
found. The PR is mergeable once the lead decides on FND-001 (a one-line tag
fix in this PR or a deferral for the file-wide convention) and FND-002.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The rewritten test `a_clause_holding_an_attempt_and_a_post_test_repeat_keys_its_catalog` keeps `#[trace("FR-375-AC-23", "TC-310")]`. FR-375 and TC-310 are filament-specification artifacts; FSL has neither. AGENTS.md "Trace tag ids" says a bare id names an FSL artifact and an FSpec id carries the `FSpec-` prefix, so the tag should be `FSpec-FR-375-AC-23`/`FSpec-TC-310`. As written, `quire matrix` binds the test to nothing. The five sibling tests in the file carry the same bare tag (pre-existing). | fsl-semantics/tests/it/refinement_class_catalog.rs:257 |
| FND-002 | low | The new comment says equal keys in two object universes are "unrelated type hierarchies, as two models' are". Two selected models are not always in unrelated hierarchies: a model that imports or extends another model's object type shares that type's connected component, and so its universe. The code is right because it keys on the computed `UniverseId`, but the comment states a general rule that is false. Drop the "as two models' are" clause. | fsl-eval/src/simulation/model.rs:1588 |

## Dispositions

Round 1, head a3b75703c07f28be0a00bc35d627d834c45753cb (fix commit a3b75703c0 over 020c4bdf1c). The fix commit changes only comments in `fsl-eval/src/simulation/model.rs` and `fsl-semantics/src/check/lowering/protocol.rs`, `#[trace]` arguments in `fsl-semantics/tests/it/refinement_class_catalog.rs`, and adds the two SR files. All six prefixed ids exist on filament-specification origin/main (FR-375-AC-23 and FR-375-AC-27 in `spec/functional/analysis/FR-375-declare-model-refinement.md`, FR-432-AC-14 in `spec/functional/protocol/FR-432-map-every-protocol-step-in-refinements.md`, `spec/test-cases/TC-310-refinement-declaration-admission.md`), and FSL has no FR-375, FR-432 or TC-310. Merge with current origin/main (12 commits past the base) is clean, and none of those commits changes admission, universes, instancing or `over` binding. The stale `ProtocolConstruct` docs and uncreated variants in `fsl-semantics/src/check/refusal.rs` (PostTestRepeat, RepeatDecisionReadsBody, TerminalReadsOverUnderEach, Relationships, ReferenceCycle) predate this PR and are not touched by it; they are not a finding here.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | All six `#[trace]` tags in the file now carry `FSpec-` (`FSpec-FR-375-AC-23`, `FSpec-TC-310`, `FSpec-FR-375-AC-27`, `FSpec-FR-432-AC-14`); ids verified on FSpec origin/main. |
| FND-002 | fixed | Comment now reads "Equal keys in two object universes (unrelated type hierarchies) name two objects."; the false general clause is gone. |
