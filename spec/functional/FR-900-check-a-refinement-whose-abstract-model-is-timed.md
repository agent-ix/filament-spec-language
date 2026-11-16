---
id: FR-900
title: "Check a refinement whose abstract model is timed"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-430
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-141
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-244
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-145
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-377
    type: implements
  - target: ix://agent-ix/filament-specification/FR-740
    type: implements
---
# FR-900: Check a refinement whose abstract model is timed

## Description

S3 SHALL admit a refinement whose abstract model is timed (`time dense`,
FR-231), over a timed or an untimed concrete model. EN-6 SHALL decide it
exactly, by the following rules (FSpec FR-740).

**Abstract clocks are derived.** Every abstract clock starts at 0 in every
initial state, as FR-181's timed replay starts every clock. After that it
advances with every delay and is reset by the abstract operation that a step
row maps a concrete step to. The step map therefore fixes every abstract
clock's value along a concrete timed behaviour, and the product carries the
abstract clocks as **auxiliary clocks** beside the concrete clocks. No row
maps an abstract clock. S3 SHALL refuse a field row whose abstract field is
a clock with `invalid_model_binding`/`malformed-declaration` at the row,
naming the clock, because the clock's value is derived. FR-430's refusal of
a row that reads a concrete clock is unchanged.

**Product.** The safety half SHALL be decided by EN-6 on the symbolic
product of (FR-142's discrete product state, a zone over the concrete and
auxiliary clocks):

- **Delay.** A concrete delay advances every clock by the same amount. The
  zone is intersected with the concrete target's time invariants and with
  the mapped abstract state's time invariants. A delay that would leave the
  abstract invariant fails the step rules with failure
  `AbstractTimeInvariant{state}`.
- **Discrete step.** `check_step` (FR-141) runs on each discrete edge and
  additionally requires the mapped abstract operation's clock guard over the
  auxiliary clocks. The abstract operation's resets are applied to the
  auxiliary clocks. A stutter row resets no auxiliary clock. A guard that no
  point of the zone satisfies fails with `AbstractClockGuard{operation}`.
- **Divergence.** FR-430's divergence rule applies: a failing step refutes
  only when its post-state has a point with a time-divergent continuation.

**Untimed concrete model.** An untimed concrete model fixes no delays, so
its behaviours admit every delay between steps. The refinement holds when
every concrete behaviour has some delay assignment that the abstract model
admits. The zone along a path is exactly the set of admitted assignments, so
the same forward product decides it: a path fails when its zone becomes
empty.

**Results.** These are FR-430's:

- a safety counterexample is a finite timed prefix with exact rational
  delays, chosen canonically (FR-241);
- a liveness counterexample is a timed lasso with positive total delay
  (ADR-026 EZ-6), whose abstract enabledness is read at every instant
  (ADR-026 TD-5);
- a proved safety half returns FR-244's refinement-form zone certificate,
  with the auxiliary clocks among its clocks, and settles `proved` only
  after FR-245's checker accepts it.

FR-145 replays a counterexample by FR-181 timed replay of the concrete
prefix and of its mapped abstract run, checking each abstract guard and
invariant in exact rationals. A counterexample with one delay altered across
the failing guard's bound does not reproduce.

Negotiation routes the record to EN-6 when either side declares `time dense`.

### Decision: an abstract clock is a function of the concrete run, so the check is exact

Timed language inclusion between two timed automata is undecidable in
general, because the abstract side can guess when to reset a clock. A
refinement here never guesses: each concrete step maps to one named abstract
operation, and that operation's resets are declared. Every abstract clock's
value is therefore determined by the concrete timed run, and the check is
reachability in a product timed automaton with extra clocks, which is
decidable and certifiable with the zone machinery that timed concrete
refinement already uses. Mapping rows for clocks would only restate that
function and could contradict it, so they are refused.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-900-AC-1 | Abstract `TReq` (`time dense`; clock `y`; `send` guarded `y >= 1`, resetting `y`) refined by concrete `TReq2` (clock `x`; `send` guarded `x >= 2`, resetting `x`), step map `send -> send`, checks and settles the safety half `proved` with a refinement-form zone certificate holding the auxiliary clock `y`, which FR-245's checker accepts. | Test (TC-2538) |
| FR-900-AC-2 | The variant whose concrete `send` is guarded `x >= 0` is refuted with `AbstractClockGuard{send}`. Its counterexample is a timed prefix whose `send` happens at a delay below 1, and FR-145 reproduces it. Altering that delay to 1 does not reproduce. | Test (TC-2538) |
| FR-900-AC-3 | An abstract invariant `y <= 3` with a concrete model that can wait 4 time units before its first `send` is refuted with `AbstractTimeInvariant`, at the delay that crosses 3. | Test (TC-2538) |
| FR-900-AC-4 | A field row `y = self.x` mapping the abstract clock refuses `invalid_model_binding`/`malformed-declaration` at the row, naming `y`. A row that reads a concrete clock in a non-clock field still refuses as FR-430 states. | Test (TC-2539) |
| FR-900-AC-5 | An untimed concrete `Req` refining the timed `TReq` holds, because every behaviour can delay 1 before `send`. With an abstract invariant `y <= 0` and guard `y >= 1`, it is refuted: the zone after the first step is empty. | Test (TC-2539) |
| FR-900-AC-6 | A timed abstract liveness constraint `ensure fair weak Spec::TReq::send` over a concrete loop with positive total delay that never sends, while `send`'s guard holds at every instant, is refuted with a timed lasso. | Test (TC-2539) |

## Dependencies

- ADR-026 TD-5, TS-4, EZ-6, CT-3.
- [FR-430](FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md):
  timed refinement with an untimed abstract model, whose rules this extends.
- [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md):
  the step function.
- [FR-244](FR-244-emit-a-zone-certificate-with-every-proof.md): the
  certificate.
- [FR-145](FR-145-replay-a-refinement-counterexample.md): replay.
- FSpec FR-377 and FR-740.
