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
request to FR-072's result. Scope: FR-862-AC-1, FR-862-AC-2.

## Test Procedure

1. Run a `native-replay/1` request carrying FR-640-AC-11's replay of `entry`, then the same request with an added `clocks` member, with an added `extra` member, and an empty `request` object.
2. Run the complete request with the package's `lib-ms` dependency clock selection removed.

## Expected Results

- Step 1: `reproduced-without-witness` with the in-process value, exit 0; then stage `request`, `invalid-request`, exit 20, naming `clocks`, `extra` and the first required member respectively, none as an unknown format.
- Step 2: `missing-clock-selection` naming `lib-ms` and `test/plant`.

Test: `tests/it/replay_request_file.rs`.
