---
id: FR-1030
title: "Request a refinement transfer from a native run"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-specification/FR-380
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-820
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-1020
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-144
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-529
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: depends_on
---
# FR-1030: Request a refinement transfer from a native run

## Description

FSL SHALL read a `native-run/1` request's `refinement_transfer` member,
which names a model refinement declaration, its abstract and concrete model
selections with their initial snapshots, and the infinite-trace clauses over
the abstract model to transfer. For each clause FSL SHALL settle one FSpec
FR-380 transfer item through FSpec FR-820's transfer request, deciding
FR-1020's step-for-step condition from the refinement's step rows, and SHALL
write the FSpec FR-820 planner results as the run outcome's
`refinement_transfer` member. The concrete subject is never explored for a
transfer; only the refinement premise explores it (FR-144).

## Inputs

The `refinement_transfer` member is a closed object, read with FR-100's
closed-envelope rules (unknown and duplicate members refuse):

```json
"refinement_transfer": {
  "refinement": "ThirdRefinesSpec",
  "abstract": {"model": "Spec", "initial": ["spec-0"]},
  "concrete": {"model": "Third", "initial": ["third-0"]},
  "clauses": ["PrevSafe", "SoonTwo"],
  "observations": {"spec-0": "spec-0.json", "third-0": "third-0.json"},
  "premises": {"refinement": {}, "claims": {"PrevSafe": {}}},
  "limits": {"max_work": 100000},
  "observation_limits": {}
}
```

- `refinement` — required: the name of a refinement declaration of the
  program (FR-135).
- `abstract`, `concrete` — required: `model`, the alias of a `model`
  selection of the unit, which SHALL be the refinement's declared abstract
  and concrete subject; `initial`, a non-empty list of `observations` keys
  naming that model's initialization snapshots (`filament.state.snapshot/v1`,
  FR-106). Together they are FR-144's concrete subject and abstract initial
  snapshots.
- `clauses` — required, non-empty, no name twice: names of `temporal`
  declarations under the infinite-trace profile whose `over` parameter is
  typed in the abstract model. Each is one transfer item, in list order.
