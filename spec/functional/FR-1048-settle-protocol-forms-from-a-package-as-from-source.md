---
id: FR-1048
title: "Settle every protocol form from an admitted package as from source"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-216
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-730
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-895
    type: references
  - target: ix://agent-ix/filament-specification/FR-171
    type: references
  - target: ix://agent-ix/filament-specification/FR-426
    type: references
  - target: ix://agent-ix/filament-specification/FR-428
    type: references
  - target: ix://agent-ix/filament-specification/FR-466
    type: references
  - target: ix://agent-ix/filament-specification/FR-736
    type: references
  - target: ix://agent-ix/filament-specification/FR-785
    type: references
  - target: ix://agent-ix/filament-specification/FR-822
    type: references
  - target: ix://agent-ix/filament-specification/FR-831
    type: references
---
# FR-1048: Settle every protocol form from an admitted package as from source

## Description

When `fsl analyze` receives a `package` selection (FR-850) whose protocol uses
a form the source route settles, FSL SHALL select, execute and replay that
form from the admitted package bytes and SHALL write the item record, verdict,
counterexample, replay result and process exit the source route writes for
the same unit. FR-940 owns the sealed `AnalysisView` and the rule that no
package-route item settles `unsupported` for a fact the source route reads;
this requirement states that rule for each protocol form whose source
behavior is owned elsewhere, so that each form has a package-route criterion
of its own.

## Inputs

- A `package` request (FR-850) over the handoff directory of a compiled unit
  that declares the protocol form.
- The same unit as a `program` request, for the source route the package
  route is compared with.

## Outputs

For each form below, the package route's item records, verdicts,
counterexamples and replays equal the source route's over the same emitted
bytes, and a form the source route settles is never `unsupported`, never
refused `unsupported_construct`/`not-yet-implemented`, and never dropped on the
package route.

## Behavior

### One rule for every form

The package route reads each form from the admitted `AnalysisView` rows and
nodes alone (FR-940). It builds no `CheckedPackage` or `CheckedGraph` from
admitted bytes (FR-087 R-10) and reads no source file. A package whose
strict-reader admission refuses a form's rows refuses the whole request with
the reader's code, cause and pointer before an engine runs. A form that
admission accepts and whose evaluation fails settles its own item with the
evaluation outcome the source route gives; a sibling item is unaffected.

The forms and their source owners are: correlated `attempt` and `recommend`
(FSpec FR-831, FR-822); `effect`, `event`, `commit` and `fence` (FR-216,
FSpec FR-425, FR-822); compensation templates (FSpec FR-736); proof-bearing
`repeat` (FR-730); replicated-role lifetimes (FSpec FR-428); symbolic queue
capacity (FSpec FR-785); and `quorum` joins (FSpec FR-171, FR-426, FR-466,
FR-218). The `predicate` join stays unsupported on the package route:
FR-895-AC-36 owns that outcome and this requirement does not change it.

### Quorum joins

