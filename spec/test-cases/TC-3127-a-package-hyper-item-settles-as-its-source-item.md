---
id: TC-3127
title: "A package hyper item settles as its source item"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-940
    type: verifies
---
# TC-3127: A package hyper item settles as its source item

## Description

Verify that hyper items of every form settle on the package route with the
source route's records, counterexamples, replays and certificates, that
atoms and conjuncts evaluate equally through the evaluator port, and that a
malformed hyper node or a refused package is refused without a verdict.
Scope: FR-940-AC-17 and FR-940-AC-19.

## Test Procedure

1. Compile TC-3126's unit, and its variant with `Equal`'s bound lowered to
   `c.value @ b <= 2`, each with a `handoff` member.
2. Analyze `Equal`, `Partner`, `CanReachThree` and the projection-aligned
   clause by a `program` request and by a `package` request over each
   handoff directory; check each certificate with FR-163's checker; evaluate
   each atom and conjunct at each explored joint state through the
   `ModelEvaluator` on both routes.
3. Analyze `Equal`, the projection-aligned clause and `Partner` over
   test-only packages the strict reader admits: `Equal`'s `slot_sc_sb`
   renamed `slot_sc_sz`, `slot_sc_sb` written at level 0, a read rooted in
   `slot_sc_sa` and `slot_sc_sb`, and a projection-aligned conjunct slot
   renamed `opx_sa`; also analyze the unmodified projection-aligned clause.
4. Analyze handoff directories whose package has `Equal`'s `body` binding
   referencing the atom's `expression` node, or its `quantifiers` Tuple
   removed.

## Expected Results

- Step 2: equal records, counterexamples (one trace per copy, `Partner`'s
  exhausted witness set), replays with `reproduced` `true`, and accepted
  certificates; `Equal` is `proved` and `certified` on both routes; no
  record is `unsupported`, no diagnostic is `not-yet-implemented`; equal
  values and charge logs per atom and conjunct.
- Step 3: the modified clause settles `failed` with `MalformedNode` naming
  its `hyperproperty` node key and no verdict; `Partner` and the unmodified
  projection-aligned clause (an atom slot and a conjunct slot both at level
  0) settle as in step 2.
- Step 4: whole-request refusals at the request stage with the reader's
  code, cause and pointer (`ill_typed`/`operator-ineligible` at the
  `hyperproperty` node for the body case), `items` `[]`, `last_stage` `null`, exit 20, and no
  source compiled.
