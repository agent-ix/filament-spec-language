---
id: SR-2026112117335575793309
title: "spec-review (base) of QSL-1978 imported-locus wire amendment"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language@720f29572ab49e9bc6d05953f8c57f35f2b1037f; FR-267 Refusal locus wire, FR-267-AC-33, FR-267-AC-34, FR-663 Refusal locus, FR-663-AC-4, TC-1746, TC-3081"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-1746
    type: reviews
  - target: ix://agent-ix/filament-spec-language/TC-3081
    type: reviews
---

## Summary

Ticket: QSL-1978 (PR filament-spec-language#3352). This base spec-review covers the new
and edited requirement text and criteria only. It checks clarity, testability, and whether
each stated oracle can tell valid shapes from malformed ones. The FCD origin spelling was
checked against FR-056, FR-710 (parameters carry `origin`) and TC-213. TC-213 shows the
existing `source` origin with optional `endLine`/`endColumn`. Criterion-strength analysis
was skipped: no Jev client is installed on this machine, so no SR id was allocated for it.

## Verdict

**FAIL (three medium findings).** The two closed `located` forms are stated precisely. The
negative vector set in TC-3081 step 11 can distinguish every listed malformed shape from the
valid imported point and the native control. That set is: missing identity, null identity,
one-sided byte on start and on end, byte zero on both endpoints with identity, null byte,
unequal endpoints, added native pair, and identity on the native span. AC-33 and AC-34 bind
to TC-3081, and FR-663-AC-4 binds to TC-1746. The findings cover three things: an origin
shape the projection leaves unstated, an "unavailable" outcome with no wire value, and a
"no normalization" rule that no vector can fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The imported projection is stated only "for a point origin" (startLine/startColumn copied to both endpoints). FCD `source` origins may also carry `endLine`/`endColumn` (TC-213 uses one), and the reader rule says "the imported form's two endpoints SHALL name the same point". The writer rule for a range origin is therefore unstated. One implementer writes the end coordinates, and the strict reader refuses its own producer's output. Another silently drops them. No vector covers a range origin. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:258 |
| FND-002 | medium | FR-663 says that when the parameter has no retained origin "its authored position is unavailable". It forbids substituting a program location or a byte-zero point, but never states the resulting `locus` value. FR-663's own list allows `null` or another locus, and FR-056 locates an operation-parameter refusal at the operation node's origin. FR-663-AC-4 ("leaves its authored position unavailable") and TC-1746 step 6 ("explicitly unavailable") therefore accept a `null` locus, the operation's origin and an omitted locus alike. The oracle cannot fail on the choice that matters. | spec/functional/FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md:104 |
| FND-003 | medium | The writer "SHALL preserve the URI without normalizing it", and AC-33/AC-34 assert an "exact `sourceIdentity`". But every vector (`urn:example:invoice:model:v1`, `...:v2`, `ix://external/model`, `ix://example/model`) is ASCII, lowercase and already in normal form. A writer or reader that percent-decodes, case-folds the scheme or host, or applies Unicode NFC passes all of TC-3081 and TC-1746. The exact-identity rule has no vector that can fail. | spec/test-cases/TC-3081-input-document-refusal-locus-round-trips.md:53 |

## Gates

`quire validate --scope .` over the four changed artifacts at their exact paths exited 0. No
runtime, cargo or full gate was run (reviewer-only brief). Skipped: criterion-strength (no Jev
client).