A `join quorum(n)` over `m` joined branches is admitted exactly when
`1 <= n <= m` and the join states an `outstanding` rule (FR-218-AC-23,
FR-218-AC-40; FSpec FR-466-AC-4 for the package member `count`). The join step
is enabled when `n` distinct branches have completed and not before; under
`outstanding cancel` the remaining branches become `cancelled` in the join
step and take no further step, and under `outstanding continue` they keep
taking steps (FSpec FR-426-AC-2). A recorded execution reaches the join at the
`n`-th distinct completed branch (FR-895 item 21). The package route reads
`count` from the admitted `parallel` node's `join` Group and applies the same
rules.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-1048-AC-1 | A protocol that declares a correlated `attempt` (an attempt with a `related` clause) and a `recommend` control with a correlation clause (FSpec FR-831-AC-3, FR-831-AC-12, FR-822-AC-3) is compiled with a handoff directory and analyzed as a `program` request and as a `package` request. Both routes admit the same forms (`send`, `receive`, `attempt`, `effect`, `event`, `recommend`) and give equal item records, equal `recommend` steps (`step` `recommend`, the `protocol-recommendation` record with `node`, `role`, `operation`, receiver and arguments), equal counterexamples and equal replays with `reproduced` `true`; a replay with the record's `arguments` altered refuses on both. The package route's correlation clauses come from the admitted member in source order; a package whose `related` is nonempty and whose clause member is absent refuses `unavailable_correlation_provenance` and uses no guessed premises (FSpec FR-831-AC-12), and a control whose `related` is empty is not refused. No item of either request is `unsupported` for an operation the source route supports. | Test (TC-3215) |
| FR-1048-AC-2 | A protocol that runs `effect`, `event`, `commit` and `fence` forms (each with its static-path identity and binder lifetime, FR-216-AC-4 to FR-216-AC-7, FSpec FR-425-AC-16) settles the same positive execution from source and from its package: the same steps, the same enabling, read and write footprints, the same business-effect ordinals and occurrence keys across loop iterations, the same binder lifetime at each step, equal verdicts and equal replays. The same five actions behind `observe sequence Review` refuse `ill_typed`/`operator-ineligible` at admission exactly as FSpec FR-822-AC-2 states, on the package route as on the source route, and the package route reads that boundary from the admitted package. | Test (TC-3215) |
| FR-1048-AC-3 | A protocol with a compensation template under `attempts 2` (FSpec FR-736-AC-1 to FR-736-AC-3) settles from its package with the source route's outcomes: exploration reaches attempts `failed`, `timed-out` and `successful`, a successful retry whose predecessor keeps `failed`, registrations whose aggregate fact is `pending`, `successful`, `failed` and `timed-out` and one that closes with no fact, and instance summaries `partially-recovered`, `recovered`, `unrecovered`, `untriggered` and `pending`, with no registration beyond two attempts; verdicts and replays are equal on both routes. The template is selected and executed from the admitted package forms alone. A package whose template rows the reader refuses refuses the whole request (refusal case); an item over a template whose execution fails settles that item `failed` with the source route's outcome while a sibling item over the same package settles normally (negative case). | Test (TC-3215) |
| FR-1048-AC-4 | A protocol whose `repeat Retry` has no maximum and the loop proof `invariant { left >= 0 } variant { left }` (FR-730-AC-1 to FR-730-AC-6) settles from its package the `loop-invariant` and `loop-termination` items with the source route's results: `proved` for the decreasing body; `loop-termination` `refuted` with a counterexample ending at the second decision of the first back edge that replays, for the unchanged body; `incomplete` naming the state limit under a state limit below the reachable count. A repeat with both a maximum and a loop proof derives only its invariant item on the package route as on source, a maximum-only repeat derives none, and a `temporal.max_depth` selection is refused on both. Item order, request indices and the absence of a `max_depth` member equal the source route's. | Test (TC-3215) |
| FR-1048-AC-5 | A protocol with `role w each M::Worker from workers max 2 lifetime until(w.done)` and with the lifetimes `workflow` and `scope` (FSpec FR-428-AC-1 to FR-428-AC-12) is admitted and executed from its package with the source route's results: `max_live_instances` bounds the live instances and settles `inconclusive` `instance-bound-reached` at the same state count; a retired object is not spawned again; `scope` instances retire at the same step for a loop, a parallel and the root `run`; each `actor` member names its acting instance; an `until` predicate that evaluates undefined settles every item over the subject `refuted` `undefined-evaluation` naming the predicate's locus; an empty population universe deadlocks `no-role-instance` naming the role. Two requests that differ only in `max_live_instances` carry one obligation identity on both routes. | Test (TC-3215) |
| FR-1048-AC-6 | A protocol whose channel `Order` has capacity `symbolic(Bound)` with overflow `block` (FSpec FR-785-AC-1 to FR-785-AC-7), analyzed from its package with `capacities` `{"Bound": "1"}`, settles deadlock freedom `refuted` `full-queue` naming the channel, and with `{"Bound": "2"}` `proved`, with equal item records, obligation identities (holding the binding) and counterexample replays on the source and package routes. With no binding the package route returns `requires-bound` before any state is expanded, naming `Bound` and each channel that names it, never `unsupported`; an unknown, malformed or repeated binding entry refuses with the code FSpec FR-785-AC-3 states. After the unit's source is overwritten with non-native bytes and the handoff `sources/` is deleted, the package route settles the same records. | Test (TC-3215) |
| FR-1048-AC-7 | Over three branches, `join quorum(2) outstanding cancel` and `join quorum(2) outstanding continue` settle from the package as from source: the join is enabled exactly at the second distinct completed branch and not before, the remaining branch is `cancelled` in the join step under `cancel` and keeps running under `continue`, and verdicts, counterexamples and replays are equal. A package whose `parallel` node carries `quorum` with `count` 0 or above the joined branches is refused by admission `invalid_package`/`invalid-value` at the join (FSpec FR-466-AC-4) before an engine runs, and one carrying `quorum` with no `count` member is refused by the reader as an absent required member (FSpec FR-466 `parallel` member table, `count` required exactly for `quorum`); on the source route S3 refuses `quorum(0)` and `quorum(3)` over two branches `invalid_package`/`invalid-value` at the list (FR-218-AC-23). A package holding a `predicate` join still settles `unsupported` `unsupported_construct`/`not-yet-implemented` (FR-895-AC-36) and no criterion here changes that outcome. | Test (TC-3215) |

## Dependencies

- FR-940: the sealed read view and the no-`unsupported` rule.
- FR-087 R-10: checked typestate is built only by S3 and S4.
- FR-895-AC-36: predicate joins stay unsupported on the package route.
