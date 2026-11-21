---
id: SR-20261121163642
title: "code review of PR 3341 carried route honours missing valuations in window aggregates"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3341 head 04e3388ca5494ea04dfae955650461d3fa66202c; fsl-replay/src/spine/clause/clause_state.rs, fsl-replay/src/spine/clause/temporal.rs, fsl-analyze/src/monitor_session.rs, fsl-analyze/tests/it/monitor_incremental.rs, spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md"
review_set: subset
---

## Summary

Ticket: QSL-2138. The carried route now takes an observation's absent atoms with the position. Reordering and restore keep them with the position, and they feed the carried run, so carried equals batch. A position with no absence keeps its fingerprint byte for byte. The oracle correction is sound and does not weaken the test. The whole-trace fallback that was removed had no caller. FR-993 still defines the fingerprint as the digest of the sample's value bytes alone, so the spec and the code disagree once an absence is present.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | With an absence, the fingerprint digests a wrapper object, but FR-993 still says the digest is of the sample's canonical value bytes | fsl-replay/src/spine/clause/temporal.rs:343-355 |
| FND-002 | low | advance accepts a non-empty absent on readings that cannot evaluate it: Streamed drops it, and Progressed fingerprints it while its batch form never can | fsl-replay/src/spine/clause/clause_state.rs:709-711 |
| FND-003 | low | from_bytes accepts an unsorted or repeated absent list, breaking the Seen invariant | fsl-replay/src/spine/clause/clause_state.rs:1970-1975 |
| FND-004 | low | read_anchor_letter reads with no absences, so the persisted letter of an anchored aggregate atom whose window holds an absent member is definite | fsl-eval/src/temporal/observed.rs:2247-2255 |
| FND-005 | low | spelled_samples has no positive control showing its needle matches a sample's Debug form | fsl-analyze/tests/it/monitor_incremental.rs:2988-2990 |

## Verdict

Fix round needed, small: one FR-993 sentence (FND-001). The rest are low. Checked clean: an empty absence keeps the fingerprint, no drop or duplicate through reorder, mark/restore or from_bytes, strict on a missing or non-array member, carried equals batch at every step across a restore, the removed fallback was unreachable from ClauseState, the oracle needle matches an escaped sample text, tags present, no Python.
