---
id: TC-2602
title: "A malformed tl_lowering member is refused"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-931
    type: verifies
---
# TC-2602: A malformed tl_lowering member is refused

## Description

Verify every refusal of the `tl_lowering` member. Scope: FR-931-AC-3.

## Test Procedure

Run `Low` with, in turn: `"correspondence": "mltl.closed-trace/v2"`; `"correspondence": 1`; `"correspondence": null`; no `clause`; an extra member `formula`; `cross_evaluate` with both `trace` and `initial`; with neither; with `max_steps` 0; with a `lasso` trace; `"clause": "Nothing"`.

Test: `tests/it/request_surface_s1_16.rs` `a_malformed_tl_lowering_member_is_refused`.

## Expected Results

- The unknown identity refuses `unknown_profile`/`unsupported-selection` naming `tl_lowering.correspondence` and `mltl.closed-trace/v2`, exit 20.
- Each malformed member refuses `invalid-request` at stage `request`, exit 20, naming the member.
- `Nothing` refuses `missing_declaration`/`missing-name` naming `tl_lowering.clause`.
- No refusal writes a result document.
