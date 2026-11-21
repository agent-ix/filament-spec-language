---
id: FR-663
title: "Report the parsed unit and the locus of every refusal"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-001
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-095
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: references
---
# FR-663: Report the parsed unit and the locus of every refusal

## Description

A user who parses a unit needs to see what was parsed and how close it came
to the S1 limits, and a user whose input is refused needs to see where.
The `parse` outcome document (FR-002, FR-286) SHALL therefore carry a
`result` for a completed parse, and every refusal record of every outcome
document SHALL carry a `locus` member.

### Parse result

For a parse whose S1 and S2 complete, whether the unit is admissible or
not, `result` is:

```json
{"kind": "parsed",
 "source": {"authority": "agent-ix", "identity": "test:parent"},
 "path": "tests/fixtures/spine-compile.native",
 "bytes": "412",
 "usage": [
   {"setting": "s1.input_bytes", "bound": "...", "used": "412"},
   {"setting": "s1.tokens", "bound": "...", "used": "..."},
   {"setting": "s1.nodes", "bound": "...", "used": "..."},
   {"setting": "s1.work_units", "bound": "...", "used": "..."}
 ]}
```

`source` holds the unit's two FR-001 labels exactly; `path` is the path
label the caller gave; `bytes` is the exact count of source bytes read;
`usage` lists the four FR-255 S1 resource settings in this order (the diagnostic bound `s1.diagnostics` bounds the report, not the parse, and is not listed: FR-762), each with the
bound the parse ran under and the count the parse reached. Counts are ASCII
decimal strings in FR-038's integer spelling. The unit's identity is the
pair of labels the caller gave; the report adds no other identity of the
bytes. A parse refused before S1 completes (an identity, read or limit
refusal, or cancellation) has `result` `null`, and its refusal record is a
diagnostic.

### Refusal locus

Every refusal record, a diagnostic of the document and the refusal record
an item carries alike, SHALL hold `code`, `kind` and `locus`. `locus` is
always present and is exactly one of:

- a source locus (FR-095) for a refusal located in a source unit;
- the `located` imported-source point of [FR-267](FR-267-write-run-results-as-native-run-result-2.md#refusal-locus-wire)
  for a refusal located at an imported declaration's retained FCD source point,
  preserving its exact `sourceIdentity` URI, authored path and line/column
  without inventing native source labels or byte offsets;
- an artifact locus (FR-095) for a refusal located in an identified
  artifact;
- a document locus for a refusal located in a JSON input document (a
  request, snapshot, invocation or model document):
  `{"kind": "document", "document": {"authority", "identity"} | null,
  "path": P | null, "pointer": "<RFC 6901 pointer>",
  "object": {"population", "key"} | null, "member": "<member name>" | null}`,
  where `object` names the snapshot object whose member failed and `member`
  the failing member's name;
- an input locus (FR-095 `Locus::Input`) for a refusal located in a
  caller-named input that is not an FR-001 artifact or a JSON input
  document, such as a member file of a handoff directory (FR-665);
- `null` when the refusal has no position, such as an unreadable request or
  input document file, whose path stays in the cause.

A snapshot or invocation admission refusal (FR-106 checks 1 to 11) SHALL
carry the document locus of the first failing member: the snapshot's
labels, the pointer to the member (to the object for a missing member),
the object's population and key, and the member name.

Every non-null locus is one of the six tagged forms of FR-267's
**Refusal locus wire** (`region`, `located`, `occurrence`, `artifact`,
`document`, `input`), with the members that table lists, in an outcome document and
in a `native-run-result/2` document alike, so the strict reader of either
document reads every locus its producer writes.

This imported-source projection applies to both compilation and compilation
reached through native `run`. When the refusal names an operation parameter,
the adapter SHALL use that parameter's retained origin rather than the PLAN
declaration's or program's source region. It SHALL use actual retained native
provenance for a native span; the scheme of an imported `sourceIdentity` URI
does not establish native provenance. FR-267 owns the two conditional `located`
forms and their readback/refusal rules. If the parameter has no retained origin,
its authored position is unavailable and the adapter SHALL NOT substitute a
program location or a byte-zero point for it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-663-AC-1 | `OutcomeDocument::from_parse` over the `parse` operation's result for `tests/fixtures/spine-compile.native` with identity `agent-ix`/`test:parent` has `result` of kind `parsed` whose `source` is those labels, `path` the fixture path, `bytes` the fixture's byte count, and `usage` the four S1 settings in order with each `bound` the default and each `used` the count the parse reached, `s1.input_bytes` used equal to `bytes`. A unit holding `a < b < c` has a `parsed` result beside its `invalid_syntax` diagnostic. A parse with a blank label has `result` `null`. | Test (TC-1745) |
| FR-663-AC-2 | Parsing the fixture twice gives equal `result` members; parsing it with `s1.tokens` set to its token count reports `used` equal to `bound` for `s1.tokens`. | Test (TC-1745) |
| FR-663-AC-3 | A clause run over a current snapshot whose object `child` of population `config_history` lacks a required field `parent` refuses `invalid_runtime_input`/`missing-member` with a document locus naming the snapshot's labels, the pointer to `child`, `object` `{config_history, child}` and `member` `parent`; an unknown member `extra` on that object gives `unknown-member` with `member` `extra` and the pointer to it; a run whose request file is absent gives a refusal record whose `locus` is present and `null`. | Test (TC-1746) |
| FR-663-AC-4 | For TC-1746's operation-parameter refusal with an imported FCD point, both the compile adapter and the native-run compile adapter write the imported `located` form with the exact URI, path and point, no bytes and no substituted program source-label pair. Changing program labels preserves that locus; removing the parameter origin leaves its authored position unavailable. An `ix://` URI without actual retained native provenance remains imported. | Test |

## Dependencies

- [FR-002](FR-002-parse-native-units.md): the `parse` operation.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the S1 settings.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): the outcome document.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): the source locus.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): the admission checks whose refusals carry a document locus.
