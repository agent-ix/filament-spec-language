---
id: TC-2242
title: "Every documented FSL fence runs at its declared stage"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/NFR-017
    type: verifies
---
# TC-2242: Every documented FSL fence runs at its declared stage

## Description

Verify NFR-017's documented-fence check. Scope: NFR-017-AC-5 to NFR-017-AC-7.

## Test Procedure

1. Extract every FSL fence from `README.md` and `docs/**/*.md`, and count opening fence lines with an `ix:native` or `fsl` info string by a second line scan.
2. Run each fence at its declared stage, an excerpt after the `1-draft` header.
3. Run an in-test fence with a deliberate syntax error, tagged `stage=parse reject=invalid_syntax/unexpected-token`, untagged, and tagged with another code.
4. Run an in-test `pending` fence with its current refusal in the expect comment; then the same fence made valid, with a different expected code, and with no comment.
5. Run an in-test fence with no `stage=`.

## Expected Results

- Step 1: the two counts are equal.
- Step 2: every untagged fence passes its stage; each `reject` and `pending` fence refuses with its named code and cause.
- Step 3: passes; fails; fails.
- Step 4: passes; then each variant fails.
- Step 5: fails, naming the file and line.
