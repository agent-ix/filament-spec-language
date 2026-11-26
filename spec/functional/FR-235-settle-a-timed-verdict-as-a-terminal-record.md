---
id: FR-235
title: "Settle a timed verdict as an FR-331 terminal record"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-026
    type: implements
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-232
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-236
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-237
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-245
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-249
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-331
    type: depends_on
---
# FR-235: Settle a timed verdict as an FR-331 terminal record

## Description

FSL SHALL settle every item over a timed subject as exactly one FR-331
terminal record, extending FR-127's map with ADR-026's bases and causes
(ADR-026 TV-1, CF-4, OV-14). `fsl-replay` owns the additions to
`TerminalValue`; the zone engine's outcome (FR-239), its counterexample's
replay (FR-237) and its certificate's check (FR-245) map to it through one
function.

## Use case

A verification operator reads a run over a timed model. A proof states
whether it was certified by the zone certificate checker or exhaustive over
integer clocks. A proof over a model that admits no time-divergent
behaviour says so instead of reading as proved. A time-lock reads as
refuted, with its trap evidence.

## Inputs

- A `ZoneCheckOutcome` (FR-239).
- For `Violated`: the FR-072 result of its replay (FR-237).
- For `Holds(certificate)`: the result of `check_zone_certificate`
  (FR-245).

## Outputs

`TerminalValue` with these additions to FR-127's types:

```rust
// ProofBasis gains:
//   ZoneCertified,
//   ClosedForm { analysis: ClosedFormAnalysis },          // FR-248
//   BoundedSolver { method: SolverMethod, horizon: ExactRational, jumps: u64,
//                   exhaustive: BTreeSet<BoundDimension> },                // FR-250
//   InductiveSolver { method: SolverMethod },                                // FR-250
// InconclusiveCause gains:
//   NoAdmittedBehaviour, LassoNotConcretized, CertificateRejected,
//   SufficientTestFailed, SolverInconclusive { trace: Option<HybridTrace> }, // FR-250
//   InsufficientHorizon { horizon: ExactRational, jumps: u64,
//                         exhaustive: BTreeSet<BoundDimension> },          // FR-250
// UnsupportedCause gains: PunctualInterval
// CounterexampleKind gains: TimeLock
```

## Behavior

- The map SHALL be exhaustive, with no `_` arm, and give each input
  exactly one row:

