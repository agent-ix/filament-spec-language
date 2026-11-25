---
id: FR-163
title: "Check a hyper item's product-closure certificate in the qualified core"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-021
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-174
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-177
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-178
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-179
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-181
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-182
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: references
---
# FR-163: Check a hyper item's product-closure certificate in the qualified core

## Description

When EN-1 returns `Holds` for an HP-1, HP-2, HP-3 or HP-6 item, or
`Witnessed` for an HP-5 item (FR-181), the layer-A engine in `fsl-analyze` SHALL hand over a
`ProductClosureCertificate`, and FSL's layer-6 crate `fsl-replay` SHALL hold
`check_product_closure`, which accepts the certificate only when the
certified set holds every initial product state, is closed under the
recomputed successors, holds no undefined, refused or incomplete letter or
match, and holds no violation by the form's rule (ADR-023 HX-7). The
certificate reuses ADR-018's closure certificate (PC-3) and component
certificate (PC-4). The checker rebuilds the clause from the package the
item was analyzed through (FR-940 "Package form of a hyper clause": the
source route's own emission or the selected package, never source on the
package route), recomputes successors through that route's evaluator and
reads nothing from the engine's search but the certificate, so a certified hyper `proved`
does not depend on the engine (ADR-029 CB-2, RU-2).

## Use case

A security lead wants a noninterference proof that counts as qualified
evidence. The model checker proves the secure vault leaks nothing and hands
over the product states it explored with their component witnesses. A small
checker in the qualified core rebuilds the self-composed product from the
model, confirms the set is closed and holds no fair accepting cycle, and
accepts it, so the proof stands without trusting the model checker's
search.

## Inputs

- FR-098's request, with each alias's subject: initial snapshots in the
  byte provision and the alias's universes; the request's
  `ModelCheckLimits`.
- The certificate, serialized with RFC 8785 JCS:

```rust
pub struct ProductClosureCertificate {
    pub form: HyperForm,                   // HP-1, HP-2, HP-3, HP-5 or HP-6
    pub instance: Vec<(Name, Key)>,        // object parameter binding
    pub reduction: Option<CopySwap>,       // when FR-174 applied copy-swap
    pub closure: ClosureCertificate,       // HP-1: subject closures below;
                                           // otherwise ADR-018 PC-3 product keys,
                                           // sorted ascending, no duplicates
    pub components: Option<Vec<Component>>, // ADR-018 PC-4, for HP-2 and HP-3
}
```

For HP-1, `closure` SHALL be a sequence of subject closures, each with
exactly `alias`, `states` and `executions`: the checked model alias, its
ascending, duplicate-free sequence of actual
`filament.simulation.state-key/v1` keys, and its execution edge sequences.
The sequence SHALL contain each alias used by the relation once, in model
alias declaration order in the checked unit, rather than first execution
use order. Each `executions` entry SHALL contain exactly `execution` and
`transitions`: the execution variable's name and its complete applicable
labelled edges in FR-179's canonical edge order. These entries SHALL follow
the relation's execution-binding order restricted to this alias. Each edge
SHALL carry the existing `pre_state`, `transition`, `post_state` and typed
`result` of FSpec FR-331's `HyperTransitionTuple`, retaining different
post-states or results even for equal pre-states and arguments.
Two executions over one alias share that entry; two different aliases keep
separate entries even when their state-key bytes are equal. Each key SHALL
serialize in the existing unpadded base64url StateKey spelling. A
certificate edge's `pre_state` and `post_state` SHALL each be that
base64url StateKey string, not the State object `{key}` that FSpec FR-331's
`HyperTransitionTuple` carries; a rejection locus writes State objects. No
HP-1 entry carries an automaton key. This is the HP-1 representation of the
existing `closure` member; the other forms' representation is unchanged.
The one-alias case uses this same sequence shape with one entry, for example
`[{"alias":"V","states":[...],"executions":[{"execution":"x","transitions":[...]},{"execution":"y","transitions":[...]}]}]`;
it has no separate single-subject encoding. The omitted array members in
this shape example stand for the actual keys and complete typed edges,
never ellipsis strings in a certificate.
An HP-1 certificate has empty `instance` and absent `reduction` and
`components`, because the relation binds executions, not object parameters
or a reduced temporal product.

