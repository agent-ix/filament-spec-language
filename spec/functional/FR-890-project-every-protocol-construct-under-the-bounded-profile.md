---
id: FR-890
title: "Project every protocol construct onto a role under the bounded profile"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-570
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-575
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-882
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-174
    type: implements
  - target: ix://agent-ix/filament-specification/FR-735
    type: implements
---
# FR-890: Project every protocol construct onto a role under the bounded profile

## Description

Under the profile `bounded` (FR-575, ADR-036 PA-26), local projection
(FR-570) SHALL project a checked protocol onto a selected role whatever
constructs the protocol holds. It SHALL return a sound local graph, a
counterexample, an `inconclusive` result at the horizon, or a `failed` result
at a run limit. It SHALL return `unsupported` only for
`evaluator-unsupported`, under FR-575's two `fsl-eval` limits. The rules are
ADR-036 PA-27 to PA-34, summarized here.

- **Decision points.** Each point at which the run can go two ways is a
  decision point with an owner, and PA-10's merge rule decides for every
  role whether that role can tell the ways apart (PA-27, PA-32):
  - a `choice`, owned by its `by` role;
  - a `repeat` guard read (continue, exit, exhausted), owned by its `by`
    role, with a post-test `repeat` running its body before the first read;
  - an `await` (match or timeout), owned by the role of its match event;
  - a `join any`, `join quorum(n)` or `join predicate(f)` (the disposition
    sets at which the policy first holds, under `outstanding cancel` or
    `continue`), owned by the environment;
  - a compensation activation, owned by the role whose step binds the
    trigger;
  - the commit cutoff of a compensation thread, owned by the commit node's
    role;
  - the acting instance of a replicated role's event, owned by the
    environment.
- **Local events (PA-28).** These are kept in the owning role's local graph,
  each with its record type: `attempt`, `effect`, `event` and `commit` (as
  internal actions), the local loop with its maximum and `test`, the local
  await with its window and clock, the join with its policy, joined labels
  and `outstanding` rule, the compensation region, `activate`, and `spawn` and
  `retire`. A `check` and a `fence` belong to no role. They are erased, the
  order through them is kept, and the correspondence table maps them to
  `global-only`.
- **Replicated roles (PA-29).** A replicated role class projects to one
  local graph, which each live instance runs. An event of the class is taken
  by the thread's session instance, which the thread's first event of the
  class pins. That event SHALL be a `receive`. Otherwise the projection is
  refuted with an acting-instance counterexample.
- **Activation on each (PA-30).** The local graph is the body's, run once per
  instance from its `activate` event. A role's observation is partitioned by
  instance ordinal.
- **Symbolic capacity (PA-31).** The capacity's name is written into the
  channel premise, and the local graph is as for a literal capacity.

The local graph SHALL keep FSpec FR-174's correspondence: every global
communication maps to one local send and one local receive, every global
event of the role maps to its local event, and every decision point's
alternatives that a role merges are recorded as alternatives. A
counterexample SHALL name the decision point, its kind and its owner (a role
or `environment`), the role, two global paths from the decision point to the
role's first divergent action, and the two actions (PA-34). It is minimal and
unique by PA-11's measure.

### Decision: one merge rule for every decision point

Every construct that the bounded-static fragment answered `unsupported` is a
place where the run branches: a loop's next iteration, a join's set of
finished branches, an await's timer, a compensation's trigger, or which
replicated instance acts. Projection asks one question about each of these
places: can the role tell which way the run went before it must act
differently? PA-10 already answers that question for a `choice`. Treating
every branch place as a decision point with an owner answers it everywhere,
with no second rule. Projection therefore covers the whole language and
stays sound by construction (PA-12).

Two of the new places needed a fact about their owner. Time is local to the
awaiting role, so an `await`'s timeout is that role's own decision. Thread
scheduling belongs to no role, so a join's firing and a replicated instance
choice belong to the environment, and every role takes the external side. A
replicated instance cannot know it is to act unless a message told it, which
is why its first event on a thread must be a receive. A protocol that breaks
this rule gets a counterexample, not a refusal, because the protocol has a
real defect: two instances could both act.

## Inputs

A projection request (FR-570) with profile `bounded` over a checked protocol
holding any construct FR-218 admits.

## Outputs

FR-570's outcomes: a `ProjectionWitness` whose local graph holds PA-28's
events, a `ProjectionCounterexample` with the `decision` member, an
`inconclusive` or a `failed` result, settled by FR-573. Under `bounded` the
result is `unsupported` only for `evaluator-unsupported`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-890-AC-1 | Under `bounded`, each FR-575-AC-1 protocol (`activation on each`, a replicated role, a symbolic capacity, `join any`, `join quorum(1)`, `join predicate(both)`, an `outstanding` rule under `join all`, a `repeat`, a compensation, a timed `await`, and each of `check`, `attempt`, `effect`, `event`, `commit` and `fence`) projects onto every role to `proved` with a local graph, never `unsupported`. Its correspondence table is total, and composed local traces are included in the global traces on the TC-205-style corpus. | Test (TC-2520) |
| FR-890-AC-2 | A `repeat Loop by Buyer max 2` whose body sends `Order` to `Seller` projects onto `Seller` with a local loop whose continue and exit first sets differ by message type, and `Seller`'s graph records the maximum 2. The same loop whose exit sends nothing to `Seller`, with no other distinguishing evidence, is refuted with `decision` `repeat`, owner `Buyer`, naming `Loop`. A role with no event in the body and none in the exhausted branch has no loop in its graph. | Test (TC-2521) |
| FR-890-AC-3 | A timed `await Wait ... within [0, 120]` whose match is a `receive` by `Seller` projects onto `Seller` as a local await carrying the window and clock name. When `Warehouse` acts differently in `then` and `timeout` and is told neither, the projection onto it is refuted with `decision` `await`, owner `Seller`. | Test (TC-2521) |
| FR-890-AC-4 | `join any [a, b] outstanding cancel`, where `Warehouse` sends in branch `b` after the join may cancel it, is refuted onto `Warehouse` with `decision` `join` and owner `environment`. The same join with `outstanding continue` projects to `proved`, with `b`'s remainder concurrent with the continuation in `Warehouse`'s graph. | Test (TC-2521) |
| FR-890-AC-5 | A replicated role `Pool each M::Cell from cells max 2` whose thread first receives `Job` from `Buyer` and then sends `Done` projects to one class graph whose `Done` send is pinned to the receiving instance, and the graph starts at `spawn`. A protocol in which `Pool` first sends is refuted with `decision` `acting-instance`, owner `environment`, naming the send and two instances. | Test (TC-2522) |
| FR-890-AC-6 | A compensation `Refund` by `w`, activated by an `event Fail` by `Seller`, projects onto `w` with a compensation region whose first event is the attempt. When `w` is told of the activation by no message, the projection onto `w` is refuted with `decision` `activation`, owner `Seller`. A `check` and a `fence` map to `global-only` in every role's correspondence table. | Test (TC-2522) |
| FR-890-AC-7 | Over a protocol holding none of the constructs outside the bounded-static fragment, the `bounded` result equals the `bounded-static` result in every member but the key's `profile`. Two runs on one `bounded` key give one result. | Test (TC-2520) |

## Dependencies

- ADR-036 §7 (PA-26 to PA-34): the rules.
- [FR-570](FR-570-project-a-global-protocol-onto-one-role.md): the
  projection request and outcomes.
- [FR-575](FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md):
  the profile words.
- [FR-882](FR-882-step-every-specified-protocol-construct.md): the stepped
  constructs.
- FSpec FR-174 and FR-735.
