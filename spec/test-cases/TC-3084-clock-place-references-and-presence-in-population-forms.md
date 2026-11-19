---
id: TC-3084
title: "Clock-place references and presence reads check and lower in population forms"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1040
    type: verifies
---
# TC-3084: Clock-place references and presence reads check and lower in population forms

## Description

Verify that `ref(p)` and `present(p)` over clock places check at S3 and lower
at S4 to FSpec FR-829's place-reference application and FSpec FR-322's
presence read, per instance under a population binder and through an object
reference, and that the remaining wrong forms refuse with typed causes.
Scope: FR-1040-AC-1 to FR-1040-AC-7.

## Test Procedure

Fixture: FSpec FR-415-AC-15's `Call` model under `time dense unit ms`, with
the fields `v: Option<Clock>`, `q: Sequence<Option<Clock>>[0, 2]`,
`ts: Sequence<Timer>[0, 2]` (`Timer { x: Clock; n: Int }`), `m: Map<Text, Timer>[0, 2]`, `peer: Reference<Call>` and
`n: Int[0, 9]` added, a population `calls`, and a
refinement whose subject admits `calls = {c1, c2}`.

1. Compile the `ref` forms of FR-1040-AC-1 and FR-1040-AC-2 and read the
   checked and emitted nodes.
2. Compile and evaluate the population forms of FR-1040-AC-3 and
   FR-1040-AC-4.
3. Compile the presence variants of FR-1040-AC-5.
4. Compile each negative form of FR-1040-AC-6.
5. Compile FR-1040-AC-7's function and its caller, then the ill-typed
   variant.

## Expected Results

- Steps 1, 2 and 5: each form checks and lowers to the nodes stated, with no
  `unsupported-feature` or `not-yet-implemented` refusal, and evaluates as
  stated.
- Steps 3 and 4: each form refuses with the code, cause and locus stated, or
  checks where the criterion says it does.

Tests: `fsl-semantics/src/check/lowering/tests.rs`
`ref_of_a_clock_place_with_an_id_lowers`, `clock_place_presence_read_lowers`,
`a_call_to_a_function_without_a_key_yet_lowers`, and
`fsl-semantics/tests/it/refinement_populations.rs` for the population forms.
