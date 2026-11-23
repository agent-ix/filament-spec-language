---
id: SR-20261123010001
title: "Code review of filament-spec-language PR #3575: lift nested reference rows (option, collection, map)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@af2554ebd81b6632cec71c046914f1ef2c2a102e; PR #3575 diff against origin/main 891e6137: fsl-semantics/src/check/refinement.rs, fsl-semantics/src/check/refinement/history.rs, fsl-semantics/src/check/refinement/protocol.rs, fsl-replay/src/refinement/map.rs, fsl-semantics/tests/it/refinement_declarations.rs, fsl-replay/tests/it/refinement_unmapped.rs, fsl-analyze/tests/it/analyze_family_cli.rs, fsl-analyze/tests/it/refinement_reads.rs, tests/support/ring_world.rs, fsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
---
# Code review of filament-spec-language PR #3575

## Summary

Ticket: QSL-2945. PR: filament-spec-language#3575, head af2554eb. The Rust
lane (rust-review) is folded into this file. Arch scope ruling: this PR lands
option, collection and map lifting only; composite (record, tuple, union)
nested references follow in a second PR.

The S3 removal is clean. `UnsupportedFeature::NestedReferenceRow`,
`nests_reference`, `holds_reference`, the `Nested` enum, `RowContext::field_row`
and the `Checker::profile` field are gone, and nothing still uses them. The only
deleted logic ran when `field_row` was true and the row type nested a
reference. Non-nested rows go through the same typer, conformance and refusal
path as before, so their behaviour does not change. The replay `lift_nested`
recursion is total: no `unwrap`, no `expect`, no indexing. It keeps the
element order (`iter().map().collect()`) and the map entry keys and order
(`from_admitted_map` keeps the order it is given). An unmapped element gives
`Lifting::Unmapped`, which the caller turns into `Undetermined` with
`UnmappedType`.

Focused tests at 4f7e9983 (the last commit, af2554eb, changes one doc comment
in fsl-package only): `fsl-semantics --test it refinement_declarations`: 31
passed; `fsl-replay --test it refinement_unmapped`: 10 passed;
`fsl-analyze --test it a_nested_reference`: 1 passed. `cargo check --locked
--workspace --all-targets` passes. The only warning in a touched file is
FND-006.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Option and Map arms of replay `lift_nested` have no test. Only the Sequence arm is tested (FR-140-AC-16). A wrong payload type, a dropped `none`, a changed map key or a reordered map entry would all pass. | fsl-replay/src/refinement/map.rs:534-560 |
| FND-002 | medium | No test at any layer exercises a reference nested more than one level deep (for example `Sequence<Option<Reference<Ring>>>`), although FR-135-AC-6 and FR-140 claim lifting "at every depth". | fsl-replay/src/refinement/map.rs:519-562 |
| FND-003 | low | When the value and declared shapes differ, `(plain, _) => Ok(plain)` passes the value through unchanged, and any concrete references inside it are not lifted. The removed code returned `Lifting::Fault` there. If that invariant breaks, the mapped abstract state silently holds concrete references instead of `MappingFailure::Fault`. | fsl-replay/src/refinement/map.rs:561 |
| FND-004 | medium | Pre-existing, adjacent. S3 types step-application arguments with `lift()`, which lifts nested references, so it admits a `Sequence<Reference<Ring>>` argument for a `Sequence<Reference<Queue>>` parameter. Replay `lift_value` lifts only top-level references, so the abstract operation receives concrete references. `lift_nested` is the fix. | fsl-replay/src/refinement/map.rs:597-610 |
| FND-005 | low | rustfmt is not clean in touched hunks: a misindented `};` at refinement.rs:3851, plus diffs at map.rs:524 and :547 and analyze_family_cli.rs:362. | fsl-semantics/src/check/refinement.rs:3851 |
| FND-006 | medium | `CollectionKind` is now an unused import in map.rs: its only use was the removed `CollectionKind::Set` guard. Building fsl-replay prints `warning: unused import: CollectionKind`, so the repo's `cargo clippy --workspace --all-targets -- -D warnings` (Makefile:61) fails. | fsl-replay/src/refinement/map.rs:25 |

## Verdict

