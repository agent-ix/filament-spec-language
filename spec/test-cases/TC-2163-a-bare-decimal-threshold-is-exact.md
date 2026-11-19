---
id: TC-2163
title: "A bare decimal threshold is the exact rational it spells"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-764
    type: verifies
---
# TC-2163: A bare decimal threshold is the exact rational it spells

## Description

A bare decimal threshold is the exact rational it spells.

Scope: FR-764-AC-5, FR-764-AC-6.

## Test Procedure

Check `expected accumulate R until holds(B) <= 0.5` over an integer reward, the same claim with `decimal(5, 1)` and `rational(1, 2)`, and with a threshold of `0.` followed by 65536 digits. Test: `fsl-semantics/tests/it/probabilistic_declarations.rs` `a_bare_decimal_threshold_is_the_exact_rational_it_spells`.

Tag the tests `#[trace("TC-2163", "FR-764-AC-5")]`.

## Expected Results

- The three claims check as `ExpectedReward` with `AtMost(1/2)` and are equal.
- The long threshold refuses `stage_limit_exceeded`/`work-budget-exceeded` naming `s3.decimal_literal_scale`, bound 65535, actual 65536, at the literal.
