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
| Enabledness, one per observation | every causal predecessor of the observed step (ADR-027 CI-1) has an observation in the same protocol instance that precedes it in the recording's causal order. The predecessors a `sequence` supplies are every earlier step of that sequence, whatever its role or kind (message, `event`, `attempt`, `effect`, `commit`), including the steps of earlier nested `sequence` siblings. A predecessor never observed, or observed but not yet ordered before the step, leaves the verdict `pending` while the closure record is not `closed` and makes it `violated`, cause `unexpected`, once it is `closed`; a predecessor the step itself causally precedes makes it `violated`, cause `unexpected`, at once |
| Channel premise, one per channel | deliveries respect the delivery policy (FR-894's `duplicate-delivery` only where duplication is admitted, loss only where it is admitted), FIFO order per key value, and capacity and overflow |
| Constraint, one per bound event node | the node's constraint holds over the observed record |
| Control, one per `choice`, `repeat`, `join` and `check` | the case taken has its guard true; a repeat runs within its `max`; a join fires only when its policy holds; a `check` holds |
| Window, one per `await` and compensation | the observed match or attempt falls within its `within` window by the recording's readings of the declaration's clock, the FR-640 configuration naming which clock that is |
| Compensation, one per registration | registration only after the forward effect and never after the commit; attempts within `attempts n`; each outcome in FR-893's partition |
| Requirement, one per `requires temporal` clause | the clause holds over the recording read as its trace subject (FR-283); a guarded clause none of whose guards is true creates no obligation (FSpec FR-093-AC-6), so it is satisfied when the recording is closed and pending when it is open |
| Identity claim, one per obligation instance of each temporal clause of the package that holds a step identity read (FSpec FR-828) of this protocol | the clause holds over the recording, each observation a position and each read taking the identities the observation's FSpec FR-051 bindings name, each obligation instance bound to one protocol instance by its FSpec FR-287 key (FR-894 "Step identity reads"); `violated` at the first observation where it fails. Every such clause of the package is assessed, requested or not, as every `requires temporal` clause is: declaring a clause that reads this protocol's identities makes it an obligation of the protocol's recordings, so adding one can turn a recording's `proved` into `refuted` |
| Closure, one per protocol instance | `finish` is observed and the closure record is `closed` with every required premise (FSpec FR-059-AC-5) |

**Causal order and misorder.** The recording's causal order is the
transitive closure of each role's local order joined with the message
correlations (FSpec FR-737), and nothing else: the position of an
observation in the `observations` array never orders it, and a receive
listed before its send in the array, with no other relation between them,
is conforming. A pair of observations that lies on a cycle of that relation
is misordered: when the target of a communication or edge precedes its
source through local order and some other correlation, while the
correlation itself orders the source first, the pair's causal-order
obligation is `violated` with cause `misordered`, naming both observations,
the item settles `refuted`, and the counterexample's `trace_position` is the
index of the receiving (target) observation. A cycle is the only way two
observations can be out of causal order, because a relation with no cycle
always has an order consistent with it.

**Pending, never passed.** An obligation whose deciding observation is not
in the recording is `pending` while the closure record is not `closed`, and
`violated` with cause `missing` once it is `closed`. A recording that stops
early therefore leaves its undecided obligations `pending`. An obligation is
never `satisfied` by the absence of a violation.

A requirement obligation whose `requires temporal` clause is inactive over
the recording, every activation guard false so the clause creates no
obligation (FSpec FR-093-AC-5, AC-6), SHALL be `satisfied` on a `closed`
recording and `pending` on an open one, and an item that settles `proved`
on a `closed` recording stays `proved` with it. This is not satisfaction
by the absence of a violation: for an inactive clause the `closed` closure
record is the deciding evidence, because it asserts that the recording is
complete, so no further observation can make a guard true and no
activation can occur. An open recording has no such evidence, since a later
observation could still activate the clause, so the obligation stays
`pending` there.

**Result.** The item settles with the FR-360 labels:

- `proved`, basis `closed-scope`, when every obligation is `satisfied`;
- `refuted` when any obligation is `violated`. The counterexample is the
  recording's prefix up to the first violating observation in causal order,
  with an FSpec FR-351 record whose `trace_position` is that observation's
  index;
- `inconclusive`, cause `open-history`, otherwise.

The record lists every obligation's verdict, with FR-893's outcome facts
and FR-894's duplicate kinds. Every causal-order verdict, `satisfied` or
`violated`, names its two observations as `witness` `[receiving, sending]`.

**Incremental assessment.** An assessment session takes the recording's
observations one at a time and, after each, gives the assessment of the
prefix of the observations taken so far (FSpec FR-059-AC-4). A prefix keeps
the local order and correlations among its own observations and has closure
`open`, since further observations may arrive; only the last prefix carries
the recording's own closure record. The last prefix's assessment equals the
batch assessment. An obligation `violated` in one prefix is `violated` in
every later prefix: a verdict changes only from `pending` to a decision, or
from `satisfied` to `violated`, never from `violated`. FR-896's `assessment`
member selects the session on an item.

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
| FR-895-AC-2 | Over the order fixture extended with a channel `Tell` from `Seller` to `Buyer` (`Seller` sends `Note` after `Got`, `Buyer` receives it as `Heard` before `Placed`), a closed recording whose observations are `Got` (0), `Note` (1), `Heard` (2) and `Placed` (3), with local order `Seller: [0, 1]`, `Buyer: [2, 3]` and correlations `3 → 0` and `1 → 2`, puts `Placed` and `Got` on a causal cycle: the causal-order obligation of `Placed`/`Got` is `violated`, cause `misordered`, naming observations 3 and 0 as `witness` `[0, 3]` (receiving, then sending), and the item settles `refuted` with `trace_position` 0, the index of `Got`. | Test (TC-2530) |
| FR-895-AC-3 | The AC-1 recording cut after `Got`, with closure `open`, leaves the `choice` control and closure obligations `pending` and settles `inconclusive`, `open-history`, never `proved`. The same cut with closure `closed` makes them `violated`, cause `missing`. | Test (TC-2530) |
| FR-895-AC-4 | A recording that names a role `Courier` the protocol does not declare is refused `invalid_runtime_input`/`missing-name` naming `Courier` and the observation's index, before any verdict. | Test (TC-2531) |
| FR-895-AC-5 | Over the AC-2 protocol, a recording in which `Seller` sends `Note` on `Tell` before any `Got` is observed, a point where the protocol enables no send of `Seller`, gives that observation's enabledness obligation `violated`, cause `unexpected`, and settles `refuted`. | Test (TC-2531) |
| FR-895-AC-6 | A compensation `within [0, 120]` on clock `c`, whose recorded first attempt has a reading of 130 on `c`, gives the window obligation `violated`. With the reading absent and closure `open`, the obligation is `pending`. Batch and incremental assessment of each recording give equal verdicts. | Test (TC-2531) |
| FR-895-AC-7 | The closed AC-1 recording with `Got` listed before `Placed` in its `observations` array, its local orders and correlations unchanged (the `Placed` to `Got` correlation being the only relation between the two), has no causal cycle: every obligation is `satisfied`, the item settles `proved`, and the verdicts equal those of the AC-1 recording. | Test (TC-2815) |
| FR-895-AC-8 | The AC-1 protocol with a `requires temporal` clause activated `on each` under a guard that is false at every observation of AC-1's recording (an `eventually` over an order total above any the recording holds), gives the clause's requirement obligation `satisfied` and the item `proved` when the closure is `closed`; the same recording with closure `open` gives that obligation `pending` and the item `inconclusive`, `open-history`, never `proved`. | Test (TC-2530) |
| FR-895-AC-9 | Over the AC-2 protocol, the closed recording `Placed`, `Got`, `Note`, `Heard`, the `yes` case and `finish`, with `Seller`'s local order `Got`, `Note`, `yes`, `finish`, `Buyer`'s `Placed`, `Heard` and correlations `Placed` to `Got` and `Note` to `Heard`, has every step's predecessors observed before it: `Note`'s enabledness obligation is `satisfied` and the item settles `proved`. | Test (TC-2531) |
| FR-895-AC-10 | The AC-9 recording with `Seller`'s local order changed to `Note`, `Got`, `yes`, `finish`, so `Got` is observed but comes after `Note`, gives `Note`'s enabledness obligation `violated`, cause `unexpected`, and the item `refuted`, never `proved`; listing `Got` earlier in the `observations` array changes nothing. | Test (TC-2531) |
| FR-895-AC-11 | Assessing the AC-9 recording incrementally gives `inconclusive` for each of its first five prefixes and `proved` for the last, which equals the batch assessment. Assessing the AC-10 recording incrementally gives `inconclusive` for the first two prefixes and `refuted` for the rest, `Note`'s enabledness `violated`, cause `unexpected`, from the third prefix on, and the last prefix equals the batch assessment. | Test (TC-2531) |

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
