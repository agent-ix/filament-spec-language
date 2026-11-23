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
