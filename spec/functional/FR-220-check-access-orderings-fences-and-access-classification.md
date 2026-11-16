---
id: FR-220
title: "Check access orderings and fences, and classify every access"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-025
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-219
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-435
    type: depends_on
---
# FR-220: Check access orderings and fences, and classify every access

## Description

S3 SHALL check an attempt's optional `ordering o` (`relaxed`, `acquire`,
`release`, `acq_rel`, `seq_cst`) and the `fence Name ordering o;` control
(`acquire`, `release`, `acq_rel`, `seq_cst`) (ADR-025 MM-6); the shared
grammar's `fence-order` does not admit `relaxed`, so S1 refuses a fence
written with it. S3 SHALL record each attempt's ordering and mark the attempt atomic when it has an
ordering and non-atomic otherwise, and SHALL refuse an ordering that no access
kind the attempt's operation leaves possible takes (ADR-025 MA-3). EN-1 SHALL
classify each instantiated attempt as a load, store, read-modify-write (RMW) or
local access over its shared footprint (ADR-025 MA-1, MA-2). Under a weak
model, S3 SHALL refuse the access shapes it decides without object identity,
and the EN-1 pre-check SHALL settle every other shape (ADR-025 MA-4).

EN-1 SHALL distinguish potential footprints from enabled footprints.
EN-1 SHALL derive potential footprints over the subject's universes and
argument domains without evaluating selected effective preconditions or
attempt constraints. Potential locations shared by branches SHALL reserve
only the memory-model backing needed to preserve operations that may become
enabled, including their store-buffer or message behavior. Potential
footprints SHALL NOT make a location shared for access classification or
cause a V-8 refusal. At each reachable state, enabled footprints SHALL
include only applications whose selected effective precondition evaluates
`Completed(true)` over that thread's observation and whose record satisfies
the attempt constraint (ADR-025 MA-1). Enabled footprints alone SHALL decide
sharing, access classification and V-8 refusal.
EN-1 SHALL maintain each deferred local-ordering candidate by the attempt's
static declaration path, typed transition identity and each canonical model
location its footprint touches. While reachable states remain to classify,
EN-1 SHALL keep a local access unresolved.

- While EN-1 has not completed reachable-state classification, EN-1 SHALL
  defer an ordering mismatch for a local access that carries an ordering.
- When EN-1 classifies one of a deferred candidate's canonical model locations
  as shared in a reachable state, EN-1 SHALL discharge that candidate for the
  location.
- When EN-1 completes reachable-state classification exhaustively, EN-1 SHALL
  list each remaining candidate whose name was never shared as an
  `ordering-restrictions` entry with access `local`.
- When exploration ends incomplete, EN-1 SHALL return an incomplete result
  without refusing unresolved candidates.
- Potential footprints MAY identify names to track.
- Potential footprints SHALL NOT discharge a candidate, make a name shared, or
  cause a V-8 refusal.

## Use case

A verification operator annotates each attempt with the ordering its Rust
code uses. The compiler tells them when an annotation cannot apply, such as
an `acquire` store, and when an operation touches two shared fields at once,
which no single atomic instruction does, so that what they check under a
weak model corresponds to real accesses.

## Inputs

- The checked `parallel` controls and their attempts (FR-218, FR-114), with
  each attempt's model read and write footprints (ADR-021 POR-1, POR-3).
- The source default (FR-219) and, in the pre-check, the resolved model.

## Outputs

- On each checked attempt in a `parallel`: `class: Option<AccessClass>`, which
  is `Local` for an attempt whose footprint touches no data place and `None`
  for every other attempt, and its `ordering: Option<Ordering>`.
- From EN-1, for each enabled instantiated attempt, the class
  `AccessClass::{Load, Store, Rmw, Local}` over its enabled shared footprint
  (ADR-025 MA-2).
- From EN-1, potential footprints and potential shared locations that reserve
  memory-model backing; they do not contribute to access classification or
  the meaning of an access (FR-902).
- A checked `fence` control with its ordering.
- The S3 refusal `ill_typed`/`operator-ineligible` for an attempt's ordering.
  S3 refuses no access shape, and no item settles `unsupported` with cause
  `weak-access-shape`: every shape has the meaning FR-902 gives it (multi-location accesses, multi-location
  read-modify-writes, node readers of shared locations, orderings a kind
  does not take, and non-`all` joins). The item's record lists each
  ordering restriction applied as `ordering-restrictions`.

