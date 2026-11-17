---
id: FR-881
title: "Refuse a FIFO key that is not FIFO-key-stable at S3, at the key"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-726
    type: implements
  - target: ix://agent-ix/filament-specification/FR-465
    type: depends_on
---
# FR-881: Refuse a FIFO key that is not FIFO-key-stable at S3, at the key

## Description

The S3 protocol check (FR-218, Channels) SHALL decide the type of a FIFO
channel's key by the standard's FIFO-key-stable rule (FSpec FR-726). It walks
the checked key type through `Option`, collection, record, tuple and variant
members to every leaf, follows FSpec FR-322's equality-stable rule, and
excludes `Float32` and `Float64` leaves. The S3 check SHALL refuse a key
whose type is not FIFO-key-stable with `ill_typed`/`operator-ineligible`.
The refusal SHALL meet all of these conditions:

- the locus is the span of the key expression, never the channel or the
  protocol declaration;
- it retains the operator `=`, the channel name, the key expression's source
  text and the key type;
- its message reads `key type <T> of channel <C> is not FIFO-key-stable`.

Every other key type is admitted at S3 and written at S4 (FR-880).

### Decision: one key rule, shared with the reader, applied at S3

When a check admits `=` over a key that the package reader then refuses, the
author gets a refusal at the wrong stage. That refusal names no construct and
points at the whole protocol. S3 and the reader now apply one rule, the
standard's, so every key S3 admits is admitted by a conforming reader, and
every key S3 refuses would be refused by that reader. Applying the rule at S3
puts the refusal on the key expression, where the author can fix it.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-881-AC-1 | FIFO channels keyed by `Float64[nearest-even]`, by `Option<Float32[nearest-even]>`, by a record field of type `Float64[nearest-even]` and by an event-typed value each refuse at S3 with `ill_typed`/`operator-ineligible`. The locus is the key expression's span. The refusal retains `=`, the channel name, the key text and the type, and its message reads `key type <T> of channel <C> is not FIFO-key-stable`. No S4 refusal is produced. | Test (TC-2504) |
| FR-881-AC-2 | For each carries and key type in FSpec TC-1770's vectors, S3 admits the channel exactly when the FSpec FR-726 reader admits it. A checked unit whose package the reader refuses for a channel `carries` or `key` does not exist in the corpus. | Test (TC-2504) |
| FR-881-AC-3 | A unit holding a protocol with a float key and a good protocol reports one S3 refusal, located at the key, for the first protocol, and writes the second protocol's nodes (FR-383 per-protocol results). | Test (TC-2504) |

## Dependencies

- [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): the
  channel check.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  refusal loci.
- [FR-880](FR-880-write-every-checked-protocol-construct-at-s4.md): the
  emission of every admitted key.
- FSpec FR-726: the FIFO-key-stable rule.
