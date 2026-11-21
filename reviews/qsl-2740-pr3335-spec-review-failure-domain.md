---
id: SR-202611210022442313144108530
title: "failure-domain review of FR-1045 admission request member forms"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/filament-spec-language; spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md; PR 3335 head 7a44bc4ef6081ca54b9c6a4f92cf28dc3c16721e"
review_set: subset
---

## Summary

Ticket: QSL-2740. Request-shape refusals and resolver outcomes were checked class by class. Wire shape, snapshot identity and undeclared-type refusals each fall on exactly one side. Three classes do not: snapshot endpoints have no stated subject mapping, constructor refusals have no stated precedence, and the Producer refusal has no stated position relative to file I/O. The limit encoding is also inconsistent and depends on the platform.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | How a snapshot `{population, key}` endpoint and `relationship` name become a QualifiedSubject and declaration identity is unspecified, so match and conflict results are not determined | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:71-75 |
| FND-002 | medium | Precedence of constructor ReferenceRefusal over admit outcomes (ResourceLimit, InvalidSelection, MissingClosure) is unstated, and the row-196 Malformed cell contradicts lines 236-241 | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:236-241 |
| FND-003 | medium | The Producer bundle refusal is placed before resolver evaluation but not before selected-file I/O, and FR-1045 does not name its code | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:383-387 |
| FND-004 | medium | Limits are JSON integers up to u64::MAX, against FR-014's decimal strings and the FR's own 2^53 rationale, and do not fit a 32-bit usize | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:195 |

## Verdict

Fix round needed.

## Scope and evidence

Library `admit` order checked at baa30aa5 src/lib.rs:593-824. FR-106 snapshot relationship form checked at spec/functional/FR-106-admit-snapshots-and-invocations.md:307-330.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | Snapshot relationship name and Producer relationshipIdentity may use different spellings, so byte-for-byte resolution can fail for every entry | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:71-75 |
| FND-006 | low | Constructor build order omits the binding, expected-subject and record SubjectKind constructions it now lists as refusal sources | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:265-273 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The deliberate departure makes the kind the UTF-8 of the RFC 8785 text of an FR-322 node id {package, path, ordinal}. The standard already fixes a different spelling. FSpec FR-204 says the type component under filament.model.complete/v1 is the effective declaration identity of the most-specific type "as supplied by the population member's typeIdentity", and AD-006 says runtime subject references u |
| FND-002 | fixed | fixed at 1cfcdac6 |
| FND-003 | fixed | fixed at 1cfcdac6 |
| FND-004 | fixed | fixed at 1cfcdac6 |
| FND-001 | fixed | fixed at 64c6b654 (round 2) |
| FND-005 | fixed | fixed at 64c6b654 (round 2) |
| FND-006 | fixed | fixed at 64c6b654 (round 2) |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | Inputs, the FR-100 refusal row and FR-106-AC-74 still treat every snapshot relationship instance as a correlation candidate | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:54-55 |
| FND-008 | medium | AC-12 cannot fail an implementation that still checks or passes unmapped entries, and omits the unmapped-required-slot outcome | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:503 |
| FND-009 | low | "before the snapshot is used" leaves the mapping refusal unordered against the snapshot read | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:126-129 |
| FND-010 | low | A mapping to a declaration with swapped endpoint types returns ReversedEndpoints, not WrongEndpointKind | spec/functional/FR-1045-admit-native-run-relationships-through-the-observation-resolver.md:105-107 |
