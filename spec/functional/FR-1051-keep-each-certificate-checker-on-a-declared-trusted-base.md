---
id: FR-1051
title: "Keep each certificate checker on a declared trusted base"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-282
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-921
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-170
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-284
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-038
    type: depends_on
---
# FR-1051: Keep each certificate checker on a declared trusted base

## Description

A certificate checker (FR-282) accepts a `proved` only when its own code
establishes the claim from the certificate. A function that the checker and
the producer of the certificate both call is trusted without being checked:
a defect in it writes a wrong certificate that the checker then accepts. This
requirement names the code a checker may rely on (its trusted base), forbids
every other sharing between producer and checker, and requires two kinds of
evidence that a checker rejects what it should: single mutations of accepted
certificates, and false claims whose truth a definitional reference computes.

This requirement narrows ADR-028 CE-7 and ADR-026 CF-6. CE-7 lets the EN-5
checker share the EN-1 product construction; here that sharing is the
recorded exception S-1, to be removed. CF-6 makes only the zone checker's
DBM operations separate code; here every function outside the trusted base
is separate. It generalises FR-921 item 7 (no monitor shared by the
timed-product producer and checker) to every checker of FR-282's table and
to the replay checkers below, including FR-170's trap replay. FR-098's
recompilation is a trusted-base service.

### Derived members and the one-type rule

The project rule "one type per interface, and each derived field from one
function both sides call" applies here to data types, and to functions used
on one side only. A certificate document's data types are one type shared by
its writer and its checker. A function that derives a member value (a key, a
rank, an objective, a fairness record, a value, an identity) SHALL NOT be
called by both a producer and a checker: the checker SHALL re-derive every
derived member it compares, with its own code. Sharing the function would
make the comparison agree with itself.

### Where the item lists live

This requirement states the rules, the classes of code, the trusted base by
crate and module, the exception rows (one per sharing) and the routes. The
item-level lists (each check entry, codec function, trusted-base service
item and exception item) are the arch-lint check's Rust data, and each entry
there names the row, route or codec class of this requirement it belongs to.
This requirement does not repeat them.

## Behavior

### Classes of code

Every non-test Rust source file of the workspace SHALL be in exactly one
class below. Test code is code under `#[cfg(test)]`, `#[cfg(kani)]` or
`#[cfg(any(test, kani))]`, the files reached only through it, `tests/`,
`benches/` and `examples/` directories, and the crates of the Tools row.
Code under `#[cfg(feature = "test-support")]` is not test code. Paths are
relative to `fsl-replay/src` unless they name a crate.

| Class | Members |
| --- | --- |
| Checker code | `probability_certificate.rs`, `probability_certificate/**`; `certificate.rs`, `certificate/**`; `hybrid_replay.rs`, `hybrid_replay/**`; `model_check/replay.rs`, `model_check/replay/**`, `model_check/race_replay.rs`, `timed/explore.rs`; `model_check/graph_replay.rs`, `model_check/graph_hyper.rs`; `model_check/hyper_replay.rs`, `model_check/finite_hyper_replay.rs`; `refinement.rs`, `refinement/**`, `model_check/refinement_replay.rs`; `contract_check.rs`; the method `RefinementQueryRequest::check_smt_proof` in `refinement_query.rs`; the function `replay_task_product_trace` and the type `TaskProductTraceReplayRefusal` in `outcome/counterexample.rs`; the crate `tools/fsl-probability-check` |
| Settling code | `model_check/settle.rs`, `model_check/graph_settle.rs`, `model_check/hyper_settle.rs`, `model_check/existential_settle.rs`, `model_check/refinement_settle.rs`, `model_check/step_settle.rs`, `model_check/step_safety.rs`, `hybrid.rs`, `proof_result.rs`, `proof_result/**` |
| Trusted-base crates | the workspace crates `fsl-foundation`, `fsl-cst`, `fsl-source`, `fsl-forms`, `fsl-semantics`, `fsl-package`, `fsl-route`, `fsl-walk-grow` and `fsl-eval` (what checker code may name of `fsl-eval` is set by the Trusted base table) |
| Out of scope | `statistical/**`, `protocol_analysis/**` |
| Producer code | the crates `fsl-analyze`, `fsl-hybrid-reference`, `fsl-command`, `fsl-inspect`, `fsl-conformance`, `fsl-jit`, `tools/fsl-hybrid-test-provider` and the root crate `filament-spec-language`; and every file of `fsl-replay/src` in no other class |
| Tools | `xtask`, `tools/arch-lint`, `fsl-string-edge`, `fsl-proptest-policy`, `fsl-bench`, `tests/fixtures/**`, `fuzz`, the mutation harness crate and the definitional reference crate |

Classes are by file, except the two items of checker code inside the producer files `refinement_query.rs` and `outcome/counterexample.rs`. The Trusted base table says what checker code may name; it does not class files, so a file of `fsl-replay` whose type items are trusted base (rows B-11, B-12) is still producer or out-of-scope code for its functions.