- `observations` — required: FR-312's map from key to snapshot file.
- `premises` — optional. `refinement` is an FSpec FR-492 premise result for
  the refinement item; `claims` maps a clause name of `clauses` to the
  premise result of its abstract claim. A premise present here is read and
  admitted under FSpec FR-820, never settled again; a premise absent here is
  settled by FSL in the same run: the refinement item by FR-144, and the
  abstract claim item over the abstract subject under the clause's fairness
  set, over its stutter closure when the clause is a liveness clause
  (FR-529's `stutter_closure`). In-process premises keep their own
  certification (FSpec FR-492).
- `limits` — optional: `max_work`, FSpec FR-492's planner budget, setting
  `planner.max_work`. Premise items settled in the run use their engines'
  published default limits.
- `observation_limits` — optional: FR-106's `ObservationLimits` overrides.

The request's `models`, `libraries` and `program` are FR-100's.
`refinement_transfer` is a selection member: a request carries exactly one
of `call`, `clause` (FR-312), `items` (FR-651), `synthesis` (FR-1010),
`export` (FR-1015), `export_evidence` (FR-1022), `tl_lowering` (FR-931),
`model_based_test`, `verification_plan` (FR-1038), a selected
`package` and `refinement_transfer`.

## Outputs

The run outcome's `refinement_transfer` member:

```json
"refinement_transfer": {
  "refinement": {"name": "ThirdRefinesSpec", "node": "<NodeId>",
                 "step_for_step": {"holds": true}},
  "transfers": [
    {"clause": "PrevSafe", "result": {"format": "filament.planner-result/v1"}}
  ]
}
```

- `refinement.step_for_step` — FSpec FR-820's `step_for_step` fact,
  decided once from the refinement node's step rows (FR-1020).
- `transfers` — one entry per clause, in `clauses` order; `result` is the
  item's FSpec FR-820 planner result, with its `transfer` record, every
  value in FR-650's run value form.
- The exit code is FR-285's: `0` when every transfer is `proved`, `22` when
  any is `inconclusive` or `incomplete`. A transfer is never `refuted`, so
  this member never exits `10`.

## Behavior

- FSL SHALL compile the program through the spine (FR-100) before reading
  the member's names, and SHALL resolve `refinement`, both `model` aliases
  and every clause in the compiled package.
- For each clause FSL SHALL build one FSpec FR-820 request: `item.node` the
  refinement node, `item.clause` the clause node, `item.subject` the
  concrete model and its `initial` snapshots, slot 0 the refinement premise
  and slot 1 the abstract-claim premise, and SHALL settle it through
  `planner::settle` (FR-529). The refinement premise is settled at most
  once and shared by every clause of the request.
- FSL SHALL settle each item by FSpec FR-820's rule order, so a clause in
  FSpec FR-380's stutter-invariant fragment transfers through any proved
  refinement, and a clause using previous (`once[1,1]`) or an interval
  operator transfers exactly when FR-1020 finds the refinement
  step-for-step.
- A refinement that is not step-for-step is not a refusal: each
  position-counting clause settles `inconclusive`, `unsettled`,
  `transfer-not-preserving{reason: fragment}` naming its first such
  operator, and each fragment clause settles as any other transfer.
- Settling one transfer SHALL leave every other transfer of the request
  unchanged.
- An abstract premise whose proof FR-127 cannot settle `proved` is a typed
  unproved slot 1, never `proved`. A past or interval (TP-3) clause `on
  origin` over an ordinary model subject is certified by the core (FR-338),
  so its premise settles `proved`. A TP-3 clause the core does not certify,
  one `on each` or over a protocol subject, settles `failed`; each transfer of
  such a clause is `inconclusive`, `premise-unproved` naming slot 1 with that
  result. That limit is honest and stays: it is not a `proved` and not a
  refusal, and certifying those general modes is outside this requirement
  (architecture ruling of 2026-11-21 on QSL-2247; a separate related ticket
  carries it).
- A liveness clause's abstract premise is settled over the abstract stutter
  closure (FR-529's `stutter_closure`). A violating loop that is the closure's
  own self-loop at a nonterminal abstract state is a closure-only lasso. It is
  written and replayed by FR-128's closure-stutter rule, and slot 1 settles
  `refuted` through that replay only; it is never `unsupported`
  (`ClosureStutterLoop`). The transfer then settles `inconclusive`,
  `premise-unproved` naming slot 1 with `result` `refuted`, since a transfer
  is never `refuted`, and the exit code is `22`. The abstract claim's
  selected `stutter_closure`, which FSL reads from the admitted item's
  obligation identity and not from the slot key FSpec FR-820 matches without
  it, is the authority for that replay. A fair clause such as `EventuallyTwo`
  still proves, and a terminal-stutter refutation such as `PrevStutter`'s
  still replays unchanged. No empty certificate or engine verdict settles a
  premise `proved`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1030-AC-1 | A `refinement_transfer` request over the step-for-step `ThirdRefinesSpec` for `PrevSafe` (`always (holds(c.value = 1) implies once[1,1] holds(c.value = 0))`) and `SoonTwo` (`always (holds(c.value = 1) implies eventually[0,2] holds(c.value = 2))`), with no `premises`, settles the refinement and both abstract claims in the run and writes `step_for_step.holds` `true` and two transfers, each `proved`, `decisive-witness`, with both premise identities in its `transfer` record; exit `0`. | Test (TC-2920) |
| FR-1030-AC-2 | The same clauses through `CasRefinesCounter` write `step_for_step.holds` `false` with `row` naming `Impl::Counter::beginA`, `stutter`, and each transfer `inconclusive`, `unsettled`, `transfer-not-preserving{reason: fragment}` naming `once[1,1]` and `eventually[0,2]`; `always holds(c.value <= 2)` in the same request transfers `proved`; exit `22`. | Test (TC-2920) |
| FR-1030-AC-3 | With `premises.refinement` and `premises.claims.PrevSafe` supplied as complete proved FSpec FR-492 premise results marked `certified`, FSL settles neither premise again and the transfer is `proved`, `uncertified`; with the supplied claim premise `refuted`, the transfer settles `inconclusive`, `premise-unproved` naming slot 1 and `refuted`. | Test (TC-2920) |
| FR-1030-AC-4 | FSpec FR-380-AC-8's liveness clause, with no `premises`, is settled over the abstract stutter closure and transfers through `CasRefinesCounter` `proved`. | Test (TC-2920) |
| FR-1030-AC-5 | A transfer premise reads its model with the same semantics as any model-subject claim, so the terminal state repeats under stutter extension: the clause `PrevStutter`, `always (holds(c.value = 2) implies once[1,1] holds(c.value = 1))`, is not proved at the repeated terminal state 2 (the state one step back is 2), and its transfer through the step-for-step `ThirdRefinesSpec` settles `inconclusive`, `premise-unproved` naming slot 1 with `result` `refuted`, the slot's own record showing `refuted`; exit `22`. | Test (TC-2920) |
| FR-1030-AC-6 | Through `CasRefinesCounter`, the liveness clause `StuckTwo` (`always eventually holds(c.value = 2)`, no fairness) has an abstract premise violated by the closure stutter at value 0 and at value 1, so EN-1 writes one closure-only lasso, whose stem is whichever EN-1's discovery order yields. The run writes slot 1 `refuted`, which settles only when the production replay gate reproduced the lasso. The transfer outcome carries no lasso; the lossless equality of a written and read closure lasso and its post-read replay over the source and package routes are asserted by the FR-128-AC-14 and FR-128-AC-15 tests over the same closure-stutter mechanism. The transfer is `inconclusive`, `premise-unproved` naming slot 1 with `result` `refuted`, exit `22`, and never `unsupported`. | Test (TC-2920) |
| FR-1030-AC-7 | The proposed clause `PendingTwo`, declared in `TRANSFER_DECLARATIONS` with `temporal PendingTwo using inf over (c: Spec::Counter) clock "model-steps" on origin { always (holds(c.value = 1) implies eventually holds(c.value = 2)) }` (the template of `StuckTwo`, no fairness), has the closure stutter at value 1 as its only violating loop, since the stutter at value 0 and the terminal stutter at value 2 satisfy it. Through `CasRefinesCounter` slot 1 is `refuted` through the replay gate over a closure-only lasso with a one-step real prefix (the written and read lasso of that shape is asserted by the FR-128-AC-14 test), and the transfer is `inconclusive`, `premise-unproved` naming slot 1, `result` `refuted`, exit `22`. | Test (TC-2920) |

## Rationale

The `PrevSafe` fixture clause is a bounded-past premise that holds under
terminal stutter. Its earlier form, `PrevStutter`, was ruled not to hold
(architecture ruling of 2026-11-20 on QSL-2247): a transfer premise has no
special case for a step-for-step refinement, and at the repeated terminal
state the state one step back is the terminal state itself. FR-1030-AC-5 pins
that reading.

## Dependencies

- FSpec FR-380 (the transfer rule) and FSpec FR-820 (its request, settlement
  and record); FR-1020 (the step-for-step decision); FR-144 (the refinement
  premise); FR-100 and FR-312 (the request envelope and observations);
  FR-529 (`planner::settle` and the planner-only causes).
