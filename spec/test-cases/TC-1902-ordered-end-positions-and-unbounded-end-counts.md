---
id: TC-1902
title: "Ordered-end positions and unbounded-end counts"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-364
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-106
    type: verifies
---
# TC-1902: Ordered-end positions and unbounded-end counts

## Description

Verify FR-364-AC-15 and FR-364-AC-16: ordered multi-valued ends carry positions that order navigation, malformed snapshot positions refuse at admission, model-layer extents use the same position rules, and an unbounded end checks only its lower bound.

## Test Procedure

Admit the snapshots of FR-364-AC-15 through FR-106 and navigate from `o1` over the ordered and unbounded ends. For the ordered target end, try (a) missing `positions.target` on instance 1, (b) a supplied `positions.source` for the unordered source end, (c) duplicate `positions.target` values for two instances with the same source, (d) `positions` as a non-object, (e) an end position as a JSON number, (f) a noncanonical and an out-of-range decimal string, and (g) an unknown position key. Run each with otherwise valid inputs. Repeat the semantic cases (required target position, forbidden source position, duplicate position) with model-layer `RelationshipExtent` values; include two different source groups that reuse a target position to verify uniqueness is scoped per opposite-end reference. Also supply a position on a single-valued ordered end. Test: `fsl-replay/src/spine/clause/tests/ordered_end_positions.rs` (ordered order and malformed positions, through `run_clause` with snapshot JSON); add the corresponding model-layer admission cases to the existing extent admission test for that API.

## Expected Results

Targets come back in ascending numeric position regardless of instance-list order. The missing position refuses `invalid_runtime_input`/`invalid-value` at `/relationships/0/instances/1`; the unordered-end position refuses the same at `/relationships/0/instances/1/positions/source`; and the duplicate position refuses the same at `/relationships/0/instances/1/positions/target`. A non-object `positions` refuses `wrong-value-kind` at `/relationships/0/instances/1/positions`; a JSON-number target position refuses `wrong-value-kind` at `/relationships/0/instances/1/positions/target`; a noncanonical or out-of-range decimal string refuses `invalid-value` at that target member; an unknown key `other` refuses `unknown-member` at `/relationships/0/instances/1/positions/other`. These malformed position cases reach FR-364 condition 5 after the instance's type and repeated-instance checks, except malformed member shape/spelling and unknown keys, which FR-106 reads earlier. A model-layer extent produces the same ordering and semantic refusals, naming the relationship and end but with no snapshot pointer; equal positions in different source groups admit, while a duplicate in one group refuses. A position on a single-valued ordered end also refuses. The unbounded end refuses `below-minimum` at zero links and returns four references at four.
