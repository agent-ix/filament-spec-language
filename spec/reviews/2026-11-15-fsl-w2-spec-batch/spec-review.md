---
id: SR-20261115234251000001
title: "Combined review of the FSL wave 2 spec readiness batch across the standard and the language"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language fsl-spec-w2 @ 6172a88d8 (PR #2728) and agent-ix/filament-specification fsl-spec-w2 @ 0f6a4f713 (PR #659), each `git diff origin/main...HEAD`; tickets QSL-2195, 2196, 1965, 2154, 2183, 2151, 2152, 2153, 2157, 2156, 2192 and the open questions of QSL-2121 and QSL-1964"
review_set: subset
---
# Combined review of the wave 2 spec readiness batch

## Summary

One reviewer read both commits as one batch. The lenses ran in this order: requirement quality, EARS conformance, integrity and cross-repo agreement, failure domain, and test-matrix coverage. Ticket text was read as data. Both branches are level with `origin/main`.

Verdict: CHANGES REQUESTED. There is one blocking finding and seven major ones.

## Method

- `quire validate --scope .` over every added or changed `spec/**/*.md` file in each repo passes. It gives only module warnings.
- `quire matrix --scope . --format tsv` was run in the language repo. Every new or changed language AC is `tagged` or `tagged-by-ignored-test`. The one exception is FR-640-AC-8, which is `method-without-symbol` (FND-009).
- Id collisions: FR-762 to FR-764 and TC-2148 to TC-2185 (language), and FR-662 and TC-1648 to TC-1650 (standard), are not on `origin/main` in either repo.
- Ticket-key, audit-path and "this batch" grep over the added requirement text: clean. Two wordings read as build-log text (FND-012).
- Narrowing check: the batch removes refusals (zero-weight components, strict clocks, form shapes, zero-delay cycles, time-locks, `on each`, multi-profile units, forms-stage null causes). It adds no permanent refusal. Continuous delays still settle `unsupported` for exact evidence only, and they route to statistical evidence. That is stated with its reason.
- Stubs: every new stub carries `#[trace]` with `FSpec-` prefixes on standard ids, and has an ignore reason keyed to a code ticket. The two un-ignored stubs, FR-762-AC-5 and FR-480-AC-19, assert behaviour that exists today.
- Cross-repo pairs:
  - FR-762 against FSpec FR-272-AC-30: they agree on code, locus, retained bound and refusal outcome. They disagree on the work-budget trigger (FND-004).
  - FR-764 against FSpec FR-662: they agree on each other. Both disagree with the language type rules (FND-001).
  - FR-204 against FSpec FR-421, and FR-199 against FSpec FR-411: they agree on regions for strict constraints, the `always` complement, the monitor product, discrete delays under every scheduler, transient delay-free end components, terminal time-locks, `delay-distribution` only for continuous delays, and the unweighted zero-weight fraction.
- QSL-1965 decision: `0.9` = `decimal(9, 1)`, `5.0` = `decimal(50, 1)`, `0.90` = `decimal(90, 2)`, with no reduction. This agrees with FR-091 `DecimalValue` and the FR-093 scale limits. The comparison typing does not hold (FND-001).
- QSL-2192's FR-027 claim was checked (FND-006).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Blocker. FR-764-AC-3 and FSpec FR-662-AC-2 say `n < 0.9` with `n: Integer` refuses `type-mismatch` at `0.9`, and that this equals the `decimal(9, 1)` result. The language gives a constructor a type only from a declared result or an arithmetic operand (FR-093). The ordering operators supply no expected type (FR-104). So `n < decimal(9, 1)` is `ambiguous-literal`, and so is the motivating `self.level < 0.9`. The stub's `decimal(9, 1)` arm would fail even after implementation. FSpec FR-140 gives a literal "the other operand's static type when that operand is not a literal". Fix: amend FR-093 "Decimal constructors" step 1 and the FR-104 ordering bullet to adopt FR-140's rule for comparison operands. Add an AC where `self.level < 0.9` over `level: D` checks. Settle the locus as either the literal or the comparison (FR-093:624 says the comparison). | FR-764:66-72,92; FR-093:433-445,624; FR-104:223-226; FSpec FR-140:129-136 |
| FND-002 | medium | FR-764-AC-6 refuses a 65536-digit threshold on `s3.decimal_literal_scale` in an exact rational position. The "Exact positions" bullet does not say the scale limits apply there. FR-093's order would refuse `ambiguous-literal` first in a position with no `Decimal[..]` type. Fix: state that in exact positions the literal is charged FR-093 steps 2 and 3, and that step 1 does not apply. | FR-764:73-78,95 |
| FND-003 | medium | The FR-763 `ConstructKind` table has no rows for `none` or `result`. FR-091 lists both as `UnrepresentedConstruct` sites, and FR-091-AC-7 refuses `none`. FR-763 says every site has a row and no refusal has a null payload. Fix: add `none-value` and `result-value` rows with messages, and extend AC-5 to cover them. | FR-763:70-78; FR-091:380-381,1237 |
| FND-004 | medium | FR-762 also uses `diagnostic-list-cut` when `s1.work_units` runs out during recovery. FSpec FR-272 and native-diagnostics define that cause only for "the diagnostic bound". This path, header resynchronisation and the no-resumption-point rule have no AC. Fix: widen the FR-272 row and the native-diagnostics row to "the diagnostic bound or the parse work budget, retaining which setting". Add FR-762 ACs and stubs for the work-budget cut and for header recovery. | FR-762:78,113-119; FSpec FR-272:66; native-diagnostics.md:61 |
| FND-005 | medium | Trace. Active, un-ignored tests still assert the superseded refusals and stay tagged to the amended ACs, so the matrix counts those ACs as covered by tests that contradict them. The tests: `zero_delay_cycle_and_form_shape_are_unsupported` (FR-204-AC-4), `a_strict_guard_is_unsupported` (AC-2), `discrete_delays_under_a_workload_are_a_chain` (AC-3, `DelayDistribution` under every scheduler), `a_later_guard_leaves_the_tie_as_the_maximum` (AC-7), the `ZeroWeightComponent` arm (FR-199-AC-3), and the FR-326-AC-4 two-profile test. Fix: remove those AC ids from the tags now, or rewrite the assertions as ignored stubs keyed to QSL-2156 and QSL-1964. | fsl-analyze/tests/it/digital_clocks.rs:178,392,425; free_delays.rs:299; weighted_long_run_certificate.rs:65,268-272; fsl-semantics/src/check/temporal_clause/tests.rs:308 |
| FND-006 | low | QSL-2192. The standard has no FR-027 and no empty-program rule. The contradicting rule is the language's own FR-027 ("An empty list refuses with `invalid-request`"). The ruling stands, but FR-480-AC-19 cites "FSpec FR-027". Fix: cite FR-027. | FR-480:264; FR-027:112 |
| FND-007 | medium | QSL-2192 item 3 (`--provider-config` shape and refusals) is not specified or pointed to anywhere in the batch, so QSL-2192 cannot be marked Specced in full. Fix: specify the file's shape and its refusals, or name the driver requirement that owns them. | QSL-2192 AC 4-5 |
| FND-008 | medium | Weak criteria. FR-204-AC-16 ("min and max lie in [0, 1]") cannot fail. AC-13 names no value. AC-2 and AC-4 accept "proved or refuted". Fix: state the exact minimum and maximum each model gives. | FR-204:236,238,247,250 |
| FND-009 | medium | FR-640-AC-8 is verified by Inspection and has no stub (`method-without-symbol`), although the replay refusal can be tested. Fix: make it a Test and add an ignored stub keyed to QSL-1964 for the `clocks` refusal and the identity equality. | FR-640:178; TC-2182 |
| FND-010 | low | The QSL-2121 question on monitor items without axes is not answered. FR-621 gives `axes` only "for an item that has an activation scope", and FR-286 says a `monitor` item carries axes. Fix: state whether every `monitor` item carries `axes`. | FR-621:100; FR-286:309 |
| FND-011 | low | Three stubs are thinner than their ACs. TC-2154 does not run the `parse` operation, so it does not check exit 20. TC-2167 does not check the instance count per `Msg` key. TC-2179 does not exercise the library assembler's `EmptyUnitList` half. Fix: add the missing assertions. | fsl-cst/tests/it/complete_cst.rs (TC-2154); fsl-analyze/tests/it/probability_from_source.rs:155; tests/it/compile_command.rs:1239 |
| FND-012 | low | Wording. FR-762-AC-5 says "gives today", and FR-764's rationale says "made the language harder", which are build-log phrasing. NFR-130 says "no frame-count ... limit" while the live-occurrence count counts frames. Fix: write "gives without recovery". Write the rationale in the present tense. Reword NFR-130 as "no limit on depth or frame count alone". | FR-762:139; FR-764:38; NFR-130:22 |

## Verdict

CHANGES REQUESTED.

FND-001 blocks: the bare-decimal decision does not hold against the language's typing rules. The rest is sound:

- The recovery design (FR-762) and its bound.
- The forms-stage payload (FR-763) apart from the two missing kinds.
- The live-occurrence limit. It is a resource count consistent with ADR-030 D-1, and its reason is stated.
- The `on each`, time-position and quantity-position rulings (FR-186), with their standard mirror FR-407.
- The timed and zero-weight rulings in both repos.
- The per-clause profile rule (FR-326) and the derived clocks (FR-640).
- The analyze wire answers (FR-620, FR-621).
- The id allocation.

## Dispositions (fix round 1)

| ID | Disposition | Where |
| --- | --- | --- |
| FND-001 | Fixed. A comparison operand takes its expected type from the other operand's static type when that operand is not a literal (FSpec FR-140); with no such operand the literal is `ambiguous-literal`, located at the literal. `l < 0.9` over `l: D` (as `self.level < 0.9`) checks. | language FR-093 "Decimal constructors" and step 1, FR-104 ordering bullet and AC-16 (`e2` locus), FR-764 S3 bullet and new AC-7, TC-2186, stub `a_comparison_operand_types_a_bare_decimal_literal`; standard FR-662 Behavior and AC-2, TC-1648 |
| FND-002 | Fixed. In exact positions the literal is charged FR-093 steps 2 and 3; step 1 does not apply. | FR-764 "Exact positions" |
| FND-003 | Fixed. `none-value` and `result-value` rows added; AC-5 covers them. | FR-763 table and AC-5, TC-2159, `tests/it/forms_refusal_cause.rs` |
| FND-004 | Fixed. Both standard rows widened to the diagnostic bound or the parse work budget, retaining which setting; FR-762 AC-8 (work-budget cut), AC-9 (header recovery), AC-10 (no resumption point). | standard FR-272 row and AC-30, native-diagnostics.md, TC-1649; FR-762, TC-2187, three stubs in `fsl-cst/tests/it/complete_cst.rs` |
| FND-005 | Fixed. Stale assertions moved into ignored stubs asserting the new behaviour, trace tags kept; still-valid assertions stay active. | `digital_clocks.rs` (zero-delay and `always`, FR-498-AC-3 pair, unbounded cycles, strict guard, discrete delay split, time-lock), `free_delays.rs` (AC-7 split), `weighted_long_run_certificate.rs` (zero-weight), `exact_probabilistic.rs` (strict RPC; FR-254-AC-3 amended), `unbounded.rs` unit test split, `temporal_clause/tests.rs` (FR-326-AC-4 split), `fsl-cst/tests/it/probabilistic.rs` (bare decimal split), `probabilistic_declarations.rs` (timed profile, not-yet forms, `expected elapsed`), `format_outcome_facade.rs` (`result`) |
| FND-006 | Fixed. Cites the language FR-027. | FR-480-AC-19 |
| FND-007 | Fixed. `--provider-config` shape and refusals are FSpec FR-561's, named in the driver contract. | FR-287 Behavior |
| FND-008 | Fixed. Exact values stated: strict guard minimum `99/100`, maximum `999/1000` (MeanTime maximum `20/9 ms`, infimum `10/9 ms`); zero-delay `Deadline` minimum and maximum `1`; `always` complement minimum `1/10000`, maximum `1/100`. | FR-204-AC-2, AC-4, AC-13, AC-16; TC-2172, TC-2175; stubs in `digital_clocks.rs` |
| FND-009 | Fixed. AC-8 is a Test: `clocks` rebuilt from the package rows compiles to identical bytes; `ReplayRequestWire` has no `clocks` member. | FR-640 replay bullet and AC-8, TC-2182, stub `clocks_rebuilt_from_package_rows_compile_the_same_package` in `tests/it/run_wire.rs` |
| FND-010 | Fixed. Every `monitor` item carries `axes`. | FR-621 `axes` row |
| FND-011 | Fixed. TC-2154 runs the `parse` operation (exit 20); TC-2167 checks the one instance against the `on origin` claim's basis; TC-2179 exercises `EmptyUnitList`, whose cause is `established-invariant-broken`. | `tests/it/parse_operation.rs`, `probability_from_source.rs`, `fsl-semantics/tests/it/dimension_equivalence.rs`; FR-186-AC-10, FR-480-AC-19 |
| FND-012 | Fixed. | FR-762-AC-5 ("without recovery"), FR-764 Rationale, NFR-130 Statement and Rationale |
