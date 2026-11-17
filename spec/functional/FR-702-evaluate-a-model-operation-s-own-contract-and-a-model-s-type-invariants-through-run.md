---
id: FR-702
title: "Evaluate a model operation's own contract and a model's type invariants through run"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-003
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-366
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-652
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-651
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-207
    type: references
---
# FR-702: Evaluate a model operation's own contract and a model's type invariants through run

## Description

A domain package may give an operation its own contract, a `requires` list
and an `ensures` list (FR-103), and may give a record value type or an event
type its own invariants (FR-366). Both are part of the model, so a user who
runs the model through `fsl run` expects them to hold. `check` already parses
and checks them; this requirement names the command that evaluates them and
how a failure is reported.

The `run` command evaluates both. An operation's own contract is evaluated by
the run of that operation over one invocation, the `frame` selection of the
clause run request (FR-312, FR-115): an invocation is the one observation
that holds the pre state, the post state and the parameters a contract
reads, so that run is where "did this invocation keep the operation's
contract" is asked. A model's type invariants are evaluated by FR-652's
`value_invariants` selection of an FR-651 `items` request, which reaches
FR-106's `ValueInvariants` admission and FR-366's evaluation; it is the one
wire form for that selection.
A false clause is a logical violation of the model, reported like a frame
violation: category `violation`, `truth: false`, exit 10, naming the clause.

## Inputs

- A clause run request (FR-312) whose selection is `frame` (an operation and
  an invocation), or an FR-651 `items` request carrying FR-652's
  `value_invariants` selection, with the unit and the selected domain
  packages.
- The operation's kept `requires` and `ensures` clauses (FR-103) and the
  model's effective type invariants (FR-366), as checked.

## Outputs

- For a `frame` run, a clause run report: success with `truth: true`, a
  violation naming the failing clause, or the refusal or incomplete outcome
  of the step that stopped.
- For a `value_invariants` selection, FR-652's items, one per evaluated
  invariant and value, in an FR-651 items document.

## Behavior

### Operation contract in an operation run

- After the frame check of FR-115, the `frame` run SHALL evaluate each kept
  `quire`-language `requires` clause of the selected operation at the pre
  snapshot, in list order, and then each kept `quire`-language `ensures`
  clause over the pre and post snapshots, in list order, binding `self`, the
  parameters by name and, for `ensures`, `result` as FR-104 states. A clause
  in another language stays unevaluated (FR-103).
- The run SHALL stop at the first clause that does not evaluate to `true`. A
  frame violation is reported before any contract clause; a `requires`
  failure before any `ensures` clause.
- A clause that evaluates to `false` SHALL give the disposition `evaluate`,
  category `violation`, `truth: false`, exit 10, whose witness names the
  operation's `DeclarationKey`, the clause kind (`requires` or `ensures`),
  its position `k` and its kept source identity (artifact id and span), the
  `ModelOperationClause` origin of FR-103, located at `Operation.pre[k]` or
  `Operation.post[k]`.
- A clause whose evaluation is undefined or incomplete SHALL give that
  outcome, located the same way, and never `truth: false`.
- The run reports success, `truth: true`, only when the frame and every
  evaluated contract clause hold. An operation with empty `requires` and
  `ensures` lists runs exactly as FR-115 states.

### Type invariants in a value-invariants run

- The `run` command SHALL evaluate a model's type invariants through
  FR-652's `value_invariants` selection of an FR-651 `items` request,
  `{"value_invariants": {"snapshot": K}}` or
  `{"value_invariants": {"snapshot": K, "invocation": K2}}`, which reaches
  FR-106's `ValueInvariants` admission and FR-366's evaluation of the
  effective invariants over every reachable value of each record value type
  and event type. No other selection evaluates them, and FR-652 states that
  selection's member set, its request-stage refusals and its item order.
- When an invariant evaluates to `false`, its item SHALL report the
  disposition `evaluate`, category `violation`, `truth: false`, with
  FR-652's `invariant` (the clause's declaration key) and `path` (the
  value's FSpec FR-207 runtime value path), and the items document SHALL
  take category `violation`, exit 10, as FR-651 settles a document with a
  violating item.
- When an invariant's evaluation is undefined or incomplete, its item SHALL
  report that outcome. When every item holds, each item SHALL report success
  with `truth: true` and the document SHALL report success, exit 0.


## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-702-AC-1 | Over the ConfigVersion unit and invocation where `child.versionNumber` moves from 2 to 3 inside `attemptUpdate`'s frame, with the domain operation given `ensures` text `self.versionNumber = pre(self.versionNumber)`, the `frame` run of `Config::ConfigVersion::attemptUpdate` reports stage `evaluate`, category `violation`, `truth: false`, exit 10, naming the operation, kind `ensures` and position 0. With `ensures` text `pre(self.versionNumber) < self.versionNumber` the same run reports success with `truth: true`; that `ensures` holds no arithmetic operand, so it carries no range obligation, while `self.versionNumber = pre(self.versionNumber) + 1` over the fixture's `Int[0, 1000]` field is refused at check `undefined_expression`/`unproved-range` (proved `[1, 1001]`), because no fact flows from a kept `requires` into a kept `ensures` (FR-577). | Test (TC-1908) |
| FR-702-AC-2 | With `requires` text `self.versionNumber > 5` on the same operation and invocation, the `frame` run reports a violation naming kind `requires` and position 0, and evaluates no `ensures` clause; with `requires` text `self.versionNumber < 5` it evaluates the `ensures` clause next. A `requires` or `ensures` clause in a language other than `quire` is not evaluated and changes no result. | Test (TC-1908) |
| FR-702-AC-3 | Through `run`, over TC-235 K2 with `Money`'s invariant `NonNegative` `self.amount_minor >= 0` and an `orders` population, an `items` request with the selection `{"value_invariants": {"snapshot": "s1"}}` over a snapshot holding an `Order` whose `total` has `amount_minor` -1 reports exactly one violating item, whose `invariant` is `NonNegative`'s clause key and whose `path` names that `total` field, and the document reports `violation`, exit 10; with `amount_minor` 7 the document reports success, exit 0. | Test (TC-1908) |

## Dependencies

- FR-103 keeps and checks an operation's own `requires` and `ensures`;
  FR-366 classifies, checks and evaluates type invariants; FR-115 owns the
  frame run; FR-312 owns the clause run request reader; FR-651 owns the
  `items` request and FR-652 its `value_invariants` selection; FR-106 owns
  the `ValueInvariants` admission.
