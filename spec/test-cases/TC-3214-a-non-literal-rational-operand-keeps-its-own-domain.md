---
id: TC-3214
title: "A non-literal Rational operand keeps its own domain"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-495
    type: verifies
---
# TC-3214: A non-literal Rational operand keeps its own domain

## Description

Verify that a Rational arithmetic operand takes its peer's domain as a hint only when it is a literal, so a nested non-literal operand is not narrowed to its sibling's domain (FR-495). Scope: FR-495-AC-27.

## Test Procedure

1. Check, under `CheckMode::Linked` and under `CheckMode::Kernel`, each of `a + (a + c)`, `a * (a + c)`, `a + (a * c)`, `(a + a) + c` and `(a + c) * a` as the body of a function over `a` and `c` of `Rational[-20, 20; 1, 9]` with the result type `Rational[-10000, 10000; 1, 1000]`.
2. Check `a + rational(1, 2)` under `CheckMode::Linked` over the same operands.

## Expected Results

- Step 1: every body checks under both modes.
- Step 2: the body checks.

Test: `fsl-semantics/tests/it/check_mode.rs`
`a_non_literal_rational_operand_keeps_its_own_domain`, tagged `#[trace("FR-495-AC-27", "TC-3214")]`.
