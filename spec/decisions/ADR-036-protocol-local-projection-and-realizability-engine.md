---
id: ADR-036
title: "Protocol analysis: the local-projection and realizability engine"
type: ADR
relationships:
  - target: ix://agent-ix/filament-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-031
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-057
    type: relates_to
  - target: ix://agent-ix/filament-spec-language/FR-519
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-053
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-054
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-170
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-172
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-174
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-175
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-288
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-351
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-360
    type: depends_on
  - target: ix://agent-ix/filament-protocol/FR-302
    type: relates_to
---
# ADR-036: Protocol analysis: the local-projection and realizability engine

Status: Proposed (QSL-1889, split from QSL-1868 gaps G1 and G2). This record
decides the engine; it adds no requirement of its own. Slice 1 of the
decomposition under "Implementation slicing" writes the FSL requirements and test cases.

## Context

**What exists.** FSL has vocabulary and routing for the two capability kinds
and no decision procedure. `local-projection` and `realizability` are
`Capability` kinds of FR-290 family `protocol`
(`fsl-semantics/src/check/capability.rs`, `capability_kinds!`), have
`ClaimKind` and `same_kind` arms in `fsl-replay/src/planner/key.rs` and
`fsl-route/src/lib.rs`, and are listed among the protocol kinds in
`fsl-analyze/src/analyze.rs`. A request for either settles
`NegotiationCause::UnsupportedRequestedCapability(kind, backend)`, code
`unsupported_projection` / `unsupported-requested-capability` (FR-057-AC-15),
because no candidate serves it. `NegotiationCause` has exactly three variants
(`UnknownBackend`, `AmbiguousBackend`, `UnsupportedRequestedCapability`).

**What FSpec requires.** FSpec FR-174 projects a checked global protocol
onto selected roles, preserving observable sends and receives, causal order,
queues and obligations; unobservable choice or an incompatible merge yields a
minimal global-path counterexample, an exhausted bound or an unsupported
construct yields a non-conclusive result with no substitute graph (FR-174-AC-2,
AC-3), and a branch may merge only on compatible next actions or explicit
distinguishing evidence (FR-174-AC-5). FSpec FR-175 asks whether compliant
local strategies realize the global protocol: a game in which the environment
chooses admitted external events and channel outcomes and each role chooses
from its local history only; witnesses map classes of indistinguishable
histories to actions, counterexamples are two indistinguishable histories
needing incompatible actions or an environment play that defeats every choice
(FR-175-AC-1, AC-2, AC-4, AC-5); a missing premise or a limit is
`inconclusive`, never realizable (FR-175-AC-3). Both results are derived
artifacts with a structural key and no digest (FR-174-AC-6, FR-175-AC-6, as
merged by FSpec PR 650): FR-174's key is the FR-288 qualified definition key,
the role, the profile, the channel, choice and visibility premises as selected
(FR-054) and the bounds; FR-175's is the definition key, the strategy profile,
the role knowledge and visibility, the channel and environment premises, the
bounds and the producing analyzer identity and version. The key is carried on
the result; an absent member is refused
`invalid_runtime_input`/`missing-member` naming it; the analyzer is
deterministic for a fixed key.

**What filament-protocol needs.** filament-protocol ADR-010 "FSL gaps" lists G1 and
G2 as the two decision procedures it does not build and will not work around:
it dispatches both kinds to an FSL backend, binds the communication and
visibility premises to each disposition (its FR-030), carries the witness with
the FR-351 trace-position witness record its FR-302 imports byte-exactly (its
FR-152-AC-7 refuses a decisive realizability result without a trace position,
which is not a graph identity), and
returns `unsupported` while no backend exists. It imports the checked protocol
through the FR-519 read surface (`fsl_package::protocol_view`), which now
includes `choice_visibility` (FR-519-AC-20, the evidence of FSpec
FR-172-AC-6). Projection and realizability corpora in filament-protocol are
blocked on this engine.

**What the other engines settled.** ADR-018 (EN-1), ADR-024, ADR-026 and
ADR-028 (EN-5) put in-process engines in layer A (`fsl-analyze`) and their
verdict types, replay arm and settlement map in layer 6 (`fsl-replay`). ADR-027
fixes one protocol transition system, in `fsl-eval`, with the FR-170 queue
semantics. The owner's standing rule is one evaluator and no duplicate
semantics.

## Decision

### 1. Placement

| ID | Rule |
| --- | --- |
| PA-1 | **Engine location.** The engine is a new module `protocol_analysis` of the layer-A crate `fsl-analyze` (ADR-011 section 6.1), beside `model_check`. No new crate and no new layer edge: layer A already depends on layers 6, 5, 4, 3, R, F, C, E, SV and K, which is all the engine reads. It runs in stage S6c. |
| PA-2 | **Types in layer 6.** The key, bounds, premises, outcome, witness and counterexample types, the settlement map and the counterexample replay arm (PA-20) live in `fsl-replay` module `protocol_analysis`, as ADR-028 SP-2 places EN-5's types and ADR-018 LA-3 its replay arm. |
| PA-3 | **One graph reader.** The engine reads the checked protocol only through `fsl_package::protocol_view` (FR-519): `protocol_view`, `causal_relation`, `channel_endpoints`, `delivery`, `queue_capacity`, `choice_visibility`, so it names the same nodes as filament-protocol. Projection needs three more FR-054 members that the surface lacks today: the channel's ordering policy, its FIFO key expression and its overflow policy. Slice 2 adds those three accessors under FR-519 (an amendment filament-protocol also imports). The engine does not read `fsl-semantics` internals. |
| PA-4 | **Backend boundary.** The engine is one provider manifest advertising `local-projection` and `realizability`. It adds no `NegotiationCause` variant (FR-057-AC-15): negotiation still returns the three existing causes when no candidate serves a kind, and an unsupported construct found by a served engine is an engine result (PA-21). Results reach filament-protocol as the driver's disposition and witness envelope, so no FSL type enters a backend repository (ADR-011 FB-05). |

