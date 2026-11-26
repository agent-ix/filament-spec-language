---
id: TC-2443
title: "A replay request file is read and replayed, and a clocks member is refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-862
    type: verifies
---
# TC-2443: A replay request file is read and replayed, and a clocks member is refused

## Description

Verify that `command::run` reads a `native-replay/1` request in FSpec
FR-336's replay request encoding, refuses a `clocks` member, an unknown
member and a missing member at stage `request`, and replays a complete
request to FR-072's result. Scope: FR-862-AC-1 to FR-862-AC-4.

## Test Procedure

1. Run a `native-replay/1` request carrying FR-640-AC-11's replay of `entry`, then the same request with an added `clocks` member, with an added `extra` member, and an empty `request` object.
2. Run the complete request with the package's `lib-ms` dependency clock selection removed.
3. Replay a result whose row at position 0 fails to encode.
4. On a 2 MiB stack, replay requests that hold an array nested 1,000, 16,000 and 100,000 levels at each position the mapping reads or copies, and under an unknown member of an otherwise empty `R`.

## Expected Results

- Step 1: `reproduced-without-witness` with the in-process value, exit 0; then stage `request`, `invalid-request`, exit 20, naming `clocks` and `extra`, and for the empty `R` listing all fourteen members in `details.missing` in FR-862's order (FR-1047), none as an unknown format.
- Step 2: `missing-clock-selection` naming `lib-ms` and `test/plant`.
- Step 3: `internal-failure`, exit 30, `last_stage` `null`, one `output-failure` diagnostic naming position 0, and no `results` document.
- Step 4: `invalid-request`, exit 20, naming the position, or listing all fourteen members as missing; no stack overflow.

Test: `tests/it/replay_request_file.rs`; step 4,
`fsl-command/tests/it/replay_depth.rs`.
