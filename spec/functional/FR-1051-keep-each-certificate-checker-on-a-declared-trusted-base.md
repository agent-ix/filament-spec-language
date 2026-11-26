---
id: FR-1051
title: "Keep each certificate checker on a declared trusted base"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-284
    type: references
---
# FR-1051: Keep each certificate checker on a declared trusted base

## Description

A certificate checker (FR-282) accepts a `proved` only when its own code
establishes the claim from the certificate. Any function that the checker
and the producer of the certificate both call is trusted without being
checked: a defect in it writes a wrong certificate that the checker then
accepts. This requirement names the code a checker may rely on (its
trusted base), forbids every other sharing between producer and checker,
and requires two kinds of evidence that a checker rejects what it should:
single mutations of accepted certificates, and false claims whose truth a
definitional reference computes.

ADR-028 CE-7 and ADR-026 CF-6 state the small trusted base for the EN-5 and
zone checkers; FR-921 item 7 states the no-shared-monitor rule for the
timed-product checker. This requirement states one rule for every checker
of FR-282's table and the replay checkers below.

### Terms

- **Checker code**: the code under these paths of `fsl-replay/src`:
  `probability_certificate.rs` and `probability_certificate/**`;
  `certificate.rs` and `certificate/**`; `hybrid_replay/certificate/**`;
  `model_check/replay.rs` and `model_check/replay/**`; `refinement/**`;
  and `replay_task_product_trace` in `outcome/counterexample.rs`; and the
  binary `tools/fsl-probability-check`.
- **Check entry**: a public function of checker code that returns a
  checker's verdict on a certificate or witness: the `check_*` functions
  of FR-282's checkers (for example `check_probability_certificate_document`,
  `check_probability_certificate_source`, `check_zone_certificate`,
  `check_closure`, `check_smt_proof`, `check_simulation_certificate`,
  `check_product_closure`, the hybrid `check_certificate`) and the replay
  entries (`replay_model_trace*`, `replay_protocol_trace*`,
  `replay_timed_counterexample`, `replay_probabilistic_witness*`,
  `replay_on_each_witness`, `replay_task_product_trace`).
- **Document codec**: a public function of checker code that converts
  between a certificate's typed value and its canonical bytes
  (`read_*`/`write_*`) and derives no member value; together with the
  document's own data types, which writer and reader share as one type.
- **Producer code**: code that searches for, computes or writes a
  certificate or witness: `fsl-analyze`, `fsl-hybrid-reference`, and any
  code outside checker code that calls a provider or engine for one (for
  example `fsl-replay/src/model_check/smt_query.rs`).
- **Route**: a checker profile (the document's `profile` member, or the
  checker spelling of FR-282 where the checker has no profiles) together
  with the abstraction it checks under (for example FR-921 item 1's
  digital and region routes, zones, explicit states).

## Behavior

### Trusted base

Checker code SHALL call, outside checker code itself, only the following:

| Part | Crates and modules |
| --- | --- |
| Front end and checked package | `fsl-foundation`, `fsl-cst`, `fsl-source`, `fsl-forms`, `fsl-semantics`, `fsl-package`, `filament-checked`, `filament-contract-model`, `filament-semantic-value`, and the crates these depend on |
| Canonical codec | `quire-canonical`, `fsl_foundation::node_wire`, `serde`, `serde_json`, `serde_path_to_error`, `base64`, `sha2` |
| Exact arithmetic | `filament-exact`, `num-bigint`, `num-integer`, `num-traits` |
| Model stepping and keys | `fsl_eval::simulation` (`TransitionSystem`, `StateKey`, expansion and limits), `fsl_eval::simulation::model` (`ModelSystem`, `TimedSystem` and their steps, guards, invariants and urgency; `PredicateReading`; model keys), `fsl_eval::simulation::protocol` (`ProtocolSystem` stepping and capacities) |
| Predicate and formula reading | `fsl_eval::model_subject` (`ModelSubject`, `ModelLetters`), `fsl_eval::temporal` (the ADR-018 SM-1 evaluator), `fsl_eval::value` |
| Position automaton | `fsl_eval::automaton::position_automaton` and the types it returns (`PositionAutomaton`, `ClockedEdge`, `Read`, `Stamp`, `WindowClaim`) |
| Services | `fsl-replay`'s request, limits, identity and source-admission code that FR-282 and FR-098 name, and `std` |

These parts are shared with producer code by design. A defect in them is
not caught by mutation; the false-claim corpus below is the evidence
against them.

### No other shared semantics

- Outside the trusted base, no function SHALL be called both by checker
  code and by producer code.
- Producer code SHALL call checker code only through a check entry or the
  document codec.
- A writer in checker code that derives a member value (a key, a rank, an
  objective, a value) is not document codec. The checker SHALL re-derive
  every derived member it compares, by its own code; sharing the
  document's data type does not permit sharing the function that computes
  the member.

