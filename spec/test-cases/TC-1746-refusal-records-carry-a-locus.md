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
   diagnostic and any command-error source context.
5. Repeat step 4 after changing the program labels to `example` / `other`
   and its path to `other.native`; then independently replace only the
   imported `sourceIdentity` by `ix://external/model`.
6. Repeat step 4 with the parameter origin absent.
7. Repeat step 4 with the imported URI `ix://example/model` and separately
   retained actual native-producer provenance whose source-label pair is
   `example` / `model`. Keep the declaration imported and its supplied
   origin point unchanged; do not replace it by a native byte-span input.

### Imported parameter origin

```json
{"source":{"sourceIdentity":"urn:example:invoice:model:v1","path":"models/invoice.fcd","startLine":7,"startColumn":11}}
```

This is the retained origin object of the parameter, not a replacement model
document or a new request member. This origin supplies no byte offsets, end
coordinates or native authority/identity pair. The adapters consume the
existing typed refusal and its retained origin; no URI parsing confers native
provenance.

Tag the tests `#[trace("TC-1746", "<AC id>")]`.

## Expected Results

- Step 1: `missing-member` with a document locus: the snapshot's labels, the pointer to `child`, `object` `{config_history, child}`, `member` `parent`.
- Step 2: `unknown-member` with `member` `extra` and its pointer.
- Step 3: `locus` present and `null`.
- Step 4: each adapter preserves stage S3 and the original code/cause,
  exits 20 and creates no checked package or synthesis request. Its
  diagnostic locus is exactly TC-3081's imported-point vector. No endpoint
  has a `byte` member; no native source-label pair is attached to the
  imported path and point. Both document readers preserve that form under
  FR-267-AC-33.
- Step 5: changing the program labels/path leaves the imported locus
  unchanged. Changing only the imported URI changes only its
  `sourceIdentity`; the `ix://` URI remains exact and does not introduce
  native labels or bytes.
- Step 6: the parameter's authored position is explicitly unavailable,
  with no program location or fabricated byte-zero point substituted.
- Step 7: each adapter still writes the imported form with exact URI
  `ix://example/model`, the supplied path and point, absent bytes and no
  native source-label pair in the locus or its command-error source
  context. Known native-producer provenance does not change the imported
  origin's form. TC-3081's ordinary native-span control remains unchanged.