- The arch-lint check SHALL fail, naming the crate or file, when a workspace
  member crate, or a non-test file of a crate whose class is given by file,
  is in no class or in two.
- Settling code settles an engine record on a check entry's verdict; it is
  neither checker nor producer code.
- Three paths settle a proof with no FR-282 checker. `protocol_analysis/settle.rs`
  settles a protocol-analysis `Proved` as `proved`, `Uncertified`, and the
  execute bounded shadow (`execute/composite_parity.rs`) settles
  `VerifiedShadowResult::Proved`, each with no checker. The operation-contract
  arm also settles `proved`, `Uncertified`, from its case enumeration with
  no certificate. The require-certified policy (QSL-3503) is what keeps these
  from a caller who asks for certified results; this requirement adds no
  checker for them.

### Names and detection

The arch-lint check is a rule over module paths, built on the existing
`syn`-based api-surface machinery (FR-060). It SHALL find what code names
from these paths only:

- `use` trees, including globs, `self`, nested groups and `as` aliases, and
  `extern crate` aliases;
- qualified paths in expressions, types, trait bounds, struct literals and
  patterns, including associated-function paths `T::f`;
- path token runs written inside macro arguments, as the api-surface rule
  matches them;
- a bare path bound by a `use` leaf or glob import, and a path through a `type` alias, each naming the item it stands for;

each resolved through the `pub use` re-exports of the workspace crates to
the item it names. The check SHALL scan every non-test file with every
feature's code included; `#[cfg(feature = "test-support")]` code is
scanned, not skipped.

The check does not see calls made on values, and this requirement does not
claim it does: method calls `x.m(..)`, operators, `Deref`, the `From`
conversion of `?`, formatting traits, calls through trait objects and
generic bounds, macro bodies and expansions, and code a build script
generates. Sharing that happens only through such calls is covered by the
mutation harness, the false-claim corpus and review, not by the check. The
same misreading written twice in separate code (design mechanism B) is
caught only by the false-claim corpus.

### Trusted base

Checker code SHALL name, outside checker code, only items of this table or
of an exception row. A row whose extent is "whole crate" allows every item
of the crate; "module" allows every public item of the module and its
submodules except the parts the listed exception rows name; "type items"
allows the module's structs, enums, type aliases and traits and only the
functions the check's data records under that row, each a service or codec
function that derives no member value. Text in parentheses describes a row
and does not narrow it. That a function recorded under B-9, B-11 or B-12
derives no member value is checked by the review of the change that records
it, not by the arch-lint check; B-10 admits data items only, which the check
tests (AC-8).

| ID | Part | Crates and modules | Extent |
| --- | --- | --- | --- |
| B-1 | Front end and checked package | `fsl_foundation`, `fsl_cst`, `fsl_source`, `fsl_forms`, `fsl_semantics`, `fsl_package`, `filament_checked`, `filament_contract_model`, `filament_semantic_value` | whole crate |
| B-2 | Data and support crates | `filament_eval`, `filament_walk`, `filament_observation`, `fsl_route`, `fsl_walk_grow`, `thiserror`, `ix_trace_rs`, `std` | whole crate |
| B-3 | Canonical codec | `quire_canonical`, `serde`, `serde_json`, `serde_path_to_error`, `base64`, `sha2` | whole crate |
| B-4 | Exact arithmetic | `filament_exact`, `num_bigint`, `num_integer`, `num_traits` | whole crate |
| B-5 | Model stepping and model keys | `fsl_eval::simulation` (transition systems, state keys, expansion and limits) | module, except S-5 and S-10 |
| B-6 | Model stepping, frames and model keys | `fsl_eval::simulation::model` (`ModelSystem`, `TimedSystem`, their steps, guards, invariants, urgency and frame judgement, predicate reading, model and reference key encoding) | module, except S-5 and S-8 |
| B-7 | Protocol stepping | `fsl_eval::simulation::protocol` (protocol stepping, memory models, capacities and bounds) | module, except S-9 |
| B-8 | Predicate reading | `fsl_eval::model_subject` | module |
| B-9 | Position automaton | the position automaton of `fsl_eval::automaton` (`position_automaton` and the types it returns and reads: clocked edges, reads, stamps, window claims, atoms, letters, state ids, automaton errors) | the items the check's data records under B-9 |
| B-10 | Declared route table and locked-run mapping | `fsl-eval`: the declaration of the route admission table (its rows and the names of the conditions each row tests, FSpec FR-421 "Closed and strict conditions") and of the mapping from a claim monitor kind and reading to the value of a locked run (FSpec FR-416 "How claims read a time-lock"), as data only. No function that tests a condition or reads a subject | data items only (constants, statics, type items) |
| B-11 | Replay services | `request`, `bounds`, `limits`, `identity`, `execute`, `spine`, `application`, `model_check::identity`, `model_check::limits` | type items |
| B-12 | Replay codecs and result data | `reference_codec`, `product_action`, `outcome`, `outcome::counterexample`, `result`, `result::wire`, `proof_result`, `witness`, `task_transition`, `exact_probabilistic`, `hybrid`, `protocol_analysis`, `model_check::counterexample`, `model_check::hyper_counterexample`, `model_check::graph_settle`, `model_check::outcome`, `model_check::state_graph`, `model_check::item` | type items |

