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
| Enabledness, one per observation | every causal predecessor of the observed step (ADR-027 CI-1) has an observation in the same protocol instance that precedes it in the recording's causal order; "Enabledness predecessors" below states which steps are predecessors |
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
never `satisfied` by the absence of a violation, so a channel premise stays
`pending` while the closure record is not `closed`, unless a violation has
shown.

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

**Enabledness predecessors.** Each rule below is one requirement.

1. A `sequence` SHALL supply every earlier step of that sequence as a
   predecessor, whatever the step's role or kind (message, `event`,
   `attempt`, `effect`, `commit`), including the steps of earlier nested
   `sequence` siblings.
2. An earlier `choice` SHALL supply the case actually taken, as its `case`
   observation and the steps of its body, and no untaken case.
3. A `choice` with no observed case SHALL leave the verdict undecided, as an
   unobserved predecessor does.
4. An earlier `parallel` under `join all` SHALL supply the steps of every
   branch.
5. An earlier `parallel` under any other join SHALL supply the steps of the
   branches whose steps were all observed causally before the step, when
   the join is reached by those branches (rules 19 to 23); a branch that finishes
   after the join, as `outstanding continue` allows (ADR-027 ST-7), is not a
   predecessor. Until the join is reached, the steps of every branch are
   predecessors.
6. The assessment SHALL treat the steps of one `parallel` branch as unordered
   against those of a sibling branch.
7. Within a `repeat`, a step of one iteration SHALL have as predecessors the
   steps before the `repeat` and the earlier steps of the same iteration, the
   n-th occurrence of a step answering to the n-th occurrence of its
   predecessor.
8. An earlier iteration's steps SHALL neither be required by nor satisfy a
   step of a later iteration.
9. A `choice` in an iteration SHALL supply the case taken in that iteration.
10. A step after a `repeat` SHALL have as predecessors the steps before the
    `repeat` and every step of the last iteration begun causally before the
    step, taking for each `choice` in it the case taken in that iteration.
11. A `repeat` run zero times SHALL supply no predecessor.
12. An iteration begun concurrently with the step, or after it, SHALL NOT be
    a predecessor of the step; it is judged on its own steps.
13. A step after a `repeat` SHALL have as predecessors the steps of the
    `exhausted` path when that path ran.
14. An `exhausted` step SHALL have as predecessors the steps of the last
    iteration; the `exhausted` path is an alternative exit that runs once
    after the last iteration, not part of an iteration (ADR-027 FO-1).
15. The assessment SHALL treat `await` siblings as unordered, so none is a
    predecessor of another.
16. A predecessor never observed SHALL leave the verdict `pending` while the
    closure record is not `closed` and make it `violated`, cause
    `unexpected`, once it is `closed`.
17. A predecessor observed but not ordered before the step SHALL make the
    verdict `violated`, cause `unexpected`, at once: with prefixes in causal
    order no later observation can order the two.
18. A predecessor the step itself causally precedes SHALL make the verdict
    `violated`, cause `unexpected`, at once.
19. Under `join all`, the assessment SHALL take the join as reached when every
    branch has finished causally before the step.
20. Under `join any`, the assessment SHALL take the join as reached when one
    branch has finished causally before the step.
21. Under `join quorum(n)`, the assessment SHALL take the join as reached when
    `n` branches have finished causally before the step.
22. Under `join predicate(f)`, the assessment SHALL take the join as reached
    when `f`, applied to the completion of the joined branches finished
    causally before the step, in their sorted label order, is true (FR-218
    "Predicate joins").
23. The assessment SHALL NOT count a branch the step runs in as finished.
24. The assessment SHALL refuse a protocol whose predicate join names more
    than 10 labels, as `stage_limit_exceeded`/`work-budget-exceeded` with the
    fact `predicate-join-label-bound`, naming the join and the bound, before
    it evaluates any predicate.
25. The package route SHALL refuse the selection of a protocol that holds a
    `parallel`, and so a predicate join, as `unsupported`,
    `unsupported_construct`/`not-yet-implemented`, never leaving its steps
    `pending`.
26. A step of a branch of a `parallel` under `outstanding cancel` SHALL make
    the verdict `violated`, cause `unexpected`, when an observation of a step
    after the `parallel`, which needs its join, or of `finish`, in the same
    protocol instance, causally precedes it: the join step cancels the
    running branches and a cancelled thread takes no further step (ADR-027
    ST-7).
