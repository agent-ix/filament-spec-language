---
id: TC-2440
title: "A trace run fixes each activation's captures and reports a capture fault at the capture"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-860
    type: verifies
---
# TC-2440: A trace run fixes each activation's captures and reports a capture fault at the capture

## Description

Verify that a temporal clause whose `holds` conditions read a capture runs
through `command::run` to a verdict over finite and lasso record-sample
traces, that each activation keeps the capture value it took at its own
position, and that an undefined or refused capture is reported at the
capture and never as an internal fault. Scope: FR-860-AC-1, FR-860-AC-2,
FR-860-AC-3, FR-860-AC-4.

## Test Procedure

1. Run `Fast` over finite samples levels 5, 1, 1, 1 (`stopped` false), the same clause with `not (r.level > 3)` in place of the capture, and `Fast` over levels 1, 5, 5, 5.
2. Run the infinite-trace `Origin` over the lasso loops (level 5, level 1) and (level 5, level 5 stopped), and the capture-free variant over the first loop.
3. Run `Each` over levels 0, 1, 2.
4. Run `Div` over levels 2, 0 and 0, 2; its `on each` variant over 0, 2; and the `Zero` capture clause over a sample with level 1.

## Expected Results

- Step 1: `violated` at position 0, exit 10; `holds`; `holds`.
- Step 2: `violated` at position 0; `holds`; `holds`.
- Step 3: `holds`.
- Step 4: `undefined` at position 0, `division-by-zero` at the capture initializer, exit 10; `holds`; `undefined` at position 1; stage `evaluate` refusal `integer_out_of_domain`/`outside-domain` at the capture initializer, exit 20. No run exits 30.

Test: `tests/it/temporal_capture_run.rs`.