## Behavior

### Sharing and classification

- EN-1 SHALL derive potential footprints without evaluating selected
  effective preconditions or attempt constraints. A potential location that
  may be touched by conflicting operations in different branches, including
  a protocol reader in one branch and a potential writer in another, SHALL
  reserve the memory-model backing those operations need, including TSO
  store-buffer or RA message behavior. This reservation SHALL NOT make the
  location shared for access classification or trigger a V-8 refusal.
- At each reachable state, EN-1 SHALL derive enabled footprints using only
  applications whose selected effective precondition evaluates
  `Completed(true)` over that thread's observation and whose record satisfies
  its attempt constraint. Only these enabled footprints SHALL decide which
  locations are shared and which access shapes or ordering mismatches fail.
- A location SHALL be shared for access classification in a weak `parallel`
  when enabled footprints of attempts in two or more of its branches contain
  it and at least one writes it.
- Every location an enabled attempt touches that is not shared SHALL be
  branch-local for access classification; potential memory backing SHALL NOT
  change that classification.
- An `event`, `effect`, `commit`, `send` or `receive` node inside a branch of a
  weak `parallel` whose constraint reads a model location SHALL be a reader of
  that location (ADR-025 MA-1): the location SHALL count toward sharing as an
  attempt's read does. A `finish` and a node outside every weak `parallel`
  SHALL NOT be readers.
- EN-1 SHALL classify each instantiated attempt in a weak `parallel` as MA-2
  defines: a load reads its one shared location only; a store writes it with no
  pre-state read; an RMW reads it in the pre-state and writes it; a local
  attempt has no shared location.
- An access with an ordering SHALL be atomic and one without non-atomic.

### What S3 decides and what EN-1 decides

S3 knows the types of an attempt's receiver and parameters and its operation's
declared footprint, but not which objects they are: the grammar's
`attempt N by R on M::T::op` carries no receiver, and the objects are drawn from
the finite domains at each application (ADR-027 ST-1). Whether a field the
operation touches is shared or branch-local is therefore not decidable at S3,
because sharing is a property of instantiated footprints (ADR-025 MA-1), and
the access class depends on sharing.

- S3 SHALL record `class: Local` on an attempt whose footprint touches no data
  place.
- S3 SHALL record no access class on any other attempt.
- S3 SHALL NOT refuse an attempt on grounds of sharing.
- S3 SHALL NOT label a location shared or branch-local.
- S3 SHALL refuse an ordering that no access kind the attempt's footprint leaves
  possible takes (Admitted orderings).
- S3 SHALL refuse an ordering on an attempt outside every `parallel` with
  `ill_typed`/`operator-ineligible`, naming the attempt: an ordering applies
  only to an access of a `parallel` branch.
- EN-1 SHALL decide which locations are shared on instantiated footprints.
- EN-1 SHALL classify each instantiated attempt (ADR-025 MA-2).
- EN-1 SHALL step every access shape by FR-902 (Access shape), and SHALL
  restrict an ordering that the instantiated kind does not take (EN-1
  ordering rule, below).
- EN-1's instantiation of an attempt at the initial state, of its enabled
  applications and of its potential transition identities, is an FR-120
  expansion under the subject's `max_candidates` and evaluation meter. If it
  stops on either budget, the item SHALL settle FR-126's expansion stop,
  `Stopped{cause: ResourceExhausted, limit: Some({MaxCandidates |
  EvaluationMeter { counter }, value})}`, before any state is explored.

### Admitted orderings

S3 sees which fields an attempt's operation reads and writes, not whether they
are shared, so it decides an ordering only when no kind the footprint leaves
possible takes it.

- S3 SHALL refuse any ordering on an attempt that touches no data place.
- S3 SHALL refuse `release` or `acq_rel` on an attempt that writes nothing.
- S3 SHALL refuse `acquire` or `acq_rel` on an attempt that reads nothing.
- S3 SHALL admit every other ordering and an absent ordering on an attempt that
  touches a data place, including an RMW with no ordering and a `release` load
  that also writes a branch-local register, because the kind is not known until
  EN-1 instantiates the footprint.