The removal and the recursion are correct. The tests that exist assert exact
codes, causes, rows, objects and order. Before merge, FND-001 and FND-002 need
tests, FND-005 needs `cargo fmt`, and FND-006 needs the import removed. FND-004 is pre-existing. Fix it in this PR
or move it to a ticket.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The step-argument lifting fix (`lift_value` to `lift_own`) is tested only through `lift_own` with fake lifters. No test drives a real abstract-operation step row (or protocol node row) whose argument nests a reference through `check_step`, so the wiring at step.rs:1918 and :2035 and the `type_identity` row lookup are unexercised. | fsl-replay/src/refinement/map.rs:603-625 |
| FND-008 | low | `lift_own` rebuilds the argument's abstract type from the value's own type, taking the first object row with that concrete type, and does not read the abstract operation's declared parameter type. It matches S3's `lift()` typing (refinement.rs:3949) only because collection types are invariant and each concrete type has one object row, which `lift_reference` also assumes. When the own type has no row, the `Err(fault) => lift_nested(..).and(Err(fault))` fallback returns a fault unless a nested reference is unmapped. | fsl-replay/src/refinement/map.rs:628-643 |
| FND-009 | low | When two concrete map keys lift to the same abstract reference (two concrete populations mapped into one abstract population with a shared object key), `from_admitted_map` refuses the duplicate and the row becomes `Lifting::Fault` (BROKEN_INVARIANT), not a user-facing outcome. No spec text says what happens. No validated package can declare a map type today, so this cannot happen yet. | fsl-replay/src/refinement/map.rs:549-561 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | an_option_payload_is_lifted_and_none_stays_none and a_map_lifts_its_values_and_reference_keys_and_keeps_its_order (map.rs tests) cover some, none, map keys, values and order, and unmapped elements |
| FND-002 | fixed | The S3 row `maybes = self.maybes` (Sequence<Option<Reference<Ring>>>) checks; the replay test a_reference_two_carriers_deep_is_lifted_in_order passes |
| FND-003 | fixed | `(Value::Option(_) \| Value::Collection(_) \| Value::Map(_), _) => Err(Lifting::Fault)`; tested by a_value_of_another_shape_than_its_declared_type_is_a_fault |
| FND-004 | fixed | lift_value now lifts nested references through lift_own; see FND-007 and FND-008 for the test gap and the approach |
| FND-005 | fixed | The touched hunks are rustfmt-clean; the one remaining fsl-analyze diff (analyze_family_cli.rs:414, the hyper_world import) is also on main |
| FND-006 | fixed | The import is now `use filament_exact::{EffectiveId, ObjectReference, Value, ValueType};` and the build no longer warns |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | high | The new S3 refusal of two object rows from one concrete type refuses a model ADR-020 allows. `tests/support/ring_queue.rs:568` `clock_refinement` maps `Q::Queue` and `Q::Cell` both from `C::Call`: RM-2 lets one concrete population be the source of two abstract populations, and RM-3 picks the most-specific type per abstract population. Reproduced: `fsl-semantics --test it clock_places::a_refinement_row_ref_lowers_with_the_binder_as_owner` now fails with `Conflicting { name: "C::Call" }`. fsl-eval `clock_reference_v2` and `refinement_row_clock_places` use the same fixture (not run). | fsl-semantics/src/check/refinement.rs:3126-3138 |
| FND-011 | low | No `check_step` test covers the protocol node-row argument site (`step.rs:2035`), or the new FR-141 rule that an argument naming an unmapped type returns `Undetermined(MappingUndetermined)` with `UnmappedType`. Only the pass path of an abstract-operation row is tested. | fsl-replay/src/refinement/step.rs:2035 |

