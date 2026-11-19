---
id: FR-993
title: "Fingerprint each covered record sample in the progress ledger"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-820
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/filament-specification/FR-114
    type: implements
---
# FR-993: Fingerprint each covered record sample in the progress ledger

## Description

A record-sample position is identified by its index in the trace (FR-330,
FR-643). A different sample re-delivered at an index that a retained
completeness assertion already covered therefore has a covered identity.
FR-502 still requires that sample to be refused `Completeness`, because the
retained assertion said the trace was complete with the earlier sample there.
Each covered record-sample entry of a `ProgressLedger` SHALL therefore carry,
beside its index identity, a **content fingerprint**: the SHA-256 digest of
the sample's canonical value bytes, encoded by ADR-013's single RFC 8785
encoder (`quire-canonical`). Batch evaluation (FR-330) and carried evaluation
(FR-820) SHALL write and check the fingerprint the same way, so one sequence of
requests gets one verdict whichever form evaluates it (FSpec FR-114).

### Decision: an integrity fingerprint beside an index identity

The sample's value cannot be its identity, because two equal samples at
different indices are distinct positions (FR-643). It cannot be carried
either, because no report or ledger spells a sample's text or value (FR-820
"Identities", FR-330-AC-18). Without some record of the content, a request
that re-delivers a changed sample at a covered index would be accepted, and
the completeness the earlier assertion stated would be silently revised. A
fingerprint is fixed-size and tells only whether the content at an index has
changed. ADR-013 §2 classes it as integrity data, not identity. It is never
compared in place of an identity, never cited, never used as a key or for
ordering, and never part of a report's verdict members. The rule that a
normalized identity is never hashed still holds.

## Inputs

- A `ProgressLedger` given to a batch request (FR-330 `Temporal` selection,
  FR-502) or to a carried state (`ClauseOpen.ledger`, FR-820), whose entry for
  the binding covers some record-sample indices.
- A trace of record samples, of kind `Progressed` or `Timestamped`, that
  re-covers some of those indices.

## Outputs

- On acceptance: the updated ledger, each newly covered record-sample entry
  holding its index identity and its fingerprint.
- On a changed sample: FR-502's contradiction on `Completeness`, with the
  ledger and the state unchanged.
- On a digest failure: `internal_fault` with fault identifier
  `content-fingerprint-failed`, with the ledger and the state unchanged.

## Behavior

- **Recorded.** When a completeness assertion covers a record-sample position,
  the evaluator SHALL record in that position's covered entry the SHA-256
  digest of the sample's canonical value bytes as `quire-canonical` encodes
  them.
- **Compared.** When a request's trace re-covers an index that its input
  ledger already covers and the sample's fingerprint differs from the
  retained one, the evaluator SHALL refuse the request as FR-502's
  contradiction on `Completeness` and SHALL leave the ledger and the state
  unchanged. The same sample at that index is accepted.
- **One rule for both forms.** Batch and carried evaluation SHALL record and
  compare fingerprints identically and SHALL return equal ledgers for the same
  requests. `MonitorSession::step` always assigns the next index, so within a
  session no index is re-covered; re-delivery reaches a covered index through
  a later request or a carried state opened on an earlier ledger.
- **Not identity.** Covered-set membership, ordering, citation and keys SHALL
  use the index identity only. The fingerprint SHALL NOT appear as an identity
  or inside one, nor in a report's verdict members. No report, ledger or
  `filament.progress-ledger/1` document SHALL carry a sample's text or value.
- **Restored.** `from_bytes` and FR-643's `resume` SHALL restore the
  fingerprints with the ledger, so a restored state refuses a changed sample
  at a covered index exactly as the original would.
- **Bounded.** Each fingerprint belongs to one covered position and SHALL be
  counted with it in the `progress.covered_positions` bound (FR-502, FR-255).
  A position that would exceed the bound returns FR-502's
  `Incomplete(LimitReached{...})`, and no fingerprint is kept for it.
- **No empty fingerprint.** If the sample's canonical bytes or their digest
  cannot be produced, the evaluator SHALL refuse with `internal_fault`
  `content-fingerprint-failed` and leave the ledger and the state unchanged.
  It SHALL NOT record an empty or placeholder fingerprint.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-993-AC-1 | Given the ledger a batch `Progressed` request over record-sample levels 1, 2 with `Complete { through: 1 }` returns, a later batch request over levels 1, 5, 3 with `Complete { through: 2 }` is refused `Completeness` and returns the input ledger unchanged, and one over levels 1, 2, 3 is accepted. A carried state opened on the same ledger (`ClauseOpen.ledger`) and advanced over the same positions, and the same state written by `to_bytes` after the first position and read by `from_bytes`, gives the same two verdicts and equal ledgers. | Test (TC-2810) |
| FR-993-AC-2 | Every ledger above names each covered record sample by its index with a fingerprint member, and no report or ledger contains any sample's text or value; two ledgers that differ only in a fingerprint differ in no identity or verdict member of their reports. | Test (TC-2810) |
| FR-993-AC-3 | With `progress.covered_positions` set to 2, covering a third record-sample position returns `Incomplete(LimitReached{limit: node count, value: 2, setting: progress.covered_positions})` and leaves the ledger unchanged, holding two fingerprints, in both batch and carried evaluation. | Test (TC-2810) |

## Dependencies

- [FR-502](FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md):
  the completeness contradiction this fingerprint decides.
- [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md) and
  [FR-820](FR-820-advance-a-temporal-clause-one-position-at-a-time-from-carried-state.md):
  the batch and carried evaluators that write and check it.
- [FR-643](FR-643-monitor-a-trace-incrementally-one-observation-at-a-time.md):
  the checkpoint that restores it.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §2: the identity and integrity-fingerprint distinction and the single
  RFC 8785 encoder.
- FSpec FR-114: batch and incremental evaluation agree.
