---
id: TC-1669
title: "Monitor command resume and refusal"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: verifies
---
# TC-1669: Monitor command resume and refusal

## Description

Verify FR-644-AC-3 and AC-4: a checkpoint record resumes in a second call, and a request defect refuses before any record with no stream output.

## Test Procedure

Stream records 1 and 2 then `{"checkpoint": PATH}`; call again with `state` PATH and records 3 to 5. Send a request naming a clause the program lacks.

## Expected Results

- The second call's events equal records 3 to 5 of the uninterrupted stream.
- The bad request returns the command-error envelope, exit 20, the sink receives nothing, and no byte reaches stdout or stderr.