Named exception (B-10): the producer and the checker read one declared route
table and one locked-run value mapping. It is permanent and is not one of
the recorded temporary exceptions. A wrong row or entry is a defect both
sides carry; each side checks every row and entry against hand-written
expectations (FR-921-AC-5). B-10's items are not on main until FSL PR #4042's
code (QSL-3494) lands; until then the check's data records B-10 as planned,
with no items.

The parts above are shared with producer code by design. A defect in them is
not caught by mutation; the false-claim corpus is the evidence against them.
`fsl_eval` has no other part in the base: `automaton` outside B-9,
`probabilistic`, `fairness`, `hyper`, `smt`, `temporal`, `value`,
`execution`, `bare` and `policy_budget` are outside it.

### No other shared semantics

- Outside the trusted base, no function or method SHALL be named both by
  checker code and by producer code.
- A function defined in checker code is judged only by the producer-call rule; the shared-semantics rule covers functions defined outside checker code.
- Producer code SHALL name checker code only through a check entry, a codec
  function or a type item of checker code.
- A codec function SHALL belong to one codec class: C-1 a reader or writer
  between a typed value and its canonical bytes; C-2 a builder that
  assembles a document from members its caller already computed; C-3 an
  accessor that returns a member, or a reference to one, without computing
  it; C-4 a constructor of limits, meters or refusal codes. A function that
  derives a member value is in no codec class. A codec function may be
  named by both checker code and producer code: the one-type rule keeps it
  shared, and neither rule above counts it. That a codec function derives no member value is checked by the review of the change that records it.
- A check entry is a public function of checker code that returns a
  checker's verdict on a certificate, witness or counterexample, and SHALL
  belong to at least one route.

### No producer objects

- A check entry SHALL take as evidence only the certificate (bytes or typed
  document), the checked package or byte provision, the claim and subject
  binding, the checker's limits and `&Cancel`.
- A check entry SHALL NOT take, as a parameter, as `self`, or through a field
  of a checker-code type it takes, a value of a producer-built type: a
  `ModelSystem`, `TimedSystem`, `ProtocolSystem`, `TaskProductSystem`,
  `AbstractProtocolSystem`, `RefinementSystems` or `FiniteTracePopulation`;
  a `dyn` object or generic implementation of `ModelLetters`,
  `TemporalLetters`, `HyperLetters`, `TupleLetters`, `StepMatch` or
  `ModelEvaluator`; a closure, function pointer or `dyn Fn*`; or a type an
  exception row S-1 to S-9 concerns. The checker SHALL build its own from the
  checked package.

### Recorded exceptions

The sharing below exists on main and violates the rules above. Each row is
allowed only until its ticket removes it; the check's data records its items,
each entry naming the row. The change that removes a sharing SHALL delete
the row's entries, and the row when none is left, in the same change. No
other exception is allowed.

