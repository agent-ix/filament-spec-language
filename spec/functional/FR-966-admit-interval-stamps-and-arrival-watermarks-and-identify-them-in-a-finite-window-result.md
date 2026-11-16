---
id: FR-966
title: "Admit interval stamps and arrival watermarks, classify late observations, and identify both in a finite-window result"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-033
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-501
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-526
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-641
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-094
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-160
    type: depends_on
---
# FR-966: Admit interval stamps and arrival watermarks, classify late observations, and identify both in a finite-window result

## Description

The finite-window evaluator (FR-501) reads per-observation interval stamps,
the `Bounded` cover, arrival watermarks and a late-data cutoff. This
requirement states how those inputs are admitted, which instant a late-data
cutoff classifies, which positions late observations take, how they enter the
result identity, and how a run reports a refused correction, so that every
case FR-501 "Bounded cover and interval stamps" and "Late-data cutoff" and
FR-526's revisioned repair reach has one outcome.

An observation's nominal stamp is a reading inside its own uncertainty
interval, so a stamp outside it is a contradiction in the input, not a wider
interval. A late-data cutoff bounds how late an observation may arrive after
its latest possible instant (FSpec FR-094), so the classified instant is the
upper end of the observation's uncertainty, whichever cover gives it. A late
observation is excluded from evaluation, so it takes no position, and the
`late` list identifies it by its place in the request.

## Inputs

FR-501's trace input: the cover, each observation's nominal `stamp`, optional
`interval` and optional `arrival`, the progress, the lower-boundary kind and
the optional late-data cutoff; and, for a correction, FR-526's relation and
ledger.

## Outputs

Admission refusals, the `late` classification, the canonical positions of the
evaluated observations, the result identities and the run's relation
refusal.

## Behavior

### Interval stamp admission

- An observation's nominal `stamp` `t` SHALL lie in its interval stamp:
  `earliest <= t <= latest`, with `t > earliest` when `earliest_open` and
  `t < latest` when `latest_open`.
- An interval stamp with `earliest > latest`, with `earliest = latest` and an
  open end, or whose nominal stamp lies outside it, SHALL be refused at stage
  `admit`, `invalid_runtime_input`/`invalid-value`, naming member `interval`
  and `position` the observation's zero-based index in input order (canonical
  order is not formed for a malformed interval), before any position is
  visited.
- With a nominal stamp inside its interval, the establishment margin
  `latest - t` of FR-501 is never negative.

### The classified instant

- The event instant a late-data cutoff classifies SHALL be the upper end of
  the observation's uncertainty: an interval stamp's `latest`, whatever its
  flag; otherwise `t + u` under `Bounded { uncertainty: u }`, `k + u` for a
  reading `k` under `Counter { uncertainty: u }` (the last whole tick of the
  half-open cover `[k-u, k+1+u)`, which classifies exactly as the exclusive
  end does for integer watermarks), and `t` under `Exact`. Uncertainty
  therefore never makes an observation late earlier than its latest possible
  instant (FSpec FR-094).
- The `late` entry's `stamp` member SHALL be that event instant, and its
  `position` the observation's zero-based index in input order.

### Positions of on-time observations

- Canonical order and positions SHALL be formed over the on-time observations
  only: a `late` observation takes no position, so decision support,
  refusal positions and activation anchors name positions among the on-time
  observations, and a `late` entry's `anchors` are such positions.
- A trace whose every observation is `late` has no position to evaluate and
  SHALL be refused as a trace with no positions is (FR-501-AC-17): stage
  `admit`, `invalid_runtime_input`/`invalid-value`.

### Completeness within one request

- A request's completeness assertion covers the observations that request
  carries, so neither a `late` observation nor an arrival after a window's
  deadline is a completeness contradiction within one request. A
  contradiction against an earlier assertion of the same binding is the
  progress ledger's (FR-502), whether or not a late-data cutoff is selected.

### Identity

- The `result_identity` and the `input_identity` (FR-522) SHALL cover the
  cover with its kind and uncertainty (`Exact`, `Counter`, `Bounded`), each
  observation's interval stamp (both ends and both flags) and arrival
  watermark, and the late-data cutoff (its rule and its parameter), each as
  its own member, so changing any one alone gives distinct identities even
  when the classification and the verdict are unchanged.

### A refused correction through a run

