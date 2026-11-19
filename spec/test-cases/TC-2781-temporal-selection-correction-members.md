---
id: TC-2781
title: "The temporal selection reads its correction members and refuses malformed ones"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-985
    type: verifies
---
# TC-2781: The temporal selection reads its correction members and refuses malformed ones

## Description

The temporal selection reads its correction members and refuses malformed ones.

Scope: FR-985-AC-1, FR-985-AC-2.

## Test Procedure

Run FR-985-AC-1's repair request and the malformed selections of FR-985-AC-2 through `command::run`. Test: `tests/it/temporal_selection_members.rs`.

## Expected Results

- The seven-member request reads and reports FR-526-AC-31's repair; an absent `relation` reads as `original`.
- Each malformed selection refuses `invalid-request` at stage `request`, exit 20, naming the member.