### No producer objects

- A check entry SHALL take as evidence only the certificate (bytes or
  typed document), the checked package or byte provision, the claim and
  subject binding, the checker's limits and `&Cancel`.
- A check entry SHALL NOT take a value producer code built from those
  inputs, such as a `ModelSystem`, `TimedSystem`, `ClauseLetters`,
  `ModelLetters`, a product, a monitor or a measure plan. The checker
  SHALL build its own from the checked package.

### Recorded exceptions

The sharing below exists on main and violates the rules above. Each entry
is allowed only until its ticket removes it; an entry SHALL be deleted
from this table in the change that removes the sharing. No other
exception is allowed.

| ID | Rule broken | Sharing | Removed by |
| --- | --- | --- | --- |
| X-1 | Shared semantics | ADR-028 CE-7's shared EN-1 product: the property-automaton and monitor translations of `fsl_eval::automaton` (`translate`, `translate_counted`, `BoundedTranslation`, `SafetyTranslation`, `PropertyAutomaton`, `QualifiedBoundedMonitor`) | QSL-3501 |
| X-2 | Shared semantics | `fsl_eval::probabilistic::advance_nested_timed_deadline`, called by `fsl-analyze` digital stepping and the timed-product checker (FR-921 item 7 already forbids it) | QSL-3324 |
| X-3 | Shared semantics | The rest of the bounded monitor and measure step of `fsl_eval::probabilistic` (`MeasurePlan`, `MeasurePayoff`, `Accumulator`, `MonitorState`, `DeadlinePhase`, `TimedOperatorState`) | QSL-3501 |
| X-4 | Shared semantics | The fairness graph analysis of `fsl_eval::fairness` (`analyze`, `components`, `is_live`, `check_loop`) | QSL-3501 |
| X-5 | Shared semantics | The hyperproperty product of `fsl_eval::hyper` (`HyperProduct`, `ProjectedProduct`, `WitnessProduct`) | QSL-3501 |
| X-6 | Shared semantics | The SMT query encoders `fsl_eval::smt::encode_smt_query` and `encode_refinement_query`, used by the query producer and re-run by the SMT proof checker | QSL-3501 |
| X-7 | Producer call into checker | `measure_probability_objective` (the checker's own objective plan) and `measure_probability_product_key`, called by `fsl-analyze` exact measure and undefined-witness code | QSL-3501 |
| X-8 | Producer call into checker | `digital_probability_product_key` and `region_probability_product_key`, called by `fsl-analyze` digital stepping | QSL-3501 |
| X-9 | Producer call into checker | `on_each_outcome` and `on_each_has_instance`: the on-each producer lives in the checker file `probability_certificate/document/instances.rs` and shares its helpers with the on-each checker | QSL-3501 |
| X-10 | Producer call into checker | `probabilistic_formula_monitor_context`, called by `fsl-analyze` exact analysis | QSL-3501 |
| X-11 | Producer call into checker | The refinement step judge `check_initial`, `check_step` and `open_systems` (`refinement/**`), called by the zone refinement checker and by `fsl-analyze` zone refinement search | QSL-3501 |
| X-12 | Producer call into checker | `same_source_transition` and `TaskProductKey::new` (`certificate/task_product/**`), called by `fsl-analyze` task-product search | QSL-3501 |
| X-13 | Producer call into checker | The hybrid checker's model reader and interval operations (`HybridModelReader`, `box_advance`, `box_within`, `box_hull`, `box_tube`), called by `fsl-hybrid-reference` to build boxes | QSL-3501 |
| X-14 | Producer object | The checked-subject form of the EN-5 checker takes the producer's `ModelSystem` and claims slice and compares them by pointer, as `fsl-analyze` exact analysis calls it | QSL-3501 |
| X-15 | Producer object | Counterexample replay of a clause item reads atoms through the producer's `ClauseLetters`, and on the reference-literal path uses the producer's system | QSL-3501 |
| X-16 | Producer object | Task-product trace replay (`replay_task_product_trace`) checks keys of the producer's own product object | QSL-3501 |

### Rejection evidence by mutation

- Each route SHALL register at least one accepted certificate fixture with
  a mutation harness (test code only).
- For every registered fixture, every single mutation the harness makes of
  its canonical document SHALL be refused or rejected by the route's check
  entry, except a mutation on the route's allowed-to-survive list.
- The mutations SHALL include, wherever the document has such a member: a
  value changed, a row removed, a row duplicated, a key changed, a rank
  changed by one, a scheduler choice changed, a bound side flipped and the
  identity changed.
- Each allowed-to-survive entry SHALL name the member, the mutation and the
  reason the mutated certificate still establishes the claim (for example
  a value raised on an `upper` row that still satisfies its inequality and
  the threshold).

### Rejection evidence by false claims

- Each route SHALL have a corpus of false claims over small models.
- The truth of each corpus claim SHALL be computed by a definitional
  reference: test code that reads the checked package and enumerates the
  model's behaviours itself, and calls no function of `fsl-eval`, of
  producer code or of checker code.
- For every corpus claim, analysis through that route SHALL NOT settle
  `proved`, and the route's check entry SHALL NOT accept a certificate for
  it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1051-AC-1 | An arch-lint check over the workspace fails when checker code calls a crate, module or function outside the trusted base table and not in the exceptions table, naming the calling file and the called path; for example a checker module that calls `fsl_analyze` or an unlisted `fsl_eval::probabilistic` function. With every called path in the trusted base, the check passes. Verified by the arch-lint check's tests over fixture crates. | Test |
| FR-1051-AC-2 | The same check fails when a function outside the trusted base is called by both checker code and producer code, naming the function and one call site on each side. Verified by the arch-lint check's tests over fixture crates. | Test |
| FR-1051-AC-3 | The same check fails when producer code calls a checker-code function that is neither a check entry nor document codec, naming the function and the call site; a call to `check_probability_certificate_document` or `write_probability_certificate` passes. Verified by the arch-lint check's tests over fixture crates. | Test |
| FR-1051-AC-4 | The same check fails when a check entry's parameter is a producer-built value (`ModelSystem`, `TimedSystem`, `ClauseLetters`, `ModelLetters`, a product, a monitor or a measure plan), naming the entry and the parameter. Verified by the arch-lint check's tests over fixture crates. | Test |
| FR-1051-AC-5 | On main, the check's findings are exactly the entries of the exceptions table: a finding that matches no entry fails, and an entry that matches no finding fails as stale, naming the entry's ID. Verified by running the check on the repository. | Test |
| FR-1051-AC-6 | For every registered fixture of every route, every single mutation that is not on the route's allowed-to-survive list is refused or rejected by the route's check entry; a mutation the entry accepts fails the harness, naming the route, the fixture, the member path and the mutation. Verified by the harness's tests, including one where a deliberately weakened fixture checker accepts a changed value. | Test |
| FR-1051-AC-7 | An allowed-to-survive entry without a reason, or one whose mutation the harness no longer makes or the check entry now rejects, fails the harness as stale, naming the entry. Verified by the harness's tests. | Test |
| FR-1051-AC-8 | The harness fails, naming each route, when a route has no registered accepted fixture, or when a registered fixture is not accepted by its check entry. The first registered routes are the measure and quantile profile, the value-bounds profile, the fair MDP profile, zone `always possible` and `unique path`, and counterexample replay on the byte route. Verified by the harness's tests. | Test |
| FR-1051-AC-9 | A test fails, naming each route, when a route has no false-claim corpus entry. For every corpus entry the reference's truth value is false, analysis through that route does not settle `proved`, and the route's check entry does not accept the certificate a producer writes for it. Verified by the corpus tests. | Test |
| FR-1051-AC-10 | The corpus holds, each on its route, the false claims of the defects that reached `proved`: a dropped disjunct in an automaton translation (QSL-923), a nested timed deadline advanced wrongly on both sides (QSL-3324), a quantile claim false from a second initial state (QSL-3414), a region walk in steps of `1/m` (QSL-3444), an elapsed `<=` quantile true on digital clocks and false on dense time (QSL-3445), and an `always possible` claim true on whole zones and false on their valuations (QSL-3478). None settles `proved`. Verified by the corpus tests. | Test |
| FR-1051-AC-11 | The definitional reference calls no function of `fsl-eval`, `fsl-analyze`, `fsl-hybrid-reference` or checker code. Verified by the arch-lint check, which fails on a reference fixture that calls `ModelSystem`. | Test |

## Status

PLANNED. No criterion is met on main. The exceptions table records the
sharing on main as of this requirement; the arch-lint check, the
restructuring that removes X-1 and X-3 to X-16 (QSL-3501), the separate
timed monitor that removes X-2 (QSL-3324), the mutation harness
(QSL-3505) and the definitional reference with its corpus (QSL-3500) are
not yet built. Hand-written rejection tests in the checker test files
remain the rejection evidence until the harness replaces them; none of
them is removed when its checker is consolidated.

## Dependencies

- [FR-282](FR-282-check-analyze-certificates-in-the-qualified-core.md):
  the checkers, their check entries and their inputs.
- [FR-921](FR-921-check-a-timed-product-certificate-without-the-engine.md):
  item 7's no-shared-monitor rule, generalised here.
- [ADR-028](../decisions/ADR-028-exact-probabilistic-engine.md) CE-7 and
  [ADR-026](../decisions/ADR-026-dense-time.md) CF-6: the small trusted
  base of the EN-5 and zone checkers.
- [FR-284](FR-284-keep-the-qualified-core-separable-by-crate.md): the
  qualified-core crates the trusted base is drawn from.