- S3 SHALL admit `acquire`, `release`, `acq_rel` or `seq_cst` on a fence.
- A fence has no `relaxed` ordering: the shared grammar's `fence-order`
  admits only `acquire`, `release`, `acq_rel` and `seq_cst`, so `fence F
  ordering relaxed;` is a syntax error, and S1 SHALL refuse it with
  `invalid_syntax` at the `relaxed` token before S3 sees it, as S2 refuses
  a recovering CST (FR-309-AC-3). A relaxed fence orders nothing, so the
  language has no spelling for one.
- Under `sc`, orderings SHALL change no behaviour.

**EN-1 ordering rule.** The orderings a kind admits are ADR-025 MA-3's
(a load takes `relaxed`, `acquire`, `seq_cst` or none; a store `relaxed`,
`release`, `seq_cst` or none; an RMW one of the five and always has one; a
local access takes none).

- Under a resolved weak model, EN-1 SHALL restrict an ordering that the
  instantiated kind does not take to that kind, as FR-902 states: on a load
  `acq_rel` acts as `acquire` and `release` as `relaxed`; on a store
  `acq_rel` acts as `release` and `acquire` as `relaxed`; an RMW with no
  ordering acts as `relaxed`. A local access carries no memory effect, so
  its ordering changes nothing.
- EN-1 SHALL list each restriction applied in the item's record as one
  `ordering-restrictions` entry `{site, access, written, effective}`, in
  canonical transition-identity order (ADR-027 TS-2), with a local access's
  entry added once exhaustive classification shows its name is never shared.
- Under `sc`, an attempt whose instantiated class does not take its ordering
  SHALL be admitted, because orderings change no behaviour under `sc`.

### Access shape

- S3 SHALL admit every access shape, an any-object field and a population
  membership write included: which locations an access touches, and whether
  they are shared, is decided on instantiation.
- Under a weak model, EN-1 SHALL step an enabled attempt whose shared
  footprint holds two or more locations as FR-902's program-order load
  steps and apply step, and a multi-location read-modify-write as FR-902's
  locked transaction.
- Under a weak model, EN-1 SHALL take a node reader's read of a shared
  location as a `relaxed` load in the reader's step (FR-902).