| ID | Rule broken | Code it concerns | Sharing | Removed by |
| --- | --- | --- | --- | --- |
| S-1 | Shared semantics | `fsl_eval::automaton` outside B-9; `model_check::item`'s translations | ADR-028 CE-7's shared EN-1 product: property-automaton, safety, bounded and qualified bounded monitor translations and their key contexts, used by the closure, SMT, zone, product-closure, hyper-replay and EN-5 checkers and by `fsl-analyze`; and the per-instance translations of a model-check item, built by the same code for both. Removing this row amends CE-7 (AC-15) | QSL-3501 |
| S-2 | Shared semantics | `fsl_eval::probabilistic` (`advance_nested_timed_deadline`) | The nested timed deadline step of `fsl-analyze` digital stepping and the timed-product checker; FR-921 item 7 already forbids it | QSL-3324 |
| S-3 | Shared semantics | `fsl_eval::probabilistic`, `fsl_eval::probabilistic::key` (everything outside S-2) | The measure and monitor step: measure plans, payoffs, accumulators, monitor and deadline state, measure and reward keys | QSL-3501 |
| S-4 | Shared semantics | `fsl_eval::fairness` | The fairness graph analysis, used by the zone, SCC, refinement and counterexample-replay checkers and by `fsl-analyze` | QSL-3501 |
| S-5 | Shared semantics | `fsl_eval::hyper`; the copy symmetry of `fsl_eval::simulation` (`reduction.rs`); `fsl_eval::simulation::model`'s checked hyper letters (`hyper_letters.rs`) | The hyperproperty product, closed-execution search and step matching, and copy symmetry reduction, used by the product-closure and hyper-replay checkers and by `fsl-analyze` hyper search | QSL-3501 |
| S-6 | Shared semantics | `fsl_eval::temporal`, `fsl_eval::value` | The SM-1 formula evaluator and model-relation evaluation: `fsl-analyze` hyper search decides with the tuple and projected-prefix evaluators that hyper replay and the match-witness check use, and counterexample replay evaluates its formula with it. Predicate reading is B-8, not this row | QSL-3501 |
| S-7 | Shared semantics | `fsl_eval::smt`; `refinement_query.rs` (the refinement certificate request and step rows) | The SMT query encoders, used by the query producers and re-run by the SMT proof checker; and the refinement certificate request and step-row count, built by the same code for the encoder and `check_smt_proof` | QSL-3501 |
| S-8 | Shared semantics | `fsl_eval::simulation::model`'s task lowering and task product (`task_automaton.rs`, `task_key.rs`, `task_product.rs`) | Task lowering and the task product, built by the same code for `fsl-analyze` task search and the task-product checker and replay | QSL-3501 |
| S-9 | Shared semantics | `fsl_eval::simulation::protocol`'s fairness and loop proof (`fairness.rs`, the loop-proof types of `protocol.rs`, `activates_on_each`) | Protocol fairness constraints and loop proof, used by counterexample replay, the closure checker, refinement and `fsl-analyze` | QSL-3501 |
| S-10 | Shared semantics | `fsl_eval::simulation`'s zero progress (`progress.rs`) | The zero-progress fair-loop analysis of the closure checker and `fsl-analyze` model checking | QSL-3501 |
| S-11 | Shared semantics | `model_check/digital_state_graph.rs` | Digital state-graph exploration, called by `fsl-analyze` state-graph analysis and by graph replay | QSL-3501 |
| S-12 | Shared semantics | `product_action.rs` | The canonical schedule and free-delay action keys, derived by the same functions for `fsl-analyze` exact analysis and the EN-5 checker | QSL-3501 |
| S-13 | Shared semantics | `model_check/counterexample.rs` | Counterexample positions and loop (the trace position, the closure loop, step-site pointers) and the choice of the first undefined finding, derived by the same functions for the producer's writers and for counterexample replay | QSL-3501 |
| P-1 | Producer call into checker | `probability_certificate/document/measure.rs` | The checker's own objective plan, measure key and measure control, called by `fsl-analyze` exact measure and undefined-witness code | QSL-3501 |
| P-2 | Producer call into checker | `probability_certificate.rs`, `probability_certificate/key_admission.rs`, `probability_certificate/document/timed.rs` | Product-key encoding (model-state, formula-state, digital and region keys, every budget and limit form), digital clock decoding and key accounting, called by `fsl-analyze` exact, candidate and digital analysis | QSL-3501 |
| P-3 | Producer call into checker | `probability_certificate/document.rs`, `probability_certificate/document/{measure,mdp,partial,long_run}.rs`, `probability_certificate/witness.rs` | Writers that derive ranks, the objective, coverage, the bound ranking, the fair objective, fairness records, cyclic-MDP ranks and policy, partial ranks, long-run keys, optimisation and weight objective, or a subsystem's base objective | QSL-3501 |
| P-4 | Producer call into checker | `probability_certificate/document/instances.rs` | The on-each producer lives in the checker file and shares its helpers with the on-each checker | QSL-3501 |
| P-5 | Producer call into checker | `probability_certificate/bounded.rs`, `probability_certificate/cyclic.rs`, `probability_certificate/document.rs`, `probability_certificate.rs` | The monitor context, the subject admission (acyclicity, reward subject) and the claim identity and binding derivation, called by `fsl-analyze` before it writes | QSL-3501 |
| P-6 | Producer call into checker | `refinement/**`, `refinement.rs` | The refinement step judge, subject admission and system opening, the abstract protocol system (stepping, keys, fairness), the protocol step view and the refinement liveness check and abstract classes, called by `fsl-analyze` refinement, contract and protocol analysis | QSL-3501 |
| P-7 | Producer call into checker | `model_check/replay.rs` | The protocol trace evaluator of counterexample replay, called by `fsl-analyze` finite protocol replay to decide | QSL-3501 |
| P-8 | Producer call into checker | `certificate/closed_form/task_set.rs` | Closed-form subject admission and priority order, used by `fsl-analyze` schedulability analysis | QSL-3501 |
| P-9 | Producer call into checker | `certificate/task_product.rs`, `certificate/task_product/**` | Task-product key order, transition identity, queue bounds and closure method, derived by checker functions for `fsl-analyze` task search | QSL-3501 |
| P-10 | Producer call into checker | `certificate/product.rs` | The product-closure writer (fairness components, canonical closure), its key and the package-hyper subject binding, called by `fsl-analyze` hyper search | QSL-3501 |
| P-11 | Producer call into checker | `hybrid_replay/**` | The hybrid checker's model reader, interval machine, box operations, flowpipe rule probe and class admission, used by `fsl-hybrid-reference` to build and probe boxes | QSL-3501 |
| P-12 | Producer call into checker | `contract_check.rs` | The operation-contract arm in `fsl-analyze` selects the claim, builds the target, enumerates declared and initial cases, evaluates each case and writes and reads its counterexample with the code its replay uses | QSL-3501 |
| P-13 | Producer call into checker | `certificate/zone.rs` | Zone construction and DBM closure (`Zone::zero`, `Zone::from_matrix`), used by the zone producers to write certificates | QSL-3501 |
| O-1 | Producer object | `probability_certificate/**` | The checked-subject form of the EN-5 checker and the probability witness replays take the producer's `ModelSystem` and compare it by pointer | QSL-3501 |
| O-2 | Producer object | `model_check/replay.rs`, `certificate/zone_certificate.rs` | Counterexample replay and the zone checker read atoms through the producer's letters, and the reference-literal and timed replays step the producer's system | QSL-3501 |
| O-3 | Producer object | `outcome/counterexample.rs` | Task-product trace replay checks keys of the producer's own product object | QSL-3501 |
| O-4 | Producer object | `model_check/refinement_replay.rs` | Refinement replay steps the systems the producer opened | QSL-3501 |
| O-5 | Producer object | `model_check/hyper_replay.rs`, `model_check/finite_hyper_replay.rs`, `certificate/product.rs` | Hyper trace and match-witness replay take the producer's systems, letters and step matcher; model-relation replay takes its systems and evaluator; finite hyper replay decides with the producer's population and body closure; the package-hyper and model-relation closure requests carry the producer's evaluator | QSL-3501 |
| O-6 | Producer object | `contract_check.rs` | The operation-contract replay reads the `ModelSystem` the producer opened through `self` | QSL-3501 |

