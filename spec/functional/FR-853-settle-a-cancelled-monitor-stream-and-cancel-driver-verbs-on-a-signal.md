---
id: FR-853
title: "Settle a cancelled monitor stream, and cancel the driver's verbs on an interrupt or termination signal"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-643
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-644
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-287
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-620
    type: references
  - target: ix://agent-ix/filament-specification/FR-710
    type: depends_on
---
# FR-853: Settle a cancelled monitor stream, and cancel the driver's verbs on an interrupt or termination signal

## Description

The `requested` cancellation cause (FR-276) SHALL be reachable from the
command line: the driver trips the `&Cancel` it passes to `run`, `analyze`,
`prove` and `monitor` with `CancelCause::Requested` when the process receives
an interrupt or termination signal, as FSpec FR-710 states for every
conforming CLI. A cancelled `command::monitor` SHALL end its stream with a
summary in which every clause still open is `incomplete` with the
cancellation cause, and every clause already settled keeps its settlement.

### Decision: a cancelled stream is incomplete, not ended

`fsl monitor` reads records from standard input for as long as the input
stays open, so a signal is its only way to stop. A signal could be read as
the end of the input, but ending the trace closes it: a pending `eventually`
clause would then be decided over a trace the recording never closed, and the
summary would report a verdict the observations do not support. A signal
therefore cancels: what was decided stays decided, and what was open is
reported `incomplete` with the cause, exit 22, the way `analyze` keeps its
settled items and settles its open ones `incomplete` (FR-276).

## Inputs

The `&Cancel` the driver passes to `command::monitor` (FR-644), `command::run`
and the analyze entries (FR-620, FR-635), and the record iterator the driver
passes to `command::monitor`.

## Outputs

For `command::monitor`, `Ok(MonitorSummary)` with the cancelled clauses; for
the other entries, their FR-276 cancelled outcomes.

## Behavior

### The monitor command

- `command::monitor` SHALL check the handle before it reads each record,
  after each record's step, and when the record iterator ends. A tripped
  handle wins over the end of the iterator: an iterator that ends because
  the driver stopped reading when the signal arrived is a cancellation, never
  an end of input.
- On a tripped handle, `command::monitor` SHALL read no further record, make
  no `finish` call, and return `Ok(MonitorSummary)`:
  - each clause settled before the cancellation keeps its `settled`,
    category, basis, disposition and `settlement`;
  - each clause still open has `settled` `false`, category `incomplete`,
    basis `unsettled`, FSpec FR-352's `disposition.temporal` kind
    `incomplete`, and `settlement` `{"step": "<n>", "trigger": "cancelled",
    "cause": "requested"}` (or `"deadline"`), `n` the number of counted calls
    made before the cancellation (FR-643 "Counted calls");
  - the summary carries `"cancelled": "requested"` (or `"deadline"`) and the
    number of records read, which counts only records whose step completed.
- A step the cancellation interrupts is atomic (FR-643 "Atomic steps"): the
  session is unchanged by it, it is not counted, and no event is handed to
  the sink for it.
- A handle already tripped when `command::monitor` is called SHALL return the
  summary after the request is read and the session opened, with no record
  read, every clause cancelled at step 0, and no event; a request-file defect
  still refuses first (FR-644).
- `cancelled` is a `settlement.trigger` value of the monitor event and
  summary objects beside `decided`, `finish` and `limit`, used only in the
  summary.
- The exit code is FR-644's most-severe ranking with the cancelled clauses
  `incomplete`: a stream with a violated clause exits 10, one whose clauses
  are only successful and cancelled exits 22.

### The driver

- The driver SHALL install FSpec FR-710's handlers before it calls the
  verb's library entry, and trip that entry's `&Cancel` with
  `CancelCause::Requested` on the first signal.
- For `monitor`, the driver's record iterator SHALL stop waiting for input
  and end once the handle is tripped, so a blocked read of standard input
  does not delay the cancellation.
- The driver writes the entry's outcome as it writes any other outcome
  (FR-287): the cancelled `analyze` or `run` document, or the monitor
  summary line, and exits with its code.

## Acceptance Criteria

The stream fixture is FR-644-AC-1's `Reaches`
(`eventually holds(r.level = 2)`) and `Never` (`always holds(r.level < 2)`) over a `finite` stream.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-853-AC-1 | `command::monitor` over `Reaches` and `Never` with the records `level` 0 and 1, whose iterator trips the handle with `Requested` and then ends, returns `Ok` with a summary in which both clauses have `settled` `false`, category `incomplete`, `settlement` `{"step": "2", "trigger": "cancelled", "cause": "requested"}`, `cancelled` `requested` and 2 records read, makes no `finish` call, and exits 22; the same records followed by `{"end": true}` and no cancellation settle the clauses by `finish` instead. | Test (TC-2413) |
| FR-853-AC-2 | Over `Reaches` and `Never` with records `level` 0 and 2 then a trip with `Deadline`, both clauses keep the `decided` settlements of record 1 (`Reaches` success, `Never` violation), the summary's `cancelled` is `deadline`, and the exit is 10; over `Never` alone with records `level` 0 and 1 then a `Deadline` trip, `Never` is cancelled with `cause` `deadline` and the exit is 22. | Test (TC-2413) |
| FR-853-AC-3 | `command::monitor` called with a handle already tripped with `Requested` reads no record, hands the sink no event, and returns every clause cancelled at step `"0"`, exit 22; called with that handle over a request naming a clause the program lacks, it returns FR-644-AC-4's refusal instead. | Test (TC-2413) |
| FR-853-AC-4 | `fsl monitor` reading from a standard input the test holds open, after two records, receives SIGINT and writes AC-1's summary as its last stdout line within one second, exit 22; `fsl analyze` and `fsl run` over FR-276-AC-4's and FR-276-AC-3's inputs, signalled with SIGTERM while running, write their cancelled outcomes with cause `requested`, exit 22. | Driver test (TC-2413): only the driver binary installs signal handlers and reads standard input, so the driver code ticket adds the test in the driver repository. |

## Dependencies

- FR-276 owns `Cancel`, its causes and the analyze cancellation rule.
- FR-643 and FR-644 own the monitor session, its events and its summary.
- FSpec FR-710 owns the signal rule for every conforming CLI.
