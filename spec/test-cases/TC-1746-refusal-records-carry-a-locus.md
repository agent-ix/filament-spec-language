---
id: TC-1746
title: "Refusal records carry a locus"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-663
    type: verifies
---
# TC-1746: Refusal records carry a locus

## Description

Verify the `locus` member of refusal records. Scope: FR-663-AC-3 and
FR-663-AC-4. The imported-parameter cases are adapter fixtures; they do not
require a planner search or assert that a model-intake path is implemented.

## Test Procedure

1. Run a clause over a current snapshot whose `child` lacks `parent`.
2. Add an unknown member `extra` to `child` instead.
3. Run with an absent request file.
4. Supply each compile-refusal adapter (the compile command and compilation
   reached through native `run`) with the same S3
   `unsupported_projection` / `unbounded-extent` operation-parameter refusal:
   operation `adjust`, parameter `amount`, unbounded component `Integer`.
   Its parameter origin is the FCD object below, with no separately retained
   native source provenance. The compiling program's source labels are
   `agent-ix` / `program`, path `request.native`. Inspect the outcome
   diagnostic and the required command-error `details` object.
5. Repeat step 4 after changing the program labels to `example` / `other`
   and its path to `other.native`; then independently replace only the
   imported `sourceIdentity` by `ix://external/model`.
6. Repeat step 4 with the parameter origin absent and no retained input
   document pointer to that parameter. Give the owning operation a distinct
   source origin `sourceIdentity` `ix://example/Operation`, path
   `operation.fcd`, line 2, column 3, to detect borrowing it.
   Inspect the resulting diagnostic's `locus` member.
7. Repeat step 4 with the imported URI `ix://example/model` and separately
   retained actual native-producer provenance whose source-label pair is
   `example` / `model`. Keep the declaration imported and its supplied
   origin point unchanged; do not replace it by a native byte-span input.
8. Read the step-4 command-error envelope, then independently remove
   `details`, remove each of its three members, add a native `source` pair,
   and replace the flat details by a `locus` wrapper. Separately run a
   producer mutant that writes `request.native` as details `path`, or drops
   its exact URI while keeping the imported point; compare its output with
   the retained parameter origin and the exact details vector below.
9. Repeat step 6 with a directly retained JSON document locus for the actual
   parameter type reference:
   `{"kind":"document","document":{"authority":"example","identity":"invoice-ir"},"path":"models/invoice.json","pointer":"/types/0/operations/0/params/0/typeRef","object":null,"member":"typeRef"}`.
   The supplied document/pointer identifies `adjust`'s `amount` parameter;
   the operation source origin is still a decoy and is not used as a pointer.
10. Repeat step 4 after adding `endLine: 9` and `endColumn: 4` to the
    parameter's source origin. Compare both the diagnostic locus and flat
    command-error details with that complete authored range.
11. Repeat step 10 with just `endLine` removed, then with just `endColumn`
    removed. Compare each retained original FCD origin verbatim with its
    supplied partial origin, and inspect both projected spans.

### Imported parameter origin

```json
{"source":{"sourceIdentity":"ix://example/Invoice:Model_V1","path":"models/invoice.fcd","startLine":7,"startColumn":11}}
```

This is the retained origin object of the parameter, not a replacement model
document or a new request member. This origin supplies no byte offsets, end
coordinates or native authority/identity pair. The adapters consume the
existing typed refusal and its retained origin; no URI parsing confers native
provenance.

The exact flat command-error `details` for this origin is:

```json
{"path":"models/invoice.fcd","sourceIdentity":"ix://example/Invoice:Model_V1","span":{"start":{"line":7,"column":11},"end":{"line":7,"column":11}}}
```

Tag the tests `#[trace("TC-1746", "<AC id>")]`.

## Expected Results

- Step 1: `missing-member` with a document locus: the snapshot's labels, the pointer to `child`, `object` `{config_history, child}`, `member` `parent`.
- Step 2: `unknown-member` with `member` `extra` and its pointer.
- Step 3: `locus` present and `null`.
- Step 4: each adapter preserves stage S3 and the original code/cause,
  exits 20 and creates no checked package or synthesis request. Its
  diagnostic locus is exactly TC-3081's imported-point vector. No endpoint
  has a `byte` member; no native source-label pair is attached to the
  imported path and point. The command-error envelope contains `details`
  exactly as above: all three flat members, no `source` or `locus` wrapper,
  and no program path or program span. Its stage, code, cause, message and
  `basis` remain those of the original refusal. Readback preserves the exact
  details. Both diagnostic readers preserve the locus under FR-267-AC-33.
- Step 5: changing the program labels/path leaves the imported locus
  unchanged. Changing only the imported URI changes only its
  `sourceIdentity`; the `ix://` URI remains exact and does not introduce
  native labels or bytes.
- Step 6: the diagnostic contains `locus: null`; an omitted `locus`, the
  operation's origin, the program's location or a fabricated byte-zero
  point fails the oracle. No JSON document locus is invented without an
  actual retained document/pointer.
- Step 7: each adapter still writes the imported form with exact URI
  `ix://example/model`, the supplied path and point, absent bytes and no
  native source-label pair in the locus or its command-error source
  context. Known native-producer provenance does not change the imported
  origin's form. TC-3081's ordinary native-span control remains unchanged.
- Step 8: the valid command-error envelope reads back with equal details;
  each malformed shape is refused at `details` or its missing/extra member.
  Each producer mutant fails exact equality with the retained parameter
  origin and details vector. A structural reader is not expected to
  authenticate a different, otherwise well-shaped path without that origin.
- Step 9: the diagnostic retains exactly the supplied `document` locus,
  including its document labels, path, pointer and nullable members. It
  neither changes it into an authored `located` source origin nor borrows
  the operation's source position. The existing document reader restores it.
- Step 10: the diagnostic locus is TC-3081's imported-range vector. Its
  `span.start` is exactly line 7, column 11 and `span.end` line 9, column 4;
  the flat details has those same endpoints, the same imported path and URI,
  and no bytes, native source pair or wrapper. Both document readers accept
  and preserve the unequal endpoints. Repeating the start or dropping the
  authored end fails equality against this supplied origin.
- Step 11: each projected `span.end` repeats `span.start` (line 7, column 11)
  without a byte member. The incomplete authored end is not completed by
  borrowing a start coordinate. The full retained FCD origin still equals
  the respective supplied partial origin, including its one remaining end
  member; projection does not delete it from retention.
