---
id: SR-202611210022442313144108528
title: "spec review of FR-1045 admission request member forms"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md; PR 3335 head 7a44bc4ef6081ca54b9c6a4f92cf28dc3c16721e"
review_set: subset
---

## Summary

Ticket: QSL-2740. Every `call.relationships` wire member was mapped against filament-observation baa30aa5 (`AdmissionRequest`, `ScopeSelection`, `AdmittedRecord`, `ObservationBinding`, `Member`, `ScopeKind`, `ClockRange`, `Anchor`, `ValueState`, `Visibility`, `ResourceLimits`, `QualifiedSubject::from_authority`, `RequiredRelationship::new`, `Relationship::new`, `RelationshipExtent::new`, `StaticProducerBundle::admit_json`) and its private FR-014 reader (`src/authority/admission.rs`). All 16 `ScopeSelection`, 17 `AdmittedRecord` and 8 `ObservationBinding` fields have exactly one wire member, and no wire member lacks a destination. The tagged forms cover exactly the library variants, and the tag spellings (`kind`, `clock_family`, `timestamp` for the anchor, `timestamped-event` for the range, `state`, kebab-case visibility) match FR-014's reader.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `producer` member has no positive or adverse acceptance criterion | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:86-92 |
| FND-002 | low | *text* says the resolver's InvalidSelection decides emptiness; the resolver does not check several text members | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:110-112 |
| FND-003 | low | `subject_kind` must be non-empty on the wire although `SubjectKind::new` has its own typed Empty refusal | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:145 |
| FND-004 | low | The authority form validates `bundle_identity` and the `sha256:` digest spelling, which the library compares as free strings | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:130-132 |
| FND-005 | low | AC-8 says one record entry covers three anchor families | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:422 |

## Verdict

Fix round needed. The record, scope, anchor, range, value and visibility forms are correct against the library. The defects are a missing producer criterion and three places where FSL validates more than the stated principle allows.

## Scope and evidence

Library ground truth: filament-observation at baa30aa5b474fad106be4be59fbaeba3fde6317b, src/lib.rs, src/runtime_reference.rs, src/producer.rs, src/authority/admission.rs, spec FR-001, FR-014. The named test file tests/it/native_run_relationships.rs does not exist yet. That is expected for a spec-first PR whose tests land in PR 2b, so it is not recorded as a finding.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | fixed at 1cfcdac6 |
| FND-002 | fixed | fixed at 1cfcdac6 |
| FND-003 | fixed | fixed at 1cfcdac6 |
| FND-004 | fixed | fixed at 1cfcdac6 |
| FND-005 | fixed | fixed at 1cfcdac6 |
