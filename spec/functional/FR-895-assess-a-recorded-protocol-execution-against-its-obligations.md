---
id: FR-895
title: "Assess a recorded protocol execution against its obligations in replay mode"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-636
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-893
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-894
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-283
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-059
    type: implements
  - target: ix://agent-ix/filament-specification/FR-737
    type: implements
  - target: ix://agent-ix/filament-specification/FR-247
    type: depends_on
---
# FR-895: Assess a recorded protocol execution against its obligations in replay mode

## Description

In *replay mode*, FSL SHALL assess a recorded execution of a deployed system
against a checked protocol and give one verdict per protocol obligation,
which is FSpec FR-059's conformance assessment. The recording is an FSpec
FR-737 `filament.protocol-recording/1` document. It holds:

- the observed sends, receives, attempts, effects, events, commits,
  compensation attempts and finishes, each bound to a protocol node, a
  role, a protocol instance and its FSpec FR-287 runtime keys;
- each role's local order;
- message correlations;
- clock readings;
- a complete-global-conformance closure record (FSpec FR-247).

A `global-conformance` item that carries a `recording` member (FR-896)
selects replay mode. Replay mode is separate from counterexample replay
(FR-217) and from `finite-replay` (FR-636):

- those two re-execute a trace in the engine's own spelling and decline a
  step the protocol does not enable;
- replay mode reads what a deployed system did, and reports a step the
  protocol does not enable as a violated obligation.

**Admission.** Before any obligation is assessed, the recording is refused
`invalid_runtime_input` when any of the following holds:

| Defect | Detail |
| --- | --- |
| An observation names a role the protocol does not declare | `missing-name`, naming the role and the observation's index |
| An observation names a node the protocol does not hold, or a node of another kind | `missing-name` or `invalid-value` |
| A runtime key is reused with different binding facts | identity contradiction, FSpec FR-058 |
| The recording names another protocol definition key | `invalid-value` |

Each of these is a defect of the recording's correspondence with the
protocol, not behaviour of the system.

**Obligations.** Each obligation below is one verdict: `satisfied`,
`violated` with an FSpec FR-059 cause (`unexpected`, `missing`,
`misordered`, `duplicated` or `wrongly-correlated`) and the observations
that show it, or `pending`.

| Obligation | Satisfied when |
| --- | --- |
| Causal order, one per communication and per FSpec FR-052 edge | the source's observation precedes the target's in the recording's causal order, which is each role's local order joined with the message correlations, never a timestamp |
| Enabledness, one per observation | the observed step is enabled in the protocol state that the earlier observations reach, by ADR-027's step rules |
| Channel premise, one per channel | deliveries respect the delivery policy (FR-894's `duplicate-delivery` only where duplication is admitted, loss only where it is admitted), FIFO order per key value, and capacity and overflow |
| Constraint, one per bound event node | the node's constraint holds over the observed record |
| Control, one per `choice`, `repeat`, `join` and `check` | the case taken has its guard true; a repeat runs within its `max`; a join fires only when its policy holds; a `check` holds |
| Window, one per `await` and compensation | the observed match or attempt falls within its `within` window by the recording's readings of the declaration's clock, the FR-640 configuration naming which clock that is |
| Compensation, one per registration | registration only after the forward effect and never after the commit; attempts within `attempts n`; each outcome in FR-893's partition |
| Requirement, one per `requires temporal` clause | the clause holds over the recording read as its trace subject (FR-283) |
| Closure, one per protocol instance | `finish` is observed and the closure record is `closed` with every required premise (FSpec FR-059-AC-5) |

**Pending, never passed.** An obligation whose deciding observation is not
in the recording is `pending` while the closure record is not `closed`, and
`violated` with cause `missing` once it is `closed`. A recording that stops
early therefore leaves its undecided obligations `pending`. An obligation is
never `satisfied` by the absence of a violation.

**Result.** The item settles with the FR-360 labels:

- `proved`, basis `closed-scope`, when every obligation is `satisfied`;
- `refuted` when any obligation is `violated`. The counterexample is the
  recording's prefix up to the first violating observation in causal order,
  with an FSpec FR-351 record whose `trace_position` is that observation's
  index;
- `inconclusive`, cause `open-history`, otherwise.

The record lists every obligation's verdict, with FR-893's outcome facts
and FR-894's duplicate kinds. Assessing the recording in one batch and
assessing it incrementally, one observation at a time, give the same
verdicts over the same observations (FSpec FR-059-AC-4).

### Decision: a recording is evidence, so a deviation is a finding, not an input error

A recorded run of a deployed system is evidence about that system. When the
system received a message before it was sent, or sent a message the
protocol does not allow, that is what the user needs to learn. Declining the
input would hide it. FSL therefore refuses a recording only when it cannot
be read against this protocol at all: an undeclared role, an unknown node,
or a contradictory key. Everything else becomes a verdict on a named
obligation. One verdict per obligation, rather than one per recording, tells
the user which promise was broken. `pending` keeps an open recording from
looking like success, which FSpec FR-059-AC-3 requires.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-895-AC-1 | A closed recording of the FR-636 order fixture (`Placed`, `Got`, `Ship`, `finish`) in which every obligation holds settles `proved`, and every obligation's verdict is `satisfied`. | Test (TC-2530) |
| FR-895-AC-2 | A recording in which `Seller`'s local order has `Got` before `Buyer` sends `Placed`, correlated as one message, gives the causal-order obligation of `Placed`/`Got` `violated`, cause `misordered`, naming both observations. The item settles `refuted` with `trace_position` at `Got`'s index. | Test (TC-2530) |
| FR-895-AC-3 | The AC-1 recording cut after `Got`, with closure `open`, leaves the `choice` control and closure obligations `pending` and settles `inconclusive`, `open-history`, never `proved`. The same cut with closure `closed` makes them `violated`, cause `missing`. | Test (TC-2530) |
| FR-895-AC-4 | A recording that names a role `Courier` the protocol does not declare is refused `invalid_runtime_input`/`missing-name` naming `Courier` and the observation's index, before any verdict. | Test (TC-2531) |
| FR-895-AC-5 | A recording in which `Seller` sends a message on `Tell` at a point where the protocol enables no send gives that observation's enabledness obligation `violated`, cause `unexpected`, and settles `refuted`. | Test (TC-2531) |
| FR-895-AC-6 | A compensation `within [0, 120]` on clock `c`, whose recorded first attempt has a reading of 130 on `c`, gives the window obligation `violated`. With the reading absent and closure `open`, the obligation is `pending`. Batch and incremental assessment of each recording give equal verdicts. | Test (TC-2531) |

## Dependencies

- [FR-636](FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md):
  the protocol items; `finite-replay` stays separate.
- [FR-893](FR-893-reach-and-report-every-compensation-outcome.md) and
  [FR-894](FR-894-keep-message-delivery-and-business-effect-identities-apart.md):
  outcome facts and identities.
- [FR-283](FR-283-monitor-a-supplied-trace-offline.md): temporal clauses
  over a supplied trace.
- [FR-896](FR-896-decide-a-global-conformance-claim-over-the-protocol-system-or-a-recording.md):
  the item that selects replay mode.
- FSpec FR-059, FR-247 and FR-737.
