---
id: TC-3012
title: "Probabilistic declarations on the source model are emitted"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-740
    type: verifies
  - target: ix://agent-ix/filament-spec-language/FR-569
    type: verifies
---
# TC-3012: Probabilistic declarations on the source model are emitted

## Description

Verify that S4 emits a delay, a random parameter and a workload declared on
an object type the unit declares in its own source, that each operation
reference names the type's source `model`/`object_type` node, and that the
strict reader and the identity-row join admit the package. Scope:
FR-740-AC-5, FR-740-AC-6, FR-569-AC-18, FSpec FR-640-AC-6.

## Test Procedure

1. Compile, with no selected domain package, `object O { d: Clock; operation
   step() modifies self.d; }`, `object Other { n: Integer; operation poke()
   modifies self.n; }`, `time O dense unit ms;`, `delay O::step ~ uniform;`
   and `delay Other::poke ~ discrete { 1 ms: 9, 3 ms: 1 };`. Emit the package,
   read it strictly and run the identity-row join.
2. Compile step 1's unit without its `time` member.
3. Compile, with no selected domain package, `object O { n: Integer;
   operation step(k: Int[1, 3]) modifies self.n; }`, `random O::step(k) ~ {
   1: 1, 3: 3 };` and `workload W on O { weight O::step = 1; }`; emit, read
   strictly, join the identity rows and rebuild the probabilistic records.
4. Compile `object O { d: Clock; n: Integer; operation arm(x: Clock, k:
   Int[1, 3]) modifies self.d; }` and serialize its identity rows.

Tag steps 1 and 2 `#[trace("TC-3012", "FR-740-AC-5")]` and
`#[trace("FSpec-TC-2033", "FSpec-FR-640-AC-6")]`, step 3
`#[trace("TC-3012", "FR-740-AC-6")]` and
`#[trace("FSpec-TC-2033", "FSpec-FR-640-AC-6")]`, step 4
`#[trace("TC-3012", "FR-569-AC-18")]`.

## Expected Results

- Step 1: two `delay_distribution` nodes; one `declaring` references the
  source node of `O` (`declaration` `["O"]`), the other that of `Other`, whose
  delays `1` and `3` are typed by the unit node of `ms`; the strict read and
  the identity-row join admit the package.
- Step 2: S3 refuses `invalid_probabilistic_model`/`delay-on-untimed-model` at
  the delay's span.
- Step 3: one `random_parameter` and one `workload` node, `declaring` and
  `model` the source node of `O`; both reads admit the package; the rebuilt
  `RandomParameter` has probabilities `1/4` and `3/4` and the `Workload`
  equals S3's.
- Step 4: `O`'s `operations` entry for `O::arm` has `parameters` the clock
  entry `x` then the data entry `k`, and `clock_parameters` `["x"]`.
