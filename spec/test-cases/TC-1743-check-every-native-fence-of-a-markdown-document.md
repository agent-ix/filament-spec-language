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

Verify `check_native_fences` over one Markdown document. Scope: FR-662-AC-1, FR-662-AC-2.

## Test Procedure

1. Check FR-662-AC-1's `spec.md` with headings `Healthy` and `Broken` and a `text` fence.
2. Add a second `ix:native` fence under one heading and one fence before the first heading.
3. Check twice, then with a cancelled `Cancel`.

Tag the tests `#[trace("TC-1743", "<AC id>")]`.

## Expected Results

- Step 1: two results; `Healthy` success, body identity `doc#Healthy`; `Broken` `ill_typed` naming the unit, its body span and the original line and column; document operation `check`, category refusal, exit 20.
- Step 2: `extraction-clause-count` `{"actual": 2}` for both fences of the heading; `missing_declaration`/`missing-name` for the headless fence; other results unchanged.
- Step 3: equal reports; `StageFailure::Cancelled` with no report.
