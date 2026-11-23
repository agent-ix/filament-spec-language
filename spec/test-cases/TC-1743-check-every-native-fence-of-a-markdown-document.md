---
id: TC-1743
title: "Check every native fence of a Markdown document"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-662
    type: verifies
---
# TC-1743: Check every native fence of a Markdown document

## Description

Verify `check_native_fences` over one Markdown document. Scope: FR-662-AC-1, FR-662-AC-2, FR-662-AC-4, FR-662-AC-5.

## Test Procedure

1. Check FR-662-AC-1's `spec.md` with headings `Healthy` and `Broken` and a `text` fence.
2. Add a second `ix:native` fence under one heading and one fence before the first heading.
3. Check twice, then with a cancelled `Cancel`.
4. Independently trigger a whole-document extraction refusal, an extraction/check limit, cancellation, and an internal fault before any fence item is produced.
5. Put an unowned `ix:native` fence on line 3, an owned fence under heading id `unowned-3`, an owned fence under heading id `owned~unowned-3`, and an ordinary owned fence under `Healthy`; check twice.

Tag the tests `#[trace("TC-1743", "<AC id>")]`.

## Expected Results

- Step 1: two results; `Healthy` success, body identity `doc#Healthy`; `Broken` `ill_typed` naming the unit, its body span and the original line and column; document operation `check`, category refusal, exit 20.
- Step 2: `extraction-clause-count` `{"actual": 2}` for both fences of the heading; `missing_declaration`/`missing-name` for the headless fence; other results unchanged.
- Step 3: equal reports; `StageFailure::Cancelled` with no report.
- Step 4: each result has operation `check`, no items and one operation-level diagnostic; extraction refusal is category `refusal`, no stage, exit 20; limit is `incomplete` with its catalog code, cause and reached stage, exit 22; cancellation is `incomplete` with no stage and its cancellation code and cause, exit 22; fault is `internal_failure` with no stage and the fault catalog code, exit 30.
- Step 5: the unowned fence has identity `doc#unowned-3`; the owned `unowned-3` fence has identity `doc#owned~756e6f776e65642d33`; the owned `owned~unowned-3` fence has identity `doc#owned~6f776e65647e756e6f776e65642d33`; the ordinary fence retains `doc#Healthy`. The identities are distinct and deterministic on both checks.
