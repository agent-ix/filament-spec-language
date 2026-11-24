---
id: TC-3216
title: "The package route settles operation-contract items from the admitted row with source parity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-3216: The package route settles operation-contract items from the admitted row with source parity

## Description

Verify that an operation-contract item is selected from its admitted typed
requirement row, executed and replayed from admitted package forms, equal to
the source route, independent of source files, and never `unsupported`. Scope:
FR-940-AC-33.

## Test Procedure

1. Compile a unit with three operation-contract items (holding, violated by a
   found case, undefined at a case) with a handoff directory. Run the three as
   a `program` request and as a `package` request; compare item records,
   verdicts, counterexample cases, replay targets, `reproduced`, charge logs and
   process exit.
2. Overwrite the unit's source with bytes that are not a native unit, delete
   the handoff `sources/`, and run the `package` request again.
3. Submit a row the strict reader refuses, a row whose operation anchor
   resolves to no admitted node, and a duplicated row.
4. Scan the source of the package contract path of `fsl_replay`, `fsl_eval` and
   `fsl_analyze` for functions that take or return `CheckedPackage` or
   `CheckedGraph`.

## Expected Results

- Step 1: equal records on both routes; the undefined item `failed` with the
  source outcome and the sibling items unaffected.
- Step 2: the same three records with no source present.
- Step 3: whole-request refusal with the reader's code, cause and pointer, no
  engine run.
- Step 4: no match.
