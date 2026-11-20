---
id: SR-2026112021332500000000001
title: "code-review of PR 3046 native collection element forms and Bytes/JsonObject type names"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3046 head b216703b5c5a5f678440ac1e4c28fb518ddbe461; fsl-semantics/src/check/{assemble,type_form}.rs, fsl-semantics/src/model/domain_package.rs, fsl-semantics/src/model/observation.rs, fsl-semantics/src/model/observation/{document,encode,tree}.rs, fsl-semantics/src/model/observation/document/tests/collection_elements.rs, fsl-semantics/tests/it/{native_scalar_equality,native_snapshot_values}.rs, fsl-conformance/cases/type-operators/native-equality-accepted.native"
review_set: subset
---

## Summary

Tickets: QSL-1068 and QSL-2610. Rust code-review with the rust-review lane folded in. The earlier review of this PR (draft at 038f6656, posted 2026-11-19 as SR-2026111911313600000009 code-review, clean, and SR-20261119113136000000010 gap-analysis) predates a rebase onto main. A range-diff shows the first three commits are the same apart from dropping this branch's Float code in favour of main's QSL-2176 `float_payload` and `duplicate_member_indices`. Three commits are new: QSL-2610's native type names, the snapshot refusal path change in `admit_population_values`, and a clippy fix. This review covers those, the rebase interplay, and the points the lead asked about. No build or test was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Splicing the typed nested steps into a snapshot field refusal's record path makes the document-locus pointer, which fsl-replay builds from record.path, a pointer that does not resolve in the document for every refusal nested in a collection, option or record (a duplicate at element 1 of `reach` becomes `/populations/0/objects/0/fields/reach/1`, while the element is at `/…/reach/set/1`). | fsl-semantics/src/model/observation/document.rs:3314-3323; fsl-replay/src/outcome.rs:1886-1899 |
| FND-002 | medium | The reader turns a payload object that has a `kind` member and also a collection tag member into ObjectMembers, so in a non-collection slot `{"kind":"string","text":"a","set":[]}` refuses `wrong-value-kind` instead of FR-106 payload-order step 4's `unknown-member` naming `set`. | fsl-semantics/src/model/observation/tree.rs:224-226 |

## Verdict

Fix round needed: one high, one medium.

What holds. Decimal: `canonical` is set when entering a Set or Bag element and carried through options, nested collections and record fields, so a Decimal is normalized at any depth inside a set or bag element and keeps its spelling as a scalar or in a Sequence or OrderedSet outside one (FR-106 lines 437-446). Duplicate detection uses the kernel's `member_equal` (Decimal `numerically_equal`), so 1.0 and 1.00 are one member in a Set and duplicates in an OrderedSet, as FR-106 line 573 requires. A Bag groups them into one member. Frame identity normalizes a Decimal everywhere (observation.rs `structure_identity`). That matches FR-106 line 572, "the frame check compares a Decimal field by value". The lead's wording ("frame identity follows the same rule") does not quite match: the spec compares by value, the code does too, and AC-53's retag case is consistent with it. Encoding writes the normalized form only under `canonical`. Domain membership is checked on the value before normalization, so normalization cannot change it.

Float: membership is by exact bits through main's all-Float hash path, and through the kernel's `member_equal` for Float inside an Option or record. The kernel documents width-specific bit identity there, including NaN payloads and signed zero. That is consistent with FR-106 lines 599-605 and FSpec FR-144. FR-323-AC-3's numericEqual for `=` is a separate relation and is not affected.

Boundaries: `intake.rs` and `decimal_value_types.rs` are untouched. `missing_wire_form` still refuses Map, BoundedQuantity and Population elements, so a Decimal-keyed Map is still refused, and so is a non-Sequence collection whose element holds a Map (with or without a float).

Envelope order: `collection_envelope_refusal` runs before the field kind test, the bound and any element read. It covers non-object, missing tag, single other tag, all-other tags, first extra in document order, and a non-array tag, in FR-106 lines 413-426's order. Object member order comes from quire_canonical's Object iterator, which keeps document order.

QSL-2610: `native_scalar_named` adds `Bytes` and `JsonObject`. The tests cover AC-77's four functions, the aliases, Bytes against JsonObject, and `record Bytes` ambiguity with the native candidate and no span. The conformance case covers S4. No unit in the repo declares a type named Bytes or JsonObject that would become ambiguous.

FND-001: the PR says "the JSON-pointer `path` string is unchanged". That is true of the record's `path` field: it is still `field_path` or `field_path/member`. It is not true of the wire. `snapshot_admission_locus` builds the FR-663 locus `pointer` by folding `record.path`, so this change alters every emitted snapshot locus for a nested refusal. On main after #3162, a nested duplicate pointed at the field (`/…/fields/reach`), which resolves. With this PR it points at `/…/fields/reach/1`, which does not resolve: `reach` is the object `{"set": [...]}`. A Decimal defect in `rates` element 0 gives `/…/rates/0/scale` instead of `/…/rates/set/0/scale`. An option payload's `present` step and a record's `record`/`fields` steps are dropped the same way. FR-663 (lines 81-84) requires the RFC 6901 pointer to the failing member. FR-106 places the refusal at the element's path. The typed steps meet FR-106's relative-path reading, but splicing them into a document pointer gives neither form. Fix: when appending `record.path` below the field, render each typed step in its document spelling (the collection tag before an `Index`, `present` for `OptionValue`, `record` then `fields` before a record field name), so the locus pointer resolves in the supplied document. Add a test that resolves the emitted pointer of a nested duplicate against the document.

FND-002: in tree.rs, `collection.is_some_and(|(_, payload)| count != 1 || …)` runs before the `kind` test. So an object that FR-106 line 493 classes as a payload object ("A value object with a `kind` member is a payload object") is read as a collection envelope whenever it also holds a collection tag. In a Set slot, the collection rules give the right answer (`unknown-member` naming the first other member). In a String, Decimal or Float slot, the payload order should give `unknown-member` naming the tag, but the result is `wrong-value-kind`. Fix: apply the collection-tag test only to objects without a `kind` member, or carry the members of a `kind`-bearing object as Native so a collection slot still refuses it with missing-member or unknown-member in document order.

Not verifiable without a build: that the focused tests the PR body reports pass at this head, that the rebased duplicate path (main's `duplicate_member_indices` with Options of Float) reaches the kernel bit-identity branch at run time, and how the 16 main tests the PR body calls failing (`child_composite::a_snapshot_*`, `native_snapshot_values::*`) behave after the fix for FND-001.
