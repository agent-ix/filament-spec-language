---
id: TC-2626
title: "Provider limits set the process-plugin run budgets"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: verifies
---
# TC-2626: Provider limits set the process-plugin run budgets

## Description

Verify that `--provider-limit <name>=<value>` on the `quire` binary's provider-routing verbs sets the FSpec FR-462 run limits of a process plugin, that each unset limit takes its documented default, and that a malformed argument refuses before any plugin starts.

Scope: FR-287-AC-12.

## Test Procedure

1. Run `quire prove --provider-config providers.json` over a fixture plugin that records the `request` frame it receives and answers with a small `result` frame; give no `--provider-limit`. Record the request's `deadline_ms` and send a second fixture result frame of 67108865 bytes.
2. Run the same call with `--provider-limit deadline_ms=250 --provider-limit max_frame_bytes=1048576`, the fixture answering with a 1048577-byte result frame.
3. Run the same call once with each of `--provider-limit max_frames=4`, `--provider-limit deadline_ms`, `--provider-limit deadline_ms=0`, `--provider-limit deadline_ms=10s`, `--provider-limit deadline_ms=010`, `--provider-limit max_total_bytes=9007199254740992`, and `--provider-limit deadline_ms=5 --provider-limit deadline_ms=5`. Record exit code, stdout, the outcome on stderr, and whether the fixture plugin started.
4. Run `quire parse --provider-limit deadline_ms=5` with its label and source operands.

Tag the tests `#[trace("TC-2626", "FR-287-AC-12")]`.

## Expected Results

- Step 1: the request carries `deadline_ms` 600000, and the 67108865-byte frame settles the item `failed` with cause `tool-failure`, `defect` `frame-length`, `limit` 67108864 and `value` 67108865.
- Step 2: the request carries `deadline_ms` 250, and the item settles `failed` with `{"type": "tool-failure", "defect": "frame-length", "limit": 1048576, "value": 1048577}`.
- Step 3: each call refuses `invalid-request` naming `--provider-limit` and the offending argument, exits 20 with empty stdout, and the fixture plugin never starts.
- Step 4: the option is refused as unknown, exit 20.
