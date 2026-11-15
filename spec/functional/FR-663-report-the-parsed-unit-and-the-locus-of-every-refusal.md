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
`usage` lists the four FR-255 S1 settings in this order, each with the
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
- a document locus for a refusal located in a JSON input document (a
  request, snapshot, invocation or model document):
  `{"document": {"authority", "identity"} | null, "path": P,
  "pointer": "<RFC 6901 pointer>", "object": {"population", "key"} | null,
  "member": "<member name>" | null}`, where `object` names the snapshot
  object whose member failed and `member` the failing member's name;
- `null` when the refusal has no position, such as an unreadable file,
  whose path stays in the cause.

A snapshot or invocation admission refusal (FR-106 checks 1 to 11) SHALL
carry the document locus of the first failing member: the snapshot's
labels, the pointer to the member (to the object for a missing member),
the object's population and key, and the member name.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-663-AC-1 | `OutcomeDocument::from_parse` over the `parse` operation's result for `tests/fixtures/spine-compile.native` with identity `agent-ix`/`test:parent` has `result` of kind `parsed` whose `source` is those labels, `path` the fixture path, `bytes` the fixture's byte count, and `usage` the four S1 settings in order with each `bound` the default and each `used` the count the parse reached, `s1.input_bytes` used equal to `bytes`. A unit holding `a < b < c` has a `parsed` result beside its `invalid_syntax` diagnostic. A parse with a blank label has `result` `null`. | Test (TC-1745) |
| FR-663-AC-2 | Parsing the fixture twice gives equal `result` members; parsing it with `s1.tokens` set to its token count reports `used` equal to `bound` for `s1.tokens`. | Test (TC-1745) |
| FR-663-AC-3 | A clause run over a current snapshot whose object `child` of population `config_history` lacks a required field `parent` refuses `invalid_runtime_input`/`missing-member` with a document locus naming the snapshot's labels, the pointer to `child`, `object` `{config_history, child}` and `member` `parent`; an unknown member `extra` on that object gives `unknown-member` with `member` `extra` and the pointer to it; a run whose request file is absent gives a refusal record whose `locus` is present and `null`. | Test (TC-1746) |

## Dependencies

- [FR-002](FR-002-parse-native-units.md): the `parse` operation.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the S1 settings.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): the outcome document.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): the source locus.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): the admission checks whose refusals carry a document locus.
