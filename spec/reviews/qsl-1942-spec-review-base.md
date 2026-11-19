---
id: SR-202611141401001
title: "Base review of the QSL-1942 JIT requirement fixes"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #2704 at 8ef45a09e17ae92270ebd7a973209a86e40f05d3, diff origin/main...HEAD: spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md (CB-3 item 3, §7 backend table, EB-6), spec/functional/FR-294-run-every-execution-backend-behind-one-seam.md, spec/functional/FR-295-return-interpreter-outcomes-from-jit-and-aot.md, spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md (AC-2), spec/functional/FR-562-take-a-machine-width-fast-path-only-where-checked-bounds-prove-the-result-fits.md (AC-3, AC-5), spec/functional/FR-563-run-the-parity-gate-and-fail-on-any-difference-from-the-interpreter.md"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-560, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-562, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-295, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-294, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-563, type: references }
  - { target: ix://agent-ix/quire-spec-language/ADR-029, type: references }
---

## Summary

Ticket: QSL-1942. Base checklist over the four fixes the ticket names: bounded types in
FR-560-AC-2 and FR-562-AC-5, the conformance-corpus definition in FR-295-AC-1, one AOT owner
across FR-294, FR-295, FR-563 and ADR-029 EB-6, and the re-anchoring of FR-562-AC-3 away from
the FR-495-AC-3 integer `d1` refusal.

Every changed AC fixture was run through S1 (`qsl_cst::parse`), S2 (`qsl_forms::build_unit`)
and the S3 package check (`PackageDeclarations::assemble(..).check(CheckingLimits::default())`)
at 8ef45a09e, with a scratch harness outside the tree (the same path
`qsl-analyze/tests/it/value_validity.rs::package_with_source` uses, Euclidean division
admitted). Results:

| Fixture | Result |
| --- | --- |
| FR-560-AC-2 `add(a: Integer, b: Integer): Integer { a + b }` | checks |
| FR-560-AC-2 `half(a: Rational[-9, 9; 1, 9]): Rational[-5, 5; 1, 18] { a / 2 }` | refused `IllTyped(TypeMismatch)` |
| FR-560-AC-2 `name(t: Text[0, 8; nfc]): Text[0, 8; nfc] { t }` | checks |
| FR-562-AC-5 `r(a: Rational[-9, 9; 1, 9], b: Rational[-9, 9; 1, 9]): Rational[-18, 18; 1, 81] { a + b }` | refused `Unproved(RationalRange)` |
| FR-562-AC-5 `t(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean { a = b }` | checks |
| FR-562-AC-5 `u(a: Integer, b: Integer): Integer { a + b }` | checks |
| FR-562-AC-3 `d(a: Int[0, 9], b: Int[1, 9]): Integer { a div b }` | checks |
| FR-562-AC-3 `d0(a: Int[0, 9], b: Int[0, 9]): Integer { a div b }` | checks (no nonzero refusal) |
| control: old bare `Rational`, `Text` and `Decimal` parameters | S1 `InvalidSyntax` "expected `[`" |
| control: `r` with result `Rational[-162, 162; 1, 81]` | checks |
| control: `half(a: Rational[-9, 9; 1, 9], h: Rational[1, 1; 2, 2]): Rational[-9, 9; 1, 18] { a * h }` | checks |
| control: `half(a: Rational[-9, 9; 1, 9], b: Rational[2, 2; 1, 1]): Rational[-9, 9; 1, 18] { a / b }` | checks |

So the ticket's premise holds (bare `Rational`/`Text` do not parse), but two of the new
bounded fixtures still do not check.

FR-495-AC-3 integer `d1` evidence (ticket item 4): the refusal is not implemented. The
Definedness pass (`qsl-semantics/src/check/facts.rs`, `fn discharge` at line 1184) has
nonzero arms for `NodeKind::Divide` (line 1201), `Rational` (1247), `Decimal` (1277),
`Quantity` (1300) and `QuantityPower` (1310), and no arm for `NodeKind::IntegerDivision`
(`div`, `rem`, `mod`), which falls through to `_ => Ok(())` at line 1502. Run through the
checker, `d1 ... : Int[0, 9] { a div b }` (and `rem`, `mod`) is refused
`Unproved(Range { required: [0, 9], proved: unbounded })`, not `unproved-nonzero`, because
the interval walk also has no `IntegerDivision` arm; `d0` with an `Integer` result checks.
The existing test `qsl-analyze/tests/it/value_validity.rs` fixture
`function q using v(a: Int[0,3], b: Int[0,1]): Integer pure { a div b }`
(`provider_external_contract.rs` line 32) expects the package check to succeed. The only
`FR-495-AC-3` trace tags (`qsl-eval/tests/it/quantity_power.rs` lines 143, 182) cover the
quantity rows, not `d1`. `PackageDeclarations::check` takes no `CheckMode` today, so
"checked under `CheckMode::Kernel`" in FR-562-AC-3 and AC-6 is also not yet a selectable
input in code. The re-anchored AC-3 no longer depends on the `d1` refusal, which is the
correct spec-side fix.

## Verdict

FAIL on FND-001 and FND-002 (high): FR-560-AC-2 and FR-562-AC-5 still name fixtures that the
checker refuses, so no backend ever sees them and neither AC can pass. FND-003 is medium,
FND-004 low.

Examined and clean:
- FR-295-AC-1 and the FR-295 corpus paragraph: the corpus is FR-563's `runtime`/`run`
  vectors, each one case with one decoded argument tuple, which matches FR-563 "Parity cases"
  and FR-563-AC-9.
- FR-562-AC-3: `d` and `d0` both check; under `CheckMode::Kernel` FR-562's own prose
  (lines 63-65) already says no nonzero obligation is recorded and a divisor interval
  containing 0 calls the kernel, so the AC is consistent with its FR and no longer rests on
  FR-495-AC-3.
- `name`, `t`, `add` and `u` check as written.
- QSL-602 decisions: decision 1 (AOT may be owned by either lane; QSL may take it) is what
  the amendment records, and the ADR states that it replaces the 2026-11-08 amendment.
  The accepted design items 2 to 5 (trust rule, heap frames, kernel shims,
  `environment-refused`) are not touched by this diff; nothing recorded on QSL-602 is
  erased. Driver FR-008 on quire-driver main already says the driver invokes "QSL's AOT
  backend", so the cross-repo pair now agrees on the owner.

Skipped: none in this lens.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-560-AC-2 `half(a: Rational[-9, 9; 1, 9]): Rational[-5, 5; 1, 18] { a / 2 }` is refused `IllTyped(TypeMismatch)` by the checker (a `Rational` divided by an integer literal); the result bound also cannot hold `9/2` (numerator 9 outside `[-5, 5]`). The JIT never sees it, so "lists none of the three unsupported" cannot pass. `a * h` with `h: Rational[1, 1; 2, 2]` and result `Rational[-9, 9; 1, 18]` checks. | spec/functional/FR-560-translate-the-jit-coverage-set-and-report-every-other-construct-untranslatable.md:178 |
| FND-002 | high | FR-562-AC-5 `r(a: Rational[-9, 9; 1, 9], b: Rational[-9, 9; 1, 9]): Rational[-18, 18; 1, 81] { a + b }` is refused `Unproved(RationalRange)`: a bounded rational bounds numerator and denominator, and the sum's numerator reaches 162 (`9/8 + 9/7 = 135/56`). With result `Rational[-162, 162; 1, 81]` it checks. | spec/functional/FR-562-take-a-machine-width-fast-path-only-where-checked-bounds-prove-the-result-fits.md:118 |
| FND-003 | medium | ADR-029 §7 AOT row and EB-6: AOT `prepare` "builds the artifact the driver's `lower` and `generate` with the AOT target emitted", and the driver "pass[es] their artifact to the backend", but `ExecutionBackend::prepare` (FR-294) takes only `&CheckedPackage`, entries, `PrepareLimits` and `&Cancel`, and `quire-aot` may depend on no driver crate. No seam input carries the artifact or the caller-named toolchain, and driver FR-008-AC-1 passes the checked package, entries, limits, cancellation and toolchain only. | spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:827, spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md:883 |
| FND-004 | low | FR-295 "The parity program generator" Types now reads "bounded `Int`, bounded `Rational`, `Decimal` and bounded `Text`": `Decimal` was left bare, and bare `Decimal` does not parse (S1 "expected `[`"), the same defect the ticket fixed for `Rational` and `Text`. | spec/functional/FR-295-return-interpreter-outcomes-from-jit-and-aot.md:76 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | edd5dee65: FR-560-AC-2 `half(a: Rational[-9, 9; 1, 9], h: Rational[1, 1; 2, 2]): Rational[-9, 9; 1, 18] { a * h }`; checks through S1, S2 and the S3 package check at 3965516954 (probe rerun). |
| FND-002 | fixed | edd5dee65: FR-562-AC-5 `r` result is `Rational[-162, 162; 1, 81]`; checks at 3965516954 (probe rerun). |
| FND-003 | fixed | 396551695: `quire-aot` is constructed with an injected generator trait and toolchain; `prepare` keeps FR-294's signature and calls the generator (ADR-029 §7 row and EB-6). |
| FND-004 | fixed | edd5dee65: FR-295 Types reads "bounded `Rational`, bounded `Decimal` and bounded `Text`". |
