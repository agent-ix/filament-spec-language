---
id: FR-952
title: "Record each function body's check mode in the package and replay under it"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-010
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-770
    type: depends_on
---
# FR-952: Record each function body's check mode in the package and replay under it

## Description

The check mode is part of what a run produced (FR-098 "Check mode of the
recompile"), so the checked package SHALL carry it. FSL SHALL write and
read the closed member `check_mode` on every checked `function` node of a
`filament.checked-package/v2` package, as FSpec FR-322 and FR-770 define
it: `linked` or `kernel`. The member is identity-bearing. It is part of the
node's content, so two packages that differ only in one function's mode are
two packages.

The modes split along FR-495's Kernel boundary:

- **Function bodies.** A declared `function`, an operation's `body` clause
  function, and a declaration-less function node (FSpec FR-322
  "Declaration-less function nodes") carry the mode their body was checked
  under.
- **Declared clauses.** A declared clause is always checked under `Linked`:
  state, temporal, hyper and value clauses, a declared `vc`, and the
  `invariant` and `precondition` clause functions. These clause functions
  carry `linked`, and `Kernel` never admits a declared clause whose
  obligation `Linked` refuses.

`Kernel` applies to every function body evaluated under a world built with
`CheckMode::Kernel`. This includes the bodies that the model-check,
protocol, probability and hybrid paths call while they explore a subject,
not only a body that S6a calls directly. A function body that one of those
paths reaches under a Kernel world is checked under `Kernel`. Its zero
divisor is met at run time as `Undefined::DivisionByZero`, not refused at
check.

## Inputs

- The check mode of each body (FR-495).
- A v2 package's bytes, for the reader.

## Outputs

- Every emitted `function` node carries `check_mode`.
- The replay recompile checks each body under its recorded mode.

## Behavior

- **Emit.** S4 SHALL write `check_mode` on every `function` node: `kernel`
  for a function body checked under `Kernel`, `linked` for a function body
  checked under `Linked`, and `linked` for every `invariant` or
  `precondition` clause function.
- **Read.** FSL's v2 reader SHALL admit the member as FSpec FR-770 states
  and SHALL apply its refusals. The admitted package exposes each
  function's mode to the replay lifecycle.
- **Replay.** The replay recompile SHALL check each function body under the
  mode its node in the proved package records, and every declared clause
  under `Linked` (FR-098). The lifecycle fixes no mode of its own. A replay
  of a claim minted from a `Kernel` body therefore never refuses
  `ReplayRefused(UndefinedExpression)` because the body was rechecked under
  `Linked`. A function node whose recorded `check_mode` differs from the mode
  its recompile records refuses `invalid_package`/`check-mode-mismatch` at
  `/package/semantic_graph/nodes/{i}/check_mode`.
- **Kernel world.** A world built with `CheckMode::Kernel` SHALL pass that
  mode to every function body it checks or evaluates, on every analysis
  path. Its declared clauses stay under `Linked`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-952-AC-1 | `fsl compile` of `function g using v(x: Int[0, 9], y: Int[1, 9]): Integer pure { x mod y }` (checked under `CheckMode::Linked`) writes a package in which every `function` node carries `"check_mode": "linked"`; the same unit checked under `CheckMode::Kernel` with `function r using v(x: Int[0, 9]): Integer pure { 10 mod x }` added writes `"check_mode": "kernel"` on `g`'s and `r`'s nodes, and `"linked"` on a `precondition` clause function of the same unit; and the two packages are unequal. | Test (TC-2662) |
| FR-952-AC-2 | Over the second package of AC-1, a replay request at `r`'s `10 mod x` record's occurrence with `x = 0` recompiles `r` under `Kernel` and settles `reproduced-without-witness`, never a `Recompile` refusal carrying `unproved-nonzero` and never `ReplayRefused(UndefinedExpression)`; FR-098-AC-34 holds. | Test (TC-2662) |
| FR-952-AC-3 | Under a probabilistic world built with `CheckMode::Kernel`, a target predicate that calls `function z using v(p: Int[0, 2]): Boolean pure { 1 mod p = 0 }` checks, and an evaluation of `z` at `p = 0` reached on the probability path is `Undefined::DivisionByZero`, not a check refusal `Unproved(Nonzero)`; the same `1 mod x.phase = 0` written directly in the declared `probabilistic` clause still refuses `undefined_expression`/`unproved-nonzero` at S3 under that world. | Test (TC-2662) |
| FR-952-AC-4 | Negative: the FSL v2 reader refuses a package whose `function` node lacks `check_mode`, or carries `"strict"`, with `malformed_wire` at `/semantic_graph/nodes/{i}/check_mode`, and refuses `"kernel"` on an `invariant` clause function with `invalid_package`/`invalid-value` at that pointer. | Test (TC-2662) |

## Dependencies

- FR-098: the recompile that reads the recorded mode.
- FR-495: the Kernel boundary between function bodies and declared clauses.
- FSpec FR-322 and FR-770: the `check_mode` member and its reader rules.
