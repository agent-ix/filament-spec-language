---
id: SR-2026111612172400000018
title: "Code review — filament-spec-language QSL-2349"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-package/src/emit.rs, fsl-command/tests/it/main.rs, fsl-command/tests/it/short_circuit_integer_division.rs; FR-495, FR-062"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language/FR-495", type: references }
---

## Summary

Reviewed PR #2814 at `6fc8cd6518d4505208073794ca5c89d65fd9ca70`, including the emitter change and both new integration cases, against FR-495 and the repository's Rust conventions. The equality identity fix and guarded operator coverage behave as intended, but the new test duplicates the external `filament-examples` source, a P0 duplication finding under the code-review policy.

## Verdict

**FAIL** — the test embeds an example already defined in `filament-examples`, creating a second source of truth.

## Assurance Context

No applicable AssuranceProfile was found under `spec/` for the changed emitter and command integration behavior.

## Scope

- `fsl-package/src/emit.rs` — examined; Text/Plan equality operand type selection and identity validation.
- `fsl-command/tests/it/main.rs` — examined; integration module registration.
- `fsl-command/tests/it/short_circuit_integer_division.rs` — examined; new shop regression and guarded div/rem/mod cases.
- `FR-495-AC-3` — examined; excerpt: `function d5 using v(a: Int[0, 9], b: Int[1, 9]): Int[0, 9] pure { a div b }` and `if b != 0 then a div b else 0` over `b: Int[0, 9]` check.
- `FR-495-AC-17` — examined; excerpt: `function m1 using v(a: Int[0, 100], b: Int[1, 12]): Int[0, 11] pure { a mod b }` checks and the checked `mod` node has type `Int[0, 11]`.
- `FR-495-AC-18` — examined; excerpt: `q1` and `r1` with `b: Int[0, 12]` each refuse `undefined_expression`/`unproved-nonzero` at the operator, as their only refusal.
- `FR-495-AC-21` and `FR-062-AC-16` — context only; excerpts: AC-21 states the lowered application has the builtin `Integer` node as `result_type`; AC-16 states each bound's `ValueType` and type node name the same type.
- `filament-examples/examples/shop/rules/values/expressions/expr-short-circuit.fsl` — examined as the current authoritative source for the named shop example.

## Findings

| ID      | Severity | Summary                                                                                                  | Refs                                           |
| ------- | -------- | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| FND-001 | high     | The regression test embeds a second copy of the external shop example's source instead of consuming its authoritative artifact. | fsl-command/tests/it/short_circuit_integer_division.rs:55-60 |

## Review Notes

- The changed emitter uses the checked Text/Plan type nodes retained by `EqualityProducerIdentity`, verifies both operand ordinals and graph/emitted-node membership, and keeps ordinary lowered types for other equality schedules. This matches FR-062-AC-16 and the amended FR-495 contract for checked interval facts versus lowered Integer arithmetic applications.
- The shop case uses `x=7, y=0` and asserts exit 0 plus completed `false`. The external example at `filament-examples/examples/shop/rules/values/expressions/expr-short-circuit.fsl` contains the same source expression. The new test locally restates that program in `PROGRAM`; under the duplication rule, a focused test and provenance comment do not make a second copy acceptable.
- The second test exercises `div`, `rem`, and `mod` with `and`, `or`, and `implies`, each at `y=0` and `y=2`, and checks exact boolean outcomes. The targeted command passed both tests.
- No aggregate gate was run. Existing unrelated strict Clippy and repository-wide matrix noise was not attributed to this PR.

## Verdict Detail

The implementation change is narrowly scoped and the focused behavioral checks passed. The high-severity duplication finding still makes this review FAIL under the repository code-review policy.
