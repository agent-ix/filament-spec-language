---
id: FR-899
title: "Decide refinement liveness through hidden abstract fields and under strong abstract fairness"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/filament-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-142
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-143
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-145
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-377
    type: implements
  - target: ix://agent-ix/filament-specification/FR-378
    type: depends_on
---
# FR-899: Decide refinement liveness through hidden abstract fields and under strong abstract fairness

## Description

When a refinement's safety half holds and its abstract fairness set `F_A` is
non-empty, `fsl-analyze`'s second phase SHALL decide the liveness half
(FR-143) in two further cases, by these rules (ADR-020 AX-5, AX-6, FSpec FR-377 "Engines"). It SHALL never
be settled `unsupported`.

**Strong abstract fairness.** With no hidden field, the mapped abstract
behaviour is unique. An `F_A` class under `strong` is violated by a concrete
fair cycle on which the class is enabled at some mapped state and taken by
no edge. A class under `weak` keeps FR-143's reading: enabled at every state
and taken by no edge. The second phase SHALL search FR-142's retained
product for such a cycle with ADR-019's Streett filter. The steps are:

1. Decompose the product into SCCs.
2. In each SCC, for each concrete strong constraint of `F_C` whose class is
   enabled in the SCC and taken by none of its edges, remove the states
   that enable it and decompose again, until the SCC is fair under `F_C`
   or empty.
3. A fair SCC that, for some strong `F_A` class, enables it at some state
   and takes it on no edge yields a lasso. So does one that, for some weak
   `F_A` class, enables it at every state and takes it on no edge.

The lasso is the canonical shortest one through that SCC (ADR-018 CX-5).
Its failure is `AbstractUnfair{constraint}`, or `Divergence{constraint}` when
the cycle is a concrete terminal stutter.

**Hidden fields.** With a hidden field, a product state holds a candidate
set of hidden valuations (FSpec FR-378), and the mapped abstract behaviour
along a concrete behaviour is any run through the candidates. The half holds
exactly when, for every concrete behaviour fair under `F_C`, some candidate
run along it satisfies `F_A`. This is inclusion of the concrete fair
behaviours, read as the language of a finite Streett automaton, in the
language of the nondeterministic Streett automaton that the candidate graph
defines, whose acceptance is `F_A` read over candidate runs. Both automata
are finite, so the question is decidable. The second phase SHALL decide it
exactly, by a complementation-free inclusion check (an antichain search over
the candidate-set product with Streett acceptance) or any procedure that
gives the same answer.

A refutation SHALL be a concrete lasso that is fair under `F_C`, with its
**candidate graph**: the finite graph whose nodes are (lasso position,
hidden valuation) and whose edges follow the step rows, over one unrolled
loop iteration closed back to its start. No cycle of that graph satisfies
`F_A`. A candidate run is judged on its own steps: a class is enabled at a
candidate node when the abstract subject has a successor of the class from
that candidate's abstract state, and taken on a candidate edge when that
edge's own step row takes a transition of the class. Taken and enabled are
judged per run, never per product edge or per product state: enabledness or
taking by another candidate at the same product state does not count, and
`F_A` is satisfied by a run only when the run satisfies every constraint of
`F_A` together. The lasso's failure is `HiddenUnfair{constraint}`, naming
the first `F_A` constraint, in source order, that every candidate cycle
violates; when no single constraint is violated by every candidate cycle, it
names constraint `F_A[j]` for the least `j` such that no candidate cycle
satisfies `F_A[0..=j]` together.

**Timed subjects.** The two rules above apply unchanged when either subject
is timed (FR-430, FR-900), on EN-6's symbolic product. With a hidden field,
a candidate node is (lasso position, hidden valuation) of a timed lasso,
and a class is enabled at a candidate node at an instant when the abstract
subject has a successor of the class from that candidate at that instant,
read exactly on the graph split by the guards of the `F_A` classes as
FR-430 "Strong fairness over time" splits by strong constraints. A
refutation is a timed lasso with positive total delay (ADR-026 EZ-6) whose
candidate graph has no cycle satisfying `F_A`; a loop of total delay 0
refutes nothing. A strong `F_A` constraint over a timed subject is read as
FR-430 reads strong fairness over time. Neither case is settled
`unsupported`.

