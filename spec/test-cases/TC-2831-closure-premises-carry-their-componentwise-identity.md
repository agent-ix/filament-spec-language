---
id: TC-2831
title: "Closure premises carry their componentwise identity"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-990
    type: verifies
---
# TC-2831: Closure premises carry their componentwise identity

## Description

Verify that each closure premise is `{kind, instance, node[, role_instance]}`
read from the checked protocol, and that a recording premise differing in
any component is refused. Scope: FR-990-AC-6.

Catches a premise identity inferred from a path string or list position, and
a reader that accepts a premise whose kind and node disagree.

## Test Procedure

1. Run `global-conformance` over the FR-636 order fixture with no recording.
2. Run FR-895-AC-1's closed recording with the three engine premises.
3. Run it with an `instance` member on the execution premise, with the
   execution premise's node `Flow::Main::Decide::yes`, and with the branch
   premises' kind `workflow`.

## Expected Results

- Step 1: `closure.premises` is exactly the execution premise at `Flow::Done`
  and the branch premises at `Flow::Main::Decide::no` and `::yes`, in that
  order, with no `instance` or `role_instance`.
- Step 2: admitted; `closure.state` `closed`.
- Step 3: each refused `invalid_runtime_input` naming the premise, with no
  `obligations`.
