---
id: FR-754
title: "Build, check and run a bounded synthesis declaration"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-067
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-194
    type: depends_on
---
# FR-754: Build, check and run a bounded synthesis declaration

## Description

FSL SHALL carry a `synthesis` declaration through every stage and run it:
S1 parses the optional objective, S2 builds a `Synthesis` form, S3 checks
it by FSpec FR-194 "Source form" including the finite-domain rule, and the
in-tree synthesizer enumerates the finite domain in cost order and has each
found candidate validated by the native value checker.

## Inputs

A unit holding a `synthesis` declaration; the checked package; the run's
work, deadline and cancellation limits.

## Outputs

`DeclarationForm::Synthesis` at S2; a checked synthesis at S3, emitted as the
checked package's `synthesis_request` node; one FSpec FR-194 result per run.

## Behavior

- S1 SHALL parse `synthesis-objective` (`minimize e` or `maximize e`) before
  `satisfies`, matching `minimize` and `maximize` as keywords only there.
- S2 SHALL build `DeclarationForm::Synthesis` with the name, profile alias,
  grammar and domain names, objective direction and expression, and the
  `satisfies` block, each with its span.
- S3 SHALL resolve the names, bind the declaration's name to the candidate
  of type `D`, type the block and the objective, and refuse a `D` whose value
  set is not finite with `unsupported_projection`/`unbounded-extent` at the
  `domain` name, naming the first unbounded component.
- The synthesizer SHALL enumerate `D` in FSpec FR-194's cost order, evaluate
  the block natively, and hand the first satisfying candidate to the native
  value checker as a `value-validity` item over that value; it SHALL record
  the visited count, and report `no-candidate` only after visiting every
  value of `D`.
- A `synthesis` whose grammar is a state model object type SHALL run as
  FR-755's planner.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-754-AC-1 | S2 builds `synthesis Pick using p grammar Digit domain Digit satisfies { Pick * Pick = 49 };` into one `Synthesis` form with its names and block spans, and the same declaration with `minimize Pick` holds the objective direction and expression. | Test (TC-2104) |
| FR-754-AC-2 | S3 refuses a `domain` naming unbounded `Integer`, a record with an unbounded `Text` field and a recursive union each `unsupported_projection`/`unbounded-extent` at the `domain` name, naming the component. | Test (TC-2104) |
| FR-754-AC-3 | Running `Pick` returns `candidate` 7 `validated`; with `Pick * Pick = 50`, `no-candidate` after ten values; with `minimize Pick` and `Pick >= 3`, the optimal 3. | Test (TC-2104) |

## Dependencies

- FR-067 (S2 dispatch); FR-755 (plans); FSpec FR-194.
