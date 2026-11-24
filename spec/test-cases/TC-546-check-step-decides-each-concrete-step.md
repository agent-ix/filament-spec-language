---
id: TC-546
title: "check_step decides initial states and steps by the stutter, abstract-operation and any rules"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-141
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-138
    type: verifies
---
# TC-546: check_step decides initial states and steps by the stutter, abstract-operation and any rules

## Description

Verify `check_initial` and `check_step` over functional mappings, with `_`
arguments and bound results, with hidden fields tracked as a candidate set,
and with protocol step-class rows.

Scope: FR-141-AC-1 to FR-141-AC-5, FR-141-AC-11 to FR-141-AC-17 and FR-138-AC-8.

## Test Procedure

Fixtures: `CasRefinesCounter` and its lost-update model (ADR-020 §8);
`RingIsQueue`; `Coin` (FR-141-AC-4); FR-136-AC-4's protocol subject.

1. Run the initial check and the three steps of FR-141-AC-1, and the lost-update `commitB` step.
2. Run each variant of FR-141-AC-2.
3. Run the `take`, broken `take` and `put` steps of FR-141-AC-3.
4. Run `Coin`'s `toss` and `reveal` with `side` hidden; then with `side =
   self.face`.
5. Run the protocol attempt-node step and the `fork` step of FR-141-AC-5.
6. On `Coin3` (FR-141-AC-11), check the initial state at index 2, the lost-update `commitB` step at `position` 3, and `toss` with `side` hidden, and read each `abstract_successors`.
7. On `Coin` with a `reveal` no candidate allows, with a completion undecided while another is accepted, and over FR-138-AC-8's failed `writes` update (FR-141-AC-12, FR-138-AC-8).

8. On the `chain` variant of `RingIsQueue` (FR-141-AC-13), run `put(1)` over ring `a` whose `chain` is `[b, a]`.

9. On the `twin` variant of `RingIsQueue` with the `Cell` row first (FR-141-AC-14), run `put(1)` on ring `r`.
10. Over the `twin` models and the protocol node row for `put` whose argument is `self.ring` (FR-141-AC-15), run the `put` transition through `check_protocol_step`.

11. On two rows from `Ring` mapped to the subtypes `FastQueue` and `SlowQueue` of the type `Queue` that declares one inherited abstract operation, run each row's step; then over the `twin` models run the protocol node row of step 10 with the node field declared `Reference<Queue>`, with `Reference<Cell>`, and with a declared type neither row conforms to, and swap the object rows' source order (FR-141-AC-16).

The test `a_protocol_node_row_faults_when_its_reference_type_has_multiple_object_rows` (`fsl-analyze/tests/it/refinement_protocol.rs`, tagged `FR-141-AC-15`) asserted the old `StepVerdict::Fault` and is to be replaced by the implementation (QSL ticket of FR-141-AC-15): `a_protocol_node_row_lifts_by_the_declared_field_type` (tagged `TC-546`, `FR-141-AC-15`) asserts the pass of step 10, and `a_protocol_node_row_faults_when_no_object_row_conforms` (tagged `TC-546`, `FR-141-AC-15`) keeps a `StepVerdict::Fault` only for the declared type neither row conforms to. The implementation SHALL add both tests and delete the old one; step 11 is tagged `FR-141-AC-16`.

Tag the tests `#[trace("TC-546", "FR-141-AC-n")]`, and the test of step 7 also `#[trace("TC-546", "FR-138-AC-8")]`.

## Expected Results

- Step 1: pass; `Taken::Stutter`; `Taken::Abstract([inc(c)])`;
  `AbstractStepRejected{inc(c), Postcondition}`.
- Step 2: `InitialNotAbstract{initial: 0}`; `StutterChanged`;
  `Taken::Stutter`; `NoAbstractMatch`; `AbstractStepRejected{inc(c),
  Precondition{CanInc}}`.
- Step 3: pass as `deq(r)`; `AbstractStepRejected{deq(r), Postcondition}`;
  `Taken::Abstract([enq(r, 1)])`.
- Step 4: a two-state set, then the `side` 1 state;
  `AbstractStepRejected{show(c), Frame{…}}`.
- Step 5: `Taken::Stutter` through the node row; pass by RS-3.
- Step 6: `InitialNotAbstract{initial: 2}`; `AbstractStepRejected{position: 3, …}` with `abstract_successors` 1; `toss` passes with `abstract_successors` 2.
- Step 7: `NoAbstractMatch{position}`; `Undetermined(UndecidedSuccessor)`; `Undetermined(MappingUndetermined)` naming the `writes` update.
- Step 8: passes as `Taken::Abstract([enq(a, 1, [b, a])])`, the one abstract
  transition whose `rs` is the lifted chain.
- Step 9: passes as `Taken::Abstract([enq(r, 1)])` whose receiver is the
  `Queue` `r`.
- Step 10: the declared type neither row conforms to is refused at S3 `ill_typed`/`type-mismatch` and never reaches `check_protocol_step`; the `Fault` is asserted at the row-choice function over a hand-built row set (FR-141-AC-17). Otherwise passes with the `Reference<Ring>` argument lifted to the `Queue`
  reference when the node record's field is declared `Reference<Queue>` and to
  the `Cell` reference when declared `Reference<Cell>`, by the declared field
  type and never by row order.
- Step 11: each inherited-operation row decides against its own row's target
  type with no `Fault`; the node row passes as in step 10 for each declared
  type and gives `StepVerdict::Fault` only for the declared type that neither
  row conforms to; swapping the object rows' order changes no verdict.

Every call passes the concrete `ModelSystem` and the abstract system (FR-141).

## Step 12

12. Over a row set built directly in a unit test, ask the row-choice function for a declared type no row conforms to, for a type both of two unrelated rows (`Queue`, `Cell`) conform to, and for nested rows (`Queue`, `SubQueue`) in each source order; over the `twin` models check a node field declared with a type neither row conforms to and one declared with a type both unrelated rows conform to (FR-141-AC-17).

Expected: `Fault` for the first two; the `SubQueue` row for the third in either order; S3 refuses the two node fields `ill_typed`/`type-mismatch` with no step decision.