Before recomputation, the checker SHALL compare the supplied alias sequence
with the checked relation's aliases in declaration order. An absent,
extra, duplicated, reordered or unknown alias is an input mismatch: the
checker SHALL refuse the certificate request without a certificate verdict
or proof, rather than invent a state key or a certificate-member name.
The checker SHALL apply the same input refusal when an alias's execution
names are missing, extra, duplicated or out of execution-binding order.
Within each admitted alias entry, unsorted or duplicate state keys follow
the `Malformed` rule below, naming the first offending state key.

A product state key is the tuple of each component's
`filament.simulation.state-key/v1` state key in variable order, with the property
automaton's canonical state key (ADR-018 PC-3), and for HP-3 the witness
set's members as sorted (existential tuple key, automaton key) pairs.

For HP-3, `states` is the universal tuple and each witness pair's tuple is
the existential tuple, each in its own quantifier order (ADR-023 HC-2).
When there are no universal variables, the universal tuple is empty. An
empty tuple and an absent tuple are different values. The witness set may
also be empty; neither case introduces a component state or an automaton
state that the product does not contain.

## Outputs

```rust
pub enum ProductClosureCheck {
    Accepted,
    Rejected(CertificateRejection),          // FR-338's: rule and locus
    Stopped {
        limit: ReachedLimit,                // FR-126: the limit and its value
        tuple: Option<HyperTupleStop>,       // only an HP-1 tuple-budget stop
    },
}

pub struct HyperTupleStop {
    pub count: u64,                         // first unpaid ordinal, one-based
    pub first_unpaid: HyperTransitionTuple, // existing complete typed edge tuple
}
```

An HP-1 `MaxRelationTuples` stop SHALL carry `tuple: Some(...)`; other
checker-limit stops SHALL carry no tuple detail. This is the in-process
checker result, not a change to FSpec FR-331's settled limit cause.

