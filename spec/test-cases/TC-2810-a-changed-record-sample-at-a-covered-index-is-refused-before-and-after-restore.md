---
id: TC-2810
title: "A changed record sample at a covered index is refused before and after a restore"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-993
    type: verifies
---
# TC-2810: A changed record sample at a covered index is refused before and after a restore

## Description

Verify FR-993's content fingerprint: a carried state refuses a changed record
sample at a covered index `Completeness`, before and after a `to_bytes` and
`from_bytes` restore, no report or ledger carries the fingerprint or a sample
value, and the fingerprints are counted in `progress.covered_positions`.
Scope: FR-993-AC-1 to FR-993-AC-3.

## Test Procedure

1. Open a carried `Progressed` state over record samples, advance levels 1 and 2, and assert `Complete { through: 1 }`.
2. Advance a level-5 sample at index 1, then the level-2 sample at index 1.
3. Write the state with `to_bytes`, restore it with `from_bytes`, and advance the level-5 sample at index 1.
4. Inspect every report, every report's ledger and its `filament.progress-ledger/1` form from steps 1 to 3.
5. With `progress.covered_positions` set to 2, cover a third record-sample position.

## Expected Results

- Step 2: the level-5 sample is refused with `contradicted_dimension` `Completeness` and the bytes are unchanged; the level-2 sample is not refused.
- Step 3: the restored state refuses the level-5 sample `Completeness`.
- Step 4: no report or ledger contains a `sha256:` digest or any sample's text or value; covered positions are named by index.
- Step 5: `Incomplete(LimitReached{limit: node count, value: 2, setting: progress.covered_positions})`, bytes unchanged.

Test: `fsl-analyze/tests/it/monitor_incremental.rs`
`no_report_ledger_spells_a_sample_value_or_its_fingerprint` (FR-993-AC-2),
plus `fsl-replay` unit tests of `ClauseState` for FR-993-AC-1 and
FR-993-AC-3, advancing at a covered index and at the bound (the code ticket
adds them).