### Routes

A route is a checker profile or form together with the abstraction it checks
under. Each check entry in the check's data names the routes it serves.
"Reference" says whether the definitional reference's fragment covers the
route; a route it does not yet cover is tracked by QSL-3500.

| ID | Route | Reference |
| --- | --- | --- |
| R-1 to R-16 | EN-5 profiles `reachability/v1`, `state-predicate/v1`, `expected-reward/v1`, `dtmc-expected-reward/v1`, `cyclic-dtmc-eventual/v1`, `cyclic-mdp-eventual/v1`, `cyclic-dtmc-state-predicate/v1`, `cyclic-mdp-state-predicate/v1`, `cyclic-mdp-expected-reward/v1`, `bounded-fair-eventual/v1`, `cyclic-mdp-fair-eventual/v1`, `finite-horizon-measure/v1`, `value-bounds/v1`, `on-each-reachability/v1`, `dtmc-long-run/v1`, `dtmc-weighted-long-run/v1`, in that order | covered, except R-10, R-11 (fairness) and R-14 (on-each instances): not yet |
| R-17, R-18 | EN-5 `timed-product/v1`, digital route; region route | covered |
| R-19 to R-21 | Probability witness; undefined witness; on-each witness | covered, except R-21: not yet |
| R-22 to R-31 | Zone: clause safety and TT-2; deadlock freedom; time-lock freedom; refinement; `possible`; `always possible`; `unique path`; liveness; free delay; task product | covered, except R-25 (refinement): not yet |
| R-32 | Explicit state graph | covered |
| R-33 to R-37 | EN-1 closure; SCC components; components over the stutter closure; protocol closure; protocol components | covered, except R-36, R-37 (protocols): not yet |
| R-38 to R-41 | Product closure; model-relation closure; package hyper closure; hyper match witness | covered, except R-39 (model relations): not yet |
| R-42, R-43 | SMT proof; refinement SMT proof | covered, except R-43 (refinement): not yet |
| R-44 | Simulation relation | not yet (refinement) |
| R-45 | Closed form | covered |
| R-46 to R-48 | Hybrid certificate; hybrid exact replay; hybrid static replay | covered |
| R-49 to R-53 | Model trace replay, source route; byte route; protocol trace; timed counterexample; task-product trace | covered, except R-51 (protocols): not yet |
| R-54 | State-graph evidence and trap replay | covered |
| R-55 to R-57 | Hyper trace tuple; model relation; finite hyper refutation | covered, except R-56 (model relations): not yet |
| R-58 to R-61 | Refinement trace; timed refinement trace; free-delay refinement trace; protocol assumption witness | not yet (refinement, protocols) |
| R-62 | Operation-contract counterexample | not yet (contract claims) |

### Rejection evidence by mutation

