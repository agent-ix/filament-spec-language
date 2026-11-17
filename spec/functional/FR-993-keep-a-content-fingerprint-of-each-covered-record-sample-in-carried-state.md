---
id: FR-993
title: "Keep a content fingerprint of each covered record sample in carried evaluation state"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: references
---
# FR-993: Keep a content fingerprint of each covered record sample in carried evaluation state

## Description

A record-sample position is identified by its index in the trace (FR-330,
FR-643), so a different sample that arrives at an index a retained
completeness assertion already covered has a covered identity. FR-502 still
requires that sample to be refused `Completeness`, because the retained
assertion said the trace was complete with the earlier sample there. The
carried evaluation state (FR-820) SHALL therefore keep, for each covered
record-sample position, a **content fingerprint**: the SHA-256 digest of the
sample's canonical value bytes, encoded by ADR-013's single RFC 8785 encoder
(`quire-canonical`). It SHALL compare a sample that arrives at a covered index
against that fingerprint.

### Decision: an integrity fingerprint beside an index identity

The sample's value cannot be its identity: two equal samples at different
indices are distinct positions (FR-643), and a report cites positions without
spelling their values (FR-820 "Identities", FR-330-AC-18). Keeping the whole
sample would hold every covered value and grow memory with the trace. A
fingerprint is fixed-size and tells only whether the content at an index has
changed. ADR-013 §2 classes it as integrity data, not identity. It is never
compared in place of an identity, never cited and never used as a key, so the
rule that a normalized identity is never hashed still holds.

## Inputs

- A carried `ClauseState` (FR-820) over a `Progressed` or `Timestamped` trace
  of record samples, holding a ledger entry whose retained completeness
  assertion covers some indices.
- A record-sample position at a covered index, given to `advance`, and a
  state restored by `from_bytes`.

## Outputs

- Acceptance of a position whose sample's fingerprint equals the retained one.
- On a changed sample: FR-502's contradiction on `Completeness`, with the state
  unchanged.
- On a digest failure: `internal_fault` with fault identifier
  `content-fingerprint-failed`, with the state unchanged.

## Behavior

- **Recorded.** When a completeness assertion covers a record-sample position,
  the state SHALL record that position's content fingerprint, the SHA-256
  digest of the sample's canonical value bytes as `quire-canonical` encodes
  them.
- **Compared.** When a record sample arrives at a covered index and its
  fingerprint differs from the retained one, the state SHALL refuse it as
  FR-502's contradiction on `Completeness` and SHALL keep its state unchanged
  (FR-820 "No partial state").
- **Carried only.** The fingerprint SHALL be held only in the carried state
  and in its bytes (FR-820 `to_bytes`, FR-643's checkpoint). No report, no
  report's ledger and no `filament.progress-ledger/1` document SHALL carry a
  fingerprint, a sample's text or a sample's value.
- **Restored.** `from_bytes` SHALL restore the fingerprints, so a restored
  state refuses a changed sample at a covered index exactly as the original
  would.
- **Bounded.** Each fingerprint belongs to one covered position and SHALL be
  counted with it in the `progress.covered_positions` bound (FR-502, FR-255).
  A position that would exceed the bound returns FR-502's
  `Incomplete(LimitReached{...})`, and no fingerprint is kept for it.
- **No empty fingerprint.** If the sample's canonical bytes or their digest
  cannot be produced, the state SHALL refuse with `internal_fault`
  `content-fingerprint-failed` and keep its state unchanged. It SHALL NOT
  record an empty or placeholder fingerprint.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-993-AC-1 | A carried `Progressed` state over record samples with levels 1, 2 that has retained `Complete { through: 1 }` refuses `Completeness` a sample with level 5 at covered index 1 and keeps its bytes unchanged, and does not refuse the level-2 sample at index 1; after `to_bytes` and `from_bytes` the restored state refuses the level-5 sample at index 1 `Completeness` the same way. | Test (TC-2810) |
| FR-993-AC-2 | Over the same states, no report, no report's ledger and no `filament.progress-ledger/1` document contains the fingerprint, a `sha256:` digest, or the text or value of any sample; the ledger names covered positions by index only. | Test (TC-2810) |
| FR-993-AC-3 | With `progress.covered_positions` set to 2, covering a third record-sample position returns `Incomplete(LimitReached{limit: node count, value: 2, setting: progress.covered_positions})` and the state's bytes are unchanged, holding two fingerprints. | Test (TC-2810) |

## Dependencies

- [FR-502](FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md):
  the completeness contradiction this fingerprint decides.
- [FR-820](FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md):
  the carried state that holds it.
- [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md):
  the checkpoint that writes it.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §2: the identity and integrity-fingerprint distinction and the single
  RFC 8785 encoder.
