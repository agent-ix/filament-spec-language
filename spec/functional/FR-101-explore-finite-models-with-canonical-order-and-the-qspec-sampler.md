---
id: FR-101
title: "Explore finite models with canonical order, typed state keys and the QSpec sampler"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-201
    type: depends_on
---
# FR-101: Explore finite models with canonical order, typed state keys and the QSpec sampler

## Description

QSpec FR-181 is the normative source for finite simulation. This requirement
states how `qsl_eval::simulation` implements it. It adds no rule to FR-181;
where FR-181 leaves a choice open, this requirement names QSL's choice and
says so.

QSL implements these parts of FR-181:

- the exploration contract's canonical successor order;
- the simulator state key and its typed canonical form;
- seeded sampling with the `quire.simulation.sampler/v1` generator
  (QSpec `proposals/quire-v1/definitions/simulation-sampler.md`), selected
  by its identity;
- trace replay;
- the stopped outcomes: exhaustion of an exploration limit, and caller
  cancellation with cause `cancelled`/`caller-cancelled`;
- the `requires-bound` disposition returned before exploration begins.

A `TransitionSystem` implementation supplies the successor relation and
each expanded state's findings; the engine orders, keys, explores, samples
and replays it, and records the findings. The model-level successor
relation (operation × finite argument domains × frame post-states, QSpec
FR-013-AC-1), the invariant findings of FR-181-AC-4's last clause and the
effects of FR-181-AC-6 are
[FR-120](FR-120-simulate-a-checked-package-s-state-family.md)'s, through its
`ModelSystem` implementation of this trait. The trait's findings and
`ExpansionStop`, `Outcome::Stopped`, `StopReason::Stopped` and the replay
errors `FindingMismatch` and `Stopped` were added (AC-12 to
AC-14).

## Inputs

- A `TransitionSystem`: its initial states and, per expanded state, an
  `Expansion` (its successors, each a transition identity and a post-state
  in the typed canonical form of QSpec FR-181's exploration contract, and
  its findings) or an `ExpansionStop`.
- The simulation request's parameter-domain and population types, as
  `domains: &[(WireNodeId, &ValueType)]`: each type with the wire id of the
  checked node that carries it.
- The package's `TypeEnvironment` (`qsl_semantics`) and the extent walk's
  `position_limit: u64`, both passed to `classify_extent`.
- For exploration: `Limits{max_states, max_transitions}`, the horizon
  `max_depth: Option<usize>` and a cancellation poll `impl FnMut() -> bool`.
  `max_states` and `max_transitions` are caller-raisable resource budgets
  (ADR-014 B-5). `max_depth` is the search horizon `k`, a method parameter
  beside `Limits` and never a member of it: with `Some(k)`, states at depth
  `>= k` are not expanded, and a run that expands every reachable state
  below the horizon returns `Outcome::BoundReached` stating the horizon it
  used; `None` sets no horizon. `Limits::default()` publishes `max_states`
  10,000,000 (setting `explore.states`) and `max_transitions` 100,000,000
  (setting `explore.transitions`) (FR-255). A run that
  reaches `max_states` or `max_transitions` names it and its value in
  `Outcome::Bounded`, and the caller raises it by setting that `Limits`
  member.
- For `explore_request_parallel`, a worker count `workers: usize`, the
  number of threads that may expand states at once; 1 expands on the
  calling thread alone. It is a request setting named `explore.workers`,
  default 1 (FR-255 "Worker count setting names"). It is a resource
  choice, not a limit: no worker count stops a run, and it enters no
  result (see "Workers"). `explore_request` has no worker count and runs
  as `workers` 1 does.
- For sampling: a `u64` seed, a `u64` trace index, a `usize` step ceiling
  and the sampler's `DefinitionRef` (`qsl_foundation::selection`), whose
  identity names the generator.

## Outputs

- `Exploration<F>{outcome, findings}`, whose `outcome` is
  `Outcome::Exhaustive(Stats)`, `Outcome::BoundReached{stats, depth,
  frontier}`, `Outcome::Bounded{stats, frontier, limit}` (`limit` is
  `Limit::States` or `Limit::Transitions` with its value),
  `Outcome::Cancelled{stats, frontier, cause}` or
  `Outcome::Stopped{stats, frontier, cause}`.