- Each route SHALL register at least one accepted certificate fixture with a
  mutation harness. The harness is a Rust test-support crate that is only a
  dev-dependency, has no `fsl-replay` dependency, and takes each
  registration's check as a closure from the fixture to a verdict.
- A route whose certificate has a canonical byte form, except R-27 and R-28,
  SHALL register its
  canonical document; the harness mutates the canonical tree, and the
  closure either calls the route's byte or source check entry (for EN-5, the
  source or package entry) or decodes the bytes with the route's codec reader
  and calls its typed entry (the task-product routes, through their codec
  reader and check entry). R-27 and R-28 use typed adapters over
  `ZoneCertificate` calling `check_zone_certificate`, since the zone wire
  reader does not read the `state_graph` member.
- A route whose certificate has only a typed form (state graph, EN-1
  closure, SCC components, closed form, simulation, SMT proof) SHALL register
  a typed adapter that applies each mutation kind to the typed value's
  members and returns the mutated value, and a closure that calls the
  route's typed check entry. The harness SHALL treat both kinds alike.
- A replay route SHALL register an accepted witness or counterexample; the
  harness mutates it the same way, and a mutation the replay still
  reproduces fails unless it is on the allowed-to-survive list.
- For every registered fixture, every single mutation the harness makes
  SHALL be refused or rejected by the route's check entry, except a mutation
  on the route's allowed-to-survive list.
- The mutations SHALL include, wherever the certificate has such a member: a
  value changed, a row removed, a row duplicated, a key changed, a rank
  changed by one, a scheduler choice changed, a bound side flipped and the
  identity changed.
- Each allowed-to-survive entry SHALL name the member, the mutation and the
  reason the mutated certificate still establishes the claim (for example a
  value raised on an `upper` row that still satisfies its inequality and the
  threshold).
- Until a route has a harness registration, its hand-written rejection tests
  (tests that assert a check entry of the route rejects or refuses a changed
  certificate) SHALL NOT be removed.

### Rejection evidence by false claims

- Each route SHALL have at least one corpus entry. For a proof route, the
  entry is a false claim together with a self-consistent certificate for it
  (committed, or built by the test so that it meets the checker's structural
  rules), and the semantic rejection rule it expects (a `CertificateRule` or
  route rule naming an obligation, not a decoding, identity or shape
  refusal); the route's check entry SHALL reject it with that rule, and
  analysis through that route SHALL NOT settle the claim `proved`. For a
  witness or counterexample route, the entry is a true claim together with a
  forged witness or counterexample for it, which the replay SHALL NOT
  reproduce.
- For a route the Routes table marks covered, the truth of each entry's
  claim SHALL be computed by the definitional reference. For a route marked
  not yet, the entry SHALL carry its truth value derived by hand, with the
  derivation written in the test, until QSL-3500 extends the reference to it.
  A replay-route entry's true claim may instead be established by a
  hand-derived argument written in the test (for example a reachable-graph
  and cycle enumeration by hand on the tiny model); the reference need not
  decide an unbounded universal claim true.
- The definitional reference is test code in its own crate, a dev-dependency
  only, that depends directly on `fsl-package`, `fsl-semantics`,
  `fsl-foundation` and the data crates `filament-exact`, `filament-checked`,
  `filament-contract-model` and `filament-semantic-value`, and on no other
  crate of this workspace (never `fsl-eval`, `fsl-analyze`, `fsl-replay` or
  `fsl-hybrid-reference`).
- The reference SHALL read the model from the `CheckedPackage`'s
  `CheckedGraph` (operation bodies and effects, the timed model's guards,
  resets and invariants, probabilistic supports, task sets, hybrid modes)
  with its own interpreter. When package checking needs an evaluator port,
  such as `RewardEvaluator`, the reference SHALL pass its own
  implementation, never one from `fsl-eval`.
- The reference SHALL accept only this fragment, and SHALL refuse, naming the
  construct, a corpus model or claim outside it:
  - state: finitely many fields of Boolean, enumeration or integer type with
    declared bounds, at most 2^16 valuations, in one model or a fixed set of
    instances;
  - operations: conjunctive guards of comparisons between a field and a
    literal or another field; effects that assign a literal, a field, or a
    field plus or minus a literal; `next`-state frames;
  - probabilistic choice: a finite support of literal rational weights;
    rewards and weights as literal rationals per operation or state;
  - clocks: rational valuations, guards and invariants that are conjunctions
    of `x ⋈ c` and `x - y ⋈ c` with integer `c`, and resets to zero; dense or
    digital time as the route declares;
  - task sets: periodic or sporadic tasks with integer periods, deadlines and
    worst-case execution times under the declared scheduling policy;
  - hybrid modes: constant rational rates, linear guards and invariants,
    resets to literals;
  - claims: Boolean combinations of field comparisons; `next`, `until`,
    `always`, `eventually` and their bounded windows; two-copy hyper formulas
    of these; probability, expected-reward, long-run and quantile bounds over
    every or some scheduler; `always possible`, `unique path`, deadlock
    freedom and time-lock freedom; task deadlines over a hyperperiod.
