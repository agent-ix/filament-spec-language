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
reader keeps the locus kind set closed. Scope: FR-267-AC-25 to FR-267-AC-27 and FR-267-AC-29.

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
6. Read a document whose refusal locus is a `region` carrying `pointer`, one
   whose `artifact` locus lacks `identity`, one whose `artifact` locus carries
   `digest` beside `identity` and `pointer`, and one whose `artifact` locus is
   exactly `identity` `{agent-ix, test:parent}` and `pointer` `/declarations/0`.
7. Run the snapshot whose `child` field payload object has `kind` `1`.
8. Run a replay request whose member names no member of its package, and a
   run with an absent request file.
9. Read step 1's document with its locus replaced by each `input` locus of
   FR-267-AC-29, and by each defective `input` locus and the `file` kind.

Tag the tests `#[trace("TC-3081", "<AC id>")]`.

## Expected Results

- Steps 1-3: each document is stage `admit`, its `disposition.refusal.locus`
  has `kind` `document`, the snapshot's labels and path, the stated pointer
  (`/model/version`, `/model/digest` for the header runs, with `object` `null`,
  `member` `version` or `digest`, and `field` `model.version` or `model.digest`); the reader accepts it and restores an
  equal `code`, `cause`, `fields` and locus.
- Step 4: the document reads with no refusal.
- Step 5: each edit except the `null` `object`/`member` edit is refused naming
  `disposition.refusal.locus`; the `null` edit reads.
- Step 6: the first three are refused naming `disposition.refusal.locus`; the
  identity-and-pointer artifact locus reads back equal.
- Step 7: `wrong-value-kind` with a `document` locus: the snapshot's labels and
  path, the pointer to that `kind`, `object` `{config_history, child}`,
  `member` `kind`; it reads back equal.
- Step 8: the replay refusal carries a `document` locus at the request member,
  never `artifact`; the absent-file refusal carries a `null` locus.
- Step 9: the two `input` loci read back equal; each defective one and the
  `file` kind are refused naming `disposition.refusal.locus`.