- A sampled `Trace<T, F>` with its `SampleProvenance{seed, trace, sampler,
  stopped}` and its findings.
- `NotSimulated`, returned before any state is explored or sampled.
- A replay result: success, or the first step that does not replay.

## Behavior

**Entry points and owners.** Every type below is owned by `qsl-eval`
(`qsl_eval::simulation`) unless another owner is named. `qsl-eval` reaches
only the crates ADR-011 X-8 and TC-390 list; it gains no dependency on
`qsl-replay` or `qsl-cst`.

```rust
pub trait TransitionSystem {
    type State;
    type TransitionId: Clone + Eq + std::fmt::Debug + Serialize;
    type Key: Serialize;
    type Finding: Clone + Eq + std::fmt::Debug;

    fn initial(&self) -> Vec<Self::State>;
    fn key(&self, state: &Self::State) -> Self::Key;
    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop>;

    // Reduction hooks (ADR-021 EI-4; FR-162 states their behaviour; the
    // types are FR-151's, FR-152's, FR-155's, FR-158's and FR-159's).
    // `offered_reductions` says which hooks the system implements; the
    // pre-check reads it before any expansion.
    fn offered_reductions(&self) -> OfferedReductions { OfferedReductions::NONE }
    fn canonical(
        &self,
        state: &Self::State,
        group: &SymmetryGroup,
    ) -> Option<(Self::State, Permutation)> { None }
    fn footprint(&self, transition: &Self::TransitionId) -> Option<Footprint> { None }
    fn holds_constraint(
        &self,
        state: &Self::State,
        constraint: &CheckedStateClause,
    ) -> Option<Result<ConstraintValue, ExpansionStop>> { None }
}

pub struct Expansion<T, S, F> {
    pub successors: Vec<(T, S)>,
    pub findings: Vec<F>,
}

pub struct ExpansionStop {
    pub cause: CatalogCode,
}

pub struct Exploration<F> {
    pub outcome: Outcome,
    pub findings: Vec<StateFindings<F>>,
}

pub struct StateFindings<F> {
    pub state: StateKey,
    pub depth: usize,
    pub findings: Vec<F>,
}

pub fn explore_request<S: TransitionSystem>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    limits: Limits,
    max_depth: Option<usize>,
    poll: impl FnMut() -> bool,
) -> Result<Exploration<S::Finding>, NotSimulated>;

/// The same exploration with `workers` threads expanding states (see
/// "Workers"). `poll` is called from the calling thread only.
pub fn explore_request_parallel<S>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    limits: Limits,
    max_depth: Option<usize>,
    workers: usize,
    poll: impl FnMut() -> bool,
) -> Result<Exploration<S::Finding>, NotSimulated>
where
    S: TransitionSystem + Sync,
    S::State: Send + Sync,
    S::TransitionId: Send,
    S::Key: Send,
    S::Finding: Send;

pub fn sample_request<S: TransitionSystem>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
) -> Result<Trace<S::TransitionId, S::Finding>, NotSimulated>;

pub enum NotSimulated {
    /// The request's extent is unbounded (ADR-014 §4).
    RequiresBound(RequiresBound),
    /// `classify_extent` stopped: a node-count stage limit or an
    /// internal fault (`qsl_semantics::family::ClassifyFailure`).
    Extent(ClassifyFailure),
    /// `sampler` names a generator other than the one that runs.
    GeneratorMismatch { supplied: DefinitionRef },
    /// `system.initial()` returned no state to sample from.
    EmptyInitial,
    /// A state or transition identity reached during the walk has no RFC
    /// 8785 encoding under `quire-canonical`.
    KeyEncoding(EncodingRefusal),
    /// `workers` is 0 (see "Workers").
    Workers,
}

pub struct RequiresBound {
    /// Every unbounded domain, by key, with its kind
    /// (`qsl_semantics::family::UnboundedDomains`).
    pub domains: UnboundedDomains,
}

/// A `TransitionSystem::Key` or `TransitionId` has no RFC 8785 encoding: for
/// example an integer outside the exact-double range (`2^53`), a
/// non-finite float or a non-string map key. A key of any depth encodes
/// through the event API (FR-262).
pub struct EncodingRefusal(String);
```