27. A step after the `parallel` that lies in the body of a `repeat` the
    `parallel` runs in SHALL count toward rule 26 only when it runs in the
    same iteration of that `repeat` as the cancelled step, for each such
    `repeat`.
28. The assessment SHALL take the iteration of a step in the body of a
    `repeat` only from a step that runs exactly once in every iteration: a
    step on the body's main sequence, not under a `case`, a `parallel`
    branch or another `repeat`, that lies at or before the step in program
    order. The iteration is the number of observations of such a step
    causally at or before the step. A such step of another role is causally
    before the step in every recording that satisfies the step's
    enabledness, so it never makes the count read low.
29. The iteration of a step SHALL be not determined when no such step lies at
    or before it in program order, when such a step was not observed at or
    before it, or when such steps give different counts.
30. Observations of steps in a `case`, a `parallel` branch or an inner
    `repeat` SHALL NOT count toward an iteration.
31. The assessment SHALL NOT count a step toward rule 26 when either
    iteration is not determined: a missed violation, never a false one.
32. A step of a branch of a `parallel` SHALL NOT be made `violated` for the
    cancel when no such observation precedes it, though the join's policy
    holds, because the join step may fire at any later point (ADR-027 CI-3).
33. A step of a branch of a `parallel` under `outstanding continue` SHALL NOT
    be made `violated` for the cancel.


