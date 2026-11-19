---
id: FR-750
title: "Run a hybrid item through its negotiated hybrid provider and settle what it returns"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-447
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-250
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-450
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-404
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-193
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-323
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-621
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: depends_on
---
# FR-750: Run a hybrid item through its negotiated hybrid provider and settle what it returns

## Description

`analyze` SHALL run every hybrid item (an FSpec FR-331 `temporal-satisfaction`
item carrying the `hybrid-dynamics` requirement) on the hybrid provider that
negotiation routed it to, in process or in a process of its own, and SHALL
settle the provider's result by FR-250 and FR-450. FSL owns the provider
interface, the dispatch, the replay and the settlement; the solver is the
provider's, by design (ADR-026). A hybrid item with no provider settles
`unsupported`.

## Inputs

A checked package holding the claim's `CheckedHybridModel` and
`CheckedHybridClaim` (FR-447); one negotiated hybrid item; the registered
engines; and the request's limits, deadline and cancellation.

## Outputs

One terminal record per hybrid item and, when exact replay or an accepted
validated witness reproduces a trace, one `counterexamples` entry.

## Behavior

### The in-process provider interface

- `fsl-analyze` SHALL export a trait `HybridProvider`, as
  `fsl_analyze::provider::HybridProvider`, with one method that takes the
  item's typed request `HybridSolveRequest` (its `CheckedHybridClaim`, its
  `CheckedHybridModel`, its resolved `hybrid.limits` as FR-250's
  `SolverBudget`, its FSpec FR-331 `options` and `deadline_ms`, read from the
  checked package and the item) and the cancellation token, and returns a
  `HybridAnswer`: a `HybridVerdict`, optionally with an FSpec FR-652
  certificate. The trait, the request and the answer are defined in
  `fsl-replay`'s `hybrid` module beside `SolverResult` and re-exported by
  `fsl-analyze`, so a provider crate below layer A, such as
  `fsl-hybrid-reference` (FR-1019, ADR-011 layer H), implements it without
  depending on `fsl-analyze`.
- `HybridVerdict` SHALL be the sum of what a hybrid provider can answer,
  one arm per settlement path, so no arm stands in for another:

  ```rust
  pub enum HybridVerdict {
      Solved(SolverResult),                 // proved, inconclusive or incomplete: FR-250's map
      Refuted(HybridCounterexample),        // a claimed refutation: settled only by FR-450
      Unsupported(UnsupportedConstruct),    // a construct outside the provider's class
  }
  pub struct UnsupportedConstruct {
      pub form: String,                     // the recognized construct, as FR-272 names it
      pub site: ExpressionSite,             // the flow, guard, reset, invariant or initial entry
      pub term: TermLocus,                  // the construct within that expression
      pub mode: ModeIndex,
      pub variable: Option<VariableIndex>,  // for a flow, the variable whose rate it is
  }
  ```

  `Solved` takes FR-250's verdict map and coverage rule. `Refuted` holds
  the provider's complete counterexample, its kind and trace, as a claim:
  FR-450's exact replay settles it `refuted` when it reproduces, and
  `inconclusive`, `ReplayParity(DisagreementCause::HybridTrace(..))` with
  that counterexample when it gives `NotReproduced` or `NotExact`. A
  claimed refutation is never folded into `SolverOutcome::Unknown(trace)`,
  because FR-250 settles that `SolverInconclusive`, and replay parity needs
  the claim. `Unsupported` settles `unsupported` with the diagnostic
  `unsupported_construct`/`expression-form`, its `detail` naming `form`,
  the `declaration` (the hybrid model), `mode`, `variable` for a flow and
  `backend`, and its location the construct's term node (FSpec FR-193
  "Solver result on the wire"); no engine verdict is written and replay
  does not run. An `UnsupportedConstruct` whose site or term the model does
  not have is an engine fault and settles `failed`
  `runtime_invariant`/`established-invariant-broken`.