- Under `sc`, every attempt SHALL be one step whatever its shape.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-220-AC-1 | Over ADR-025 §10's `SB` with source default `sc`, S3 checks `Sx`, `Sy`, `Ly` and `Lx` with no refusal, records each as atomic with its ordering and records no access class; a variant whose loads carry no ordering records them as non-atomic; a `fetch_add` attempt with `ordering acq_rel` checks and records it atomic; an attempt whose operation touches no data place and has no ordering records class `Local`. | Test (TC-665) |
| FR-220-AC-2 | S3 refuses, naming the attempt and with `ill_typed`/`operator-ineligible`, an ordering on an attempt that touches no data place, `release` on an attempt that only reads, and `acquire` on an attempt that only writes. S3 admits an RMW with no ordering and a `release` load that also writes a register, which AC-14 leaves to EN-1. A fence with each of `acquire`, `release`, `acq_rel` and `seq_cst` checks. | Test (TC-665) |
| FR-220-AC-3 | In a `parallel` declaring `memory tso`, an attempt whose operation writes two fields of its receiver checks at S3 with no refusal and no class, as it does with source default `sc`, and an attempt that writes an any-object field also checks with no refusal (FR-902-AC-6). | Test (TC-665) |
| FR-220-AC-4 | `SB` under `sc` with every ordering `relaxed`, and again with every ordering `seq_cst`, reaches the same 16 states and settles `proved` both times. | Test (TC-665) |
| FR-220-AC-5 | Under a `tso` selection, AC-3's two-field attempt and FR-219-AC-4's `parallel` that joins `any` each explore with no `weak-access-shape` result: the attempt is stepped as FR-902's load steps and apply step, and the join merges its completed branches (FR-902-AC-1, AC-5). | Test (TC-665) |
| FR-220-AC-6 | `fence F ordering relaxed;` refuses at S1 with `invalid_syntax` at the `relaxed` token and never reaches S3. | Test (TC-665) |
| FR-220-AC-7 | Two branches, each with a `relaxed` store attempt to the same cell field of one object, and no attempt that reads it, check at S3 with no refusal under `tso` and under `ra`, each attempt atomic with ordering `relaxed` and no class recorded. | Test (TC-665) |
| FR-220-AC-8 | A branch whose attempts write a register field of its core and then read that register again (the register-read-again shape of message passing and load buffering) checks at S3 with no refusal and no class recorded. | Test (TC-665) |
| FR-220-AC-9 | ADR-025 §10's `SB` checks at S3 under a `parallel` declaring `tso` and under one declaring `ra`, with every attempt atomic, ordering recorded and no class recorded. | Test (TC-665) |
| FR-220-AC-10 | An attempt carrying `ordering seq_cst` that lies outside every `parallel` refuses `ill_typed`/`operator-ineligible` at S3, naming the attempt. | Test (TC-665) |
| FR-220-AC-11 | EN-1 classifies ADR-025 §10's `SB` under `tso` and under `ra` using only attempt applications whose selected precondition is true in the current thread observation and whose attempt constraint admits the application record: `Sx` and `Sy` are stores, `Ly` and `Lx` are loads, and (`c1`, `r`), (`c2`, `r`) are branch-local. Applications whose selected precondition is false contribute no enabled access footprint, including for a receiver or argument in the domain; their potential footprints may reserve memory backing but do not affect these classifications. | Test (EN-1 sharing; TC-665 step 10) |
| FR-220-AC-12 | EN-1 classifies AC-7's two `relaxed` stores as two stores to one shared location. | Test (EN-1 sharing; TC-665 step 11) |
| FR-220-AC-13 | EN-1 treats the register of AC-8 as branch-local, because attempts of one branch only touch it. | Test (EN-1 sharing; TC-665 step 12) |
| FR-220-AC-14 | Under `tso` and `ra`, an enabled instantiated load with `release` explores as a `relaxed` load and one with `acq_rel` as an `acquire` load; a store with `acquire` explores as `relaxed` and one with `acq_rel` as `release`; an RMW with no ordering explores as `relaxed`. The record lists each as an `ordering-restrictions` entry `{site, access, written, effective}`. A local access with an ordering whose name is never shared gets an entry with access `local` once classification is exhaustive. No item settles `unsupported`. | Test (EN-1 pre-check; TC-665 step 13) |
| FR-220-AC-15 | In `SB`'s `parallel` under `tso`, an `event` reader whose constraint reads `x.v` beside the branch whose enabled attempt stores `x.v` explores, its read a `relaxed` load in the reader's step with one successor per value its thread can see; with that store carrying `acquire`, it explores as a `relaxed` store and the record lists the restriction. Under `sc` the reader reads memory, and a `finish` whose constraint reads `x.v` after the `join` explores under `tso`. | Test (EN-1 pre-check; TC-665 step 14) |
| FR-220-AC-16 | Over ADR-025 §10's `SB` under `tso` and under `ra`, with `max_candidates` 0 the deadlock-freedom item settles `Stopped{ResourceExhausted, {MaxCandidates, 0}}` with no product state explored, its record `incomplete` with `limit-reached` naming `expansion.max_candidates`, not a refusal and not a broken invariant. In a variant whose universe also holds ten `Cell` keys the initial state does not hold, `max_candidates` 6, which the enabled applications fit within (the variant without them builds), stops listing `Cell::set`'s twelve potential identities, and the item settles `Stopped{ResourceExhausted, {MaxCandidates, 6}}` the same way; with `max_candidates` 12 it builds. | Test (EN-1 pre-check; TC-665 step 15) |

## Dependencies

- ADR-025 §1 MM-6, §2 MA-1 to MA-4; ADR-021 POR-1, POR-3 (footprints).
- [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md), FR-218
  (checked `parallel` and attempts, ADR-027), FR-219 (models).
- FSpec owns the `ordering` member, the `fence` control and the catalog
  spelling of the refusals (FSpec FR-435), and FSpec FR-435-AC-6 states the S1 refusal of a `relaxed`
  fence.

## References

- Owning ticket: Linear QSL-372. FSpec half: FSpec FR-435 (Linear STD-138),
  and FSpec FR-439 and FR-741 for the meaning of every access shape.
- EN-1 sharing, classification and the ordering rule (FR-220-AC-11 to
  FR-220-AC-14): Linear QSL-910 under QSL-565. FSpec FR-435 and FR-439 state
  the same check-time rules, pre-check outcome and `ordering-mismatch` shape
  (FR-435-AC-2, FR-435-AC-9, FR-439-AC-11).
