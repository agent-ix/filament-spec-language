---
id: TC-3060
title: "An on-each product carries its instance and activation phase"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-196
    type: verifies
---
# TC-3060: An on-each product carries its instance and activation phase

## Description

The probabilistic product of an `on each` claim keys every state by its
instance and activation phase, decides vacuity by reachability and computes
the conditioned value, under every scheduler through the reset process.

Scope: FR-196-AC-15, FR-196-AC-16.

## Test Procedure

1. Build the product of FR-875's `Link` claim `E` under `Even` over a start
   snapshot holding `m` undelivered and `n` delivered; read its initial keys,
   the keys reachable from `n`'s, and `m`'s numerator and denominator.
2. Parse and write a `post-state` product action in the single-root,
   no-reset fixture whose key has the original eight
   members with `instance` `m` and `activation` `"waiting"`, the same with
   `"active"`, the same with `instance` `n`, and an `on origin` key with both
   null; then a key with the six members of the former form. Tests:
   `fsl-replay/tests/it/conditioned_product_key.rs`
   `an_on_each_key_carries_its_instance_and_phase`,
   `a_six_member_key_is_refused`.
3. Over FSpec FR-407-AC-18's single-root model with the extra action `stay` at `s0`,
   derive the reset process and compute the conditioned minimum and maximum;
   over the model without `stay`, compute the workload value.

Tag the tests `#[trace("TC-3060", "FR-196-AC-15")]` and
`#[trace("TC-3060", "FR-196-AC-16")]`.

## Expected Results

- Step 1: initial keys (`m`, `"active"`) and (`n`, `"waiting"`); every key
  reachable from `n`'s is waiting, so `n` is vacuous; `m`'s value is
  `391/400` over 1.
- Step 2: each original eight-member no-reset key round-trips byte for byte, the waiting and
  active keys differ, the two instances differ; the six-member key is
  refused `Shape`.
- Step 3: minimum `1/2`, maximum 1, never 0; workload value `3/5`.
