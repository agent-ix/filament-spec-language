---
id: TC-3034
title: "A package state-graph claim settles as its source claim"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-3034: A package state-graph claim settles as its source claim

## Description

Verify that the package route rebuilds each state-graph claim from its
`claim`/`state_graph` node and `claim_names` row, evaluates its predicates
through the evaluator port with the source route's values, settles each item
with the source route's record, and refuses a name in both name tables on
both routes before any engine runs. Scope: FR-940-AC-11 and FR-940-AC-12.

## Test Procedure

1. Compile the Counter unit with `ReachThree`, `Back`, `FromOne` and
   `Straight` with a `handoff` member; build each claim's package form and
   rebuild it; compare with the source route's checked claims.
2. Analyze the four items by a `program` request and by a `package` request
   over the handoff directory; evaluate each predicate at each explored
   state through the port on both routes, keyed by its FR-475 role `from`
   or `target` (`Straight`'s end predicate is `target`).
3. Analyze the colliding source unit; analyze the package with the
   `ReachThree` row renamed `Bounded`.

## Expected Results

- Step 1: every rebuilt claim equals its source claim; `Back`'s `from` is the
  constant predicate.
- Step 2: equal item records, witnesses or counterexamples, replays and
  certificates; equal predicate values; no `unsupported` record.
- Step 3: `ambiguous_declaration`/`ambiguous-name` at S3 naming both, exit
  20; then the same code at the request stage with `details.path`
  `/claim_names/{i}/name`, `items` `[]`, `last_stage` `null`, exit 20.
