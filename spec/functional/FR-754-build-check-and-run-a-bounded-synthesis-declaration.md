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

This requirement is how FSL provides FSpec capability V1-RUN-019, bounded
synthesis that keeps a found candidate apart from a proof. A found candidate
carries the independent checker's verdict (`validated` or `rejected`, or
`unchecked` for an `incomplete` run's best candidate) and is never reported
as proved. `no-candidate` is reported only after every value of the finite
domain has been visited, and a run stopped by a limit is reported
`incomplete`. Synthesis runs from a `native-run/1` request's
`synthesis` member in `fsl-command`: from source by FR-1037, and from a
checked package by FR-1039. It is not an FR-281 `analyze` engine, because
those engines decide claims over every behaviour of a model, so it has no
`fsl-analyze` engine row. The conformance corpus that `qualify` runs
(FR-350) has no V1-RUN-019 vector, so the capability is verified on the
command path: FR-754-AC-3 and FR-1037-AC-1 run it from source, and
FR-1039-AC-2 runs it from a checked package.

## Inputs

A unit holding a `synthesis` declaration; the checked package; the run's
work, deadline and cancellation limits.

## Outputs

`DeclarationForm::Synthesis` at S2; a checked synthesis at S3, emitted as the
checked package's `claim`/`synthesis_request` node (FR-1036, FSpec FR-322
"Synthesis-request nodes"); one FSpec FR-194 result per run, selected and
written as the `filament.synthesis-outcome/v1` document by the run
request's `synthesis` member (FR-1037, FSpec FR-823).

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
- The check SHALL be independent of the search's own evaluation. For a
  request that has a source, the candidate is re-evaluated on the other
  route: the source route's search is checked by the emitted package's strict
  read, evaluated with its own meter. For a request over a checked package
  there is no other route, so the check is a second strict read of the node,
  evaluated with its own meter, and it is a re-read and re-evaluation check
  only. A checker disagreement is `rejected` and a limit stopping the checker
  leaves the candidate `unchecked`.
- A `synthesis` whose grammar is a state model object type SHALL run as
  FR-755's planner.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-754-AC-1 | S2 builds `synthesis Pick using p grammar Digit domain Digit satisfies { Pick * Pick = 49 };` into one `Synthesis` form with its names and block spans, and the same declaration with `minimize Pick` holds the objective direction and expression. | Test (TC-2104) |
| FR-754-AC-2 | S3 refuses a `domain` naming unbounded `Integer`, a record with an unbounded `Text` field and a recursive union each `unsupported_projection`/`unbounded-extent` at the `domain` name, naming the component. | Test (TC-2104) |
| FR-754-AC-3 | Running `Pick` returns `candidate` 7 `validated` after eight values; with `Pick * Pick = 50`, `no-candidate` after ten values; with `minimize Pick` and `Pick >= 3`, the optimal 3. | Test (TC-2104) |

## Dependencies

- FR-067 (S2 dispatch); FR-755 (plans); FSpec FR-194.
- FR-1037 and FR-1039: the run-request routes that run the synthesizer.
