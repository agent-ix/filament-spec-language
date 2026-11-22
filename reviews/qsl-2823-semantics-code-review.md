---
id: SR-967175026302197768900474181941935084745
title: "QSL-2823 QSL-2813 code-review of semantics expectations"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3480; QSL-2823 QSL-2813; eight semantics test and support paths"
review_set: subset
date: 2026-11-22
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Frozen source: `a581392bf7e7790e04a2b68aaca99cfb8f20b175`, PR #3480, QSL-2823 and assigned QSL-2813 semantics tests. This resumes the same first review after the initial missing trace-macro import was repaired; the initial pass was held before artifacts or comments. The only interim source change imports `ix_trace_rs::trace` in observation_composites. No reviewer build, test, repository executable, titan job, code edit or branch mutation ran. The lead supplied exact-target no-run PASS and all fifteen integration cases PASS at merged tree `f646b72173e13a5e3a051489c8c083b2153f33fc`, the two earlier exact Clock library cases PASS, and workspace check exit 0. These are supplied titan results, not reviewer executions. `git diff --check` passed.

All eight changed paths are tests or test support. No workflow, dependency, public API, production algorithm or budget changed. FR-982-AC-4's corrected canonical root landed in #3472 before the fixture edit (ancestry checked). The bare-native fixture now uses genuinely untyped Rational for ratio/by/qty and policy-bearing Decimal for amount/amounts; the malformed Rational payload still proves refusal before reading. Positive snapshots supply the required Decimal slots; missing-parameter and non-null-result refusals remain, and the new null-result control exercises the existing absent-result path. The collection test retains every malformed-kind row and asserts exact unknown-member, count, argument and nested path for the extra envelope field. The unit-constraint change checks the original typed refusal, D identity and exact operands.value member path. The README extraction still checks the complete rule and its three clauses. Clock alias tests keep signature/root/no-duplicate checks, require the exact sorted set and verify the canonical native target key; assembler insertion is confined to type admission/staging.

Duplication search covered existing native-value, model-operation, observation-composite and identity helpers: the new bare_native_fields helper owns its reused snapshot additions once; no copied product implementation, vendored data, identity reconstruction, mock replacement, guard that can skip assertions, new ignore or vacuous oracle was found. The existing ignored superseded aggregate fixture is unchanged; the active parenthesized-threshold test covers both profiles. The scoped trace finding is reported by gap-analysis. No applicable AssuranceProfile was found; textual hits are older review prose. Spec-review is inapplicable because this PR changes no specification or plan; optional separate semantic gap review was skipped. Requirement consistency was checked as part of the mandatory code/Rust lane.

## Verdict

PASS — no source or test-oracle defect found; the scoped trace binding requires the gap-analysis fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Static `quire matrix --scope /home/peter/dev/worktrees/qsl-2823-semantics-tests --format json` (quire 0.52.0, engine 0.62.2) returned 0; no runtime evidence was read. Static coverage also returned 0. Matrix: 5020 tagged, 1081 untagged, 68 tagged-by-ignored-test, 107 method-without-symbol, 15 retired. Coverage: 6208 untracked symbols, 7082 unmatched tags; Rust binding census 14440 candidates, 13865 tagged, 13268 bound. Diagnostics: 22853 trace-id-outside-marker, 607 untracked-id-has-minted-children, 22 uncatalogued-verification-method, 6 section-matches-nothing, 1 archetype-matches-nothing, 1 catch-all-universal. These are engine totals, not individually proven new defects.

The native `quoin matrix --repo /home/peter/dev/worktrees/qsl-2823-semantics-tests --json` attempt used `/home/peter/.npm-global/bin/quoin` 0.36.8 and returned 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. This matches the limitation already reported by the lead; the documented static fallback above was used. No native matrix or runtime-backed PASS is claimed.