`explore_request`, `explore_request_parallel` and `sample_request` are the
only public entries that explore or sample: `explore`, `sample` and the `Sampler` trait are
`pub(crate)`, so no caller skips the requires-bound check or the
`GeneratorMismatch` check. `replay` stays public; it explores nothing new.
Integration tests in `qsl-eval/tests/it/` call the two entries with a
bounded or empty `domains` set; a check that needs draw internals is an
in-crate unit test.

`explore_request` never returns `GeneratorMismatch` or `EmptyInitial`. Any
entry, including `replay` (`ReplayError::KeyEncoding`), can refuse a state or
transition identity reached mid-walk that has no RFC 8785 encoding: a
`TransitionSystem` is a public trait, and its `Key` and `TransitionId` are
implementer-supplied, not engine-built, so an encoding failure is a caller
defect the engine refuses rather than a broken internal invariant.

`RequiresBound` is not a refusal: ADR-014 §4 and QSpec FR-181 treat
requires-bound as a negotiation disposition the caller answers with a
bounded request, the same disposition `qsl-route`'s own
`Disposition::RequiresBound` carries with no catalog code
(`qsl-route/src/routing.rs:29-31`). `NotSimulated` gives `catalog_code` and
`catalog_fields` as inherent methods, not a `CatalogCoded` impl -- a
`CatalogCoded` cause is always `Category::Refusal`, which `RequiresBound`
is not -- and `catalog_code` returns `None` for `RequiresBound`. It is
`Some(invalid_runtime_input/invalid-value)` for `EmptyInitial`,
`KeyEncoding` and `Workers`, the same category ADR-014 TR-2 gives a malformed runtime
request; `GeneratorMismatch` reads the same code for a different reason
(FR-101-AC-10). `Extent` delegates to whichever of `classify_extent`'s own
two failure types stopped it: `LimitExceeded`'s own
`stage_limit_exceeded`/`<kind>-exceeded`, or `InternalFault`'s own
`runtime_invariant`/`established-invariant-broken`. No code is invented for
`NotSimulated` itself, and none for `RequiresBound`.

**Canonical order.** The engine orders successors itself. It does not keep
the order a `TransitionSystem` lists them in. Breadth-first exploration
keeps parent states in FIFO discovery order within a level. Each parent's
successors are visited in ascending unsigned-lexicographic order of the
RFC 8785 JCS UTF-8 bytes of the QSpec FR-181 transition identity.
FR-181 does not order two successors with equal transition identities and
different post-states. QSL orders them by the post-state's state-key bytes,
ascending. Initial states are admitted in ascending state-key byte order,
with equal keys coalesced into one state. Both are QSL's choices; QSpec
settles them upstream in FR-181.

**Initial states.** A `max_states` cap reached while admitting initial
states stops the run `Bounded` at `Limit::States` before any expansion. Its
frontier is the admitted initial states, then the one the cap refused, then
the rest, each in canonical order; `max_states` 0 admits none, and every
initial state is in the frontier. Exploring a system with no initial state
returns `Exhaustive` with zero states, transitions and depth: its reachable
set is empty and fully visited.

**State key.** QSL uses the typed canonical form specified by QSpec FR-181.
The state key is that form's canonical (RFC 8785) byte string; it is the
identity of a state in every output: the trace, the frontier, the findings,
a replay and a certificate. QSL coalesces states by equality of the complete
canonical key bytes, and by nothing weaker.

**Seen set and packed state.** The set of states already reached is an
index, not an identity, so how it finds a state is the engine's choice as
long as it never changes which states are equal. It may bucket keys by a
hash of an injective encoding of the key (the canonical key bytes, or the
packed form below), which keeps the lookup cost independent of key length,
provided a bucket hit is confirmed by comparing the full encoding: two
states with the same hash and different canonical bytes remain two states,
and equal encodings are equal canonical bytes. The hash
is not part of any output, is not stored in a trace, frontier, finding or
certificate, and does not decide any order; discovery order, the frontier
and the stats are functions of the canonical key bytes alone, so no
hash function, hash seed or table iteration order reaches a result.
Visiting and ordering stay as "Canonical order" states them.

