---
id: FR-1037
title: "Run a value synthesis from the run request's synthesis member and write the synthesis outcome"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-754
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1010
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1036
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-823
    type: depends_on
---
# FR-1037: Run a value synthesis from the run request's synthesis member and write the synthesis outcome

## Description

FSL SHALL read a `native-run/1` request's `synthesis` member as FSpec
FR-823's synthesis request, select the declaration it names by FSpec FR-823
"Selection" from the checked package's `claim_names` rows, run FR-754's
synthesizer when the selected `claim`/`synthesis_request` node's grammar is
a type, and write FSpec FR-823's `filament.synthesis-outcome/v1` document as
the run outcome's `synthesis` member, with the exit FSpec FR-823 assigns its
status. A plan or hybrid plan declaration selected by the same member runs
as FR-1010 and FR-1021 state.

## Behavior

- FSL SHALL read the `synthesis` member as the run request's one synthesis member and
  no second member for value synthesis; `fsl-command`'s strict
  reader SHALL refuse, before any stage runs, a member named in no column of
  FSpec FR-823's member table, `invalid_runtime_input`/`unknown-member` at
  `/synthesis/<member>`.
- When S3 has checked the unit, FSL SHALL resolve `declaration` through the package reader's
  `claim_names` lookup only, never from source text or a node path, and
  refuse as FSpec FR-823 "Selection" states at `/synthesis/declaration`.
- With a value synthesis selected, FSL SHALL refuse `initial`, `policy` and
  `dwell_grid` `unknown-member` at their `/synthesis` pointers, read
  `limits` as the closed `{max_candidates, max_work}` object of the
  `synthesis.*` settings, and read `accounting` and `deadline_ms` as FR-1010
  reads them.
- The synthesizer SHALL read the declaration only from the selected node
  (FR-1036), charge one `synthesis.max_candidates` unit per enumerated value
  and one `synthesis.max_work` unit per block or objective evaluation, and
  write `candidate` and `objective.value` in FSpec FR-181's typed value form.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1037-AC-1 | A `synthesis` member `{"declaration": "Pick"}` over FR-754-AC-3's `Pick` writes `synthesis.format` `filament.synthesis-outcome/v1`, `status` `candidate`, `check` `validated`, `candidate` `{"type": "integer", "value": "7"}` and `explored.visited` 8 with every FSpec FR-823 member present, exit `0`; `= 50` writes `no-candidate`, `explored.visited` 10, exit `10`; `minimize Pick` with `Pick >= 3` writes candidate 3 with `optimal` `true`; `{"declaration": "Pick", "limits": {"max_candidates": 3}}` over `= 50` writes `incomplete` naming `synthesis.max_candidates`, exit `22`. | Test (TC-3017) |
| FR-1037-AC-2 | A unit holding only `Pick` runs it for the member `{}`; a unit holding `Pick` and `Other` refuses `{}` `invalid_runtime_input`/`missing-member` at `/synthesis/declaration`; `declaration` `Nope` and `declaration` naming the predicate `Positive` each refuse `missing_declaration`/`missing-name` at `/synthesis/declaration`; `Pick` with `initial` refuses `unknown-member` at `/synthesis/initial`; `limits` `{"work": 1}` refuses `unknown-member` at `/synthesis/limits/work`; each refusal exits `20` with no `synthesis` outcome. | Test (TC-3017) |

## Dependencies

- FR-754 (the synthesizer); FR-1010 (the member's plan form); FR-1036 (the
  node); FSpec FR-823 (request, selection and outcome).
