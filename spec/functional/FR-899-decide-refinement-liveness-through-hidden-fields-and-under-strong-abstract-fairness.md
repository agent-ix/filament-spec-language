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
`F_A`. The lasso's failure is `HiddenUnfair{constraint}`, naming the first
`F_A` constraint, in source order, that every candidate cycle violates.

**Replay.** FR-145 SHALL reproduce each refutation from the counterexample
alone:

- a strong-fairness lasso, by re-walking it and checking that the class is
  enabled at some loop state and taken at none;
- a hidden-field lasso, by rebuilding its candidate graph from the row
  semantics and checking that no cycle of it satisfies `F_A`.

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

## Dependencies

- ADR-020 AX-5 and AX-6, ADR-019's strong filter, ADR-018 CX-5.
- [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md):
  the retained product.
- [FR-143](FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md):
  the liveness half.
- [FR-145](FR-145-replay-a-refinement-counterexample.md): replay.
- FSpec FR-377 and FR-378.