The engine may also hold a state in a packed layout derived from the
checked types of the system's state (a fixed-width record of the finite
domains' ordinals and bounded integers, for example) to cut the cost of
keeping and comparing it. A packed layout is a bijection with the typed
canonical form on the values the checked types admit: packing then
unpacking returns the same state, unpacking a packed state yields exactly
the canonical key bytes that state has, and two states have equal packed
forms exactly when their canonical key bytes are equal. A state holding a
value outside the layout's domain is not packed and is indexed by its
canonical bytes. The packed form never leaves the engine: every output
names a state by its canonical key bytes, so a run with a packed layout and
one without return byte-equal results.

*Test seam.* Every explorer that keeps a seen set (`explore_request` and
`explore_request_parallel`, and the product explorers of FR-126 and FR-142)
exposes, under its crate's `test-support` feature and never in a request, a
`SeenSetProbe { hash: Option<fn(&[u8]) -> u64>, packing: Packing }` with
`Packing::{Auto, Off}`. `hash` replaces the index hash (a constant forces
every key to collide); `packing` `Off` keeps states unpacked; `Auto` is the
engine's default. The same feature exposes the layout as `PackedLayout`
with `pack(&State) -> Packed` and `unpack(&Packed) -> State`, for a system
whose checked types admit a layout. The probe changes no result: that is
what FR-101-AC-20 to AC-22 assert. The seam is named here so a code ticket
that implements a seen set has one documented hook to test through.

**Workers.** `workers` is the number of threads that may expand states at
once. It affects the time a run takes and nothing it returns. A run over
the same system, limits, horizon and cancellation outcome returns an equal
`Exploration` (its `outcome`, `stats`, `frontier` and `findings`, the
order of each included) for every `workers >= 1`, and the same
`NotSimulated` result. A worker count above the cores the process may use
is allowed and changes no result. `workers` 0 returns
`NotSimulated::Workers` before any `TransitionSystem` method is called.
Only `explore_request_parallel` takes `workers`, and its bounds
(`S: Sync`, `S::State: Send + Sync`, `S::TransitionId`, `S::Key` and
`S::Finding: Send`) are the only ones the engine asks for; `poll` is called
from the calling thread alone and needs no bound. A system that holds
single-threaded state (`RefCell`, `Rc`) does not meet them and is explored
through `explore_request`.

The engine produces results as if it expanded states one at a time in the
canonical order of "Canonical order": it merges each state's expansion in
that order, and the merge, not the completion of an expansion, assigns the
state its discovery position and charges the budgets. A worker may expand a
state ahead of the merge, but at most `workers` expansions are started and
not yet merged at any time, the one at the merge point included, so a run
stopped at a state has called `successors` on at most `workers - 1` states
beyond the stop and cannot spend unbounded work on states it discards; an expansion that the merge never reaches
(because an earlier state stopped the run) contributes nothing to the
stats, findings or frontier, and so does not count toward `max_states`,
`max_transitions` or the system's own limits such as FR-120's
`max_candidates`. Consequently `max_states`, `max_transitions`,
the horizon and the first `ExpansionStop` stop the run at the same state
whatever `workers` is: the stopped state is the first in canonical order
whose merge reaches the limit or whose expansion stops, not the first to
finish. Cancellation is the one cause that depends on timing, because it
depends on when the poll answers. A `Cancelled` outcome is the outcome the
one-worker run returns when the poll answers `true` after the merge of some
`n` states, for the `n` the run reached: its stats, findings and frontier
are those of a prefix of `n` merged states in canonical order, never a state
merged out of order.

The statistical sampling of FR-191 is the second use of workers, and states
its own rule.

