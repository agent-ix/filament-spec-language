---
id: TC-1330
title: "Native composition carries and checks premises"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: verifies
---
# TC-1330: Native composition carries and checks premises

## Description

Verify FSL's compile and read-back of FSpec FR-491's `composition-decl` and
its settlement from FSpec FR-492's request over real native `Producer` and
`Consumer` packages and independently produced premise results.

## Test Procedure

Compile `Producer` and `Consumer` (FSpec TC-983) as real native packages, and
the composing unit of FSpec TC-985 case 1 importing them. Produce each
component guarantee's result by running its own item (the in-process model
checker where it routes, a plugin result entry where the case says so), and
build the settlement request from those results. Then:

1. Run FSpec TC-985's cases through compile and read-back, and replace the
   slots with a Boolean flag (FR-530-AC-1).
2. Settle the positive request (FR-530-AC-3).
3. Supply, one per run, the dependency's guarantee over `Consumer`, a weaker
   `Producer` clause, a safety guarantee, and a same-named `Sends` from a
   third package (FR-530-AC-2).
4. Add the mutual dependency under each of the four rules, with and without a
   registered provider; then select the lattice-declaring profile and offer a
   `proved` fixed-point result (FR-530-AC-4, FR-530-AC-5).
5. Run FSpec TC-983 case 5 (FR-530-AC-6).
6. Add a discharge record naming no pool entry, one whose pool entry is over
   another subject, and one naming an `inconclusive` result (FR-530-AC-7).
7. Add a forged slot, then a premise result for another slot's identity
   (FR-530-AC-8), then one for the same guarantee under other assumptions or
   none (FR-530-AC-10).
8. Declare a `rely` on a peer clause no `guarantee` covers; a peer guarantee
   conditional on a fairness constraint nothing supplied implies; a `weaker` at
   slot 3 with an `absent` at slot 4; two `absent` slots; and, with the
   interface declared and not weaker, a peer result absent, `tested`,
   `inconclusive`, of another subject and of another clause (FR-530-AC-9).

Compile an unrelated supported clause in the composing package and settle it
in every run. The TL-9 algorithm, FR-348 negotiation and FR-380 transfer are
exercised only through the results they produce; each mismatch and cycle case
must fail against a planner that accepts a referenced result as proof. Tag
the tests `#[trace("TC-1330", "<AC id>")]` and `#[trace("FSpec-TC-983")]` or
`#[trace("FSpec-TC-985")]` for each FSpec case they run.

## Expected Results

1. FSpec TC-985's results; the flag refused `invalid_runtime_input`/
   `wrong-value-kind`.
2. `proved`, `decisive-witness`, with the discharge record.
3. Each `inconclusive`, `premise-unproved` naming the liveness-dependency
   slot.
4. Each item `unsupported`, `unsupported-requested-capability`, kind
   `composition`, form `fixed-point`, both guarantees in edge order, with no slot
   read, no provider called and no `fixed-point` slot; the same with the profile.
5. FSpec TC-983 case 5's five distinct outcomes.
6. The first two refused `invalid_runtime_input`/`invalid-value` at the
   record; the third discharges nothing.
7. Each refused `invalid_runtime_input`/`invalid-value`.
8. `refuted`, `decisive-counterexample`, `mismatch` naming the peer, the clause,
   the slot and `absent`/`weaker`, with the complete slot list; the slot 4
   `absent` for the mixed case and the lower slot for two `absent`; each
   not-weaker case `inconclusive`, `premise-unproved`, never `refuted`.

The unrelated clause settles unchanged in every run.