FR-056-AC-43/66, FR-106-AC-68, FR-982-AC-4 and FR-480-AC-9/11 have active binders. FR-230-AC-26 is untagged: its text in the changed unit_bounds test doc comment is an unmarked claim and its attribute has only TC-685. The inherited old marker is confirmed in origin/main.

Plan completion: not assessed

Reverse/stub inspection followed eight changed paths, six changed behavior groups (native members/snapshots/invocations, aggregate threshold, envelope ordering, unit constraints, README extraction, alias environments), four shared-fixture consumers, and the real intake/observation/assembler seams they exercise. No new production behavior exists to inventory; no unspecified changed behavior, source stub behind these claims, test stub or coverage-inflating replacement was found. A whole-repository reverse behavior census was not completed; the repository-wide assurance claim therefore remains incomplete. The inherited gap finding does not direct this PR to repair unrelated criteria.

Changed files examined:

- `fsl-semantics/src/check/assemble/tests.rs`
- `fsl-semantics/tests/it/aggregate_threshold.rs`
- `fsl-semantics/tests/it/bare_native_members.rs`
- `fsl-semantics/tests/it/native_snapshot_values.rs`
- `fsl-semantics/tests/it/observation_composites.rs`
- `fsl-semantics/tests/it/readme_first_rule_checks.rs`
- `fsl-semantics/tests/it/unit_typed_members.rs`
- `fsl-semantics/tests/support/native_values.rs`

## Examined scope