**Traces and frontiers record state keys.** Every state a trace or frontier
names is its state key: `Trace.initial` and each `Step.key` are `StateKey`s, a
`Frontier` is a `Vec<StateKey>`, and `ReplayError::UnknownInitial.expected` is
a `StateKey`. Replay recomputes each candidate state's key to find the
recorded initial state. It picks each step's successor by transition identity
and then by ordinal among the successors that share it, with no recorded-key
comparison. The ordinal of a successor is its zero-based position among the
successors of that state that share its transition identity, in the system's
canonical successor order; each `Step` records it.

**Sampler.** Sampling uses QSpec's `quire.simulation.sampler/v1` definition
and the sampling contract in QSpec FR-181; that contract owns the draw
preimage and selection algorithm. `sample_request` first compares the
supplied `DefinitionRef`'s identity with `quire.simulation.sampler/v1`, and
refuses any other with `NotSimulated::GeneratorMismatch`, before any draw.
QSL advances its trace step once per transition taken, including a
single-successor step that requires no draw. A system with no enabled
successor ends the trace with `StopReason::NoSuccessors`; reaching
`max_steps` ends it with `StopReason::StepLimit`. When a
`TransitionSystem` has `m > 1` initial states after coalescing equal keys,
QSL starts trace `t` from the initial state at index `t mod m` in canonical
order, with no draw. This is QSL's choice.
Provenance records the seed, the trace index and the sampler identity
(ADR-014 TR-1). `CounterSampler` is removed.

**Horizon.** With `max_depth = Some(k)`, a run whose frontier holds only states
at depth `k` once every state below depth `k` is expanded returns
`BoundReached{stats, depth: k, frontier}`: it completed its method to the
horizon, `frontier` is those depth-`k` states in next-expansion order, and
it never reports `Exhaustive`. A run whose frontier empties before depth
`k` returns `Exhaustive`.

**Stopped outcomes.** A run that stops before its frontier empties never
reports `Exhaustive`. `Bounded` carries the resource limit that stopped it.
`Cancelled` carries `cause: CatalogCode` (`qsl_foundation::diagnostic`),
always `CatalogCode::new("cancelled", "caller-cancelled")`. Both carry the
unexplored frontier in the order the run would have expanded it next.
`Outcome::category()` is FR-097-AC-5's map.

**Findings and stopped expansions.** The merge uses one `successors` result per
expanded state; with `workers` above 1 the engine may also have called `successors` on up to `workers - 1` states beyond a stop (see "Workers"), and those calls reach no output. Each expanded state whose `Expansion` has at least one
finding contributes one `StateFindings` to `Exploration.findings`, in
expansion order, holding its state key, its depth and its findings in
the order the system returned them. The entries cover exactly the expanded
states.

An `Err(ExpansionStop)` ends exploration with `Outcome::Stopped{stats,
frontier, cause}`. `ExpansionStop` carries its cause alone; the
entries cover the states expanded before the stop. The frontier is the state whose expansion stopped, then
the queued states in next-expansion order. `Outcome::category()` gives
`Stopped` the ADR-013 O-16 category of `cause`'s code: incomplete for
`resource_exhausted`, internal failure for `runtime_invariant`.

Sampling expands every state on the trace, the last included, and records
each expanded state's findings in `Trace.findings`, with depth equal to the
state's step index (0 for the initial state). The last state's successors
are not drawn from. An `ExpansionStop` at any state of the trace, the last
included, ends the trace with `StopReason::Stopped(cause)` at that state.

Replay expands each state of the trace again. It succeeds when every
recomputed expansion matches the trace: the same findings, no stop before
the trace's last state, and, at the last state, a stop with the same cause
exactly when the trace ended `StopReason::Stopped(cause)`. It refuses
`ReplayError::FindingMismatch{step}` at the first state whose recomputed
findings differ from the recorded ones, and `ReplayError::Stopped{step,
recorded, replayed}` at the first state where the recomputed expansion
stops and the trace did not stop there, the trace stopped there and the
recomputed expansion does not, or both stop with different causes;
`recorded` and `replayed` are each that state's stop cause or none.

