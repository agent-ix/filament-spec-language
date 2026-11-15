---
id: FR-730
title: "Discharge a repeat's invariant and variant over the explored protocol subject"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-024
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-215
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-124
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-217
    type: references
  - target: ix://agent-ix/filament-spec-language/ADR-014
    type: references
  - target: ix://agent-ix/filament-specification/FR-171
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-228
    type: references
---
# FR-730: Discharge a repeat's invariant and variant over the explored protocol subject

## Description

When a protocol `repeat` carries a loop proof (`invariant { I } variant { V }`,
FR-218), the protocol model checker SHALL discharge two obligations of that
repeat over the explored protocol subject (FR-215): the **invariant
obligation**, that `I` holds at every decision of the repeat, and the
**termination obligation**, that `V` decreases in its well-founded order on
every back edge of the repeat. Each obligation settles `proved`, `refuted`
or `inconclusive` by the rules below. A repeat with no loop proof has neither
obligation; a repeat with a maximum terminates by that maximum and needs no
proof.

The loop proof is what lets a repeat run without a maximum (FSpec FR-171,
FR-228-AC-5). Admission does not wait for the proof: a repeat with a loop
proof checks at S3 whatever its obligations later settle. The obligations are
how the author's claim that the loop keeps its invariant and ends is tested
against every behaviour of the subject, so the claim is never silently
assumed and never silently bounded.

## Use case

A protocol designer writes a retry loop with no maximum, `repeat Retry by R
visible (got.left) invariant { got.left >= 0 } variant { got.left } while {
got.left > 0 } ...`, where each iteration consumes one unit of `left`. The
analysis reports `proved` for both obligations of `Retry` over the subject.
If a later edit lets an iteration leave `left` unchanged, the termination
obligation settles `refuted` with the path to the iteration whose back edge
does not decrease the variant, and the designer replays it.

## Inputs

- A checked protocol subject (FR-205, FR-215) whose protocol holds a `repeat`
  with a checked loop proof (FR-218).
- The protocol check's limits and method bounds (`ModelCheckLimits`,
  `max_depth`, `max_store_buffer`, `max_messages`, `max_live_instances`) and
  the caller's cancellation.

## Outputs

- For each repeat with a loop proof, one invariant item and one termination
  item, each with its verdict and, when refuted, its counterexample, or, when
  inconclusive, the limit, method bound or cancellation that stopped
  exploration.

## Behavior

### Obligations

- For every `repeat` that carries a loop proof, the request writer SHALL
  derive two items over the repeat's protocol subject, keyed by the repeat's
  checked node identity: `loop-invariant` and `loop-termination`. They are
  derived as the deadlock-freedom item is (FR-124): no author writes them,
  and two requests over one subject derive the same two items once.
- A **decision** of a repeat is a state in which a thread of the repeat is
  about to evaluate its `while` guard: on entry of a pre-test repeat, and
  after each completed body iteration of either kind (FR-218, post-test
  semantics).
- The **invariant obligation** holds when `I` evaluates to `true` at every
  decision of the repeat in every reachable state of the subject. `I` reads
  the binders and captures readable at the decision, as the guard does.
- A **back edge** of the repeat is the pair of decisions of one repeat thread
  that encloses one completed body iteration whose guard was `true` at the
  first decision. The **termination obligation** holds when, on every back
  edge, `V` at the second decision is strictly smaller than `V` at the first
  in the variant's order, and, for an integer-typed variant or element, the
  value at the first decision is at least `0`. A tuple variant is ordered
  lexicographically, element by element, and each element obeys the same
  floor. An integer type whose declared lower bound is at least `0`
  satisfies the floor by its type.

### Discharge

- The protocol model checker SHALL discharge both obligations by the
  explicit-state exploration of the subject that FR-215 defines, evaluating
  `I` at each decision it expands and `V` at both ends of each back edge it
  expands, with the evaluator of FR-390.
- An obligation SHALL settle `proved` only when exploration expanded every
  reachable state of the subject, with no limit reached, no method bound
  truncating a successor and no cancellation, and found no violating
  decision or back edge.
- An obligation SHALL settle `refuted` when exploration finds a decision at
  which `I` is `false` (invariant), or a back edge on which `V` does not
  decrease or breaks its floor (termination). The counterexample is the
  finite path from an initial state to that decision, or to the second
  decision of that back edge, and replays by FR-217. A refutation found
  before a limit is reached is still `refuted`.
- An obligation SHALL settle `inconclusive` when exploration stops before
  expanding every reachable state without finding a violation: a limit of
  `ModelCheckLimits`, a method bound that truncated a successor (for example
  `max_live_instances` or `max_messages`) or cancellation. The result SHALL
  name what stopped it, as the other protocol items do.
- When `I` or `V` is undefined at a decision (FR-390), the obligation SHALL
  settle `refuted` with the undefinedness as the violation, because the
  author's claim does not hold at that decision.

### Claim wording

- A `proved` invariant item states: the invariant of repeat `P::R` holds at
  every decision of every behaviour of subject `S`. A `proved` termination
  item states: repeat `P::R` terminates in every behaviour of subject `S`,
  because its variant decreases on every back edge. Neither states more than
  the subject explored: a method bound that truncated nothing is part of the
  subject's identity and is named in the result.
- An `inconclusive` item states which obligation was not decided and what
  stopped it, and claims neither that the obligation holds nor that it fails.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-730-AC-1 | A protocol whose `repeat Retry` has no maximum, the loop proof `invariant { left >= 0 } variant { left }` and a body that decreases `left` by one from `2` settles both the `loop-invariant` and the `loop-termination` item of `Retry` `proved`, with exploration complete. | Test (TC-2022) |
| FR-730-AC-2 | The same protocol with a body that leaves `left` unchanged settles `loop-termination` `refuted` with a counterexample ending at the second decision of the first back edge, which replays, and still settles `loop-invariant` `proved`; with the invariant `left > 0` the invariant item settles `refuted` at the decision where `left` is `0`. | Test (TC-2022) |
| FR-730-AC-3 | The AC-1 protocol checked with a state limit below its reachable state count settles both items `inconclusive`, naming the state limit, and claims neither result. | Test (TC-2022) |
| FR-730-AC-4 | A protocol with two repeats with loop proofs and one with a maximum only derives four items, two per proved repeat, and none for the bounded repeat; two requests over the one subject derive the four items once. | Test (TC-2022) |

## Dependencies

- FR-218 checks the loop proof and its variant type.
- FR-215 defines the explored subject; FR-217 replays the counterexample.
- FSpec FR-171 states the invariant and variant semantics.
