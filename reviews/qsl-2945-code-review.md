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
