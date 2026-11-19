---
id: TC-2810
title: "A re-delivered changed record sample is refused in batch and carried evaluation"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-993
    type: verifies
---
# TC-2810: A re-delivered changed record sample is refused in batch and carried evaluation

## Description

Verify FR-993's content fingerprint: a request that re-covers a covered
record-sample index with a changed sample is refused `Completeness` in batch
and carried evaluation alike, before and after a restore; the ledger holds
index identities with fingerprints and no sample value; fingerprints are
counted in `progress.covered_positions`. Scope: FR-993-AC-1 to FR-993-AC-3.

## Test Procedure

1. Run a batch `Progressed` request over record-sample levels 1, 2 with `Complete { through: 1 }` and keep its ledger `L`.
2. With `L`, run batch requests over levels 1, 5, 3 and over levels 1, 2, 3, each with `Complete { through: 2 }`.
3. Open a carried `ClauseState` on `L` and advance the same two traces; write it with `to_bytes` after the first position, read it with `from_bytes` and advance the rest.
4. Inspect every report and ledger from steps 1 to 3.
5. With `progress.covered_positions` set to 2, cover a third record-sample position in batch and carried evaluation.

## Expected Results

- Step 2: levels 1, 5, 3 are refused with `contradicted_dimension` `Completeness` and the report returns `L`; levels 1, 2, 3 are accepted.
- Step 3: the same verdicts, before and after the resume, and ledgers equal to the batch ones.
- Step 4: covered record samples are named by index with a fingerprint member; no report or ledger contains a sample's text or value.
- Step 5: `Incomplete(LimitReached{limit: node count, value: 2, setting: progress.covered_positions})` and the ledger unchanged in both forms.

Test: `fsl-analyze/tests/it/monitor_incremental.rs`
`a_re_delivered_changed_sample_is_refused_in_batch_and_carried_evaluation`
(the batch half of FR-993-AC-1, and FR-993-AC-2), plus `fsl-replay` unit
tests of `ClauseState` for the carried half of FR-993-AC-1 and for
FR-993-AC-3 (the code ticket adds them; a `MonitorSession` assigns the next
index and refuses a stamp at or below the ledger watermark, so it never
re-covers an index).