- The reference SHALL decide truth definitionally:
  - a claim bounded by a horizon, by enumerating every path to the horizon;
  - an unbounded probability, reward or long-run value over a finite chain,
    by its own exact rational solution of the defining equations;
  - a claim over every scheduler that is false, by one scheduler (with its
    delays where timed) that the corpus entry gives and the reference
    evaluates;
  - an unbounded reachability or expected total reward claim over some
    scheduler, by enumerating every memoryless deterministic scheduler;
  - a horizon-bounded reachability or reward claim over some scheduler, by
    enumerating every deterministic scheduler that depends on the state and
    the number of steps taken; any other claim over some scheduler it
    SHALL refuse;
  - a timed claim that needs every continuation from a state, by exploring
    its own Alur-Dill region quotient from that concrete state.

The corpus seeds are the false claims that reached `proved`. Each needs this
part of the fragment:

| Seed | Route | Model fragment | Claim and falsity witness |
| --- | --- | --- | --- |
| QSL-923 | R-38 product closure, two copies | finite fields, untimed, no probability | `forall a forall b { (not holds(l@a=1)) and eventually holds(l@a=1) }`; witness the lasso `step(0)` forever from `(0,0)` |
| QSL-3324 | R-17 timed product, digital | one clock, digital time, a probabilistic race with a 3-unit delay | `always[0,0] eventually[2,2] P` from a not-P state; witness the scheduler taking the 3-unit race delay |
| QSL-3414 | R-13 value bounds | finite fields, untimed, acyclic probabilistic choice, two initial states | a `>= c` quantile with initial value 0 and a reachable tight activation; exact value by path enumeration from the second initial state |
| QSL-3444 | R-18 timed product, region | two clocks `x`, `y`, dense time, strict guards | `eventually[0 ms, 4 ms] holds(c.late)` with probability `<= 0.999` over every scheduler; witness delays `1/2` ms then `6/5` ms, value 1 |
| QSL-3445 | R-17 timed product, digital | two clocks, dense time, closed guards, a 1:1 draw | elapsed quantile `0.4 ... <= 1 ms` over every scheduler; witness `first` at `1/2` ms, value 0 |
| QSL-3478 | R-27 `always possible`, R-28 `unique path` | two clocks, dense time | `always possible Done { c.phase >= 2 }` and `unique path Done from (c.phase = 1) to (c.phase >= 2)`; witness `first` at `1/2` ms, then the region quotient shows no run reaches phase 2 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-1051-AC-1 | Over fixture crates, the arch-lint check fails when checker code names an item outside the trusted base and in no exception row, naming the file, line and the named item (for example a checker module that names `fsl_analyze::...` or `fsl_eval::probabilistic::MeasurePlan`), and passes when every named item is in the trusted base. | Test |
| FR-1051-AC-2 | Over fixture crates, the check fails when a function defined outside checker code and outside the trusted base is named both by checker code and by producer code, naming the item and one site on each side, including when the producer names it by an associated-function path `T::f`; with the function named on one side only, or when it is a codec function, it reports nothing for it. | Test |
| FR-1051-AC-3 | Over fixture crates, the check fails when producer code names a function or method of checker code that is neither a check entry nor a codec function, naming the item and the site; naming `check_probability_certificate_document`, `write_probability_certificate` or a checker type item passes. | Test |
| FR-1051-AC-4 | Over fixture crates, the check fails when a check entry takes a producer-built value of "No producer objects" as a parameter, as `self` or through a field of a checker type it takes, including a closure parameter, naming the entry, the parameter or field and the type. | Test |
| FR-1051-AC-5 | Over fixture crates, the check reports a sharing whose only path is in code under `#[cfg(feature = "test-support")]`, one written inside a macro argument, one reached through a glob import and one through an `as` alias or a `pub use` re-export. | Test |
| FR-1051-AC-6 | Over fixture crates, the check fails, naming the crate or file, when a workspace member crate or a non-test file of `fsl-replay` is in no class of "Classes of code" or in two. | Test |
| FR-1051-AC-7 | On the repository, every finding of the check matches an entry of the check's data that names an exception row and the rule it breaks; a finding that matches no entry fails, naming the item and site. An exception entry that matches no finding fails as stale, naming the entry and its row. | Test |
| FR-1051-AC-8 | The check fails, naming the entry, when an entry of its data names an item that does not exist (an entry recorded as planned, such as B-10 before QSL-3494, is exempt until its item exists), names a row, route or codec class that is not in this requirement's tables, names an item defined outside the modules its row concerns (for an O row, a check entry defined outside them), or breaks a rule other than its row's; naming the row, when an exception row of this requirement has no entry and is not recorded as planned; and, naming the item, when an item recorded under B-10 is a function or method. | Test |
| FR-1051-AC-9 | A spec-consistency test reads this requirement's "Classes of code", Trusted base, exception, Routes and "No producer objects" text and fails, naming the difference, when the check's class paths, base rows with their extents, exception row IDs with their modules and rules, route IDs or producer-built type list differ from them. | Test |
| FR-1051-AC-10 | The check fails, naming the entry, when a check entry of its data names no route, or a route of the Routes table has no check entry; and, naming the function, when producer code or settling code names a public checker-code function that is in neither its check-entry nor its codec data. | Test |
| FR-1051-AC-11 | For every registered fixture of every route, every single mutation that is not on the route's allowed-to-survive list is refused or rejected by the route's check entry (or not reproduced by a replay route); a mutation the entry accepts fails the harness, naming the route, the fixture, the member path and the mutation. Verified by the harness's own tests, including one where a deliberately weakened check closure accepts a changed value, run once over a canonical document and once through a typed adapter. | Test |
| FR-1051-AC-12 | An allowed-to-survive entry without a reason, or one whose mutation the harness no longer makes or the check entry now rejects, fails the harness as stale, naming the entry. | Test |
| FR-1051-AC-13 | The harness fails, naming each route, when a route of the Routes table has no registered accepted fixture, or when a registered fixture is not accepted by its check entry. The harness crate's manifest has no `fsl-replay` dependency and it is a dev-dependency only. The first registered routes are R-12 (finite-horizon measure), R-13 (value bounds), R-11 (cyclic-MDP fair), R-27 and R-28 (zone `always possible` and `unique path`, through typed adapters over `ZoneCertificate` calling `check_zone_certificate`, since the zone wire reader does not read `state_graph`) and R-50 (counterexample replay, byte route); R-32 (explicit state graph) registers through a typed adapter. | Test |
| FR-1051-AC-14 | A repository test holds, for each route without a harness registration, the names of its hand-written rejection tests as of this requirement. It fails, naming the route and the test, when a named test is no longer defined; it fails as stale, naming the route, when a route that has a harness registration still has a list. | Test |
| FR-1051-AC-15 | The spec-consistency test fails, naming ADR-028 CE-7, when the check's data holds no entry for row S-1 and the text of CE-7 still says the product construction is shared with EN-1. | Test |
| FR-1051-AC-16 | A test fails, naming each route, when a route of the Routes table has no corpus entry. For each proof-route entry, the route's check entry rejects the self-consistent certificate with the semantic rule the entry states, and the test fails on a rejection by any other rule or by a decoding, identity or shape refusal; analysis does not settle the claim `proved`. For each replay-route entry, the forged witness or counterexample is not reproduced. A covered route's entry has its truth from the reference or, for a replay route, a hand-derived argument; a not-yet route's entry carries a hand-derived value with its derivation. | Test |
| FR-1051-AC-17 | The corpus holds the six seeds of the seed table, each on its route with the model and witness given there; for each, the reference computes false, the check entry rejects the self-consistent certificate with the semantic rule its corpus entry states, and analysis does not settle `proved`. | Test |
| FR-1051-AC-18 | The reference crate's manifest has no direct dependency on a workspace crate other than `fsl-package`, `fsl-semantics` and `fsl-foundation`, and is a dev-dependency only; the arch-lint check fails on a reference fixture that names `fsl_eval::simulation::model::ModelSystem` or an `fsl-eval` path to a `RewardEvaluator` implementation. | Test |
| FR-1051-AC-19 | The reference refuses, naming the construct, a corpus model with an unbounded integer field, a guard that is a disjunction, a non-literal probability weight, a clock compared with a non-integer constant, or a hybrid mode with a non-constant rate, and a quantile claim over some scheduler; it returns no truth value for it. On a two-step MDP where the optimal choice at the first state depends on the steps left, its value for a horizon-bounded some-scheduler reachability claim equals the hand-derived optimum. | Test |

## Status

PLANNED. No criterion is met on main. The exception rows record the sharing
on origin/main as of this requirement. The arch-lint check and its data, the
restructuring that removes S-1 and S-3 to S-13, P-1 to P-13 and O-1 to O-6
(QSL-3501), the separate timed monitor that removes S-2 (QSL-3324), the
shared admission data of B-10 (QSL-3494, after FSL PR #4042), the mutation
harness (QSL-3505) and the definitional reference with its corpus (QSL-3500)
are not yet built.

## Dependencies

The frontmatter relationships are the links: FR-282 (the checkers, their
entries and inputs), FR-921 (item 7, generalised here), FR-170 (trap replay,
in checker code), FR-098 (recompilation, a trusted-base service), FR-284
(the qualified-core crates the base is drawn from), and ADR-028 CE-7 and
ADR-026 CF-6 (narrowed here).
