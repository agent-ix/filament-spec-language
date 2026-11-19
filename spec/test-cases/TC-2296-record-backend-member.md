---
id: TC-2296
title: "An analyze record names its backend exactly when a result settles it"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: verifies
---
# TC-2296: An analyze record names its backend exactly when a result settles it

## Description

Verify the analyze record's `backend` member: present exactly when the
record is settled by a result entry. Scope: FR-621-AC-10.

## Test Procedure

1. Analyze `Bounded`, `BelowThree` and `Missing` over the Counter subject.

## Expected Results

- Step 1: `Bounded` and `BelowThree` carry `settled_by` `result` and `backend` `en-1`; `Missing` carries `settled_by` `disposition` and no `backend` member.

Test: `fsl-analyze/tests/it/analyze_reach_rulings.rs`
`a_record_names_its_backend_exactly_when_a_result_settles_it`.
