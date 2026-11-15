---
id: FR-641
title: "Carry every temporal trace kind and the progress ledger on the clause-run request"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-312
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-330
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-502
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-504
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-267
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-640
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-642
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-645
    type: references
  - target: ix://agent-ix/filament-specification/FR-094
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-352
    type: references
---
# FR-641: Carry every temporal trace kind and the progress ledger on the clause-run request

## Description

The `temporal` selection of FR-312's clause-run request SHALL carry every trace
input FR-330 evaluates, not only `finite` and `lasso`: a model trace, a
progressed event-position or fixed-sample trace, a timestamped finite-window
trace, a timed trace (FR-642) and, for each, positions that are either snapshots
of a model population or samples of a declared record trace subject. It SHALL
also carry a progress ledger (FR-502) in and return the updated ledger in the
result. A request file can then reach every evaluator the library reaches, and a
caller that monitors in batches carries progress from one run to the next.

## Use case

A plant writes one log file per minute. Each minute the operator runs the
`Deadline` clause over the new file as a `timestamped` trace with the
minute's watermark, passing the ledger the previous run returned. A regressing
watermark is refused instead of silently re-deciding an earlier minute, and a
deadline that falls across two files settles once the second file's watermark
covers it.

## Inputs

The `temporal` selection is the closed object `{name, trace, over, ledger}`:
`name` and `trace` required, `over` required exactly when the trace's positions
are snapshot keys, `ledger` optional.

### Positions

A **position** `P` is one of:

- `K`, a JSON string: an `observations` key naming a current
  `filament.state.snapshot/v1` document (FR-106), for a clause over a model
  subject; `over` binds the clause's `over` parameter in position 0;
- `{"sample": R}`: a record sample for a clause over a declared record trace
  subject, `R` a closed JSON object with one member per record field, each value
  in the snapshot document's field-value encoding (`{"integer": "3"}`, FR-106).
  The trace binds the subject parameter to each sample, as FR-330's trace-subject
  route does; `over` is absent.

Every position of one trace has the same form.

### Trace kinds

`trace` holds exactly one member:

| Member | Shape | FR-330 input |
|--------|-------|--------------|
| `finite` | `[P, ...]` | `Finite` (a closed trace under a false-extension profile, a prefix under infinite-trace) |
| `lasso` | `{"prefix": [P, ...], "loop": [P, ...]}` | `Lasso` |
| `model` | `{"initial": K, "universes": U, "prefix": [S, ...], "loop": [S, ...]}`, `U` and each step `S` in FSpec FR-364's model-arm encoding | `Model` |
| `progressed` | `{"positions": [P, ...], "progress": G, "lower_boundary": B, "binding": T}` | `Progressed` (FR-504) |
| `timestamped` | `{"observations": [O, ...], "progress": G, "lower_boundary": B, "binding": T, "cover": C, "missing": [M, ...]}`, `cover` and `missing` optional | `Timestamped` (FR-501) |
| `timed` | FR-642's `{"prefix": [Q, ...], "loop": [Q, ...], "period": X, "through": X}` | `Timed` (FR-642) |

where

- `G`, the progress assertion: `{"watermark": W, "completeness": D, "scope":
  "closed"|"open", "execution": "closed"|"open"}`; `W` a non-negative integer
  (inclusive) or `{"half_open": E}` (an exclusive end, read as FR-501 reads a
  half-open watermark); `D` is `{"complete": {"through": N}}` or `"incomplete"`;
- `B`: `"authoritative-origin"` or `"history-cutoff"`;
- `T`, the trace binding (FR-522): `{"profile": I, "clock": NAME, "parameters":
  CLOCK}`, `CLOCK` in FR-640's clock configuration form;
- `O`: `{"position": P, "stamp": N, "order_key": {"authority": A, "key": Z}}`,
  `order_key` optional, `N` a non-negative integer tick, `Z` a signed integer;
- `C`: `"exact"` (the default) or `{"counter": {"uncertainty": N}}`;
- `M`: `{"position": N, "atom": N}`, an atom valuation absent from an admitted
  observation (FSpec FR-160): the observation's index in `observations` and the
  zero-based `holds` formula node of the checked clause.