**Incremental assessment.** An assessment session takes the recording's
observations one at a time and, after each, gives the assessment of the
prefix of the observations taken so far (FSpec FR-059-AC-4). Prefixes are
formed in the recording's causal order, the topological order of each role's
local order joined with the message correlations, ties broken by array
position; an observation's position in the `observations` array never orders
it. A prefix keeps the local order and correlations among its own
observations and has closure `open`, since further observations may arrive;
only the last prefix carries the recording's own closure record. A prefix
holds the activated population of the protocol instances it has begun, so
every prefix of an admitted recording is itself admissible. The last
prefix's assessment equals the batch assessment. An obligation `violated` in
one prefix is `violated` in every later prefix: a verdict changes only from
`pending` to a decision, or from `satisfied` to `violated`, never from
`violated`. If a prefix is nevertheless refused, the whole item is refused
`invalid_runtime_input`, with the cause of the refusal and the `prefix`
index in its detail. FR-896's `assessment` member selects the session on an
item.

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
| FR-895-AC-11 | Assessing the AC-9 recording incrementally gives `inconclusive` for each of its first five prefixes and `proved` for the last, which equals the batch assessment. Assessing the AC-10 recording incrementally gives `inconclusive` for the first prefix and `refuted` for the rest, `Note`'s enabledness `violated`, cause `unexpected`, from the second prefix on (`Placed` is observed and not ordered before `Note`), and the last prefix equals the batch assessment. | Test (TC-2531) |
| FR-895-AC-12 | Over the order fixture with a `Tell` channel, `Dropped` sent in case `no`, `Told` sent in case `yes` and `Note` sent after the `choice`, the closed recording `Placed`, `Got`, the `yes` case, `Told`, `Note`, with `Dropped` never observed, gives `Note`'s enabledness obligation `satisfied`: only the case taken supplies predecessors. | Test (TC-2531) |
| FR-895-AC-13 | The AC-12 recording without `Told` gives `Note`'s enabledness obligation `violated`, cause `unexpected`. | Test (TC-2531) |
| FR-895-AC-14 | Over the order fixture with, before `Placed`, a `parallel` of events `L` and `R` of `Buyer` under `join all`, the closed recording `L`, `R`, `Placed` in `Buyer`'s order gives `Placed`'s enabledness obligation `satisfied`. | Test (TC-2531) |
| FR-895-AC-15 | The AC-14 recording without `R` gives `Placed`'s enabledness obligation `violated`, cause `unexpected`; a closed recording holding `R` alone gives `R`'s enabledness obligation `satisfied`, since sibling branches are unordered. | Test (TC-2531) |
| FR-895-AC-16 | Over the order fixture with `Placed`, `Got` and the `choice` in a `repeat` of at most two iterations, the closed recording `Placed`, `Got`, `Placed`, `Got` with correlations of each `Placed` to the following `Got` gives the second `Got`'s enabledness obligation `satisfied`. | Test (TC-2531) |
| FR-895-AC-17 | The AC-16 recording without the second `Placed` gives the second `Got`'s enabledness obligation `violated`, cause `unexpected`, although the first iteration's `Placed` precedes it. | Test (TC-2531) |
| FR-895-AC-18 | Over a protocol whose `run` sequence holds the event `Start`, two `await`s `W1` and `W2` after it and the event `End`, the closed recording `Start`, `W2`'s matched event, `End`, with `W1`'s matched event never observed, gives `W2`'s matched event and `End` enabledness `satisfied`: `await` siblings are unordered and supply no predecessors. | Test (TC-2531) |
| FR-895-AC-19 | The AC-18 recording without `Start` gives `W2`'s matched event and `End` enabledness `violated`, cause `unexpected`. | Test (TC-2531) |
| FR-895-AC-20 | Over the AC-16 protocol with an event `After` of `Seller` after the `repeat`, the closed recording `After` alone (zero iterations), and the closed recording `Placed`, `Got`, `Placed`, `Got`, `After` with each `Placed` correlated to its `Got`, give `After`'s enabledness obligation `satisfied`. | Test (TC-2531) |
| FR-895-AC-21 | The closed recording `Placed`, `Got`, `yes`, `yes`, `After`, whose last iteration lacks its `Placed` and `Got`, gives `After`'s enabledness obligation `violated`, cause `unexpected`, although the first iteration's `Got` precedes it; the same recording with closure `open` gives `pending`. | Test (TC-2531) |
| FR-895-AC-22 | Over the AC-20 protocol, the closed recording `Placed`, `Got`, `yes`, `Placed`, `Got`, `no`, `After`, with each `Placed` correlated to its `Got`, gives `After`'s enabledness obligation `satisfied`: each iteration's predecessors use the case that iteration took. The same recording with the second `Got` missing gives it `violated`, cause `unexpected`. | Test (TC-2531) |
| FR-895-AC-23 | Over the AC-20 protocol with an event `Shipped` in the `yes` case, the closed recording of two `yes` iterations each holding `Shipped`, then `After`, gives `After` `satisfied`; with the last `Shipped` missing it is `violated`, cause `unexpected`; with `yes` in the first iteration and `no` in the last, `After` is `satisfied` without the first iteration's `Shipped` being a predecessor. | Test (TC-2531) |
| FR-895-AC-24 | Over the AC-20 protocol whose `exhausted` path holds an event `X`, the closed recording of two iterations, `X` and `After` gives `X` and `After` `satisfied`; the recording whose last iteration lacks its `Got`, then `X`, gives `X` `violated`, cause `unexpected`. | Test (TC-2531) |
| FR-895-AC-25 | Over a `parallel` of events `L` and `R` under `join any`, followed by `Placed`, the closed recording `L`, `Placed` gives `Placed`'s enabledness obligation `satisfied`; the recording `Placed`, `L` gives it `violated`, cause `unexpected`, as a predecessor the step precedes. | Test (TC-2531) |
| FR-895-AC-26 | The closed AC-20 protocol recording listed in the array order `Placed`, `Got`, `yes`, `After`, `Placed`, `Got`, `yes`, whose `After` is causally last (its role's local order puts it after the second iteration), assessed incrementally has no `After` enabledness obligation in the first six prefixes, which are in causal order, and `satisfied` in the seventh; no obligation of any kind changes from `violated`, and none from `satisfied` to `pending`, between consecutive prefixes; a channel premise stays `pending` in every prefix whose closure is `open` unless a violation has shown. | Test (TC-2531) |
| FR-895-AC-27 | The AC-26 recording with a third `Placed` by `Buyer` after the second, unordered with `After`, closure `open`, assessed incrementally has no `After` enabledness obligation in the first six prefixes and `satisfied` in the seventh and the eighth: the iteration begun concurrently with `After` does not reopen it, and no obligation changes from `violated` or from `satisfied` to `pending` between consecutive prefixes. | Test (TC-2531) |
| FR-895-AC-28 | Over the AC-25 `join any … outstanding continue` protocol, the recording `L`, `Placed`, `R` gives `Placed`'s enabledness obligation `satisfied` in batch and in every incremental prefix from the second, with no obligation changing from `violated` or from `satisfied` to `pending`; `R`, `L`, `Placed` gives `satisfied` from the third prefix; under `join all` the recording `L`, `Placed`, `R` gives `Placed` `violated`, cause `unexpected`. | Test (TC-2531) |
| FR-895-AC-29 | Over the AC-25 protocol with `outstanding cancel`, the recording `L`, `Placed`, `R` gives `R`'s enabledness obligation `violated`, cause `unexpected`, in batch and in the third incremental prefix (absent in the first two), and `L` and `Placed` `satisfied`; with `outstanding continue` the same recording gives `R` `satisfied`. | Test (TC-2531) |
| FR-895-AC-30 | Over a `parallel` of `L` and `R` followed by `Placed` under `join predicate(both) [left, right] outstanding continue`, with `both` the function `left and right`, the recording `L`, `R`, `Placed` gives `Placed` `satisfied` (prefixes: absent, absent, `satisfied`); `L`, `Placed`, `R` gives `Placed` `violated`, cause `unexpected` (prefixes: absent, `pending`, `violated`). | Test (TC-2531) |
| FR-895-AC-31 | With the predicate of AC-30 replaced by `left_only`, the function `left`, and `outstanding cancel`, the recording `L`, `Placed`, `R` gives `Placed` `satisfied` and `R` `violated`, cause `unexpected` (prefixes of `R`: absent, absent, `violated`); with `outstanding continue` it gives `R` `satisfied`; the recording `R`, `Placed` gives `Placed` `pending` while open and `violated`, cause `unexpected`, once closed. | Test (TC-2531) |
| FR-895-AC-32 | Under `join any [left, right] outstanding cancel` and under `join predicate(left_only) outstanding cancel`, the recording `L`, `R` (the policy holds, no step after the `parallel` observed) gives `R` `satisfied`; `L`, `R`, `Placed` gives `R` and `Placed` `satisfied`; `L`, `Placed`, `R` gives `R` `violated`, cause `unexpected`; an `R` by another role than `Placed`'s, unordered with `Placed`, is `satisfied`. | Test (TC-2531) |
| FR-895-AC-33 | With an inner `parallel` `join any [left, right] outstanding cancel` followed by `M` in branch `a` of an outer `parallel`, and `N` in branch `b`, the recording `L`, `M`, `R` gives `R` `violated`, cause `unexpected`; `L`, `R`, `M` gives it `satisfied`; `L`, `N`, `R` gives it `satisfied`. | Test (TC-2531) |
| FR-895-AC-34 | A predicate join over 10 labels is assessed; over 11 labels and over 64 labels, in a library assessment, in a batch item and in an incremental item, the assessment is refused `stage_limit_exceeded`/`work-budget-exceeded` (a batch or incremental item settles `declined`), with the fact `predicate-join-label-bound` and the text naming the join `Flow::Main::Many`, the label count and "at most 10". | Test (TC-2531) |
| FR-895-AC-35 | Under `join predicate(contradiction)` with the function `left and not left`, the recording `L`, `Placed` gives `Placed` `pending` in every open prefix and `violated`, cause `unexpected`, once closed. | Test (TC-2531) |
| FR-895-AC-36 | A protocol with a predicate join analysed from a package read from its bytes settles `unsupported`, `unsupported_construct`/`not-yet-implemented`, while the source route assesses the recording. | Test (TC-2531) |
| FR-895-AC-37 | Over a `repeat` whose body begins with a step `P` and a `parallel` `join any [left, right] outstanding cancel` of `L` and `R`, followed by `Placed`, `Got` and the `Decide` choice, the closed-order recording `P`, `L`, `R`, `Placed`, `Got`, `yes`, `P`, `L`, `R`, `Placed`, `Got`, `yes` gives every `L` and `R` enabledness obligation `satisfied` in batch and in every incremental prefix, with no obligation changing from `violated` or from `satisfied` to `pending` and the last prefix equal to batch; the same recording with the second `R` after the second `Placed` gives that `R` `violated`, cause `unexpected`, and the other `L` and `R` `satisfied`. | Test (TC-2531) |
| FR-895-AC-38 | Over a `parallel` `join any [left, right] outstanding cancel` whose branch `left` holds a `repeat` of `L`, followed by `Placed`, the recording `L`, `L`, `Placed` gives both `L` `satisfied` and `L`, `Placed`, `L` gives the second `L` `violated`, cause `unexpected`. | Test (TC-2531) |
| FR-895-AC-39 | Over an outer `repeat` whose body holds a step `P`, an inner `repeat` of a `parallel` `join any [left, right] outstanding cancel` of `L` and `R`, then `Placed`, `Got` and the `Decide` choice, the recording `P`, `L`, `R`, `L`, `R`, `Placed`, `Got`, `yes`, twice, gives every `L` and `R` enabledness obligation `satisfied` in batch and in every incremental prefix (no obligation leaving `violated` or going from `satisfied` to `pending`, the last prefix equal to batch); with the second outer iteration `P`, `L`, `R`, `L`, `Placed`, `R` the last `R` is `violated`, cause `unexpected`, and the other seven `L` and `R` `satisfied`; with no `P` before the inner `repeat` the same two-iteration recording leaves every `L` and `R` `satisfied`, the outer iteration of an inner step being not determined. | Test (TC-2531) |
| FR-895-AC-40 | Over the AC-37 protocol, the recording `P`, `L`, `Placed`, `Got`, `yes`, `P`, `L`, `R`, `Placed`, `Got`, `yes` (the first `R` skipped) gives the second iteration's `R` `satisfied` in batch and in every incremental prefix, as under `outstanding continue`. | Test (TC-2531) |
| FR-895-AC-41 | Over a `repeat` whose body holds `P`, an inner `repeat` of the cancelling `parallel`, another inner `repeat` of a step `X`, then `Placed`, `Got` and the `Decide` choice, the recording `P`, `L`, `R`, `X`, `Placed`, `Got`, `yes` twice gives every `L` and `R` `satisfied` in batch and in every incremental prefix, as under `outstanding continue`. | Test (TC-2531) |
| FR-895-AC-42 | Over a `repeat` whose body holds a `choice` of cases `a` and `b`, each with a step, then an inner `repeat` of the cancelling `parallel` of `L` and `R`, then `Placed`, `Got` and the `Decide` choice, and no step `P`, the recording `a`, `A`, `L`, `R`, `Placed`, `Got`, `yes`, `b`, `B`, `L`, `R`, `Placed`, `Got`, `yes` gives the second iteration's `L` and `R` `satisfied` in batch, as under `outstanding continue`. | Test (TC-2531) |
| FR-895-AC-43 | Over the AC-37 protocol with a second step `Q` after `P` that runs once in every iteration, the recording `P`, `Q`, `L`, `Placed`, `R` gives `R` `violated`, cause `unexpected`, while `P`, `L`, `Placed`, `R` (`Q` never observed, so the counts differ) leaves `R` not `violated`: the iteration is not determined. | Test (TC-2531) |
| FR-895-AC-44 | Over a `repeat` whose body holds `Ping` of `Seller`, `Pong` the receive of it by `Buyer`, and the AC-37 `parallel`, the recording `Ping`, `Pong`, `L`, `R`, `Placed`, `Got`, `yes` twice gives every `L` and `R` `satisfied` in batch and in every incremental prefix; with a step `S` of `Seller` in the body that is unordered with `Buyer`'s steps, `R` is `violated` under `outstanding cancel` and under `outstanding continue` alike. In a repeat body whose `Ping` of `Seller` precedes an `outstanding cancel` parallel with `Pong` and `L` in its left branch and `R` in its right branch, the recording `Ping1`, `Pong1`, `L1`, `Placed1`, `Got1`, `yes1`, `R2` with no `Ping2` gives `R2` `violated`; this depends on using the prior `Ping` as a cross-role iteration counter. | Test (TC-2531) |
| FR-895-AC-45 | For a `repeat` with `max` 2 whose body holds `P` then an inner `repeat` with `max` 1 of `L`, the closed recording `P`, `L`, `P`, `L` counts the inner repeat separately in each enclosing iteration: every outer and inner repeat-control obligation is `satisfied` in batch; in every incremental prefix none is `violated`, no verdict advances from `satisfied` to `violated`, and the final prefix equals batch. A prefix before descendant evidence may leave its repeat-control obligation `pending`. With no `P` to determine the enclosing iterations, the recording `L`, `L` leaves the inner repeat-control obligation not `violated` in batch and in every incremental prefix; the final prefix equals batch. With `P`, `L`, `L` in one enclosing iteration, the inner repeat-control obligation is `violated`, cause `duplicated`, in batch and from the second `L` onward; earlier prefixes may leave it `pending`, and the final prefix equals batch. | Test (TC-2531) |
| FR-895-AC-46 | When a recording is assessed against a package whose selected protocol contains a `repeat`, the package route returns the typed `unsupported` result with diagnostic `unsupported_construct`/`not-yet-implemented` and provides no per-obligation verdicts; it does not return a partial assessment. | Test (TC-2531) |

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