- FR-056-AC-43 — spec/functional/FR-056-admit-domain-package-model-declarations.md; role: examined; excerpt: Adding, one at a time: the invariants `self.memo = self.memo` and `self.ratio = self.ratio` each refuse `ill_typed`/`operator-ineligible` at the projection naming `Note` and `Field.typeRef`, while `self.amount = self.amount` and `self.amounts = self.amounts` check, because `amount` and the elements of `amounts` are Decimal value types `Decimal[-99999, 99999; 2, 2; exact]` (FR-056-AC-66)
- FR-056-AC-66 — spec/functional/FR-056-admit-domain-package-model-declarations.md; role: context_only; excerpt: Admitting and assembling the golden package `tests/fixtures/decimal-value-types.semantic-ir.json` gives each member exactly the type `tests/fixtures/decimal-value-types.expected.json` lists, compared as `DecimalType` bounds, scales and mode: `Meter.soc` (`Soc`, `scalar` `decimal`, policy `(5, 2)`, `min` `"0"`, `max` `"1"`) is `Decimal[0, 100; 2, 2; exact]`; `Meter.price` (`(10, 2)`, no bound) is `Decimal[-9999999999, 9999999999; 2, 2; exact]`; `Meter.margin` (`(4, 1)`, `exclusiveMin` `"-1"`, `exclusiveMax` `"1"`) is `Decimal[-9, 9; 1, 1; exact]`; `Meter.tax` (`(3, 3)`, `min` `"0.125"`) is `Dec
- FR-106-AC-68 — spec/functional/FR-106-admit-snapshots-and-invocations.md; role: examined; excerpt: Over TC-1171's fixture, `ids` written `{"set": [], "extra": 1}` refuses `unknown-member` naming `extra`, and `counts: Bag<Int[0, 9]>[0, 4]` written `{"bag": [], "set": []}` refuses `unknown-member` naming `set`; the first other member in document order is named. This is the native counterpart of FSpec FR-144-AC-16's extra member.
- FR-230-AC-26 — spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; role: examined; excerpt: A constraint `min` `"0"` and `max` `"10"` whose `appliesTo` is the alias `Level` of `ix://filament/unit/ms` gives `Level` the domain of the quantities of `ms` from `0` to `10`: `5 ms` checks and `11 ms` refuses `ill_typed`/`type-mismatch` at the literal; a keyword `minLength` on it refuses `invalid_model_binding`/`malformed-declaration`; a package `reference` whose `target` is `ix://filament/unit/ms` refuses `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.target`.
- FR-982-AC-4 — spec/functional/FR-982-end-a-window-aggregate-threshold-at-the-next-temporal-connective.md; role: examined; excerpt: Under the timestamped-event finite-window profile, on a clock `"events"` over a unit declaring `dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;`, the same two formulas with the window `past[0 ms, 1000 ms]` check and refuse identically. No formula carries an interval-less temporal operator, which neither profile admits (FSpec FR-090-AC-7, FR-048-AC-9).
- FR-480-AC-9 — spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md; role: examined; excerpt: With unit `a` selecting the package as `Config` and unit `b` as `History`, `b`'s function over `History::ConfigVersion` and `History::VersionNumber` calls `a`'s function over the `Config` spellings, both have one signature, and the package has one object type and one `model_root`.
- FR-480-AC-11 — spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md; role: examined; excerpt: Through a second alias of the FR-360 package, in one unit and in a second unit, a variant case, a record field read, an object type and an operation context resolve to the first alias's declarations, and each variant type is declared once. A subtype's inherited fields and operation resolve through a second alias.
- FR-1040-AC-1 — spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md; role: context_only; excerpt: Over FSpec FR-415-AC-15's `Call` model, `ref(self.x)` and `ref(self.q[0])` (under `size(self.q) >= 1`) in a postcondition write `self.r = ref(self.x)` and in a precondition `self.r = ref(self.q[0])` check as `PlaceReference` nodes typed `Reference<Clock>` and lower to `filament.op.clock_reference.ref` applications with `result_type` the `scalar_type`/`clock_reference` node, and the unit compiles with no `unsupported-feature` refusal.
- FR-1040-AC-7 — spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md; role: context_only; excerpt: A pure function `same(a: Reference<Clock>, b: Reference<Clock>): Boolean { a = b }` has a function node whose parameters are typed by the `scalar_type`/`clock_reference` node, and a refinement row or clause calling it lowers its call to that node; with the function's body made ill-typed, the unit refuses once at the function body and not at the call.
- FR-1040-AC-8 — spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md; role: context_only; excerpt: Over a domain package `acme/rpc` whose object type `Call` has clock fields `x` and `y` and a field `r` typed `ix://acme/rpc/ClockRef`, a type definition of `kind` `reference` with `target` `ix://filament/native/Clock`, intake gives `r` the type `Reference<Clock>` as a data member, and the postcondition `self.r = ref(self.x)` of an operation `point` whose frame names only `r` checks with no `clock-write` refusal and lowers the write to a `filament.op.clock_reference.ref` application typed by the `scalar_type`/`clock_reference` node, which also types the field; after `point` the state's `r` hold
- TC-685 — spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md; role: examined; excerpt: Tag the tests `#[trace("TC-685", "FR-230-AC-n")]`.
- README.md — README.md; role: examined; excerpt: This unit states three rules about the `ConfigVersion` objects of the domain model `example/config-version`:
- spec/spec.md — spec/spec.md; role: examined; excerpt: name: filament-spec-language

## Dispositions

Round 1, confined delta `a581392bf7e7790e04a2b68aaca99cfb8f20b175..320505fe7ae69fe3d75556dbb2df8c7ae7302fe4`: the only source change adds the direct `FR-230-AC-26` trace to the existing `TC-685` attribute. There were no code/Rust findings to dispose and no new findings are introduced. Assertions, fixtures, production paths and test selection are unchanged. The scoped code review remains PASS.

The lead-supplied `/tmp/qsl-3480-fr230-trace.json` records exit-0 quire trace output, `resolved: true`, exactly one structural `claims.verifies` binder for `unit_bounds_bound_the_members_of_their_unit_or_alias` at the changed path, and five separate citations which are not counted as binders. No full matrix, test or compile rerun was performed for this attribute-only head. Supplied earlier evidence remains exact-target no-run, all seventeen named cases and workspace check PASS at source `a581392bf7e7790e04a2b68aaca99cfb8f20b175`; final source `320505fe7ae69fe3d75556dbb2df8c7ae7302fe4` has static binder verification only.
