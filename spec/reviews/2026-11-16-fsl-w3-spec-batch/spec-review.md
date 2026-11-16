---
id: SR-20261116013603000001
title: "Combined review of the FSL wave 3 spec readiness batch across the standard and the language"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language fsl-spec-w3 @ 06f98544 (PR #2733) and agent-ix/filament-specification fsl-spec-w3 @ c2f50aab (PR #660), each `git diff origin/main...HEAD`; tickets QSL-1989 and QSL-2003"
review_set: subset
---
# Combined review of the wave 3 spec readiness batch

## Summary

One reviewer read both commits as one batch. The lenses ran in this order: requirement quality, EARS conformance, integrity and cross-repo agreement, failure domain, and test-matrix coverage. Ticket text was read as data. Both `origin/main` refs match `git ls-remote`.

Verdict: CHANGES REQUESTED. There are two blocking findings and three medium ones.

## Method

- `quire validate --scope .` over every changed `spec/**/*.md` file in each repo passes. It gives only module warnings and one `ac:non-canonical-shape` warning on an existing FR-145 row.
- `quire matrix --scope . --format tsv` was run in the language repo. FR-767-AC-1 to AC-6 and FR-766-AC-1, AC-2, AC-3 and AC-5 are `tagged-by-ignored-test`. FR-766-AC-4 is `tagged` by an existing passing kernel test, which is correct because that behaviour exists today. FR-767-AC-7 is `untagged` (FND-003).
- Id collisions: FR-767, FR-766 and TC-2188 to TC-2195 (language), and TC-1651 to TC-1653 (standard), are not on `origin/main`. The new AC rows FR-570-AC-6, FR-301-AC-6 and FR-145-AC-16 are new in the standard.
- Trace tags: standard ids carry the `FSpec-` prefix, and every ignore reason is keyed to QSL-1989 or QSL-2003. Nothing in the language tree traces `FSpec-FR-301-AC-6`.
- Cross-repo pairs. FR-767 and FR-670 agree with FSpec FR-570 and FR-301 on the law names, the absence of a default, header composition, `conflicting-definition` at the header literal, library isolation and the unknown-name refusal. FR-766 and the FR-096 note agree with FSpec FR-145 on the `[0, m]` interval, the lower bound of `0`, guard narrowing, `unproved-range` under linked checking and `integer_out_of_domain` under kernel evaluation.
- QSL-2003 outcome. `count` gets a run-time refusal where `sum` gets an undefined outcome. This is consistent and justified. A `sum` fold has running totals in `N`. A `count` has only a final retain into `N`, which is FSpec FR-044's domain retain. This matches the existing FR-096-AC-8 behaviour.
- Flag surface. Offering `--integer-division` on `check` only, with `compile` and `run` reading the `integer_division` member, follows the existing `--extensions` convention: request verbs take their inputs from the request file. The law still reaches `compile` and `run`, so the split does not narrow those verbs. The narrowing is in the request formats (FND-001).
- Heads-up on `count<Zero>`: confirmed (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | This narrows the feature. Only `native-compile/1` and `native-run/1` carry `integer_division`. `native-analyze/1`, which serves `analyze` and `prove`, also compiles its program from source, and so does `native-monitor/1`. Both requests are closed, so a unit with `div` or `rem` can never be analyzed, proved or monitored from the command line. The FSpec FR-301 bullet is limited to `compile` and `run`. Fix: add a top-level and per-library `integer_division` member to the FSpec FR-520 Document members table, to the FR-620 reading rules and to the FR-644 Inputs. Reword the FSpec FR-301 and FR-767 text to "every request that compiles a program". Add ACs and ignored stubs for an analyze request and a monitor request that use the member. Keep the flag on `check` only. | FR-767:44-50; FSpec FR-301:103-110; FSpec FR-520:53; FR-620:67-75; FR-644:48 |
| FND-002 | high | The heads-up holds. The `R` atom of the D0 to D24 differential clauses is `count<Zero>(m in ints(sequence[c.value]): m > 0)`, where `Zero` is `Int[0, 0]` over a `Sequence<Integer>[0, 3]` source. The separating-witness RefusedDomain case uses the same construct, and so does FR-327-AC-6's refusing monitor clause. Clause runs compile under `CheckMode::Linked`, so FR-766 refuses each of these whole units at compile, with proved `[0, 3]`. Their run-time `integer_out_of_domain` path is lost. Neither FR-766 nor the stubs say how these fixtures change. Fix: FR-766 must name a construct that still reaches an S6a domain refusal under linked checking, or state that none exists and move those ACs to a kernel-mode route. List the fixture migration in an FR-766 note or in the ignore reasons of the QSL-2003 stubs. | fsl-replay/src/spine/clause/tests/temporal.rs:35-141,363-473; witness.rs:84,1736; fsl-analyze/tests/it/monitor_inputs.rs:45-55,478 |
| FND-003 | medium | FR-767-AC-7 and FSpec FR-301-AC-6 are verified only by the driver. They are marked `Test`, have no stub, and are untagged. The language half of the option has no AC: the FR-275 `check` operation taking a selection, every fence of `check --markdown`, and the top-level law reaching each unit of a multi-unit program. Fix: write the Verification as "Driver test (TC-2189/TC-1651): QSL-1989 adds the test in filament-driver", following the FR-624-AC-4 precedent. Add a language AC with an ignored stub at the `check` operation for a single unit, a Markdown fence and a multi-unit program. | FR-767:123-137,170; FSpec FR-301:143 |
| FND-004 | medium | The unknown-law message template in FR-767 does not include the value's source. The stub asserts that every refusal message contains `integer_division`, so it would fail against a correct implementation. Fix: add "supplied by `<source>`" to the template, or have the stub assert the locus and not the message. | FR-767:100-104; fsl-command/tests/it/integer_division_request.rs:183 |
| FND-005 | medium | Two FR-766 behaviour bullets have no AC: the clause-run item's category, result and cause, and the rule that a clause-run kernel refusal never projects a `null` cause. The `null` cause is the defect QSL-2003 reports. AC-5 checks only the compile refusal. Fix: add an AC with an ignored stub asserting the run item `{refusal, declined, undefined_expression/unproved-range}`, and an AC on the projection over a kernel refusal record. | FR-766:95-101,111 |
| FND-006 | low | The stubs are thinner than their ACs. AC-1 asserts non-empty bytes, not the law role in the package selection. AC-4 checks only the first diagnostic, not every `div` and `rem`. The `q14_count_proves_when_the_bound_fits` control passes today but is marked ignored. The AC-1 test does not tag FSpec FR-570-AC-6, whose first clause it covers. Fix: decode the package and assert the selection's role, check every diagnostic, remove the ignore from the control, and add the tag. | integer_division_request.rs:96-125,201-215; fsl-eval/tests/it/collection_queries.rs:2134-2146 |
| FND-007 | low | Two wording problems. The FR-766 rationale says a domain omitting `0` is refused "unless a guard proves ... in a way this requirement does not read", so the exception can never apply. FR-767 says the `check` operation "without one ... behaves as today", which is build-log wording. Fix: delete the "unless" clause, and write "with no caller selection it checks under the header law alone". | FR-766:43-47; FR-767:134-137 |

## Dispositions

| ID | Disposition | Resolution |
| --- | --- | --- |
| FND-001 | fixed | `integer_division` (top level and per library) is now admitted on every request that compiles a program: FSpec FR-520 Document members (row and `libraries` shape) and new FR-520-AC-5 (TC-1654), FR-620 Inputs, FR-644 Inputs; FR-767 and FSpec FR-301 say "every request that compiles a program". New FR-767-AC-9 (analyze) and AC-10 (monitor), TC-2196, ignored stubs `fsl-analyze/tests/it/analyze_request_wire.rs::an_analyze_request_supplies_the_integer_division_law` and `tests/it/run_wire.rs::a_monitor_request_supplies_the_integer_division_law`. The flag stays on `check` only; it now refuses as an unknown option on `analyze`, `prove` and `monitor` too. |
| FND-002 | fixed | The linked check is kept. FR-766 Notes say no linked integer construct reaches S6a `integer_out_of_domain`, so a fixture that needs a run-time refusal builds its package with `PackageDeclarations::check(CheckMode::Kernel, ..)`. FR-268-AC-3 now names the kernel-checked package. The run-time `outside-domain` AC (FR-766-AC-4) is already under kernel checking. Every QSL-2003 stub's ignore reason lists the fixtures to migrate: temporal.rs:35-141,363-473, witness.rs:84,1736 and monitor_inputs.rs:45-55,478. |
| FND-003 | fixed | FR-767-AC-7 and FSpec FR-301-AC-6 now use the FR-624 driver-test verification wording. New FR-767-AC-8 (TC-2196) covers the language half: `spine::check_with_integer_division`, `spine::check_units_with_integer_division` and FR-662's `NativeFenceRequest` (FR-662 Inputs amended). The multi-unit half has an ignored stub (`top_level_law_reaches_every_unit_of_a_multi_unit_program`). The single-unit `check` and fence-check entry points do not exist yet, so no stub can compile against them; TC-2196 names their tests (`fsl-replay/tests/it/integer_division_check.rs`), which land with the operation. |
| FND-004 | fixed | The unknown-law message template now says "supplied by `<source>`", so the stub's `integer_division` assertion matches it. |
| FND-005 | fixed | New FR-766-AC-6 (clause-run item `{refusal, declined, undefined_expression/unproved-range}`) and AC-7 (kernel refusal record projected with its code and cause, never `null`), with ignored stubs in clause_paths.rs; TC-2195 extended. |
| FND-006 | fixed in part | AC-1 now decodes the package and asserts every `div`/`rem` binds the selected law, and it carries `FSpec-FR-570-AC-6`. AC-4 now checks every diagnostic (two, each `missing-selection`). `q14_count_proves_when_the_bound_fits` stays ignored, because test suites cannot be run in this lane to confirm it passes today. |
| FND-007 | fixed | The FR-766 "unless a guard" clause is deleted. FR-767 now says "with no caller selection it checks under the header law alone". |

## Verdict

CHANGES REQUESTED.

FND-001 and FND-002 block. The rest of the batch is sound:

- The law names, the header composition and the library isolation, with the language and standard texts in agreement.
- The decision to have no default law, which follows FSpec FR-147.
- The `check`-only flag, which follows the `--extensions` convention.
- The count interval rule and its guard narrowing.
- The run-time refusal for kernel-mode counts, which is consistent with `sum` and given its reason.

## Dispositions (round 2)

Reviewer verification against 677e6915a (language) and b3647444f (standard). Both branches match `origin/fsl-spec-w3`. `quire validate` passes on every changed spec file in both repos with module warnings only. TC-2196 and TC-1654 are not on `origin/main`.

| ID | Disposition | Resolution |
| --- | --- | --- |
| FND-001 | fixed | Verified in five places. `integer_division` is added to FSpec FR-520 (member row, `libraries` shape, AC-5), FR-620 Inputs and FR-644 Inputs. FR-767 and FSpec FR-301 now say "every request that compiles a program". The `package` conflict now also covers analyze. FR-767-AC-9 and AC-10 have ignored stubs with real assertions (analyze_request_wire.rs:341, run_wire.rs:929). |
| FND-002 | fixed | FR-766 Notes now say that no linked integer construct reaches S6a `integer_out_of_domain`, and they move the run-time-refusal fixtures to a package checked under `CheckMode::Kernel`. FR-268-AC-3 is amended to match. Every QSL-2003 ignore reason lists the fixtures to migrate. |
| FND-003 | fixed | AC-7 and FSpec FR-301-AC-6 now use the driver-test verification wording. FR-767-AC-8 adds the language half, and FR-662 Inputs is amended. The ruling on the missing single-unit and fence stubs is FND-008. |
| FND-004 | fixed | The message template now carries "supplied by `<source>`". |
| FND-005 | fixed | FR-766-AC-6 and AC-7 are added, with ignored stubs at clause_paths.rs:297 and 330 that assert the item's category, result and cause. |
| FND-006 | accepted-no-change | The control stays ignored. Ruling: acceptable. Its ignore reason ties it to QSL-2003, and the coder enables it together with the refusal stubs it controls, so the gap in protection lasts only until that ticket lands. The other parts are fixed: AC-1 decodes the bound law, AC-4 checks every diagnostic, and the FSpec FR-570-AC-6 tag is added. |
| FND-007 | fixed | Both wordings are corrected. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | Ruling on FR-767-AC-8. Naming the tests in TC-2196 is acceptable. No existing in-process `check` route takes a selection: fsl-command has no check operation, and `command::run` with `program.extraction` goes through spine compile, not `check_native_fences`. A stub on a stand-in path would assert another surface under AC-8's id. The defect is that the multi-unit stub tags AC-8 as a whole, so the matrix reports the single-unit and fence clauses as covered. Fix: split those two clauses into their own AC (FR-767-AC-11, TC-2196), which stays untagged until QSL-1989 adds `integer_division_check.rs`. | FR-767:189; integer_division_request.rs:331-334 |
| FND-009 | low | In TC-2195's Test Procedure the fix replaced step 5 (the `AtMostOne` compile, test `count_out_of_domain_names_its_cause`) with a line numbered `.`. FR-766-AC-5 now has no procedure, while Expected Results still lists step 5. Fix: restore step 5 and number the new line `6.`. | TC-2195:25 |
| FND-010 | low | The analyze stub covers FSpec FR-520-AC-5 but carries no `FSpec-TC-1654`/`FSpec-FR-520-AC-5` tag. FR-766 Notes reads "and link it", which should be "and links it". Fix: add the tag and correct the verb. | analyze_request_wire.rs:338; FR-766 Notes |

## Verdict (round 2)

APPROVE. All seven round-1 findings are resolved: six are fixed, and FND-006 is accepted with a stated reason. The three new findings are low. They are edits to text and tags that the lead can make in the trailing commit before merge, and they need no further review round.
