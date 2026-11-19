---
id: TC-3006
title: "Engines read float model fields as IEEE bits or refuse them by name"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-056
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-120
    type: verifies
---
# TC-3006: Engines read float model fields as IEEE bits or refuse them by name

## Description

Verify the engine table of FR-056 "Float model members": the explicit-state
checker and the simulation evaluator read a float field as IEEE bits and solve
a float equation by `numericEqual`, the SMT route settles a float item
`unsupported` at negotiation and refuses it `field-sort` at the encoder, and
export writes the field through FSpec FR-812's binary floating point mapping.
No engine rounds a float to an exact value. Scope: FR-056-AC-60 to
FR-056-AC-62, FR-120-AC-34.

## Test Procedure

Fixture: a model whose object type `Gauge` (population `gauges`, universe
`{g}`) has the field `a` typed `ix://filament/native/Float64` and the operation
`flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`.

1. Explore the model from initial `a` `3ff0000000000000`, then from initial
   `a` `0000000000000000`, and record the states, their keys and the candidate
   count of each application.
2. Simulate `flip()` twice from `a` `3ff0000000000000`.
3. Solve `post F { self.a = e }` over a `Cell` with a `Float64` field `a` for
   `e` the constants `float64(bits: 0x3ff0000000000000)`,
   `float64(bits: 0x8000000000000000)` and `float64(bits: 0x7ff8000000000000)`,
   comparing each count with the enumeration's; then for a field typed by a
   record holding a `Float64` member.
4. Negotiate an SMT item `always holds(g.a = g.a)` over the model, then call
   `encode_smt_query` for it directly with `Unrolling{depth: 1}`.
5. Export the model with the invariant
   `self.a != float64(bits: 0x7ff8000000000001)`.

Tag steps 1–2 `#[trace("TC-3006", "FR-056-AC-60")]`, step 3
`FR-120-AC-34`, step 4 `FR-056-AC-61` and step 5 `FR-056-AC-62`.

## Expected Results

- Step 1: no `RequiresBound`. From 1.0 the states are 1.0 and
  `bff0000000000000`, one candidate per application. From +0 the states are
  +0 and `8000000000000000`, with distinct keys, and each application has two
  candidates.
- Step 2 ends at `3ff0000000000000` with no flag.
- Step 3: one candidate `3ff0000000000000`; two candidates
  `0000000000000000` then `8000000000000000`; no candidate, no successor and
  no finding. Each count equals the enumeration's. The record-typed field is
  enumerated.
- Step 4: negotiation settles the item `unsupported` with
  `unknown_required_feature`/`unsupported-feature` naming `float64`, and the
  package stays admitted. The direct call refuses `SmtEncodingRefusal`
  `field-sort` naming `g` and `a`, and writes no script.
- Step 5: `a` is a tagged binary floating point value with sign, integer
  significand and exponent, `-` is the generated negation, and no rational or
  decimal value stands for `a`. The NaN constant is
  `[tag |-> "nan", bits |-> "7ff8000000000001"]`.