### 2. Inputs and premises

| ID | Rule |
| --- | --- |
| PA-5 | **Request.** A projection request is (definition key, role, profile, premises, bounds); a realizability request is (definition key, strategy profile, premises, bounds). The definition key is the FR-288 qualified key type the obligation activation binding uses (FSpec FR-479); the engine names the protocol by it and by no display name, path or span. |
| PA-6 | **Premise record.** `ProtocolPremises` has: (a) per channel, the five FR-054 members plus queue capacity and overflow policy; (b) per `choice` control, `owner` and `visible` (`choice_visibility`; FR-172 "Choice visibility carrier"); (c) role knowledge: declared shared facts a role may condition on beyond its own observations; (d) environment: the admitted external events and channel outcomes. Groups (a) and (b) are populated from the linked package through the PA-3 accessors. Groups (c) and (d) are realizability-only and are authored by the claim. A projection takes (a) and (b) alone, which are the premises of FR-174's key. |
| PA-7 | **How each premise acts.** Ordering: `fifo` orders deliveries within the pair of exact channel key and evaluated key value (FR-054-AC-2); `unordered` imposes none. Delivery: a send has one alternative per delivery count `d` in the policy's range, `d = 0` its only loss and `d > 1` its only duplication (FR-054, FR-170-AC-7). **V1 decides usefully only under `exactly-once-premise` or `[1,1]`.** Under a policy that admits loss, any receive-dependent obligation has a defeating environment play; under an open range (`at-least-once`, `unknown`) the counts are cut at the channel's capacity (its literal, or a symbolic capacity's bound value, FSpec FR-785) and the max_depth horizon, and a result that depended on a cut count is `inconclusive`. Projection does not depend on delivery counts. Capacity and overflow bound each queue (literal capacity only, PA-21). Choice visibility: a non-owner role distinguishes a choice's branches through earlier messages it sent or received that carry the whole decision (FR-053-AC-1: the checked package's `visible` holds the expression nodes of the decision facts, each reading the payload binder of one received message, so a role is told the outcome exactly when every message the facts read is one it sent or received), or through a divergent first receive (PA-10). Environment: realizability quantifies over exactly the admitted events and outcomes (FR-175-AC-5). |
| PA-8 | **No inferred premise; absent is not empty.** A premise the claim does not state is never taken from a default, a sample or replay history (FR-175 "Behavior"). Groups (c) and (d) are optional values: absent (`None`) makes a realizability result `inconclusive` (FR-175-AC-3); declared-empty (`Some` of an empty set, "no shared facts", "no external events") is a premise and the analysis decides under it. An absent *key member* is a different failure and is refused (PA-13). |

### 3. Projection (G1)

| ID | Rule |
| --- | --- |
| PA-9 | **Local graph.** From `causal_relation` build the global event structure. For role `r` keep the events `r` performs or owns, erase the rest, and keep the induced order: `r`'s event `a` precedes `b` when a causal path from `a` to `b` exists through any events. Every global communication yields exactly one local send and one local receive, and a total correspondence table maps each global event of `r` to its local event (FR-174-AC-4). `parallel ... join all` stays a local partial order: the engine never enumerates interleavings, and FIFO order per (channel, key) is kept as local receive order (FR-174 "Projection correctness"). |
| PA-10 | **Choice and merge.** Let `first(r, branch)` be the minimal events of `r`'s local graph in that branch, a set (several elements when the branch starts a `parallel` region or a `parallel` region precedes the choice's continuation). At a `choice`, `r` is the owner (local internal choice), or every message the decision facts read is one `r` sent or received (it observes the outcome), or the branches distinguish by receive: every branch's `first` holds a receive from one peer and the message types differ across branches (local external choice keyed by the message type). Otherwise branches merge only when `first` sets pair off by (direction, peer): a pair of sends merges on equal message type, a pair of receives on equal message type, and merged pairs recurse on their continuations under this rule. An empty `first` against a non-empty one, a send against a receive, or differing peers or message types for sends do not merge. A branch holding a choice is one run per alternative of each nested choice, and every run of one branch must merge with every run of the other: each nested alternative is compared against the other branch on its own, by plain merging on equal (direction, peer, message type) with no receive-type distinguishing between runs, so a protocol that relies on it is refused with a counterexample. More than 256 runs in a branch is reported as a horizon cut (`inconclusive`), and each comparison of two runs is charged to `max_transitions`. A merged choice's paired events are recorded in the local graph as alternatives. No branch is dropped and no graph is returned for a failure (FR-174-AC-2, AC-3, AC-5). |
| PA-11 | **Counterexample.** The counterexample names the choice, its owner, the role, two global paths from the choice to the first divergent action of `r`, and the two actions. Length is the sum of the two paths' event counts; the engine reports the smallest length and breaks ties by the lexicographic order of (choice node id, path_a node ids, path_b node ids), which also picks among several choices that fail to merge. The counterexample is therefore minimal and unique. |
| PA-12 | **Soundness is by construction.** V1 runs no composition check. Soundness of the projection is the construction of PA-9 and PA-10: the correspondence table is total and order-preserving, each local event maps to one global event, and a branch merges only under PA-10. A sound projection settles `proved` with certification `Uncertified` (ADR-011 S6c; ADR-018 PC-1: a first-party engine with no core checker) and basis `closed-scope` over the finite `bounded-static` profile. FR-174-AC-1 is shown by the test corpus (TC-205), which compares each projected graph's events and causal order with the global event structure through the correspondence table; this ADR adds no second transition system (PA-22). |

### 4. Identity, bounds and determinism

| ID | Rule |
| --- | --- |
| PA-13 | **Key.** `ProtocolAnalysisKey` is structural: the members of FR-174-AC-6 for a projection and of FR-175-AC-6 for a realizability result, each a typed value, none a digest. Construction is fallible (`new` returns the first absent member as `invalid_runtime_input`/`missing-member` naming it), nothing defaults a member, and the key is stored on the result as a structured value. A reader rebuilds and compares it and refuses an artifact with an absent member the same way. Display name, path, span, producing run and timestamp are not fields of the type. |
| PA-14 | **Bounds follow ADR-018 V-7.** `Bounds` has two key members: `max_depth` (the horizon in events along a path or play) and `max_history` (local history length, realizability). No default; a search that completes its method up to the horizon without a counterexample is `inconclusive` (FR-360), never `proved`. `max_states`, `max_transitions` and the evaluation meter are run limits as V-7 defines them, with published defaults, outside the key: a run that one of them stops before the method completes settles `failed` with execution `resource-incomplete`, carries no artifact, and states the limit and value used. |
| PA-15 | **Determinism.** One key yields one result. All collections are ordered by `CheckedNodeId` or an explicit canonical order, and no hash iteration order, randomness, clock or scheduler reaches the result; search ties break by canonical order. The analyzer identity is `fsl-analyze::protocol_analysis` with the crate version. For a projection that identity is not a key member (FR-174-AC-6), so the algorithm must be a function of the five members only: a change that alters any projection output is a requirement change, not an engine-version change. For realizability the identity and version are a key member (FR-175 "Derived-artifact identity"). The engine is single-threaded; the determinism test is two runs on one key (FR-175-AC-6). |

### 5. Realizability (G2)

| ID | Rule |
| --- | --- |
| PA-16 | **Game.** The arena is the global protocol's state graph under the one `fsl-eval` protocol transition system (ADR-027, FSpec FR-170), explored to the bounds. Two kinds of move: the environment's (channel outcomes per PA-7, admitted external events, the record a send carries and the order in which threads step) and each choice owner's branch selection. In the V1 `bounded-static` profile every move is the environment's: a `choice` decision reads only its owner's received records, protocol captures and constants (FR-218; a `send` binder is never a decision fact, and `attempt` and `event` are outside the profile, PA-21), and `fsl-eval` enters the one case whose guard holds, so each branch is fixed by earlier environment moves or the initial state. A role's observation is the records it sent and received, each with its control, the protocol captures every role of its instance holds (FR-218), as bound when the instance starts, and the group (c) facts; its own choice outcomes and the outcomes visible to it (PA-7) are functions of these. Two histories of one role are indistinguishable when their observation sequences are equal. The environment cannot make a role act: a role whose committed action is not enabled waits while others move (FSpec FR-175). A play wins when it satisfies the global safety, progress and terminal obligations (FR-175 "Strategy game"). Slice 4 adds two reads to `fsl-eval` and nothing else: a state's protocol capture values, per live instance (`capture_bytes`), and a step's binder record as FR-181 canonical bytes (`binder_record_bytes`); each step's role and control are read through its existing transition accessors; where `fsl-eval` refuses a construct (ADR-010 G7: an unordered channel with a head/tail loss selector, an unenumerable step binder) the result is `unsupported` carrying that `fsl-eval` cause. |
| PA-17 | **Evaluation.** Backward induction over the bounded acyclic arena: environment nodes win only if every environment move wins; owner nodes win if some branch wins. Per role and observation class the engine collects the set of winning actions at each history in the class. The class commits to an action in the intersection of those sets. In `bounded-static` an owner node has exactly one branch, so its value is that branch's value and the evaluation is the environment's conjunction throughout. |
| PA-18 | **Refutation is claimed only when forced.** (i) *Indistinguishable histories*: two histories of one role with equal observation sequences whose winning-action sets are disjoint, **and neither history can be avoided by an earlier owner choice** (an owner could steer play away from it, so refuting would claim too much; otherwise the result is `inconclusive`), giving the two required actions and the violated obligation (FR-175-AC-2). (ii) *Environment defeat*: the root is lost, i.e. the environment defeats **every** owner choice; the counterexample records, for each owner branch at the first lost owner node, a defeating environment play. A class whose winning sets are non-empty pairwise but have an empty common intersection, or a defeat that depends on a choice the exploration cut at a bound, is `inconclusive`, not `refuted`. In `bounded-static` no history is avoidable by an earlier owner choice and no owner choice is cut, so neither `inconclusive` case arises (FR-571-AC-6): (i) is claimed for two non-empty, disjoint winning-action sets whose disjointness rests on no cut action, only on forced plays: on both histories every environment step is forced (no table filters it) and every role step is taken where every move is that same step, and every move at either end is a step of the role; and (ii) records one defeating play, forced the same way. A role's table is not an owner's branch choice: it chooses among the role's own enabled actions and waits while its committed action is not enabled, so it can delay a step or reorder around it; a pair or defeat that is not forced is `inconclusive` (`unforced-refutation`). |
| PA-19 | **Strategy witness and its argument.** The witness is, per role, a table from observation class to the committed action, with one committed branch per owner class (none in `bounded-static`, where no owner branch is free), over every class reached, plus the closure record (plays explored, horizon, closed). FR-175-AC-4 holds because the engine computes every class with the single function `observation(role, history)` and commits one action per class from the intersection of PA-17, so a class whose members have no common winning action cannot be committed: it is refuted or inconclusive by PA-18. The test for AC-4 recomputes `observation` for every explored history and checks that equal sequences get equal actions. **Replay** (FR-175-AC-1) re-executes the table against the environment premises: it walks every environment play of the premises, selects each owner's and role's action from the table by recomputed class, and checks that the play wins and that every action is enabled; a role whose committed action is not enabled waits. Committing one action per class need not compose across classes (a commitment at one class can change which histories reach another), so the walk is a gate inside the engine, not test support: `proved` is returned only after the walk passes over the finished table, and a table that fails it, or whose exploration did not close, is `inconclusive`, never realizable. |

### 6. Settlement, trace position and honesty

| ID | Rule |
| --- | --- |
| PA-20 | **Settlement.** Both kinds settle with the FR-360 labels through the FR-241/242/243 combination FR-360 names, by the existing native-engine rule (ADR-011 S6c, ADR-018 SM-7, PC-1), with no rule of its own. Projection: a sound projection is `proved`/`closed-scope`, `Uncertified`; a PA-11 counterexample is `refuted`/`decisive-counterexample`; a search that reaches the horizon is `inconclusive`; PA-21 is `unsupported`; a run limit or engine failure is `failed`. Realizability: realizable with a closed exploration is `proved`/`closed-scope`, `Uncertified`; PA-18 is `refuted`/`decisive-counterexample`; absent group (c)/(d) premise or a cut is `inconclusive`; the rest as for projection. A refutation settles only after its counterexample replays (SM-7). The V1 replay arms live in `fsl-replay` module `protocol_analysis`: each re-walks the stated paths or moves against the checked graph and the premises, checks each step is an admitted successor and that the walk ends in the stated divergence or violation, and an arm that disagrees settles `inconclusive`. The projection arm is `ReplaySource::ProtocolAnalysis` over the checked package. The realizability arm has no replay source (a replay source carries no protocol subject): it walks the `fsl-eval` arena, each play naming its initial state and each step's successor index, refuses a play that passes a state with a finding or takes a role step at a state where another move exists, and for indistinguishable histories checks that the role is the sole mover at both ends and that no action is enabled at both (PA-18 (i), FSL FR-573). FR-360's Scope does not list these two kinds; slice 1 raises the Scope amendment with FSpec instead of assuming it. |
| PA-21 | **Unsupported, not guessed; amended by §7.** Under the profile `bounded` (PA-26) no construct of a checked protocol is answered `unsupported` for want of a rule: §7 projects and decides every construct below, and only the two `fsl-eval` limits of PA-33 remain. The rest of this row is the `bounded-static` profile, which a request still selects by its word and which keeps its answers. Under `bounded-static`, the engine answers `unsupported`, naming a located construct cause, for the following. `activation on each` (an instance per trigger, whose roles' steps in two instances would share one observation; FR-575 `activation-on-each`); dynamic role populations; symbolic queue capacity, bound or not (`fsl-eval` steps a bound one at the protocol subject's value, FSpec FR-785; filament-protocol G4); `join` with a `predicate` policy or an unruled `outstanding` (G3; fsl-eval refuses both); `join any` and `quorum`; `repeat`; compensation and timed awaits; no cause is stated for a role with an enabled local choice the protocol assigns to no owner (distributed synthesis with imperfect information, undecidable in general): FSpec FR-466 lets a `protocol` node own a choice, but the FSL checker refuses it (`NodeRole`), so no checked package holds one. `join any`, `quorum` and `repeat` are out because projecting them needs rules beyond the minimum (what a role observes when branches are cancelled; fixpoint merge over a loop-carried history), not because `fsl-eval` cannot step them. The cost of the fragment: **under `bounded-static`, a corpus cannot use retry loops or non-`all` joins under projection or realizability; under `bounded` it can (§7).** The `bounded-static` fragment is activation `on origin`, static roles, literal capacities, sequence, send, receive, choice and `parallel` with `join all`; a request naming that profile is answered within the fragment. A profile word other than `bounded` (PA-26) or `bounded-static` is refused at admission, as an unknown FR-054 ordering member is. |
| PA-22 | **No samples, no second evaluator.** The engine never concludes from replayed or sampled executions and builds no transition system of its own. |
| PA-23 | **No compatibility layer.** Nothing is kept for the pre-engine behaviour. When the engine is served, the unserved-kind assertions (the TC-155 negotiation tests for the two kinds) are rewritten for the served case in the same change; TC-1302 (kinds apply to protocol claims) is unchanged. |
| PA-24 | **Trace-position witness (FR-351).** Every `decisive-counterexample` result carries exactly one FR-351 separating-witness record, which filament-protocol FR-302 imports byte-exactly for realizability (its FR-152-AC-7 refuses a decisive realizability result without one). The record travels on FSpec FR-352 `native-run-result/2`, the document FR-302 imports; without it FR-302-AC-4 settles the claim `unsupported`. `deciding_element`: the canonical string of the `CheckedNodeId` (the canonical string of a `CheckedNodeId` is the RFC 8785 canonical text of its `CheckedNodeId` V2 wire object (`quire_canonical::to_vec` of the id); a string that does not read back as a node id and re-encode to the same bytes is refused) of the violated obligation (realizability) or the divergent action's global node (projection). `value_path`: the FR-207 runtime value path of that node under the assessed protocol subject's root. `trace_position`: the decimal zero-based index, in the counterexample's ordered event sequence (for (i) the longer of the two histories, for (ii) the defeating play of the first owner branch), of the step at which the divergence or violation occurs. `index` and `deciding_quantifier` are absent. FSL's `SeparatingWitnessRecord` (`fsl-replay/src/result.rs`; ADR-031 SW-5, SW-7) does not fit this today: `quantifier` is a required `OccurrenceKey`, `deciding_element` a kernel `Value`, and the wire (`RecordWire::of` in `result/wire.rs`) refuses a record with no `index`. **This ADR amends ADR-031 SW-5 and SW-7**: `quantifier` becomes `Option<OccurrenceKey>`, absent for a family with no quantifier occurrence; `deciding_element` becomes a sum of the kernel `Value` and a protocol element (the canonical string above); and the wire accepts an absent `index` where the family assigns none (FR-351). Slice 2 lands the amendment, with its ADR-031 text, and a state-clause record must still round-trip unchanged. A `closed-scope` result carries no record (filament-protocol FR-302). Slice 1 pins the `value_path` derivation against FR-207 and states it as a requirement, and slice 2 adds a path accessor to the FR-519 surface if the package lacks one. Node references in the artifact remain `CheckedNodeId`s of the linked graph; they are not the trace position. |
| PA-25 | **Artifact shape.** `ProjectionWitness { key, local_graph, correspondence, merge_evidence }`; `ProjectionCounterexample { key, choice, owner, role, path_a, path_b, action_a, action_b, separating_witness }`; `RealizabilityWitness { key, strategies, closure }`; `RealizabilityCounterexample` is `IndistinguishableHistories { key, role, history_a, history_b, action_a, action_b, obligation, separating_witness }` or `EnvironmentDefeat { key, defeats_per_branch, obligation, separating_witness }`. A `LocalGraph` holds typed local events (send, receive, internal choice, external choice, parallel region and join), their peer and message type, and causal edges. All are plain data in the existing result envelope and carry no hash. |

### 7. The `bounded` profile: every construct (amendment)

This section amends PA-21. It extends PA-9 to PA-19 to every construct a
checked protocol holds, using one idea: every point at which a protocol's
run can go two ways is a **decision point**, and PA-10's merge rule decides,
for every role, whether the role can tell the ways apart. A `choice` is the
decision point PA-10 was written for. The rules below name the others and
their owners. FSL FR-890 and FR-891 state the requirements; FSpec FR-735
states the standard's side.

| ID | Rule |
| --- | --- |
| PA-26 | **Profiles.** A request names one of two profile words. `bounded` covers every construct of a checked protocol. `bounded-static` is the fragment PA-21 lists and keeps PA-21's answers, so a consumer that asks for the fragment gets the fragment. Over a protocol inside the fragment, the two profiles give equal results in every member except the key's profile member (FR-572). Any other word is refused at admission (FR-575). |
| PA-27 | **Decision points and their owners.** Each decision point has a set of alternatives, each a continuation, and an owner, which is a role or the environment. **`choice`**: alternatives are the cases; owner its `by` role. **`repeat`**: at each guard read, alternatives are *continue* (the body, then the next guard read), *exit* (the repeat's continuation) and, with `max`, *exhausted*; owner its `by` role. A pre-test `repeat` reads its guard on entry; a post-test `repeat` (FSpec FR-729) runs its body once first. **`await`**: alternatives are *match* (the match event, then `then`) and *timeout* (the `timeout` control); owner the role that owns the match event (the `from` role of a `send`, the `to` role of a `receive`, the `by` role of an `attempt` or `event`, the attempt's role for an `effect`). The window and clock are carried on the local await. **`join` with policy `any`, `quorum(n)` or `predicate(f)`**: alternatives are the disposition sets at which the policy first holds, each with its cancelled or continuing branch remainders (`outstanding cancel` cuts a remainder, `outstanding continue` keeps it concurrent with the continuation); owner the environment, because thread scheduling decides which branches finish first. **Compensation activation**: alternatives are *activated* (the compensation region) and *not activated*; owner the role whose step binds the trigger record. **Commit cutoff**: at a compensation thread's retry point after its template's commit node can have occurred, alternatives are *retry* and *end*; owner the role of the commit node. **Acting instance** (PA-29): owner the environment. |
| PA-28 | **Local events.** The local graph (PA-25 `LocalGraph`) gains these typed events: internal actions (`attempt`, `effect`, `event`, `commit`, each kept in the graph of the role that owns it, with its record type), the local loop (owner, guard reads, maximum, `test`, body, exit, exhausted), the local await (window, clock, match, then, timeout), the join with its policy, its joined labels and its `outstanding` rule, the compensation region (registration at the forward effect, activation, the attempt loop of at most `attempts n` attempts with its retry relation, end), `activate` (the start of the role's protocol instance, carrying the trigger record), and `spawn` and `retire` (the start and end of a replicated role instance). A `check` and a `fence` belong to no role: each is erased from every local graph, the induced order through it is kept, and the correspondence table maps it to `global-only`. The correspondence table stays total: every global event maps to the local events of every role that performs or owns it, and every communication to one send and one receive (FR-174-AC-4). |
| PA-29 | **Replicated roles.** A replicated role class `r` (`role r each T from pop max n`) projects to one local graph, which each live instance runs over its own observation. An event `by r` is taken by one live instance (ADR-027 RR-2). Its acting instance is pinned by the thread's **session** with `r`: the instance that took the latest earlier event of `r` on the same thread path. The first event of `r` on a thread is a `receive`, whose taker is the instance that dequeues the message, so that instance learns it is the session instance from the receive itself. A thread whose first event of `r` is not a receive has no instance that can know it is to act, so the projection is refuted with a counterexample whose decision point is the acting-instance choice, naming the event, the class and two live instances that could each take it. A composition in which every event of `r` is taken by its session instance has traces included in the global traces (FSpec FR-174-AC-1), because pinning only removes interleavings that ADR-027 also admits. `spawn` starts an instance's local graph and `retire` ends it. Both are environment moves. A spawn is never enabled past the role's authored `max n` (ST-14), so the bound is a property of the arena, not a refusal. |
| PA-30 | **Activation on each.** Each protocol instance has its own threads, captures and queues (ADR-027 AE-1). Each role's local graph is therefore the projection of the protocol body, run once per instance and started by that instance's `activate` event, which every role of the instance observes with its trigger record. A role's observation is partitioned by instance ordinal, so steps of two instances are never one observation. Instances interact only through the shared model state that `attempt` steps read and write. Projection preserves each attempt as a local internal action, and realizability decides that interference over the global arena, which holds every instance. |
| PA-31 | **Symbolic capacity.** A local graph does not depend on queue capacity (PA-7), so projection writes the capacity's name into the channel premise and decides as for a literal capacity. Realizability reads a value for each named capacity from the protocol subject's capacity binding (FSpec FR-785) and records it in premise group (a); a name the binding leaves unbound is recorded as not stated, so the key member is present. With the value absent, the result is `inconclusive`, as for an absent group (c) or (d) premise (PA-8). With the value present, the arena is built under it, so the capacity is a stated premise of the key. |
| PA-32 | **Merging at every decision point.** PA-10 applies at every decision point of PA-27, with the following owner rules. When the owner is a role, that role takes the decision locally: an internal choice, or an external choice when its first events are distinguishing receives. When the owner is the environment, every role takes the external side. For a `repeat`, the guard's reads and the first sets are properties of the source, the same in every iteration, so one merge per guard read decides every iteration, and no fixpoint over a loop-carried history is needed. A role with no event in a `repeat` body merges *continue* with *exit* trivially, and the repeat is erased for it unless *exhausted* differs. A role with no event in a remainder that a `join` cancels is unaffected by the cancellation. The 256-run cap of PA-10 and the `max_transitions` charge apply to the alternatives of every decision point. |
| PA-33 | **Realizability over every construct.** The arena of PA-16 is the `fsl-eval` protocol system over every construct (FR-882), explored to the bounds and to `max_live_instances` (ADR-027 AE-4). The environment moves are those of PA-16 plus: `activate`, `spawn` and `retire`; the record and post-state of every `attempt`, `effect`, `event`, `commit` and compensation attempt; the step at which a `join` policy fires; and `timeout`, which the untimed arena admits whenever its `await` waits (ADR-027 "Waiting"). The role moves are the role's own enabled actions, as in PA-18, plus one owner choice: at a compensation thread's retry point, the template's role chooses `cattempt` or `cend` (ADR-027 CP-5). Because owner choices now exist, PA-17's OR nodes and PA-18's avoidability conditions apply as written, and a refutation that rests on an avoidable history or on a cut choice is `inconclusive`. A role's observation (PA-16) also holds its own internal actions with their records, its timeouts, and, for a replicated role instance, its own object reference, so a class's table may depend on the instance. A run that reaches `max_live_instances` with no counterexample settles `inconclusive`, cause `instance-bound-reached` naming that member. The only `unsupported` results under `bounded` are `evaluator-unsupported` for `fsl-eval`'s `UnenumerableRecord` and `WeakMemory` (FR-882), which name a limit of the request or of the selected system rather than a missing rule. |
| PA-34 | **Counterexample shape.** PA-11's counterexample generalizes from a `choice` to any decision point. `choice` holds the decision point's node: the `choice`, `repeat`, `await`, `parallel` of the join, compensation template, commit node, or the event whose acting instance is unpinned. `owner` is a role or `environment`. A new member, `decision`, holds the decision-point kind. The two paths, the role and the two actions are as before. The PA-11 measure and tie-break are unchanged, with the decision point's node id in place of the choice node id. |

## Worked example

Global: `Buyer -> Seller: order`, then `Seller` owns a choice `ship | cancel`
whose decision fact reads the payload of the `Buyer -> Seller: order` message, so `Buyer` and `Seller` are told the outcome. Branch `ship`: `Seller -> Warehouse: pick`;
branch `cancel`: nothing for `Warehouse`. Project onto `Warehouse` (neither
owner nor a party to the message the decision reads): `first` is `{receive pick}` against empty, which does not
merge, and no earlier message carries the outcome. Projection is `refuted`
(PA-11: the choice, owner `Seller`, the two paths, `receive pick` against end)
and realizability is `refuted` with `IndistinguishableHistories` for
`Warehouse`. Change the `cancel` branch to `Seller -> Warehouse: cancelled`:
both `first` sets hold one receive from `Seller` with distinct message types,
so `Warehouse` learns the branch from the receive (PA-10), and both results
are `proved`. A different repair is an earlier `Warehouse -> Seller: report` message
before the choice whose payload the choice's guards test (the decision reads
it, FR-053-AC-1), so `Warehouse` sent what Seller decides on; a
`Seller -> Warehouse: decision` message that no decision fact reads, of one
type in both branches, does not distinguish them. A decision that also reads
a message `Warehouse` took no part in does not inform it.

## Consequences

- G1 and G2 close in FSL; filament-protocol replaces its `unsupported` stub with
  dispatch and carrying, and TL-387 unblocks once slice 5 lands.
- The `bounded-static` profile decides the fragment PA-21 lists. The
  `bounded` profile (§7) decides every construct of a checked protocol:
  loops, every join policy, replicated roles, `activation on each`, symbolic
  capacity, timed awaits, compensation and the internal steps. Both are
  useful only under exactly-once or `[1,1]` delivery (PA-7); that cost is
  stated, not hidden.
- Every `proved` is `Uncertified`: no core checker exists for these engines.
  A checker for witnesses is a possible follow-up; FR-174 and FR-175 do not
  ask for it.
- The owner-assigned-choice rule makes realizability an AND/OR search over the
  global arena; richer strategy games need a new ADR.

## Open items for the requirement slice

1. FR-360 Scope must list local projection and realizability (FSpec change;
   FSL FR-573 "FR-360 Scope" states the wording, QSL-1898 carries it).
2. The V1 profile word and the unsupported-cause slugs are fixed by the FSL
   requirements, under the existing `unsupported_projection` code, with no new
   code family unless the FSpec catalog is amended.
3. The `EvidenceKind` the manifest advertises is an existing variant; slice 5
   chooses it from FR-290 and adds none. Settled by slice 5: FR-290 gives
   evidence kinds to `probabilistic-satisfaction` alone, so the manifest
   advertises none.
4. The FR-207 `value_path` derivation for a protocol graph node (PA-24).

## Implementation slicing

Minimum set, in order. Each ticket is one PR carrying code, tests and the
spec lines the code needs; slices 3 and 4 stay unserved (the existing
unsupported negotiation result) until slice 5.

| # | Title | Acceptance criteria | After |
| --- | --- | --- | --- |
| 1 | Specify protocol analysis behaviour: FSL FRs and TCs | Requirements only for what filament-protocol consumes: projection and realizability results, the key, the FR-360 settlement map with `Uncertified`, the FR-351 trace-position record (PA-24), the unsupported causes and the V1 profile; FR-174/175-AC-1 to AC-6 traced; FR-360 Scope amendment raised with FSpec; spec-only, validation clean | ADR |
| 2 | Add `ProtocolAnalysisKey`, `Bounds`, `ProtocolPremises` and outcome types to `fsl-replay`, the FR-519 accessors and the ADR-031 SW-5/SW-7 record amendment | Key members per FR-174-AC-6 / FR-175-AC-6; absent member refused `missing-member` naming it, by builder and reader; display name, path, span and run never participate; no digest; run limits kept out of the key (PA-14); FR-519 gains accessors for ordering policy, FIFO key expression and overflow policy (and a node value-path accessor if needed, PA-24); premises (a) and (b) built from `protocol_view`; `SeparatingWitnessRecord` takes an optional quantifier, a protocol deciding element and an absent index on the wire, state-clause records unchanged (PA-24) | 1 |
| 3 | Project a role: events, choice, merge, counterexample, unsupported and bounds | One send/receive pair per communication with a total correspondence table (FR-174-AC-4); induced order and FIFO receive order kept; PA-10 merge including the receive-distinguished and parallel `first`-set cases and the corrected worked example (FR-174-AC-2, AC-5); minimal unique counterexample by the PA-11 measure; every PA-21 construct returns `unsupported` with no substitute graph (FR-174-AC-3); horizon is `inconclusive`, run limit `failed`/`resource-incomplete`; `proved` is `Uncertified`; counterexample replay arm and FR-351 record (PA-20, PA-24); AC-1 by the TC-205 corpus | 2 |
| 4 | Decide realizability over the global arena | Observation function and classes; evaluation over environment moves (no owner move exists in `bounded-static`, PA-16) over FR-054 delivery alternatives and admitted events (FR-175-AC-5); winning-set intersection per class; witness with one committed action per class and its in-engine replay gate before `proved` (FR-175-AC-1, AC-4); both refutation forms only when forced, `inconclusive` otherwise (PA-18, FR-175-AC-2); absent group (c)/(d) premise is `inconclusive`, declared-empty decides (FR-175-AC-3); fsl-eval G7 causes surface as `unsupported`; FR-351 record on `refuted`; two-run determinism (FR-175-AC-6) | 3 |
| 5 | Serve both kinds: manifest, settlement and end-to-end driver path | Provider manifest advertises `local-projection` and `realizability`; the TC-155 unserved assertions for the two kinds are rewritten for the served case, `NegotiationCause` unchanged; FR-360 labels settle through the PA-20 map; a driver run settles one item of each kind and each label that can arise; handoff note to filament-protocol (TL-387) naming the result envelope, the carried key and the FR-351 record | 4 |

Deferred, not required by FR-174 or FR-175: an independent witness checker in
`fsl-replay` that would let `proved` become `Certified`, and any parallel
analysis engine with a thread-count determinism gate.

## Handoff to filament-protocol (TL-387)

Slice 5 serves both kinds. What filament-protocol reads:

- **Result envelope.** The `analyze` record (`fsl_analyze::analyze::AnalyzeRecord`)
  of an item built by `projection_item` or `realizability_item` and run on
  the linked engines (backend `protocol-analysis`): its FR-331 terminal
  record, settled by the FR-573 map, and `protocol_analysis`, the
  `ProtocolAnalysisArtifact` holding the result; `settlement()` gives the
  FR-360 label with its execution, truth and basis. `protocol_analysis` is
  absent for a refusal before analysis or an engine failure; a run stopped
  by a limit or the meter carries its result and key, with no witness or
  counterexample.
- **Carried key.** The result's `key`, a `ProtocolAnalysisKey` (FR-572),
  stored as a structured value and read back with `from_json`, which
  refuses an absent member `invalid_runtime_input`/`missing-member`.
- **FR-351 record.** The `separating_witness` of a `refuted` result's
  counterexample (FR-574), in the FR-352 record wire with a protocol
  `deciding_element`, no `quantifier` and no `index`.
- **Run-result record.** `ProtocolAnalysisArtifact::record` gives the
  result's FSpec FR-352 `native-run-result/2` protocol-analysis record (key,
  FR-360 label and axes, FR-331 settlement entry and, on a refutation, the
  FR-351 record), `ProtocolAnalysisRecord::to_bytes` writes it, and
  `read_native_run_result` reads it back (FR-574 Outputs). A refusal before
  analysis and an engine failure have no record.

