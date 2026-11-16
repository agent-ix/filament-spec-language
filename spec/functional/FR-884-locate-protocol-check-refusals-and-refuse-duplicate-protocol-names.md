---
id: FR-884
title: "Locate every protocol check refusal at its subject and refuse duplicate protocol names"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-088
    type: references
---
# FR-884: Locate every protocol check refusal at its subject and refuse duplicate protocol names

## Description

Every S3 refusal of a protocol (FR-218, FR-381) SHALL carry, as its refusal
region (FR-096), the span of its *subject*, the smallest construct that the
refusal is about. It SHALL NOT carry the protocol declaration's span. The
subject of each kind of refusal is:

| Refusal | Subject, and so the region |
| --- | --- |
| A read refused in a slot (`operator-ineligible` for a read of `over` or model state, `wrong-observation`, an out-of-scope `missing-name`) | the read expression (FR-218-AC-17) |
| A guard refused as a whole (`non-boolean-root`, a guard type mismatch) | the guard expression |
| `unproved-exhaustiveness` of a `choice` | the `choice` header: `choice <name> by <owner>` |
| A decision whose owner is undeclared | the owner name |
| `ambiguous-name` for two members of one name (roles, channels, relationships, cases, branches) | the name of each declaration, in source order |
| A missing name (`missing-declaration`) | the name as written |
| A requirement `definition-cycle` | the requirement (`requires temporal <name>`), naming the protocol and the clause (FR-218-AC-25); a causal `definition-cycle` keeps the locus FR-218 gives it |
| A FIFO key (FR-881) | the key expression |

Each refusal's message SHALL name its subject: the read text, the guard's
owner and case, the `choice` name, the duplicated name, or the key text.
The declaration span is the region only of a refusal about the declaration
as a whole, such as a protocol that selects no protocol profile.

S3 SHALL refuse two protocols of one name in one unit with
`ambiguous_declaration`/`ambiguous-name` at the name of each declaration, in
source order, each refusal naming the other declaration. One name
used for protocols in two different units is admitted, because each unit
has its own declaration scope (FR-088).

### Decision: the region is the construct the author has to change

A protocol declaration can span thousands of bytes. A refusal located at the
whole declaration makes the author search it for the cause, and an editor
can underline nothing more useful than the entire protocol. The construct
that needs to change is known when the refusal is made, so the refusal
carries that construct's span. Two members of one name are refused at both
names, as FR-218 already refuses duplicate channels, because neither one is
more wrong than the other. A duplicate protocol name is refused for the same
reason any duplicate declaration is: a later reference would bind to the
wrong protocol.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-884-AC-1 | A `choice` guard that reads `input.ready` refuses `ill_typed`/`operator-ineligible` with region `input.ready`. A guard that reads an unlisted fact `got.ok` refuses `wrong_snapshot`/`wrong-observation` with region `got.ok`. No region of either refusal contains the `choice` keyword or the protocol name. | Test (TC-2508) |
| FR-884-AC-2 | A `choice Pick by R` whose guards are `true` and `true` refuses `undefined_expression`/`unproved-exhaustiveness` with region `choice Pick by R`. Two roles `R` refuse `ambiguous_declaration`/`ambiguous-name` twice, each with region `R`, in source order. A channel to an undeclared role `Buyer` refuses with region `Buyer`. | Test (TC-2508) |
| FR-884-AC-3 | Each refusal of AC-1 and AC-2 has a message that holds its subject's text: `input.ready`, `got.ok`, `Pick`, `R` or `Buyer`. | Test (TC-2508) |
| FR-884-AC-4 | A unit with two protocols `Flow` refuses `ambiguous_declaration`/`ambiguous-name` twice, each with region `Flow` at its own declaration's name and naming the other. A unit with protocols `Flow` and `Drain` checks. Two units that each declare `Flow`, compiled separately, each check. | Test (TC-2509) |
| FR-884-AC-5 | Over a corpus holding one minimal rejected protocol for each S3 protocol refusal cause, every refusal region is shorter than its protocol declaration and lies inside it, except the refusal of a protocol that selects no protocol profile, whose region is the declaration. Each construct in the corpus also has a minimal accepted protocol that checks. | Test (TC-2509) |

## Dependencies

- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): the
  protocol check and its refusals.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  refusal regions.
- [FR-088](FR-088-clause-name-and-type-identity.md): declaration names within
  a unit.
