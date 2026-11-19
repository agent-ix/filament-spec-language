---
id: TC-1901
title: "Navigation over unbounded, ordered and undeclared relationship ends"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-363
    type: verifies
---
# TC-1901: Navigation over unbounded, ordered and undeclared relationship ends

## Description

Verify FR-363-AC-4 and FR-363-AC-7: unbounded and ordered destination ends navigate to their collection types and an end with no multiplicity refuses as a typed semantic refusal.

## Test Procedure

Over a model with `Order` ends `watchers` (`{0,unbounded}` unique), `crowd` (`{0,unbounded}` non-unique), `steps` (ordered unique `{0,3}`), `queue` (ordered non-unique `{0,3}`), `ranks` (ordered unique `{0,unbounded}`), `first` (ordered `{0,1}`) and `notes` (no multiplicity), check `size(self.<end>) >= 0` (or `present(self.first)`) in an invariant on `M::Order` and read the `Navigate` node's type. Test: `fsl-semantics/tests/it/checker_rulings.rs`.

## Expected Results

Each navigation types as FR-363-AC-7 states; `notes` refuses `ill_typed`/`operator-ineligible` at the form; no refusal has code `unsupported_construct`.
