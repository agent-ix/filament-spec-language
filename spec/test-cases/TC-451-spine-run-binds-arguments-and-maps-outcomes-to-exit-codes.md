---
id: TC-451
title: "Spine run binds arguments by name and maps each outcome and refusal to its exit code"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: verifies
---
# TC-451: Spine run binds arguments by name and maps each outcome and refusal to its exit code

## Description

Verify that `command`'s `run`, called in process, of a `1-draft` program joins each argument to its
parameter by declared name, converts it by FR-098's canonical integer rule,
renders the S6a outcome in `spine-run-result/1`, and refuses each pre-call
case in its refusal envelope with its stage, code, `details` and FR-301 exit status.

Scope: FR-100-AC-4 to FR-100-AC-6.

## Test Procedure

The unit declares `edition "1-draft"`, `type Digit = Int[0, 9]`,
`record Point { x: Digit; y: Digit; }`,
`lt(a: Digit, b: Digit): Boolean { a < b }`,
`flag(b: Boolean): Boolean { b }`, `id(x: Digit): Digit { x }`,
`px(p: Point): Digit { p.x }`, `corner(p: Point): Point { p }` (a function
whose declared result is a record) and `seven(): Digit { 7 }`.

1. Run `lt` with `b = 3` given before `a = 5`; `flag(1)`; `id(4)`; `flag` with `{"boolean": false}`; `px` with `{"record": {"type": "Point", "fields": {"x": {"integer": "3"}, "y": {"integer": "4"}}}}`.
2. Run `flag(2)`, `id(12)` and `px(1)`.
3. Run `id` with an argument naming `y`; with `x` bound twice; with no
   argument.
4. Run `id` with `value` `true`, `"7"`, `1.5` and `9223372036854775808`.
5. Run `function` `nope`, `module.seven`, `""`, `seven.` and `7x`; run
   `corner` with the step 1 record.
6. Run a `1-draft` source with a syntax error; run a `1-draft` source
   declaring `inv(x: Digit): Boolean { 1 / x > 0 }`.
7. Run `seven` with `call.accounting` `{"work_units": 0}`.

Tag the tests `#[trace("TC-451", "FR-100-AC-4")]` (steps 1 to 4),
`#[trace("TC-451", "FR-100-AC-5")]` (steps 5 and 6) and
`#[trace("TC-451", "FR-100-AC-6")]` (step 7).

`origin` is a reserved keyword (`fsl-cst`'s token table), so the fixture's
record-result function is named `corner`, not `origin`, in
`tests/fixtures/spine-run.native`; every fixture name above is exact.

## Expected Results

- Step 1: exit 0 each; outcomes `completed` with
  `{"boolean": false}`, `{"boolean": true}`, `{"integer": "4"}`,
  `{"boolean": false}` and `{"integer": "3"}` (FR-650 run value forms).
- Step 2: each refuses `invalid_runtime_input` at stage `call`, exit 20,
  no result document, `details` `{"position": 0, "path": ""}`, cause
  `invalid-value` for `flag(2)` and `id(12)` and `wrong-value-kind` for `px(1)`; `flag(2)` and `px(1)` refuse
  before the call and `id(12)` at S6a admission.
- Step 3: each refuses `invalid_runtime_input` at stage `call`, exit 20,
  no result document, `details` `{"parameter": "y"}`, `{"parameter": "x"}` and
  `{"parameter": "x"}` in turn.
- Step 4: each refuses `invalid-request` at stage `request`, exit 20, no result document, naming `/call/arguments/0/value`.
- Step 5: each name refuses `missing_declaration` at stage `call`, exit 20,
  no result document, `details` `{"function": <that string>}`; `corner` completes
  with the step 1 record in FR-650's record form, exit 0.
- Step 6: the syntax error refuses `invalid_syntax` at stage `source`; `inv`
  refuses `ill_typed` at stage `check` (integer `/` with no `Rational`
  expected type); each has exit code 20 and no result document.
- Step 7: outcome `{"kind": "incomplete", "limit": "work_units"}`, exit 22.
