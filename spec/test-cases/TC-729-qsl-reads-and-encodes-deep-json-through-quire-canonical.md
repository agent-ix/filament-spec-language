---
id: TC-729
title: "FSL identities, state keys and allocation failures over quire-canonical at any depth"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-259
    type: verifies
---
# TC-729: FSL identities, state keys and allocation failures over quire-canonical at any depth

## Description

Verify FSL's identities and state keys over `quire-canonical` at 100,000
deep, and the one outcome FSL reports for its allocation failure. The
crate's own behaviour is tested in its repository. FR-260-AC-2 (TC-730) and
FR-261-AC-1 (TC-733) cover the malformed-input refusals of the sites that
parse.

## Test Procedure

Scope: FR-259-AC-6 to FR-259-AC-8, in addition to AC-3 and AC-5.

Run steps 1 and 2 on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Mint the simulation state key of a 100,000-long recursive list value,
   on the small stack and on a thread with an 8 MiB stack.
2. Read a 100,000-deep JSON package
   document, with `intake.input_bytes` raised to fit, on both stacks, and
   encode its RFC 8785 text.
3. Hand an allocation failure of 4096 requested bytes, from the reader and
   from the encoder, to domain package intake, observation admission
   and package identity.
4. Encode and read identity payloads with `identity.input_bytes` set to 0, a
   bound B below the payload size, the exact payload byte count and
   `u64::MAX`; use the setting in every exposed entry point.
5. Compare a malformed input refusal and byte offset with the allocation
   failure outcome from step 3.

Tag the tests of steps 1 and 2 `#[trace("TC-729", "FR-259-AC-3")]` and
those of step 3 `#[trace("TC-729", "FR-259-AC-5")]`; tag step 4 with `FR-259-AC-6` and `FR-259-AC-7`, and step 5 with `FR-259-AC-8`.

## Expected Results

- Step 4: `0` and `u64::MAX` are accepted; B refuses at the input-byte limit naming B, actual and `identity.input_bytes`; exact-size input succeeds.
- Step 5: malformed input retains the byte offset and is not reported as a limit; the allocation failure remains `resource_exhausted`/`allocation-failed`.
- Step 1: the two keys are equal.
- Step 2: the two RFC 8785 texts are byte-equal.
- Step 3: each site refuses `resource_exhausted`/`allocation-failed`
  carrying 4096, and none reports a limit, a malformed value or malformed
  input.
