---
id: TC-1590
title: "A refinement item with internals, wider history rows and nested references"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-630
    type: verifies
---
# TC-1590: A refinement item with internals, wider history rows and nested references

## Description

Verify FR-630's request item and decisions. Scope: FR-630-AC-1 to FR-630-AC-10, FR-135-AC-6 and FR-135-AC-13 to FR-135-AC-16, FR-138-AC-9, and FR-140-AC-19 to FR-140-AC-22.

## Test Procedure

1. Analyze a register refined by a one-place queue with a correct and with a wrong mapping.
2. Analyze with and without `refinement_internals`.
3. Check and analyze refinements with a decimal and an enum history row.
4. Check and analyze, over the Counter fixture with a `chain: Sequence<Reference<Counter>>[0, 2]` field on both models, a refinement whose object row is `chain = self.chain` (the shape FR-135-AC-6 states over `RingIsQueue`; the Counter fixture is the one this test case's other steps use).
5. Analyze the `Box` record case of FR-630-AC-7 with rings `a` and `b`,
   where `a.box.target` names `b`, and the mapped `box.target` stays
   unchanged on each concrete and abstract transition.
6. Analyze the `Pair` tuple case of FR-630-AC-8 with
   `a.pair = Pair(Reference<Ring>(b), true)`, preserved by each concrete and
   abstract transition.
7. Analyze the `Holder` union case of FR-630-AC-9 with
   `a.holder = Holder::Has(Reference<Ring>(b))`, preserved by each concrete
   and abstract transition; also analyze the `Empty` tag.
8. Analyze the nested `Parcel` case of FR-630-AC-10 with
   `a.parcel.owners = [Some(Reference<Ring>(b)), None]`, preserved by each
   concrete and abstract transition.

## Expected Results

- Step 1: `proved` `certified`; `refuted` with a replayed counterexample carrying per-step records.
- Step 2: the internals reach compilation; absent means the empty list.
- Step 3: both check and settle `proved`.
- Step 4: checks and settles `proved`, `certified`.
- Steps 5–8: each item settles `proved`, `certified`; its checked simulation
  certificate maps the record, tuple, union or nested sequence/option value
  exactly as the corresponding FR-630 acceptance criterion states.
- Assertions compare the outcome document's typed members and the exit code, never message text.