- `HybridAnswer::read_entry(entry, counterexamples, model)` SHALL read one
  FSpec FR-331 hybrid result entry (its value, basis or cause, trace,
  counterexample and `certificate`) into a `HybridAnswer`, refusing an
  entry FSpec FR-331 refuses with that cause and pointer. It takes the
  claim's `CheckedHybridModel` because every trace it reads is checked
  against the model before it is kept (FR-250, FSpec FR-193 "Hybrid
  trace"): mode and variable names become indices only through the model,
  and a trace that names no mode of it is refused at its pointer. It takes
  the frame's `counterexamples` array because a `refuted` entry's
  `counterexample` member is a position in it (FSpec FR-331): the entry it
  names is read against `model` into `HybridVerdict::Refuted`, and a
  position outside the array refuses `invalid_result`/`invalid-value` at
  `/counterexample`. A `proved` entry reads into `Solved` by its `basis`,
  an `inconclusive` or `incomplete` entry into `Solved` by its `cause`, and
  an `unsupported` entry carrying the `expression-form` diagnostic into
  `Unsupported`, its `detail` and location read against `model`. Any other
  admitted entry, such as `failed` with `tool-failure` or `unsupported`
  with `tool-unavailable`, is not a solver answer: `read_entry` returns
  `HybridEntryRefusal::NotRead` and the item takes FR-290's settlement of
  that entry unchanged. The process path below reads a provider's entry through it,
  and an in-process provider may build its answer through it.
- Every exact number the hybrid reader and writer handle SHALL be the FSpec
  FR-331 `Rational` object `{"numerator": n, "denominator": d}`: a
  `horizon`, a `delta`, each `lower` and `upper` of an `Interval`, a
  `where.time` and an FR-652 certificate's `step`. A plain string or a JSON
  number there refuses `invalid_result`/`wrong-value-kind` at the member
  (FSpec FR-193 "Solver result on the wire"). Only an item's `options`
  values are strings, as FSpec FR-331 keeps `options`; FR-1019's `step`
  option is such a string.
- `HybridCounterexample::to_wire(model)` SHALL write every counterexample,
  of either kind, in FSpec FR-193's form, and SHALL return a value, not an
  `Option`. Kind `Formula` writes `"kind": "formula"`. Kind
  `UndefinedEvaluation{at, cause}` writes FSpec FR-364's envelope
  `{"type": "undefined-evaluation", "where": {"position", "time",
  "locus"}, "cause"}`: `position` is `at.state`; `time` is a point
  `Interval` when the instant is rational and otherwise the isolating
  interval of the instant's root, with rational ends; `locus` is the FSpec
  FR-331 `Locus` (node and occurrence) of the term node that `at.locus`
  names in the checked package (FSpec FR-481); and `cause` is the
  catalogued reason, `division-by-zero` for `DivisionByZero`. The plugin
  reader SHALL read that form back, keeping a non-point `time` as the
  interval it read, so a provider's undefined-evaluation claim reaches
  FR-450 as a `Refuted` answer. The record's counterexample is always the
  one FR-450's exact replay reproduced, written from the replay's own
  instant; a provider's `where` survives only inside its claim on a
  `replay-parity` record.
- A refuted hybrid item's `replay.target` SHALL be `{"state_clause": Q}`,
  `Q` the hybrid claim's declared qualified name, as FR-621 writes it for
  every temporal clause: a hybrid claim is a temporal clause whose `over`
  parameter is typed by a hybrid declaration (FSpec FR-193 "Hybrid
  claims"), and FSpec FR-323 resolves `state_clause` among the package's
  state clauses and declared claims. FR-098's executor SHALL dispatch a
  replay request whose `state_clause` resolves to a hybrid claim to FR-450
  over the recompiled claim and model, and write the FSpec FR-323 `hybrid`
  row: `reproduced-with-evaluated-witness` with the reproduced
  counterexample, `inconclusive` with the `hybrid-trace` disagreement, or
  `incomplete` with the `work_units` limit.
- `Engine::hybrid(descriptor, provider)` SHALL make an engine from a
  descriptor whose `dynamics` holds `hybrid` and that advertises
  `temporal-satisfaction`. A descriptor that does not is refused with a typed
  error naming the backend and the cause `invalid-dynamics`, and no engine
  is made.
- `Engine::hybrid_process(descriptor, tool, exchange)` SHALL make the engine
  of a process provider from its `Process` descriptor, under the same
  descriptor rule, and an exchange that takes the FR-447 process request
  frame bytes and the cancellation token and returns the provider's FSpec
  FR-331 result frame bytes or a typed tool failure. The driver's plugin host
  implements the exchange over the child process (FR-635); FSL writes the
  frame and reads the result, so the transport carries bytes only.
- `HybridItem`, the `ItemForm::Hybrid` settings, SHALL carry the item's
  proof bounds, its `hybrid.limits` (`max_solver_steps` and
  `max_solver_time`, each optional and taking the serving descriptor's
  `bounds` when absent, FSpec FR-193 "Solver budgets") and its FSpec FR-331
  `options`. FR-620's item resolution SHALL build `ItemForm::Hybrid` for
  every item whose claim is a checked hybrid claim, reading the document
  item's `hybrid` and `options` members, so a `native-analyze/1` item over a
  hybrid claim reaches the dispatch below.

### Dispatch

- For a hybrid item whose disposition is `supported` by a candidate that an
  engine made by `Engine::hybrid` serves, `analyze` SHALL call that engine's
  provider once, and SHALL settle its `SolverResult` by FR-250's verdict map
  and coverage rule and replay its traces by FR-450, so a provider's
  `refuted` settles `refuted` only after exact replay or an accepted
  validated witness.
- For a hybrid item whose candidate is a process provider (FR-404's plugin
  replay context), `analyze` SHALL write the FR-447 process request with the
  emitted package's v2 bytes, read the provider's FR-331 result entry, and
  settle it by the same rules. A clause request (a `native-analyze/1`
  request whose items name hybrid clauses by declared name, FR-620) SHALL
  offer process-provider composition for its hybrid clauses exactly as for
  an item request: the process provider is one of the `providers` the
  command entry takes (FR-635), made by `Engine::hybrid_process`.
- A settled hybrid item SHALL be written into FR-621's outcome item: its
  `record` carries `result`, the proof `basis` or the `cause`, the
  `certification` of a proved record and `backend`; a `refuted` item carries
  its one `HybridCounterexample` as `counterexample` with `replay`; and an
  item whose provider returned a certificate carries `certificate`, with
  `rule` and `at` when the certificate was rejected (FR-621).
- A hybrid item with an empty candidate set SHALL settle `unsupported`,
  `HybridProviderAbsent` (`unsupported_projection`/
  `unsupported-requested-capability`, its warning naming
  `temporal-satisfaction` and `hybrid-dynamics`), and no engine runs.
- A hybrid item routed to a candidate that no registered engine or process
  provider serves SHALL settle `unsupported`,
  `unsupported_projection`/`tool-unavailable` naming that backend.
- A provider that returns a certificate SHALL have it checked by FR-751
  before the record is written.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-750-AC-1 | With an in-process fixture provider registered through `Engine::hybrid`, `SlowEnough` over `Braking` is dispatched once to it, and its `delta-complete` `unsat` covering horizon 10 settles `proved`, `bounded-solver`, `uncertified`, naming the fixture backend. | Test (TC-2100) |
| FR-750-AC-2 | `Engine::hybrid` with a descriptor whose `dynamics` lacks `hybrid` returns a typed error naming the backend and makes no engine. | Test (TC-2100) |
| FR-750-AC-3 | With no hybrid provider registered, the hybrid item settles `unsupported`, `HybridProviderAbsent`, `unsupported-requested-capability`, its warning naming `temporal-satisfaction` and `hybrid-dynamics`, through `analyze` and through `provider::run`. | Test (TC-2100) |
| FR-750-AC-4 | A hybrid item routed to `hybrid-fixture` while no engine serves it settles `unsupported`, `tool-unavailable`, naming `hybrid-fixture`. | Test (TC-2100) |
| FR-750-AC-5 | A process fixture provider receiving the v2 bytes of a clause request over `SlowEnough` returns a `refuted` point trace that exact replay reproduces; the item settles `refuted` with one `HybridCounterexample`; a trace that does not reproduce settles `inconclusive`, `replay-parity`. | Test (TC-2100) |
| FR-750-AC-6 | `HybridAnswer::read_entry` reads a `proved` entry whose `horizon` is `{"numerator": "10", "denominator": "1"}` into `Solved` with horizon 10; the same entry with `horizon` `"10"` or `10` refuses `invalid_result`/`wrong-value-kind` at `/basis/horizon`, and a trace interval with `lower` `"0"` refuses at that member's pointer; an item with `options` `{"step": "1/16"}` reaches the provider with that string. | Test (TC-3035) |
| FR-750-AC-7 | An in-process fixture answering `Unsupported` for the flow `v' = -a / (1 + v)` of `Moving`, and a process fixture whose entry is `unsupported` with `unsupported_construct`/`expression-form` naming the same, each settle the item `unsupported` with that diagnostic, `detail` naming `form`, `declaration`, `mode` `Moving`, `variable` `v` and the backend, located at the flow's term node, with no `counterexamples` entry and no replay; an `Unsupported` naming a mode the model does not have settles `failed`, `runtime_invariant`. | Test (TC-3035) |
| FR-750-AC-8 | An in-process fixture answering `Refuted` with the FR-450-AC-2 point trace against `always [0, 12] holds(h.v > 0)` settles `refuted` with one `HybridCounterexample`; the same answer with `before` `v = 1` settles `inconclusive`, `ReplayParity(DisagreementCause::HybridTrace(NotReproduced{counterexample}))` carrying the fixture's complete counterexample, never `SolverInconclusive`; and a `Solved` `Unknown(Some(trace))` with that trace settles `SolverInconclusive{trace, replay}`. | Test (TC-3035) |
| FR-750-AC-9 | Over `Braking` against `always [0, 12] holds(1 / (h.v - 5) != 0)`, the reproduced counterexample's `to_wire` is `{"type": "hybrid-counterexample", "kind": {"type": "undefined-evaluation", "where": {"position": 0, "time": {"lower": {"numerator": "15", "denominator": "2"}, "upper": {"numerator": "15", "denominator": "2"}}, "locus": L}, "cause": "division-by-zero"}, "trace": ...}`, `L` the division's term node, and the plugin reader reads it back to a `Refuted` answer that FR-450 reproduces; a reproduced instant that is irrational writes a `time` with rational ends `lower < upper` that contains it; the analyze outcome item carries that counterexample and never settles `failed` for an unspelled counterexample. | Test (TC-3035) |
| FR-750-AC-10 | The refuted hybrid item `TooSlow` carries `replay` `{"target": {"state_clause": "TooSlow"}, "reproduced": true}`; the replay request built from it resolves to the hybrid claim in the recompiled package and FR-098 writes the FSpec FR-323 `hybrid` row `reproduced-with-evaluated-witness`; the same request with a non-reproducing trace writes `inconclusive` with the `hybrid-trace` disagreement. | Test (TC-3035) |
| FR-750-AC-11 | `read_entry(entry, counterexamples, model)` reads a `refuted` entry whose `counterexample` is 0 from `counterexamples[0]` against `model` into `Refuted`; `counterexample` 1 with one entry refuses `invalid_result`/`invalid-value` at `/counterexample`; a `solver-inconclusive` trace naming a mode `Halted` the model lacks refuses `invalid_result`/`invalid-value` at `/cause/trace/start/mode`; a `failed` `tool-failure` entry returns `NotRead` and the item settles `failed` by FR-290. | Test (TC-3035) |

## Dependencies

- FR-447 (request writing), FR-250 (verdict map, budgets), FR-450 (replay),
  FR-404 (process provider context), FR-751 (certificates).
- FR-621 (the outcome item and its replay target) and FR-098 (the replay
  executor).
- FSpec FR-193, FR-323, FR-331 and FR-364.
