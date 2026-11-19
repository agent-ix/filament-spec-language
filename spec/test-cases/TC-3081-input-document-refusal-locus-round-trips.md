---
id: TC-3081
title: "Input-document refusal loci round-trip through the strict run-result reader"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: verifies
---
# TC-3081: Input-document refusal loci round-trip through the strict run-result reader

## Description

Verify that a snapshot or model-header refusal written as `native-run-result/2`
carries a `document` locus that the strict reader reads back, and that the
reader keeps the locus kind set closed. Scope: FR-267-AC-25, FR-267-AC-26.

## Test Procedure

1. Through `command::run`, run FR-663-AC-3's clause request over a snapshot
   whose `child` lacks `parent`, then with an unknown member `extra` on
   `child`.
2. Run FR-868-AC-1's requests whose snapshot `model` carries `version`, then
   `digest`.
3. Pass each written document to `read_native_run_result`.
4. Run the healthy snapshot and read its document.
5. Edit step 1's missing-member document: `kind` `snapshot`, `kind` removed,
   `member` removed, an added member `line`, `pointer` `child`, `pointer`
   `null`; separately set `object` and `member` to `null`.
6. Read a document whose refusal locus is a `region` carrying `pointer`, and
   one whose `artifact` locus lacks `digest`.

Tag the tests `#[trace("TC-3081", "<AC id>")]`.

## Expected Results

- Steps 1-3: each document is stage `admit`, its `disposition.refusal.locus`
  has `kind` `document`, the snapshot's labels and path, the stated pointer
  (`/model/version`, `/model/digest` for the header runs, with `object` `null`
  and `member` `version` or `digest`); the reader accepts it and restores an
  equal `code`, `cause`, `fields` and locus.
- Step 4: the document reads with no refusal.
- Step 5: each edit except the `null` `object`/`member` edit is refused naming
  `disposition.refusal.locus`; the `null` edit reads.
- Step 6: both are refused naming `disposition.refusal.locus`.
