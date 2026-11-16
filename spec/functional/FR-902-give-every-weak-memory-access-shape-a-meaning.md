---
id: FR-902
title: "Give every weak-memory access shape a meaning"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-219
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-220
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-882
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-741
    type: implements
  - target: ix://agent-ix/filament-specification/FR-439
    type: depends_on
---
# FR-902: Give every weak-memory access shape a meaning

## Description

Under a weak memory model (`tso` or `ra`, FR-219), EN-1 SHALL step every
access shape a checked protocol can hold, by the rules below (FSpec FR-741,
amending ADR-025 MA-4). It SHALL NOT settle any item `unsupported` with cause
`weak-access-shape`. S3 SHALL NOT refuse an attempt that writes an any-object
field or a population membership.

| Shape | Meaning |
| --- | --- |
| **Multi-location access**: an enabled attempt whose enabled shared footprint holds two or more locations, after an any-object location is instantiated to its object and a population membership counted as one location `(population, membership)` | The attempt is a sequence of single-location accesses in program order. First, one **load step** per shared location it reads, in canonical location order (ADR-021 POR-1). Each load step reads the value the memory model gives the thread (TSO-3: own buffer, else memory; RA-2: one successor per message the thread's view admits) and holds it in the thread's pending-access record, which is part of the state key. Then one **apply step** evaluates FR-120's application over the loaded values and the branch-local state. It writes one store per shared location the post-state changes, in canonical location order: under `tso`, appended to the thread's store buffer in that order (TSO-2); under `ra`, as one message per location in that order, each later message's view including the earlier ones (RA-3). An attempt that reads no shared location has only the apply step. A precondition that fails over the loaded values disables the apply step, and the thread waits there as at any disabled attempt. |
| **Multi-location read-modify-write**: an attempt with two or more shared locations that reads and writes a shared location | A **locked transaction** in one step. Under `tso`, the step is enabled only when the thread's store buffer is empty; it reads and writes memory directly, and acts as a full fence (MF-1). Under `ra`, it reads the modification-order-latest message at each location, writes each new message immediately after the one it read, and acquires and releases by its ordering, with `acq_rel` when it has none. A single-location RMW keeps its ADR-025 meaning. |
| **Node reader of a shared location**: an event, `send`, capture or `finish` constraint that reads a shared location | A `relaxed` load by the reader's thread, taken in the reader's own step, with one successor per value the memory model admits. |
| **Ordering a kind does not take** (ADR-025 MA-3) | The ordering is restricted to the access kind. On a load, `acq_rel` acts as `acquire` and `release` as `relaxed`. On a store, `acq_rel` acts as `release` and `acquire` as `relaxed`. An RMW with no ordering acts as `relaxed`. Each sub-access of a multi-location attempt takes the attempt's ordering restricted the same way. The item's record lists every restriction applied as `ordering-restrictions`, each with the attempt, the access kind and the written and effective orderings. |
| **Non-`all` join**: `join any`, `quorum` or `predicate` | At the join, the merge (ADR-027 SE-1 f) takes the completed branches only. A cancelled branch thread under `tso` keeps its store buffer: its entries still flush by memory steps carrying that thread's identity, because cancellation stops issue and not draining. Under `ra`, a cancelled thread's messages stay, and its view is discarded. Under `outstanding continue`, the running branches keep their buffers and views. |

The only memory-model `unsupported` result left is a model the selected
system does not implement (`WeakMemory`, FR-882).

### Decision: decompose, as the hardware does

An operation that touches several words is not one hardware access, and that
is exactly why refusing it hid real behaviour: on TSO and under C11
release/acquire, such an operation is a series of single-word accesses that
other threads can interleave with. Splitting the loads into their own steps
lets another thread's flush or message land between two loads, which is the
torn read a programmer must reason about. Appending the stores together loses
nothing, because a store buffer is private and drains in order, and an RA
reader already picks each location's message independently. A read-modify-write
over several words has no unlocked hardware meaning, so it is a locked
transaction, which is what an implementation must use. Restricting an
ordering to its kind is how compilers read a half that does not apply, and
the record names each restriction so the author can see it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-902-AC-1 | FR-220-AC-5's attempt `Bx`, which writes two shared fields of its receiver, explores under `tso` and under `ra` with no `weak-access-shape` result. Under `tso` its two stores enter the buffer in canonical location order and flush in that order. A load attempt reading both fields in the other branch reaches a state that reads the new first field and the old second field. | Test (TC-2542) |
| FR-902-AC-2 | A two-location compare-and-swap attempt under `tso` is enabled only with its thread's buffer empty and is one step. Under `ra`, it reads the latest message at both locations. No state of either run shows one location updated by it without the other. | Test (TC-2542) |
| FR-902-AC-3 | FR-220-AC-15's `event` reader of `x.v`, beside the store to `x.v`, explores under `tso` with one successor per value the reader's thread can see. No `MultiLocation` entry is produced. | Test (TC-2543) |
| FR-902-AC-4 | A load with `release` and a store with `acq_rel` explore under `tso` and `ra` as `relaxed` and `release`. The record's `ordering-restrictions` lists both. | Test (TC-2543) |
| FR-902-AC-5 | FR-219-AC-4's `join any … outstanding cancel` under `tso` explores. A state after the join holds the cancelled branch's buffered store, which a later memory step flushes. | Test (TC-2544) |
| FR-902-AC-6 | In a `parallel` declaring `memory tso`, an attempt writing an any-object field checks at S3 with no refusal and explores, its location instantiated per application. The store-buffering litmus written with a multi-access write (`WriteX` touching two fields) compiles, explores and settles under `tso`. | Test (TC-2544) |

## Dependencies

- ADR-025 MA-1 to MA-5, TSO-2, TSO-3, RA-2, RA-3, MF-1; ADR-027 SE-1.
- [FR-219](FR-219-declare-and-select-the-memory-model-of-a-parallel.md) and
  [FR-220](FR-220-check-access-orderings-fences-and-access-classification.md):
  models, orderings and access classification, amended here.
- [FR-882](FR-882-step-every-specified-protocol-construct.md): the remaining
  stepper limit.
- FSpec FR-439 and FR-741.
