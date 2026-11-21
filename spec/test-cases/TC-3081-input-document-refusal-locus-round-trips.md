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
reader keeps the locus kind set closed. Scope: FR-267-AC-25 to FR-267-AC-27
and FR-267-AC-29 to FR-267-AC-31. The imported-point vectors also check the
same locus contract in `filament-outcome/1` diagnostics.

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
10. Replace step 1's refusal locus by the imported-point vector below, then
    by the native-span control. Read each with the native-result reader.
    Separately put each vector in an otherwise valid `filament-outcome/1`
    diagnostic and read it with that document's reader. Re-encode each
    restored document and read it again.
11. In each document kind, mutate the imported point independently: remove
    `sourceIdentity`; set it to `null`; add `byte: 0` to just `start`, just
    `end`, then both; add `byte: null` to both; change `end.column` to 12;
    add `source: {"authority":"agent-ix","identity":"program"}` beside
    the locus members. Separately add `sourceIdentity` to the native-span
    control. Read each mutated document.
12. In the valid imported point, replace the URI by
    `urn:example:invoice:model:v2` while retaining the path and point; then
    use `ix://external/model` without supplying native provenance. Read and
    re-encode each. Compare restored loci with the original.

### Located vectors

Imported point, with no native source-label pair or byte offsets:

```json
{"kind":"located","file":"models/invoice.fcd","sourceIdentity":"urn:example:invoice:model:v1","span":{"start":{"line":7,"column":11},"end":{"line":7,"column":11}}}
```

Ordinary native-span control, rendering the one-byte source `x`:

```json
{"kind":"located","file":"model.native","span":{"start":{"byte":0,"line":1,"column":1},"end":{"byte":1,"line":1,"column":2}}}
```

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
- Step 10: both forms read in both document kinds and re-encode without
  losing any member or adding missing byte/source-label members. The
  imported URI, path and point remain exact. The ordinary native control
  retains its real byte zero and one, with no `sourceIdentity` added.
- Step 11: every mutation is refused naming the locus or its defective
  member. The original imported form cannot be repaired with fabricated
  zero-byte members; adding both while retaining `sourceIdentity` is the
  forbidden native-span-with-imported-identity shape.
- Step 12: both URIs read and remain exact; each restored locus differs
  from the original in source identity despite the equal path and point.
  The `ix://` spelling remains the imported form and does not create a
  native `{authority, identity}` pair.