| Verdict | Input | FSpec FR-360 label | FSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds(certificate)` and the checker accepts | `proved` | `closed-scope` | `Proved{basis: ZoneCertified, certification: Certified}` | success |

| V-4 | `Violated` whose replay reproduces, `kind` `Formula` or `Deadlock` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Violated` with `kind: UndefinedEvaluation{where, cause}` whose replay reproduces the undefined value at `where` | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-10 | `Violated` with `kind: TimeLock` whose replay reproduces | `refuted` | `closed-scope` | `Refuted` | violation |
| V-6 | `Holds(certificate)` and the checker rejects | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected)` | inconclusive |
| V-6 | `Undecided(NoAdmittedBehaviour)` or `Undecided(LassoNotConcretized)`; a replay that settles `inconclusive` or refuses | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `Stopped(cause, limit)`; for task automata this is a bounded search stopped before its proof or counterexample is checked | `failed`, execution `resource-incomplete` for generic stops; `incomplete`, execution `resource-incomplete` for task-search stops | `unavailable` | `Incomplete(LimitReached{limit, value, setting})`, or `Incomplete(Cancelled{source})`; a bounded task search additionally carries FSpec FR-331 `task_search` evidence | incomplete |
| V-8 | `PunctualInterval` disposition; no candidate | `unsupported` | `unavailable` | `Unsupported(PunctualInterval)` | unsupported |

- A `Holds(certificate)` outcome SHALL settle `proved` only after
  `check_zone_certificate` accepts its certificate. The certificate SHALL
  travel with the outcome from S6c over ADR-018's E11, as a counterexample
  does.
- A `Violated` outcome SHALL settle `refuted` only through its replay.
- For a task-automaton `Stopped` outcome, the terminal-record writer SHALL
  use FSpec FR-331's `incomplete` mapping and include its `task_search`
  evidence. Generic stops continue to use FR-127's generic mapping. The
  FSpec reader owns frontier validation; the evidence justifies the
  incomplete status and SHALL NOT settle `proved`.
- `TerminalValue::category` SHALL map `ZoneCertified`, `ClosedForm`,
  `BoundedSolver` and `InductiveSolver` to success, and `InsufficientHorizon`
  to inconclusive.
- A counterexample's length SHALL be its number of discrete steps, so V-5's
  definition is unchanged.
- The record SHALL carry the method: `zone-certified`,
  `exhaustive`, or FR-127's methods.
- A verdict SHALL hold for exactly its subject and claim; the record's
  obligation identity binds both (ADR-013 O-09).
- The record of an item EN-6 decides on its liveness product (FR-240's
  divergence acceptance set), and of one the digital-clock route decides by
  EN-1's SCC phase (FR-632), other than a TT-1 or TT-2 claim (FSpec FR-416
  "Non-Zeno premise"), SHALL carry `ZoneCheckRecord::non_zeno`,
  written as the FSpec FR-416 result-entry member `non_zeno`:
  `NotRequired` (`not-required`) for a proof whose product has no accepting
  cycle even without the divergence set; `Assumed` (`assumed`) for a proof
  whose product has accepting cycles without the divergence set and none
  with it; `Checked` (`checked`) for a refutation whose lasso loop passes
  the divergence set and whose delays FR-237's replay reproduces; and
  `Refuted` (`refuted`) for `Inconclusive(NoAdmittedBehaviour)`. EN-6
  decides `NotRequired` against `Assumed` by running FR-126's candidate rule
  on the split graph a second time without the divergence set. A cycle of
  positive but bounded total delay, such as FR-240-AC-1's `reply` self-loop
  that resets no clock under `x <= 3 ms`, fails the divergence set, so that
  proof is `Assumed`. Only the digital-clock route, where delays are integer
  ticks, decides divergence by a cycle's positive total delay. Every other
  row of the map carries `None`, the V-4 `refuted` row with cause
  `UndefinedEvaluation{where, cause}` included, because its finite prefix
  decides no liveness cycle; so does every TT-1, TT-2, time-lock-freedom and
  deadlock-freedom record. A zone-certified proof states the value in its
  certificate's `non_zeno` member (FSpec FR-418), and FR-245's checker
  recomputes it from its own successors and the certificate's components
  and rejects a mismatch, `CertificateRejected` with rule
  `NonZenoMismatch` at `CertificateMember{non_zeno}`, so the value is
  certified with the verdict and settles from the accepted certificate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-235-AC-1 | Each table row maps its input to the stated `TerminalValue`, FR-360 label, FR-243 basis and O-16 category; `ZoneCertified`, `ClosedForm{FixedPriorityRta}` and `BoundedSolver{…}` each map to success. | Test (TC-690) |
| FR-235-AC-2 | `Settles` over `Rpc` with `T = 4 ms`, decided by the zone search, settles `proved`, `Proved{ZoneCertified, Certified}`; the same outcome with one certificate node removed settles `inconclusive`, `CertificateRejected`.  `NoLateReply` over the same subject also settles `Proved{ZoneCertified, Certified}`. | Test (TC-690) |
| FR-235-AC-3 | `NoLateReply` with `T = 3 ms` settles `refuted` only after FR-237's replay reproduces; with the `timeout` step's delay changed to `5/2` the replay refuses and the item settles `inconclusive`. The strict-guard variant's time-lock-freedom item settles `refuted`, basis `closed-scope`, with a counterexample whose `kind` is `TimeLock`. | Test (TC-690) |
| FR-235-AC-4 | FR-232-AC-3's `Stall` claim settles `inconclusive`, `NoAdmittedBehaviour`; FR-234-AC-3's punctual liveness claim settles `unsupported`, `PunctualInterval`; a zone search stopped by `max_symbolic_states` settles `incomplete`, `limit-reached`, naming the limit. | Test (TC-690) |
| FR-235-AC-5 | FR-239-AC-5's outcome settles `refuted`, `decisive-counterexample`, category violation, cause `UndefinedEvaluation{where, cause: division-by-zero}` with `where` at position 2, time stamp 2, after FR-237's replay reproduces it; the time-locked variant's `Undecided(NoAdmittedBehaviour)` settles `inconclusive`, `NoAdmittedBehaviour`. | Test (TC-710) |
| FR-235-AC-6 | An interval-demand task product stopped at `max_symbolic_states = 2` settles `incomplete`, `LimitReached{limit: max_symbolic_states, value: 2}` and carries a `task_search` record whose product identity and effective limit match the request; removing a reachable frontier node or changing an edge makes the record fail replay and the item settle `inconclusive`, `ReplayParity`. The incomplete record never settles `proved`. | Test (TC-704) |
| FR-235-AC-7 | Settled through `settle_zone_check` with the core checker and FR-237's replay: FR-240-AC-1's response claim over `Rpc` with `T = 4 ms` proves with `non_zeno` `NotRequired`; its `reply` self-loop variant proves with `Assumed`; its `echo` variant refutes with `Checked`; FR-240-AC-4's `always eventually holds(true)` over `Stall` settles `Inconclusive(NoAdmittedBehaviour)` with `Refuted`; `NoLateReply` over `Rpc` and FR-239-AC-5's `UndefinedEvaluation` refutation carry `None`. | Test (TC-3021) |
| FR-235-AC-8 | Through `fsl analyze`, FR-632-AC-2's `Within` proved on the zone route and on the digital-clock route carries the same `non_zeno` in both records, one of `not-required` and `assumed`; `Early`, refuted on both routes, carries `checked` in both. | Test (TC-3021) |

## Dependencies

- ADR-026 §7 TV-1, §8.1 CF-4, §16 OV-14; ADR-018 V-1 to V-8 as amended by
  ADR-026; ADR-022 V-10; ADR-013 O-09, O-16 and O-24.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md),
  [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-232](FR-232-derive-the-time-lock-freedom-item-and-read-deadlocks-over-time.md),
  [FR-236](FR-236-carry-exact-rational-delays-in-a-timed-counterexample.md),
  [FR-237](FR-237-replay-a-timed-counterexample.md),
  [FR-245](FR-245-check-a-zone-certificate-in-the-qualified-core.md).

## References

- FSpec half: FSpec FR-416 and FR-417 (Linear STD-139) own the verdict
  rows, the new causes and the `TimeLock` kind in FSpec FR-331 (ADR-026
  OV-6).
