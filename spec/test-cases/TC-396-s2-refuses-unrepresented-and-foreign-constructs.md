---
id: TC-396
title: "S2 refuses unrepresented constructs, and the check stage refuses another family's construct with that family's cause"
type: TC
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-091
    type: verifies
---
# TC-396: S2 refuses unrepresented constructs, and the check stage refuses another family's construct with that family's cause

## Description

Verify two things. S2 refuses the whole unit, and never yields a partial
form, for a construct with no `Expression` variant (ADR-011 §2.3 E2). And
S2 builds the `forms`-core variant for `deref`, `pre` and `allInstances`
nested in a function body, which the check stage then refuses with the
owning family's catalogued cause.

This catches a production that refuses another family's construct at S2
with `unsupported_construct`. That would discard the `forbidden-pre-read`
distinction that FR-090 assigns to `ProtocolClause`.

Scope: FR-091-AC-7, FR-091-AC-8, FR-091-AC-39.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. For each body below, build an admissible unit whose one function body is
   that construct, and run S2: `xs[0]`; `size<Int[0, 1]>(c)`; `none`;
   `reaches(a, b, M::R)`.
2. Run S2 on `a div b`, `a rem b`, `a mod b` and
   `if true then 1 else a mod b`, each under a state-only declaration
   without `filament.value.complete/v1`; pass the resulting forms to check.
3. For each body `B` of `deref(x)`, `allInstances<M::T>(x)` and `pre(x)`,
   build an admissible unit, with no admitted domain package, whose one
   declaration is `function f using v(x: Int[0, 9]): Boolean pure { B }`.
   Do the same for
   `function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }`.
   Run S2, then the assembler, then `PackageDeclarations::check` on
   whatever the assembler returns.

Tag the test `#[trace("FR-091-AC-7", "FR-091-AC-8", "FR-091-AC-39")]`.

## Expected Results

- In step 1, `xs[0]` builds as `Index` with its operand, index and exact
  span (FR-680); `size<Int[0, 1]>(c)` builds as `Size` with its operand,
  result type and exact span (FR-981). Test:
  `value_forms::formerly_unrepresented_value_constructs_build_with_their_spans`.
  `none` builds as `Absent` (FR-091-AC-76). `reaches(a, b, M::R)` builds
  as `Reaches` with both operands and the entire edge name and span;
  S2 does not resolve `M::R`. The latter admission is asserted by
  `value_forms::a_qualified_edge_in_a_function_builds_with_its_name_and_span`.
- Step 2 returns parsed forms with distinct operators and original spans.
  Check then refuses each with located `unsupported_construct`, including
  the dead branch's `mod`; it never returns `UnrepresentedConstruct` at S2.
- In step 3, S2 returns a parsed unit holding `Deref`, `AllInstances` or
  `Pre` where the construct is written.
- `pre(x)`, in `f`'s body and in `g`'s measure, refuses at check with
  `wrong_snapshot`/`forbidden-pre-read`.
- `deref(x)` refuses at check with `ill_typed`/`type-mismatch`.
- `allInstances<M::T>(x)` refuses in the assembler with an
  unresolved-type-name error naming `M::T`, code
  `missing_declaration`/`missing-name`.
- No step-3 refusal has code `unsupported_construct`.