- Through `command::run`, a relation refusal of FR-526 (`no-ledger`,
  `predecessor-unknown`, `predecessor-not-latest`, `unchanged-input`,
  `correspondence-changed`) SHALL refuse the run at stage `evaluate` with
  `invalid_runtime_input`/`invalid-value`, its `cause` and member
  `relation`, as the evaluator refuses it, and SHALL write no result
  document and no ledger.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-966-AC-1 | Over a `timestamped` trace whose first observation is stamped 0 with no interval, a second observation stamped 5 with interval `[6, 5]`, one stamped 5 with `[5, 5)`, one stamped 7 with `[4, 6]` and one stamped 6 with `[4, 6)` each refuse at stage `admit`, `invalid_runtime_input`/`invalid-value`, naming member `interval` and `position` 1, before any position is visited; stamped 6 with `[4, 6]` and stamped 5 with `[4, 6)` are admitted. | Test (TC-2721) |
| FR-966-AC-2 | Under `Bounded { uncertainty: 5 }`, a request `holds(req)` stamped 0 with interval `[0, 0]` and an `ack` stamped 9 with interval `[9, 10]`, `holds(req) implies eventually[0,10] holds(ack)`, `W` and `Complete { through: 17 }`: `Completed(true)`; the same two observations without intervals give `Indeterminate { reason: StraddlesBound }`. | Test (TC-2721) |
| FR-966-AC-3 | Under `AllowedLateness { lateness: 2 }` and `Exact`, an observation stamped 3 with interval `[2, 4]` arriving at 6 is `on-time` and one stamped 3 with `[2, 3]` arriving at 6 is `late` with `stamp` 3; under `Bounded { uncertainty: 1 }` with no interval, stamped 3 arriving at 6 is `on-time` and stamped 2 arriving at 6 is `late` with `stamp` 3; under `Counter { uncertainty: 1 }`, reading 3 arriving at 6 is `on-time` and reading 2 arriving at 6 is `late` with `stamp` 3. | Test (TC-2722) |
| FR-966-AC-4 | Over FR-501-AC-29's observations (stamp 0 arrival 0, stamp 4 arrival 6, stamp 3 arrival 6) with `p` false at 0 and true at 4 and 3, `eventually[0,5] holds(p)` `on origin`, `W` 7: under `AllowedLateness { lateness: 2 }` the result is `Completed(true)` with decision support {(position 1, p)} and `late` `[{"position": 2, "stamp": 3, "arrival": 6, "anchors": [0]}]`; under `FixedInstant { cutoff: 5 }` both later observations are late, `late` is `[{"position": 1, "stamp": 4, "arrival": 6, "anchors": [0]}, {"position": 2, "stamp": 3, "arrival": 6, "anchors": [0]}]` and the result is `Completed(false)`; a trace of only the stamp-4 and stamp-3 observations under `FixedInstant { cutoff: 5 }` refuses at stage `admit`, `invalid_runtime_input`/`invalid-value`. | Test (TC-2722) |
| FR-966-AC-5 | Over AC-4's request under `AllowedLateness { lateness: 2 }`, changing one arrival watermark (6 to 7 on the stamp-4 observation), the rule (to `FixedInstant { cutoff: 2 }`), the parameter (2 to 3), the cover (`Exact` to `Bounded { uncertainty: 0 }`), or adding an interval `[4, 4]` to the stamp-4 observation, each alone, gives a `result_identity` and an `input_identity` distinct from AC-4's. | Test (TC-2723) |
| FR-966-AC-6 | `once[0,4] holds(p)` `on each` over AC-4's observations with `p` false at 0 and true at 4 and 3, `HistoryCutoff`, `AllowedLateness { lateness: 2 }` and `W` 7: the activation at position 0 is `Indeterminate { reason: MissingHistory }`, the one at position 1 is `Completed(true)`, the outcome is `Indeterminate { reason: MissingHistory }`, and `late` is exactly the stamp-3 entry with `anchors` `[1]`; the same request with no late-data cutoff reads all three observations, writes no `late` member and reports no completeness contradiction. | Test (TC-2722) |
| FR-966-AC-7 | Through `command::run`, FR-526-AC-31's third request (`r1`'s own request as `Superseding(r1)` with `r1`'s ledger) refuses at stage `evaluate`, `invalid_runtime_input`/`invalid-value`, `cause` `unchanged-input`, member `relation`, and writes no result document and no ledger; a `Superseding` naming a `result_identity` the ledger does not hold refuses the same way with `cause` `predecessor-unknown`. | Test (TC-2723) |

## Dependencies

- FR-501: the evaluator, covers, interval stamps and the late-data cutoff.
- FR-522: the identities; FR-526: the correction relation; FR-502: the
  progress ledger; FR-641: the request-file spelling.
- FSpec FR-094 "Late-data cutoff" and FR-160: the classification rule and the
  interval endpoints.