**Requires-bound.** Before any `TransitionSystem` method is called,
`explore_request` and `sample_request` classify `domains` with the ADR-014
§4 extent rule (`qsl_semantics::family::classify_extent(domains, types,
position_limit)`). `ClaimExtent::Unbounded(domains)` returns
`NotSimulated::RequiresBound`. A `ClassifyFailure` returns
`NotSimulated::Extent` with it, which is neither `RequiresBound` nor an
exploration. `requires-bound` is never an `Outcome` variant. No
exploration `Limits` value answers it, because an accounting limit is not a
proof bound (ADR-014 §1).

**Traceability.** Every simulation test in `qsl-eval/tests/it/` traces to
this requirement's ACs and to TC-453 to TC-455. None carries QSpec's
`TC-210` or `FR-181-AC-*` ids, which name different artifacts in this
repository (this repository's TC-210 is a witness-envelope case).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-101-AC-1 | Exploration visits each parent's successors in ascending JCS byte order of their QSpec FR-181 transition identities, whatever order the `TransitionSystem` lists them in and whatever their post-states' keys: a state listing `z` before `a` is expanded `a` first, and `step(10)` is visited before `step(9)`, because `{"arguments":[{"type":"integer","value":"10"}],…}` precedes `{"arguments":[{"type":"integer","value":"9"}],…}` bytewise. Successors with equal transition identities are visited in ascending state-key byte order. Parent states keep FIFO discovery order within a level, whatever their keys. | Test (TC-453) |
| FR-101-AC-2 | The state key is the JCS encoding of FR-181's typed canonical form, and exploration coalesces two states exactly when their full key bytes are equal. The state whose `semantic` member is `float64` bits `0000000000000000` and every map empty has its JCS key bytes as its key; with bits `8000000000000000` (negative zero) it has its own, different, key bytes, and the two remain two states. Two NaNs with different payloads are two states. Each key is a `StateKey` under `quire.simulation.state-key/v1`. | Test (TC-453) |
| FR-101-AC-3 | The sampler reproduces QSpec TC-210's vector, as amended with QSpec FR-181's `choice` preimage member: seed `424242`, trace `0`, choice `0`, `n = 5` draws indices `1, 3, 3, 4, 4` at steps 0 to 4,, each draw made by QSpec FR-181's sampler contract. Seed `424242`, trace `1`, `n = 5` draws `4, 4, 1, 3, 4`; seed `424242`, trace `0`, `n = 3` draws `1, 2, 0, 0, 1`. `n = 1` selects index 0 and makes no draw, and still advances the step: seed `424242`, trace `0`, with `n = 1` at steps 0 and 1 and `n = 5` at step 2, selects index `3` at step 2. `n = 0` ends the trace with `StopReason::NoSuccessors`. | Test (TC-454) |
| FR-101-AC-4 | Two sampled runs with equal seeds and trace indices over the same `TransitionSystem` produce identical traces, including provenance. The provenance records the seed, the trace index and the sampler identity; a different seed or trace index gives different provenance. With `m > 1` initial states after coalescing equal keys, trace `t` starts at canonical initial state `t mod m`, and no draw selects the start. | Test (TC-454) |
| FR-101-AC-5 | A trace records its initial state and each step's state as state keys and replays against the `TransitionSystem` that produced it without the simulator, by recomputed key; replay picks among successors sharing a transition identity by transition identity and then by ordinal, compares no recorded key, and refuses a trace whose initial key matches no initial state with `UnknownInitial` and a step naming a transition, or an ordinal, the current state does not offer with `MissingTransition`, at that step. | Test (TC-454) |
| FR-101-AC-6 | A run the poll cancels returns `Outcome::Cancelled` with `cause` `CatalogCode::new("cancelled", "caller-cancelled")`, category incomplete, and the unexplored frontier as state keys in next-expansion order. It never returns `Exhaustive` or `Bounded`. | Test (TC-455) |
| FR-101-AC-7 | A run that reaches `max_states` or `max_transitions` before its frontier empties returns `Outcome::Bounded` with that `Limit` and the unexplored frontier as state keys in next-expansion order, category incomplete. On the chain 0 → 1 → 2, `max_depth` 2 returns `Outcome::BoundReached{depth: 2}` with frontier `[<2>]`, category inconclusive, and `Limit` has no depth member. The same model returns `Exhaustive` once `max_states` is at least its reachable state count, `max_transitions` at least its transition count, and `max_depth` greater than its deepest state's depth. `Exhaustive` is returned only when the frontier is empty, and its `Stats` count distinct states, explored transitions and the deepest depth. | Test (TC-455) |
| FR-101-AC-8 | `explore_request` or `sample_request` over `domains` that include an unbounded domain under the ADR-014 §4 extent rule returns `NotSimulated::RequiresBound` naming each unbounded domain's key and kind, calls no `TransitionSystem` method, and returns no `Outcome`. A `classify_extent` stage limit returns `NotSimulated::Extent`. The same request with every domain bounded explores. Raising exploration `Limits` does not change a `RequiresBound` result. | Test (TC-455) |
| FR-101-AC-9 | Initial states are admitted in ascending state-key byte order, and equal keys coalesce into one state. A `max_states` cap reached during admission returns `Bounded` at `Limit::States` with frontier: admitted, then refused, then the rest, in canonical order; `max_states` 0 admits none and puts every initial state in the frontier. Exploring a system with no initial state returns `Exhaustive` with zero stats. | Test (TC-453) |
| FR-101-AC-10 | `sample_request` refuses before any draw with `NotSimulated::GeneratorMismatch` when the supplied `DefinitionRef`'s identity is not `quire.simulation.sampler/v1`, and with `NotSimulated::EmptyInitial` when the system has no initial state. A run that reaches `max_steps` stops with `StopReason::StepLimit`, whether or not the current state has successors. `GeneratorMismatch`'s catalog code is `invalid_runtime_input`/`invalid-value`. | Test (TC-454) |
| FR-101-AC-11 | A `TransitionSystem::Key` or `TransitionId` with no RFC 8785 encoding -- for example a `u64` above `2^53` -- refuses `NotSimulated::KeyEncoding` from `explore_request` and `sample_request`, and `ReplayError::KeyEncoding` from `replay`, instead of aborting the process. | Test (TC-453) |
| FR-101-AC-12 | On a test system `0 → {1, 2}`, `1 → 3`, whose expansion of `1` returns `ExpansionStop` with `resource_exhausted`/`insufficient-next-charge`, exploration returns `Outcome::Stopped` with that cause, frontier `[<1>, <2>]` (the stopped state, then the queue), category incomplete; with `runtime_invariant`/`established-invariant-broken` it returns `Stopped`, category internal failure. | Test (TC-474) |
| FR-101-AC-13 | On a test system whose expansion of state `s` returns finding `f`, `Exploration.findings` holds one `StateFindings` for `s` with its key, its depth and `[f]`, in expansion order; a system whose `s` sits in a `Bounded` frontier has no entry for it; the stopped state of AC-12 has no entry. | Test (TC-474) |
| FR-101-AC-14 | Sampling the chain `0 → 1`, whose expansion of `1` stops with `resource_exhausted`/`insufficient-next-charge`, ends with `StopReason::Stopped(resource_exhausted/insufficient-next-charge)` at step 1, and that trace replays successfully. The same trace replayed against a system whose expansion of `1` does not stop refuses `ReplayError::Stopped{step: 1, recorded: Some(<cause>), replayed: None}`; against one that stops with `runtime_invariant`, `recorded` and `replayed` name the two causes; a trace sampled with `max_steps` 1 from a chain whose `1` does not stop ends `StepLimit` at `1`, and replayed against the stopping chain it refuses with `recorded: None`. A trace whose recorded findings differ from the recomputed ones refuses `ReplayError::FindingMismatch` at that step. | Test (TC-474) |
| FR-101-AC-15 | `Limits::default()` is `max_states` 10,000,000 and `max_transitions` 100,000,000, and `Limits` has no depth member; `max_depth: None` sets no horizon. A chain of 3 states explored with `Limits::default()` returns `Exhaustive`; with `max_states` set to 2 and the other members at their defaults it returns `Outcome::Bounded` at `Limit::States` with value 2. | Test (TC-536) |
| FR-101-AC-16 | Over a test system with at least 200 reachable states and findings on some of them, `explore_request_parallel` with `workers` 1, 2, 4 and 64 returns `Exploration` values equal to each other and to `explore_request`'s: the same `Outcome` variant with equal `Stats`, the same frontier and the same `findings`, each in the same order. A system whose `successors` sleeps for a pseudo-random time per state, so that expansions finish out of order, returns the same result as the one-worker run. | Test (TC-1417) |
| FR-101-AC-17 | `workers` 0 returns `NotSimulated::Workers`, catalog code `invalid_runtime_input`/`invalid-value`, and calls no `TransitionSystem` method. A system that records every `successors` call, explored with `workers` 4 and a run stopped by `max_states` 5, has `Stats.states` 5, no `findings` entry for a state beyond the stop, a result equal to the one-worker run's, and a recorded call count of at most 5 plus `workers - 1` calls beyond the merged states. | Test (TC-1417) |
| FR-101-AC-18 | On a test system with a wide first level, a run stopped by `max_states`, by `max_transitions`, or by a system whose expansion of two different states returns `ExpansionStop` with different causes, returns for `workers` 1, 2, 4 and 64 the same `Bounded` limit and value, the same `Stopped` cause and frontier: the stopped state is the earlier of the two in canonical order, though the later one is made to finish first. | Test (TC-1417) |
| FR-101-AC-19 | A run whose poll answers `true` after a delay, with `workers` 4, returns `Outcome::Cancelled` whose stats, findings and frontier equal those of the one-worker run whose poll answers `true` after the merge of the same number `n` of states, for some `n` from 0 up to the reachable state count; no state outside the first `n` in canonical order appears in its stats, findings or frontier. | Test (TC-1417) |
| FR-101-AC-20 | For each explorer that keeps a seen set (`explore_request`, `explore_request_parallel` and the product explorers of FR-126 and FR-142), a `SeenSetProbe` whose `hash` returns one constant for every key returns a result equal to the default probe's, over a system of 100 distinct states and over two initial states whose keys differ and whose hashes are equal (both admitted): the same `Stats.states` 100, frontier, findings and, for the model checker, verdict and counterexample. Two other hash functions give the same result. Equal full key bytes still coalesce into one state. No `StateKey`, trace, frontier, finding or certificate carries the hash. | Test (TC-1417) |
| FR-101-AC-21 | For a system whose checked types admit a `PackedLayout`, `unpack(pack(s))` is `s` and its canonical key bytes equal those of `s`, for every reachable state of a 200-state system; two reachable states have equal packed forms exactly when their canonical key bytes are equal, checked over every pair, including `float64` zero against negative zero (two states) and an integer at each end of its declared range. A state that holds a value outside the layout's domain is explored, and coalesces with another exactly when their canonical key bytes are equal. | Test (TC-1417) |
| FR-101-AC-22 | For each explorer of AC-20, a run with `Packing::Auto` and one with `Packing::Off` return byte-equal results: every `StateKey` in the frontier, the trace, the findings and a replay is the canonical key bytes; `replay` of a trace from one run succeeds against the other; no output holds a packed form. | Test (TC-1417) |

FR-101-AC-5 withdrawn clause: the `KeyMismatch` refusal (recorded key compared with the recomputed successor key), retired because no QSL check could fail on it.

## Dependencies

- QSpec FR-181's exploration contract, typed canonical form and transition
  identity; QSpec TC-210's sampler vector, read in place from the QSpec
  checkout.
- QSpec `quire.simulation.sampler/v1`
  (`proposals/quire-v1/definitions/simulation-sampler.md`), which owns the
  sampler's draw contract.
- [ADR-014](../decisions/ADR-014-temporal-trace-and-boundedness-architecture.md)
  §1, §4, §7 and TR-1, TR-6, TR-7.
- [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  §2: `quire-canonical` is the one RFC 8785 encoder.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  X-8: `qsl-eval`'s `[dependencies]` gain `quire-canonical` and `serde` for
  the state key and the sampler preimage.
- [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md): the
  extent rule and `Outcome::category()`.