**Abstract protocol subjects.** When the abstract side is a protocol
(FR-147, FR-148), the abstract control state (rest points, binders, queues
and instances, FSpec FR-432) is hidden: the candidate set FR-148 keeps is
the hidden valuation, and the hidden-field rule above decides the `F_A` half
over it, a class being enabled at a candidate when the abstract protocol has
a step of that class from that candidate's configuration. The half is
decided whenever `F_A` is non-empty and the safety half holds, and is never
settled `unsupported`.

**Replay.** FR-145 SHALL reproduce each refutation from the counterexample
alone:

- a strong-fairness lasso, by re-walking it and checking that the class is
  enabled at some loop state and taken at none;
- a hidden-field lasso, by rebuilding its candidate graph from the row
  semantics and checking that no cycle of it satisfies `F_A`, and that the
  named constraint is the one the rule above selects. The candidate set
  after one loop iteration must equal the set at loop entry; otherwise the
  replay is inconclusive, not reproduced.

A counterexample whose loop is edited so that a candidate cycle becomes
fair does not reproduce.

Settlement is FR-143's: a holding half `Proved{basis: Exhaustive,
certification: Uncertified}`, a violated one `refuted` after replay.

### Decision: decide both cases exactly, and certify the refutation by replay

Both cases are finite-state questions with known decision procedures. The
strong reading is ADR-019's filter, which the concrete side already uses.
The hidden-field case is language inclusion, which avoids explicit Büchi
complementation through the antichain method. Settling them `unsupported`
would leave a refinement author with no answer for a hidden abstract
register or a strongly fair abstract operation, which are ordinary
specification idioms. Because a lasso's candidate graph is finite, the
refutation can be checked independently of the search that found it, so
replay gives the same assurance as for a functional mapping.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-899-AC-1 | `RegisterHistory` without its `prev` row and with `ensure fair weak Spec::Register::write`, over a concrete model whose writes recur, settles the liveness half `Checked(HoldsReduced{reductions: []})`. Its variant whose concrete model can loop forever on `peek` while the abstract `write` stays enabled for every candidate settles `Checked(Violated)` with failure `HiddenUnfair{write}`. | Test (TC-2536) |
| FR-899-AC-2 | A hidden-field refinement in which one candidate run along the concrete loop takes `write` and another does not settles `Checked(HoldsReduced{...})`, because some candidate run is fair. | Test (TC-2536) |
| FR-899-AC-3 | FR-143-AC-6's `Pair` with `ensure fair strong whole Spec::Pair::flipB` settles `Checked(Violated)` with failure `AbstractUnfair{flipB}`, the loop visiting `a = 0` at some state and taking no `flipB`. The same refinement with `weak` holds. A refinement that is only weakly fair, where strong fairness is needed, is therefore refuted. | Test (TC-2537) |
| FR-899-AC-4 | With `assume fair strong each Impl::Pair::flipB` in `F_C`, the AC-3 loop is not fair under `F_C` (concrete `flipB` is enabled at `a = 0` and never taken), so it is removed. The refinement then holds. | Test (TC-2537) |
| FR-899-AC-5 | FR-145 reproduces the AC-1 and AC-3 refutations, and refuses each with one loop step edited so that the cycle takes the constraint's class. | Test (TC-2537) |
| FR-899-AC-6 | A hidden-field refinement whose abstract `fire` is enabled at every candidate and taken only on a candidate step that leaves the candidate run's loop settles `Checked(Violated)` with failure `HiddenUnfair{fire}`, stem `[tick, tick]` and loop `[tick]`, although the loop's product edge takes `fire` for some candidate. Fixture: abstract `Spec::M` with visible `v: Int` mapped from concrete `v`, hidden `h` in `{0, 1, 2}`, initial `v = 0, h = 0`; `advance` with pre `h = 0` and post `h' = 1, v' = v`; `fire`, enabled at every `h`, with post `h' = 2, v' = v` from `h = 1` and `v' = v + 1, h' = h` otherwise; no operation returns to `h = 1` or `h = 0`; `ensure fair weak Spec::M::fire`; concrete `Impl::C` with `v` fixed at 0 and one always-enabled operation `tick` that changes nothing, `F_C` empty, `step Impl::C::tick -> any`. FR-145 replays it to `Reproduced`. With `fire` also mapping `h = 2` to `h' = 1` keeping `v`, so that `1 -> 2 -> 1` is a fair candidate cycle, the check holds and the earlier counterexample does not reproduce. | Test (TC-2536) |
| FR-899-AC-7 | A hidden-field refinement with hidden `h` in `{A, B}`, both initial and never changed, `fire1` keeping `v` only at `A` and `fire2` only at `B` (each otherwise `v' = v + 1`), both enabled everywhere, `ensure fair weak Spec::M::fire1` declared before `ensure fair weak Spec::M::fire2`, over AC-6's `tick` loop mapped `-> any`, settles `Checked(Violated)` with failure `HiddenUnfair{fire2}`: no single candidate run is fair for both, although each constraint alone is satisfied by some run, so no constraint is violated by every candidate cycle and the least `j` with `F_A[0..=j]` unsatisfiable is 1. FR-145 reproduces it, and refuses the same counterexample naming `HiddenUnfair{fire1}`. Without the `fire2` row the check holds. | Test (TC-2536) |
| FR-899-AC-8 | A hidden-field refinement with hidden `h` in `{A, B}`, both initial and constant, `fire` enabled only at `A` (and changing `v` there), and `ensure fair weak Spec::M::fire`, over AC-6's `tick` loop mapped `-> any`, settles `Checked(HoldsReduced{reductions: []})`, because the `B` run never enables `fire`. A hand-built counterexample naming `HiddenUnfair{fire}` for that loop does not reproduce. | Test (TC-2536) |
| FR-899-AC-9 | Over FR-430-AC-5's `Req` refined by `TReq` without the time invariant, with `Req` given a hidden `h: Boolean` free at the initial state (no row maps it) and `send`'s precondition `not self.sent and not self.h`: the liveness half settles `Checked(HoldsReduced{reductions: []})`, because the `h = true` run never enables `send` along the `poll` loop; with `h` initially `false` only, it settles `Checked(Violated)` with failure `HiddenUnfair{send}` and a timed lasso whose loop is one `poll` step of positive delay, which FR-145 reproduces through FR-241. FR-430-AC-5's refinement with `ensure fair strong Req::send` in place of `weak` settles `Checked(Violated)` without the time invariant and `Checked(HoldsReduced{reductions: []})` with it. None of the four settles `unsupported`. | Test (TC-2536) |
| FR-899-AC-10 | FR-147's `CasTwice` (with `peek` removed) and `ensure fair weak Spec::Counter::inc`, whose safety half holds (FR-148-AC-2), settles its liveness half `Checked(HoldsReduced{reductions: []})`, decided over the abstract protocol's candidate configurations, never `unsupported`; the same refinement with `peek` kept and mapped `-> stutter` refutes its safety half by divergence (FR-148-AC-5) and its `liveness` is `None`. | Test (TC-2536) |

## Dependencies

- ADR-020 AX-5 and AX-6, ADR-019's strong filter, ADR-018 CX-5.
- [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md):
  the retained product.
- [FR-143](FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md):
  the liveness half.
- [FR-145](FR-145-replay-a-refinement-counterexample.md): replay.
- FSpec FR-377 and FR-378.
- [FR-430](FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md)
  and [FR-900](FR-900-check-a-refinement-whose-abstract-model-is-timed.md):
  timed subjects; [FR-147](FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md)
  and [FR-148](FR-148-decide-steps-against-an-abstract-protocol.md): an
  abstract protocol subject.