The checker uses FR-338's `CertificateRejection{rule, at}`; `Rejected`
settles ADR-018 PC-2's `CertificateRejected{rule, at}`. It reuses
FR-338's `InitialMissing` (a missing initial product state),
`SuccessorMissing` (a missing successor), `NotPartition` (components that
do not partition the members), `BackwardEdge` (an edge from a component to
an earlier one) and `WitnessFails` (a component whose witness fails), and FR-338's `CertificateRule` gains
`Malformed`, `UndefinedMember` and `ViolatingMember`. `CertificateLocus`
gains `HyperProductState(ProductStateKey)`, the hyper product's state key,
`HyperTransitionTuple`, the ordered tuple of labelled edges of an HP-1
check, and `CertificateMember`, the name of a malformed certificate member.
An HP-1 edge retains its pre-state key, transition identity (operation,
receiver and arguments), post-state key and observed result as FR-179
retains them. The tuple is in execution-variable binding order. It is not
the tuple of source states: two edges from the same source state can have
different arguments, post-states or results. FR-331's Certification table
spells these loci and lists the rules admitted at each (FR-338 "Rejection
spellings").

## Behavior

- **Production.** When EN-1 returns `Holds` for one of these forms, or
  `Witnessed` for HP-5, the
  `fsl-analyze` engine SHALL write the certificate from its retained
  product: every stored product state, and for HP-2 and HP-3 the
  components of its SCC decomposition, each with the first PC-4 witness
  that holds for it in FR-126's witness order. The certificate SHALL be a
  function of the request, the item and the limits: for HP-1, `closure`
  follows the alias and per-subject key order above; otherwise it lists its
  product keys ascending. The components follow FR-126's component rule over the
  hyper product, each component's vertex index the least FR-101 canonical
  breadth-first index of its states in that product; each component's
  states are ascending in the closure's key order; and every `sub` and
  constraint list follows FR-126's refinement and constraint-list rules.
  The certificate covers one object parameter binding (`instance`), so no
  order across bindings arises. The checker requires a topological order
  only and does not reject another topological order.
- **Core code only.** `check_product_closure` SHALL build each alias's
  `ModelSystem` by FR-128's rules. The checker SHALL expand non-HP-1
  product states through `fsl-eval`'s hyper product successor function, property-automaton
  translation, witness-set step and fairness-enabledness predicates (ADR-018
  LA-2), using no `fsl-analyze` code.
- **HP-1 reconstruction.** The HP-1 certificate checker SHALL read the
  checked relation and independently reconstruct each alias's complete
  reachable state set by canonical breadth-first expansion from its initial
  states through its `ModelSystem`, using no retained engine graph. Each
  supplied state set SHALL equal that independently reachable set. The
  producer emits the reachable closure; a closed unreachable superset adds
  no proof strength, and exact equality lets the checker use the existing
  model admission and successor primitives without a state-key decoder.
  When reconstruction completes with missing initial states, the checker SHALL reject
  `InitialMissing`, first by alias declaration order and then by ascending
  state-key octets.
  When all initial membership checks pass, the checker SHALL compare each alias's supplied and reachable sets in alias declaration
  order, selecting the least key in their symmetric difference by state-key
  octets.
  When the selected key is missing from the supplied set, the checker SHALL reject `SuccessorMissing` at that missing key.
  When the selected key is extra in the supplied set, including a noncanonical or foreign key, the checker SHALL reject `Malformed` at that supplied key. This ordering also
  applies when missing and extra keys occur together. Every admitted member
  is therefore independently reached and expanded. The checker SHALL recompute every edge
  independently of the certificate's edge sequences. After closure is
  checked, the checker SHALL compare each supplied execution's complete
  typed edge sequence with the recomputed applicable edges; a missing,
  extra, reordered or altered edge is an input mismatch that refuses the
  certificate request without a certificate verdict or proof. After the
  initial, closure and undefined-state checks, the checker SHALL enumerate
  the applicable labelled edge tuples in FR-179's canonical order and
  evaluate the checked relation body through the route's evaluator.
  Edges with equal pre-state keys remain distinct when their transition,
  post-state or result differs. A tuple rejection SHALL serialize as FSpec
  FR-331's existing `HyperTransitionTuple`: exactly `transitions`, one edge
  per execution binding, each with `pre_state`, `transition`, `post_state`
  and `result`. In the rejection, `pre_state` and `post_state` SHALL each be
  a State object `{key}`, unlike the certificate edge's base64url StateKey
  string. The result uses the existing typed canonical value; JSON
  `null` denotes an operation with no result, separately from a typed null
  result. A state-member rejection SHALL use the existing hyper state
  locus with `states` containing the actual offending state key and no
  automaton key or witness set.
- **Malformed.** A certificate whose `closure` keys are unsorted or
  duplicated, or whose `components` are absent for HP-2 or HP-3, SHALL be
  rejected with rule `Malformed` before any recomputation. Absent
  `components` SHALL name `CertificateMember("components")`, also when
  `closure` is empty. An unsorted or duplicate key SHALL name the first
  offending key in the supplied closure sequence. The checker SHALL NOT
  substitute a made-up member key for an absent structure.
- **Initial.** Every recomputed initial product state SHALL be a member;
  otherwise the checker SHALL reject with rule `InitialMissing`.
- **Closure.** Every recomputed successor of every member, canonicalised by
  FR-174's copy-swap rule when `reduction` is set, SHALL be a member;
  otherwise the checker SHALL reject with rule `SuccessorMissing`, naming
  the member. For HP-1, the exact-set comparison and missing-key locus in
  "HP-1 reconstruction" apply instead; the temporal product rule is unchanged.
- **Undefined.** No member's body letters, `μ_U` or `μ_E` SHALL evaluate
  undefined, refused or incomplete; otherwise the checker SHALL reject with
  rule `UndefinedMember`.
- **Components (HP-2, HP-3).** The checker SHALL check PC-4 (b) to (d) over
  the members, in this order: the components partition them, or the
  checker SHALL reject with rule `NotPartition`; every recomputed edge goes
  from a component to itself or to a later one, or it SHALL reject with
  rule `BackwardEdge`; each component's witness holds, or it SHALL reject
  with rule `WitnessFails`.
- **Violation by form.** The checker SHALL reject with rule
  `ViolatingMember` when the
  closed set holds:
  - HP-2: a component whose PC-4 witness does not exclude a fair accepting
    cycle of the self-composition;
  - HP-3: a member whose witness set is empty and whose component is not
    `Trivial` or `UnfairWeak` under the universal variables' fairness sets;
  - HP-6: a member whose monitor state rejects (PC-3 (c));
  - HP-1: a tuple of member transitions, one per execution variable with
    its step label, on which the body evaluates false.
  For HP-1, `ViolatingMember` SHALL name the complete
  `HyperTransitionTuple`, in ADR-023 HC-3 / FR-179's canonical edge-tuple
  order. An `UndefinedMember` caused by evaluating that tuple's body SHALL
  name the same tuple. Other failures on an HP-1 subject's state SHALL name
  the actual state member, without an invented automaton key.
  For HP-5 the closure, initial and undefined rules are the whole check,
  since its witnesses are checked by their own replay (FR-181).
- **Order.** The checker SHALL apply the rules in the order listed and
  report the first failing rule with the locus specified above: the absent
  certificate member, offending state key or labelled transition tuple.
- **Limits.** The checker SHALL count recomputed states and successors
  against the request's `ModelCheckLimits` with checked arithmetic.
  For HP-1 it SHALL also count recomputed relation tuples against the same
  request's `max_relation_tuples`, in a fresh counter for this certificate
  check, independently of the engine's counter (FR-184). A tuple that
  would pass that budget SHALL stop the check before its body is evaluated,
  with `MaxRelationTuples`, the configured value and the one-past-budget
  count, identifying the first unpaid tuple by its complete ordered
  `HyperTransitionTuple`. This is a checker-stop locus, not a
  `CertificateRejected` verdict. FR-182 settles the stop with the existing
  limit cause and setting `model_check.max_relation_tuples`; it does not
  add that tuple as a counterexample or alter the limit cause's wire shape.
- **Stop.** When a count reaches its limit, the checker SHALL return
  `Stopped`, naming the limit and its value, which settles V-7,
  `incomplete`, `LimitReached{limit, value, setting}`.
  For HP-1 relation tuples, reaching the value alone does not stop the
  check: only a next tuple that would pass it does (FR-184).
- **Purity.** The checker SHALL read no path, environment variable, clock
  or search location, and its result SHALL be a function of the
  certificate, the request and the limits.
- **Settlement.** The settling entry, FR-182's `settle_hyper_check` for an
  HP-1, HP-2, HP-3 or HP-6 item and FR-169's `settle_existential` for an
  HP-5 item, SHALL settle `proved`, `Certified` only on `Accepted`, V-6
  `CertificateRejected{rule, at}` on `Rejected`, and V-7 on `Stopped`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-163-AC-1 | Accepted vectors: §8.1's secure `NonInterference` certificate (8 product states, every component `MissingAcceptance` or `Trivial`) is `Accepted`; the copy-swap certificate (6 states) is `Accepted`; §8.2's secure `Opaque` certificate is `Accepted`; FR-179-AC-1's `Det` certificate is `Accepted`; FR-181-AC-1's HP-5 certificate is `Accepted`. | Test (TC-645) |
| FR-163-AC-2 | Rejected vectors over the AC-1 `NonInterference` certificate: one member removed is `Rejected` with rule `SuccessorMissing` naming the member whose successor is missing; its keys unsorted is `Rejected` with rule `Malformed` before any recomputation; an added member whose `l` components differ, in a component marked `MissingAcceptance`, is `Rejected` with rule `WitnessFails`; one member listed in two components is `Rejected` with rule `NotPartition`; two components joined by an edge listed in the reverse order is `Rejected` with rule `BackwardEdge`; the initial product state removed is `Rejected` with rule `InitialMissing`. | Test (TC-646) |
| FR-163-AC-3 | A certificate for the leaky vault's `NonInterference` built by listing its 14 product states with the components of the secure run, which do not cover the leaky states, is `Rejected` with rule `NotPartition`, so a false proof is never accepted; a certificate for FR-181-AC-5's body listing the reached product states is `Rejected` with rule `UndefinedMember` at the first state with `l = 1`. | Test (TC-646) |
| FR-163-AC-4 | The AC-1 `NonInterference` certificate checked with `max_states` 1 returns `Stopped` with `{MaxStates, 1}` naming the limit and its value; checking it twice gives equal results. | Test (TC-646) |
| FR-163-AC-5 | HP-2 and HP-3 certificates with absent `components`, including one with an empty closure, reject `Malformed` at `CertificateMember("components")` before any successor recomputation, without selecting a state key. | Test (TC-646) |
| FR-163-AC-6 | An HP-1 certificate for FR-179-AC-2's nondeterministic `Det` subject rejects `ViolatingMember` at the first complete labelled edge tuple in canonical order; equal pre-state keys and arguments with different post-states remain distinct. FR-179-AC-4's undefined body rejects `UndefinedMember` at its complete edge tuple. | Test (TC-646) |
| FR-163-AC-7 | For ADR-023 HC-2 with zero universal variables, a supplied key with an empty universal tuple and its actual witness set retains both on serialization and reading; the empty witness set remains distinct from an absent witness set. No component is added to satisfy a nonempty-array constraint. | Test (TC-646) |
| FR-163-AC-8 | **RETIRED (staleness check withdrawn).** The certificate carries no `package_id` and the checker compares no package identity. | Test (TC-1198), retired |
| FR-163-AC-9 | Production order. §8.1's secure `NonInterference` proof run twice writes byte-equal RFC 8785 certificates, whose `closure` keys are ascending, whose components are in FR-126's component order over the self-composed product's canonical breadth-first indices, and whose components each list their states ascending in the closure's key order. The same certificate with two components that no edge joins listed in the other order is still `Accepted`. | Test (TC-646) |
| FR-163-AC-10 | The HP-1 `Det` certificate of FR-179-AC-1 produced twice has byte-equal RFC 8785 serialization and one subject closure for its repeated alias, with ascending state keys and no automaton key; the independent checker accepts it after evaluating 64 tuples. Removing its initial key rejects `InitialMissing` at that missing key; removing a noninitial reachable key rejects `SuccessorMissing` at that missing key. Adding an unreachable, foreign or noncanonical key rejects `Malformed` at that key. Initial membership failures precede other set differences; subsequent simultaneous missing and extra keys select the least symmetric-difference key in alias declaration order then state-key octet order. Reversing two keys or duplicating one rejects `Malformed` at the first offending state key before recomputation. Each state rejection serializes without an automaton key. | Test |
| FR-163-AC-11 | Over two secure-vault model aliases declared `V` then `N` and execution bindings `(y: N::Vault::step, x: V::Vault::step, z: V::Vault::step)`, a relation with body `true` has exactly the subject-closure alias sequence `[V, N]`, with execution entries `[x, z]` and `[y]` respectively, retaining both alias entries even for equal state keys. Each execution contains its complete typed applicable edges and the certificate is accepted. Omitting `N`, duplicating `V`, adding a third alias, reversing the alias entries or removing or altering a supplied execution edge refuses the certificate request with no certificate verdict or proof. | Test |
| FR-163-AC-12 | FR-163-AC-6's false and undefined tuple rejections serialize the complete labelled edges in execution-binding order. Edges with equal pre-state and arguments but unequal post-states remain distinguishable; edges with equal pre-state, transition and post-state but unequal typed results remain distinguishable. A no-result edge serializes `result: null`, while an edge returning a typed null retains that typed value. Neither tuple acquires an automaton key. | Test |
| FR-163-AC-13 | Checking FR-179-AC-1's certificate independently with `max_relation_tuples` 63 stops before evaluating tuple 64, naming `MaxRelationTuples`, value 63, count 64 and the complete labelled tuple 64 as its first unpaid tuple; settlement is incomplete with setting `model_check.max_relation_tuples`, no counterexample and the unchanged limit-cause shape, never `CertificateUnchecked` or proved. With 64 it accepts, including when the engine already evaluated its 64 tuples under the same request setting. A second independent check with 64 also accepts. | Test |

## Dependencies

- ADR-023 HX-7 and HV-1; ADR-018 PC-1 to PC-5 and LA-2; ADR-029 CB-2 and
  RU-2.
- [FR-098](FR-098-execute-a-replay-request.md) (recompile),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (PC-3 and PC-4 certificates),
  [FR-174](FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md)
  (copy-swap canonicalisation).
- FR-176 to FR-179 and FR-181 produce the certificate; FR-182 settles it,
  and FR-169 settles an HP-5 item's.

## References

- ADR-023. FSpec half: FSpec FR-399 (Linear STD-136; ADR-023 QS-5). The
  certified-proof ruling is recorded on Linear QSL-390.