## Alternatives Considered

- **A new crate `fsl-protocol-analysis`.** Rejected. ADR-011 section 7.2 lists
  the approved crate extractions; layer A already holds every engine and
  already has every edge this one needs.
- **Do the work in filament-protocol.** Rejected. filament-protocol ADR-010 consumes
  FSL's analysis and does no FSL-side work; two projection implementations
  would also drift.
- **General multi-role strategy synthesis in V1.** Rejected. It is undecidable
  with imperfect information in general, and no requirement asks for it;
  PA-21 answers `unsupported`.
- **A runtime composition check of the role-local graphs.** Rejected for V1.
  `fsl-eval` steps a global clause, nothing steps a composition of local
  graphs, and a second system is forbidden (PA-22); soundness is by
  construction (PA-12).
- **Treat an exhausted horizon as `unsupported`.** Rejected. FR-360 gives an
  analysis that completes to a depth without a counterexample the label
  `inconclusive`.
- **A hash of the artifact as its identity.** Rejected by FR-174-AC-6 and
  FR-175-AC-6.
- **Sample executions to approximate the environment.** Rejected by FR-175
  "Behavior" and FR-175-AC-5.

## References

- FSpec FR-053, FR-054 (AC-2, AC-4), FR-170 (AC-4, AC-7), FR-172, FR-174
  (AC-1 to AC-6), FR-175 (AC-1 to AC-6), FR-207, FR-241 to FR-243, FR-288,
  FR-351, FR-360, FR-479.
- FSL FR-057-AC-15, FR-519 (AC-20), ADR-011 section 6.1, S6c, FB-05, ADR-018
  V-7, SM-7, PC-1, LA-3, ADR-027, ADR-028 SP-2.
- filament-protocol ADR-010 "FSL gaps" G1 to G3, G7 (fsl-eval); FR-030, FR-152,
  FR-302.
- QSL-1868, QSL-1889.