## Dispositions (round 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-007 | fixed | a_step_argument_nesting_references_reaches_the_abstract_operation_lifted runs `put` through `check_step` and asserts that the taken `enq`'s `rs` holds the `Queue` references `b` and `a`, in order (the node-row site is FND-011) |
| FND-008 | still-open | The fix enforces the one-row-per-concrete-type assumption with a new S3 refusal, and that refusal regresses legal models (FND-010). Replay still does not read the abstract parameter's declared type, so a concrete type with two rows still lifts by its first row, in `lift_reference` as well as `lift_own` |
| FND-009 | fixed | FR-140 now has "When two concrete map keys lift to one abstract key, `map_state` SHALL return `MappingFailure::Fault`"; tested by two_keys_lifting_to_one_abstract_key_are_a_fault |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | The receiver of an abstract-operation step row is lifted with no declared type (`StepArgument::Receiver => None`), so it lifts by the first object row from its concrete type. Its abstract type is known: it is the type that declares the abstract operation, which S3 types the receiver against (`receiver_type = lift(&self.lifting, &ValueType::Reference(target_type.id))`, RM-5). With `object Q::Cell from C::Call` written before `object Q::Queue from C::Call`, a `-> enq(self, ..)` row lifts the receiver to a `Cell` and applies `Queue::enq` to it. Pass `Reference(<declaring type of operation>)` as the receiver's declared type. | fsl-replay/src/refinement/step.rs:1916 |
| FND-013 | medium | FR-140-AC-17 is tested only through `choose_row` with string stand-ins. Nothing exercises `lift_reference` building its candidates from `abstract_system.type_identity(row.abstract_type())` and comparing them with the `EffectiveId` of a declared `ValueType::Reference`. If those identities ever differ in form, `choose_row` silently falls back to the first row, and every current test still passes. A `map_state` test over a refinement with two rows from one concrete type and a field declared with the second row's type is needed. | fsl-replay/src/refinement/map.rs:598-622 |
| FND-014 | low | `choose_row` matches the declared type by exact identity only. A field or parameter declared with an abstract supertype of one candidate's type (for example `Reference<P>` with `Cell <: P`) matches nothing and falls back to the first row, which may be the unrelated `Queue`. Choosing the candidate whose abstract type conforms to the declared type (and treating more than one conforming candidate as ambiguous) would follow RM-3. | fsl-replay/src/refinement/map.rs:625-637 |
| FND-015 | low | Protocol node-row arguments are lifted with `None` (first row). S3 types them against the node record's field types (refinement.rs, `conforms(actual, &lift(&self.lifting, field))`), so the declared types exist and could be passed. No fixture has an argument-bearing node row, so this path has no `check_step` test. | fsl-replay/src/refinement/step.rs:2045 |

## Dispositions (round 3)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-008 | fixed | Replay now lifts step-row parameters against the abstract parameter's declared type (`operation_parameter_types`, `lift_value(.., declared, ..)` and `choose_row`); the receiver and node-row fallback is new FND-012 and FND-015 |
| FND-010 | fixed | The S3 refusal, FR-135-AC-13 and its test are removed; refinement.rs is byte-identical to its state before round 2; `fsl-semantics --test it clock_places` passes |
| FND-011 | fixed | `a_step_argument_naming_an_unmapped_type_is_undetermined_at_the_argument` asserts `Undetermined(Mapping)` with `UnmappedType(SubRing)` at `Argument { Slot/put, Parameter(1) }`; the node-row site moves to FND-015 |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-016 | low | The receiver's declared type is `operation_declaring_type`, the type that declares the operation. S3 types the receiver against `target_type`, the abstract type the step row names (`lift(&self.lifting, &Reference(target_type.id))`, refinement.rs:3934), which can be a subtype of the declaring type when the operation is inherited. If two rows from one concrete type both have abstract types conforming to an inherited operation's declaring parent, replay finds no most-specific row and faults (`RowChoice::Ambiguous`), although S3 admitted the row. Carry the step row's target type in `StepTarget::Abstract` and lift against it. | fsl-replay/src/refinement/step.rs:1909-1911 |
| FND-017 | low | The new `admits` filter in `map_state` takes the first row in declaration order whose abstract type the abstract population admits. When two rows from one concrete type both conform to the population's member type (`Queue` and `SubQueue <: Queue` into `queues`), the object's abstract type again depends on source order, a rule RM-3 does not contain. S3 could refuse the case, or the most-specific rule could apply as it does in `choose_row`. | fsl-replay/src/refinement/map.rs:249-263 |

## Dispositions (round 4)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-012 | fixed | The receiver lifts against `operation_declaring_type(operation)`; `the_receiver_lifts_against_the_type_that_declares_the_operation` (Cell row first) asserts the taken `enq` receiver is the `Queue` `r` |
| FND-013 | fixed | `a_reference_field_lifts_to_the_declared_type_among_two_rows` runs `map_state` over the real `twin` world in both row orders and asserts `twin` equals `abstracted.object_reference(Cell, "r")`, and that `cells` holds `r` |
| FND-014 | fixed | `choose_row` picks by conformance (`identity_conforms`, the abstract type hierarchy), most specific wins; no match or ambiguity is `Lifting::Fault`; the unit test covers the supertype, nested and lone cases with exact values |
| FND-015 | deferred | Node-row arguments still carry no declared type. FR-141 now states the interim rule (one row names the type uniquely; several rows give `StepVerdict::Fault`). No fixture has an argument-bearing node row, so the Fault branch has no test; this needs a follow-up ticket to pass the node record's field types, which S3 already uses |