`ledger` is the relative path of a `filament.progress-ledger/1` file, read once
under FR-100's `dependent_bytes` limit and relative to the request file. The file
is a closed JSON object `{"format": "filament.progress-ledger/1", "entries":
[E, ...]}`, entries in ascending binding order, each `E` a closed object
`{"binding": {"package", "clause", "occurrence", "clock", "profile"},
"watermark", "completeness", "scope", "execution", "covered": [{"identity",
"stamp"}, ...]}` holding FR-502's `LedgerBinding` (the clause as the declaration
node key's canonical text) and `ProgressLedgerEntry`, `covered` in
identity-then-stamp order.

## Outputs

FR-109's `ClauseRunReport`, written by FR-267. A run over a `progressed` or
`timestamped` trace writes the returned ledger as the result's `ledger` member
(FR-267, FSpec FR-352) in the `filament.progress-ledger/1` form above: the
updated ledger on acceptance, the supplied ledger unchanged on a refusal or a
limit stop, and no member when the request supplied no ledger and the run was
refused. The member's bytes are what a later request's `ledger` file holds.

## Behavior

- The reader SHALL decode every member under FR-100's closed-envelope rules:
  unknown, duplicate or `null` members, a trace with zero or several members, and
  a member value of the wrong JSON type refuse `invalid-request` at stage
  `request`, naming the member's path from `clause` (for example
  `selection.temporal.trace.timestamped.progress.watermark`).
- The reader SHALL construct FR-330's `TraceInput` variant named in the table
  and SHALL build each snapshot position's `DocumentRef` by FR-312's rules; a
  key naming an invocation or a non-current snapshot refuses `invalid-request`
  naming the position's path.
- A trace that mixes snapshot keys and samples, a sample trace with `over`, and
  a snapshot trace without `over` SHALL refuse `invalid-request` at stage
  `request`, naming `trace` or `over`.
- The reader SHALL check no length, stamp order or profile fit. Those are
  admission and evaluation rules, reported as FR-330 and FR-645 state, so a
  request file and an in-process caller get the same report for the same trace.
- The run SHALL apply FR-502 to a supplied ledger exactly as for an in-process
  `ProgressLedger`, and two runs whose second request carries the ledger file
  the first wrote SHALL give the same outcomes as the two in-process calls
  sharing one ledger (FSpec FR-094).
- A ledger file that is not a closed `filament.progress-ledger/1` document, or
  whose entries are out of order or repeat a binding, SHALL refuse
  `invalid-request` at stage `request`, naming `ledger`.
- The writer SHALL write the ledger canonically (members in the order above,
  entries in binding order), so writing a read ledger reproduces its bytes.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-641-AC-1 | Through `command::run`, a `timestamped` trace over three samples stamped 0, 2 and 5 with watermark 5, `complete` through 5, scope `closed`, `authoritative-origin` and a binding equal to the clause's timestamped-event row reports the outcome FR-501-AC-4 states for the same input built in process, byte-identical. | Test (TC-1662) |
| FR-641-AC-2 | Through `command::run`, a `progressed` trace of an event-position clause and of a fixed-sample clause reports the outcomes FR-504-AC-1 and AC-2 state for the same in-process inputs, and the result carries a `ledger` member. | Test (TC-1662) |
| FR-641-AC-3 | A run whose result `ledger` member is written to a file and passed as the next request's `ledger`, with the next trace asserting watermark 60 after a retained 100 under the same binding, reports stage `evaluate`, `refusal`, `contradicted_dimension` `Watermark`, and returns the supplied ledger unchanged; the same second request with watermark 120 is accepted and returns a ledger whose entry holds 120. | Test (TC-1663) |
| FR-641-AC-4 | A `model` trace and a `lasso` of samples each reach FR-330 and report its outcome; a trace mixing a key and a sample, a sample trace with `over`, a `timestamped` observation missing `stamp`, and a ledger file with two entries for one binding each refuse `invalid-request` at stage `request`, exit 20, naming the member. | Test (TC-1663) |
| FR-641-AC-5 | Reading a ledger file and writing it back from a run that changes nothing reproduces its bytes. | Test (TC-1663) |

## Dependencies

- [FR-312](FR-312-read-a-clause-run-request-and-run-it.md): the request reader
  this extends; [FR-330](FR-330-run-a-temporal-clause-through-the-spine.md): the
  trace inputs.
- [FR-501](FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md),
  [FR-502](FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md),
  [FR-504](FR-504-carry-progress-on-event-position-and-fixed-sample-traces.md),
  [FR-522](FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md).
- FSpec FR-094 (progress carried across requests), FR-352 (the result's
  `ledger` member), FR-364 (model-arm encoding).
