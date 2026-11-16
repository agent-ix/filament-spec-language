---
id: FR-951
title: "Certify and replay an EN-1 invariant item"
type: FR
relationships:
  - target: ix://agent-ix/filament-spec-language/US-015
    type: implements
  - target: ix://agent-ix/filament-spec-language/FR-625
    type: specifies
  - target: ix://agent-ix/filament-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/filament-spec-language/FR-158
    type: depends_on
  - target: ix://agent-ix/filament-specification/FR-364
    type: depends_on
---
# FR-951: Certify and replay an EN-1 invariant item

## Description

An invariant item, an `invariant` state clause over a model subject (FR-625),
SHALL be decided by EN-1 as the safety claim that the invariant holds at
every reachable state of the subject. Its proof SHALL count as `certified`
only after FR-338's closure checker accepts its certificate. Its refutation
SHALL count only after FR-128's replay reproduces its counterexample. The
item has two artifacts of its own:

- **Certificate.** A `ClosureCertificate` whose product states carry the
  invariant key-context root `["invariant",node,role,ordinal]` (FR-338
  "Canonical translation state"), over the fixed formula `always holds(I)`.
  `I` is true at a state exactly when the invariant's checked body is true
  for every object of the state whose most-specific type conforms to the
  context type. The binding is `[]`. A bad state is one where `I` is false
  or undefined.
- **Counterexample.** FSpec FR-364's finite prefix for a reachable-state
  invariant: the steps from an initial state to the first reached state
  where `I` is false, with an empty `loop`. It is the `undefined-evaluation`
  envelope when `I` is undefined there. FR-128 replays it with no fairness
  check.

The invariant item reuses the explicit-state search, certificate checker
and replay entry that clauses use, and adds a root and a letter, not a
second engine. An invariant is a state-model claim: its kind stays
`operation-contract` (FR-625 "Decision: an invariant item keeps its
family's kind").

## Inputs

- An analyze item naming an `invariant` state clause over a model subject.
- The subject's admitted initial states and universes (FR-120, FR-125).

## Outputs

- `proved` with basis `exhaustive` (or `reduced`, FR-623), certification
  `certified` once FR-338 accepts the certificate, and `backend` `en-1`.
- `refuted` with the finite-prefix counterexample and its FR-128 replay
  result, with `backend` `en-1`.

## Behavior

- EN-1 SHALL explore the subject's reachable states, evaluating `I` at each
  stored state, and SHALL report `Violated` at the first state in its search
  order where `I` is false or undefined.
- The closure checker SHALL resolve the invariant root in the recompiled
  package, rederive `I`, and reject the certificate with `BadState` at a
  product state whose model state makes `I` false or undefined.
- FR-128 SHALL replay the counterexample by its invariant rule and settle the
  item `refuted` only on `reproduced-with-evaluated-witness`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-951-AC-1 | Over FR-625-AC-4's Counter subject, `invariant AtMostThree using v on M::Counter at current { self.value <= 3 }` settles `proved`, basis `exhaustive`, `backend` `en-1`, certification `certified`; its certificate's product states carry the root `["invariant",N,"claim",O]`, `N` and `O` being the clause's `state_clause` node and `claim` occurrence ordinal, and the binding `[]`, and FR-338's checker accepts it. | Test (TC-2661) |
| FR-951-AC-2 | `invariant AtMostTwo` (`self.value <= 2`) over the same subject settles `refuted`, `backend` `en-1`, with a counterexample whose `prefix` ends at the state where `value = 3`, whose `loop` is empty and which carries no `over_binding`; its FR-128 replay settles `reproduced-with-evaluated-witness` with `trace_position` the last state's position, and the item's `replay.reproduced` is `true`. | Test (TC-2661) |
| FR-951-AC-3 | Negative: AC-2's counterexample truncated to end at `value = 2` replays to `inconclusive`, `ReplayParity`; with a non-empty `loop` added, or with `kind` `deadlock`, it refuses `invalid_runtime_input`/`invalid-value` before any step is replayed; AC-1's certificate with one product state whose model state has `value = 4` is rejected `BadState` at that state; and a certificate whose root names a `constraint` state clause is refused as a foreign clause root. | Test (TC-2661) |
| FR-951-AC-4 | Over the Counter subject, the invariant `self.value <= 6 mod (3 - self.value)` checked under `CheckMode::Linked` refuses `undefined_expression`/`unproved-nonzero` at the `mod` at check, as every declared clause does whatever the check mode (FR-495, FR-952), so no item reaches EN-1; the invariant `self.value <= 7 mod 4`, whose divisor is a nonzero literal, checks and settles `proved`. | Test (TC-2661) |

## Dependencies

- FR-625: the invariant item and its routing to EN-1.
- FR-338: the invariant key-context root and the closure check.
- FR-128: the invariant counterexample replay.
- FR-158: the truth rule over every object of the context type.
- FSpec FR-364: the finite-prefix counterexample shape.
