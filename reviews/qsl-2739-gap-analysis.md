---
id: SR-202611210022442313144108507
title: "gap-analysis of QSL-2739 PR3287 composition item through analyze (FR-1044)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; fsl-analyze/src/analyze/document/composition_item.rs, fsl-analyze/src/analyze/document.rs, fsl-analyze/src/analyze/document/package_run.rs, fsl-analyze/src/analyze/request.rs, fsl-analyze/tests/it/analyze_composition_cli.rs, fsl-command/src/command/analyze.rs; FR-1044, FR-507, FSpec FR-348-AC-3, FSpec FR-348-AC-4"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---

## Summary

Ticket: QSL-2739. This is a planless, PR-scoped gap analysis of frozen PR #3287 at ebdaca7eecb4d062bc69fce9faa4bb80d1f3fefa against FR-1044, FR-507's new row, and FSpec FR-348-AC-3/AC-4, which PR1 claims.

Matrix status for FR-1044:

- **AC-1..AC-6, AC-8 and AC-9:** tagged, each with one test in `analyze_composition_cli.rs` that runs through `fsl_command::command::analyze` with `Engines::fsl()`.
- **AC-7:** untagged.
- **AC-10:** tagged only through its source-route half.

An applicable spec/code alignment check was done by hand (intent ↔ test ↔ code per AC) and fed the findings. The known open scope blockers are recorded as gaps, not waived. `/tmp/qsl-2739-arch-ruling.txt` was reread before this report and contains no scope cut or waiver for AC-7, the AC-10 package route or text mode.

## Verdict

**FAIL.** FR-1044-AC-7 is untagged, and there are two high findings (FND-001, FND-002). No ignored or missing case is counted complete. Runtime evidence is pending: the two Draco commands are queued with no result, so even the tagged criteria have no passing run behind them yet.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-1044-AC-7 is untagged and unreachable. `PackageDeclarations::composition_lattice` is set only by `with_composition_lattice` (fsl-semantics check.rs:724), whose only caller is tests/it/composition_planner.rs:169. No source unit can select the `fixed-point-lattice` profile, so the `fixed-point` premise item that `premises_of`/`premise_item` write (index 3, `options.profile`, family `protocol`) cannot be produced through `fsl analyze`, and no test exercises it. This is a known open blocker; the arch ruling records no scope cut. | fsl-analyze/src/analyze/document/composition_item.rs:389 |
| FND-002 | high | FR-1044-AC-10's package half ("over the unit's `package` handoff (FR-850) it writes the same items, records and exit code") is not met. `settle_package_run` discards the component map (`components: _`) and the package route has no composition form. The only test of it, `the_package_route_settles_the_same_items`, is `#[ignore]`. The matrix still reports AC-10 `tagged` through the source-route test alone, which overstates coverage. This is a known open blocker; the arch ruling records no scope cut. | fsl-analyze/src/analyze/document/package_run.rs:435 |
| FND-003 | medium | FSpec FR-348-AC-3 is claimed for PR1 (arch ruling, module doc) but has no test, and the code does not realize it. A routed composition negotiated `Supported` to a registered composition-capable provider falls to `work`'s `_ => absent()` arm (request.rs:1303) and is never sent to that provider. The provider wire (`provider::request_item_wire`) writes no `premises` member either. Only AC-4's no-provider `unsupported` is evidenced. | fsl-analyze/src/analyze/request.rs:1303 |
| FND-004 | medium | FR-1044-AC-4(e) is not exercised by the AC-4 test: no FR-571 `realizability` item with `premises` `{"role_knowledge": [], "environment": []}` is sent. The tagged test covers (a)–(d) only. The coder's regression run (`protocol_item_bounds`) is not tagged FR-1044-AC-4 and is still pending. | fsl-analyze/tests/it/analyze_composition_cli.rs:278 |
| FND-005 | medium | The fixture says "Every request runs through `fsl analyze`, machine and text mode". No test exercises text mode; only the machine document is asserted. Text rendering belongs to the external driver, which is a known open blocker, so the text-mode half of every FR-1044 AC has no evidence in this repository. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:277 |

## Coverage

Plan completion: not assessed

Reconciliation used `quire matrix --scope . --format json`, static, with no run evidence. It completed and emitted repository-wide `symbol-id-collision` diagnostics outside this diff. FR-1044 results:

- **AC-1:** `a_routed_composition_is_unsupported_beside_its_proved_premises`
- **AC-2:** `the_other_rules_and_the_planner_local_item_settle_unsupported`
- **AC-3:** `each_selector_settles_by_its_own_rule`
- **AC-4:** `a_caller_may_only_restate_the_derived_premise_keys`
- **AC-5:** `a_premise_under_assumptions_is_not_run_as_the_clause_alone`
- **AC-6:** `a_cycle_without_a_lattice_refuses_the_item`
- **AC-7:** untagged
- **AC-8:** `a_shared_premise_is_written_once`
- **AC-9:** `a_refuted_premise_keeps_its_own_record`
- **AC-10:** `a_composition_is_a_default_item` (source half only; the package half is ignored)

FSpec-FR-348-AC-4 has two binders: the AC-1 test and `the_production_registry_has_no_composition_candidate`. FSpec-FR-348-AC-1/AC-2 are correctly not tagged; they remain QSL-2739 PR2 work. The matrix's `unmarked-trace-id` notes come from prose doc comments naming the AC ids above each tagged test and are not counted as defects. Test-strength defects in the tagged tests are recorded in the code-review artifact SR-202611210022442313144108506 (FND-001..FND-008) and are not repeated here.

Reverse gap: every changed behavior has an owning requirement.

- **FR-1044:** selection, premises and premise items, default items.
- **FR-507:** the `component_composition` family arm.
- **FR-620:** `unknown-kind`, `unknown-option` and the `options` echo on the routed item.
- **FR-1044 "premises ... never a proof":** the request-stage `premises` shape check.

There are no stubs, placeholder returns or mocked production seams. The one ignored test names its tracked reason (QSL-2739).

Tool versions: quoin 0.36.8, quire 0.52.0 (engine 0.62.2), module spec-artifacts-process@v0.36.0, gap-analysis skill from quoin 0.30.0 plugin cache. These CLIs are newer than the 2026-11-10 release notice; no older fallback was used. No Cargo or test gate was run by this reviewer (brief). No spec files are under review here: the FR-1044/FR-507 contract was cleared separately (spec disposition round 4), so spec-review was not rerun.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | `the_package_route_refuses_a_composition_typed` carries `#[trace("FR-1044-AC-10")]`, but it asserts the current-behavior refusal that AC-10's package half forbids. The matrix therefore counts a non-conformance test as AC-10 evidence, beside the source-route test, while the conforming test stays `#[ignore]`. It traces to the "Current system behavior" statement, not to AC-10. | fsl-analyze/tests/it/analyze_composition_cli.rs:730 |
| FND-007 | medium | AC-10's package case is a request with no `items`, and its current behavior is not stated or tested. `default_selections` builds package default items from `clause_names`, `claim_names` and model selections only, so a composition, which is unnamed in the package, is never selected. By static reading, a no-`items` package handoff then settles without the composition or its premise items rather than refusing typed. "Current system behavior" covers only "a `package` request that names a composition". | fsl-analyze/src/analyze/document/package_run.rs:1584 |
| FND-008 | low | The new FR-1044 Behavior bullet says FSL sends the provider "its `options`", but the FSpec-FR-348-AC-3 test sends no `options` and never asserts the forwarded `options` member. Only `premises`, `request_index`, `kind` and candidates are checked. | fsl-analyze/tests/it/analyze_composition_cli.rs:256 |
| FND-009 | low | Code fix round 1 adds normative FR-1044 text: a `SHALL` Behavior bullet for FSpec FR-348-AC-3 provider forwarding, and a "Current system behavior" section. No independent spec review covers it, and AGENTS.md requires /spec-review for changed requirements. The text passes `quire validate --strict` and this reviewer found it consistent with the code (`refuted` maps to `inconclusive` via `ReplayParity`, `unknown` maps to `inconclusive`). | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:253 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | FR-1044 "Current system behavior" now states what the package route does over an admitted package: `unsupported_construct`/`not-yet-implemented`, exit 21, naming the `composition` rule for no `items` and `unnamed-claim` for a named item. No test can reach that path today. The public handoff may refuse `missing_import` first, the tests accept either code, and the coder reports no reader-view test of `unnamed_composition`. The admitted-package half of the statement therefore has no evidence. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:275 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | low | `a_provider_proof_is_refused_over_a_refuted_component_guarantee` carries `#[trace("FR-1044-AC-9")]`, but AC-9's oracle is the composition `unsupported` with no provider, and this test asserts it `inconclusive` `replay-refused` under a registered provider. It verifies FR-1044's proof-gate Behavior bullet, which has no AC. The tag overstates AC-9 evidence. | fsl-analyze/tests/it/analyze_composition_cli.rs:361 |
| FND-012 | low | `a_provider_proof_is_refused_over_an_unsupported_component_guarantee` carries `#[trace("FR-1044-AC-5")]`, but AC-5 requires the composition `unsupported` naming `composition`, and the test asserts `inconclusive` `replay-refused` under a provider. It verifies the proof-gate bullet, not AC-5. | fsl-analyze/tests/it/analyze_composition_cli.rs:380 |
| FND-013 | low | `an_unassumed_proof_does_not_discharge_an_assumed_guarantee` carries `#[trace("FR-1044-AC-8")]`. AC-8 is the no-provider index layout (`Sys` twice plus `SysRely`, compositions `unsupported`), while this test is a two-composition provider request asserting `proved` and `inconclusive`. It verifies the proof gate's typed-key reading, not AC-8. | fsl-analyze/tests/it/analyze_composition_cli.rs:398 |

## Dispositions

Disposition round 1 inspected the fix diff `ebdaca7..cec151ddc635c8bad1b3296c6deb378208470e13`. It reread the full current `/tmp/qsl-2739-arch-ruling.txt` (code-gap ruling, read-only correction, text and FR348AC3 ruling). Runtime is pending: the cec module run and the three-test parent comparison are queued. AC-7 and AC-10 rows are unchanged in FR-1044 (the diff only adds lines 253-283) and stay open on QSL-2739.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Binding arch ruling 1: the source lattice declaration is a genuine standard gap that arch specifies before PR2, with the production binding in PR2 and no lanes implemented here. AC-7 is unchanged and open on QSL-2739. FR-1044 "Current system behavior" now states that a source request meets no lattice and a cycle settles by AC-6's refusal. Not complete. |
| FND-002 | deferred | Binding arch ruling 1: package handoff closure and the AnalysisView composition projection go to terra:QSL:package-consumer-api under QSL-2739. AC-10 is unchanged and open, and the conforming test stays `#[ignore = "QSL-2739: ..."]`. The current behavior is stated in FR-1044, with gaps still in it (new FND-006, FND-007). Not complete. |
| FND-003 | fixed | `work` now forwards a routed composition negotiated to a registered provider through `composition_provider_item_over` and the existing `ExternalProvider::settle` seam. `request_item_wire` writes `premises` from `ProviderTarget::Composition` beside `options`. There is no built-in algorithm and no proof is forged: `refuted` maps to inconclusive. The test uses a real recording `ExternalProvider` registered through `analyze_with_providers` and asserts the serialized `premises`. |
| FND-004 | fixed | `a_realizability_item_keeps_its_premises` (FR-1044-AC-4) sends the FR-571 realizability item with `premises` `{"role_knowledge": [], "environment": []}` and asserts exit 0 and `proved`. |
| FND-005 | deferred | Binding arch ruling 2: text rendering belongs to driver terra:QSL:codex-codex, checked at the normal FSL pin. Text coverage stays open on QSL-2739 with no AC weakened and no local renderer. PR1 lands on focused machine evidence. FR-1044 "Current system behavior" records this. Not complete. |

Disposition round 2 inspected `cec151dd..6ff10e952a660f72c8452dd1294e286619ea4818`. The round-1 outcomes stand. FND-003 and FND-004 stay fixed. FND-001, FND-002 and FND-005 stay deferred under the binding arch rulings (`/tmp/qsl-2739-arch-ruling.txt`, reread, unchanged since 02:12). AC-7, AC-10 and the text criteria are unchanged in FR-1044 and open on QSL-2739. The positive AC-10 package test is still `#[ignore = "QSL-2739: ..."]` and untraced. The composition module run at 6ff10e95 is queued and is not green.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | The current-behavior package tests (`the_package_default_items_refuse_a_composition`, `a_package_item_naming_a_composition_is_refused`) carry no AC trace, so the matrix no longer counts a refusal as AC-10 evidence. The conforming AC-10 test stays ignored with its QSL-2739 reason. |
| FND-007 | fixed | `settle_package_run` refuses a no-`items` request over a package holding an unnamed composition before it builds the default items: `PendingFamily::Composition`, `request_index` None, with the composition's node. It no longer settles without the composition. FR-1044 "Current system behavior" states this, and AC-10 is unchanged and open. That the path has no evidence is new finding FND-010. |
| FND-008 | fixed | The registered-provider test sends `"options": {"profile": "p1"}` and asserts both the provider item's serialized `options` and the document echo's `request.options`. |
| FND-009 | still-open | Independent spec review of the added FR-1044 text is pending. Reviewer 9a56's artifact (`/tmp/qsl-2739-spec-addendum/qsl-2739-fr1044-addendum-spec-review.md`) and report do not exist yet, and its output and stderr were empty when last checked. This outcome is not claimed complete. |

Disposition round 3 is the arch-authorized extra round at cf48c388fc3fe694ec6a367771f0ad8a2982a325. FND-001, FND-002 and FND-005 stay deferred under the binding rulings: AC-7, AC-10 and text are unchanged and open on QSL-2739. FND-003, FND-004 and FND-006..008 stay fixed. The FSpec FR-348-AC-3 forwarding now keeps a real recording call through a named backend with a published option (FR-967).

| FND | outcome | reason |
| --- | --- | --- |
| FND-009 | fixed | The independent spec review of the added FR-1044 text now exists: spec reviewer 9a56, SR-202611211049161, review plus disposition round 1. That review has its own open findings FND-005..007, which it tracks; the spec addendum is not clear and is not claimed clear here. |
| FND-010 | deferred | Binding extra-round arch ruling: evidence for the already-admitted package view guard is deferred to the AC-10 owner, terra:QSL:package-consumer-api, with no view-level test here. FR-1044 "Current system behavior" now states the measured public boundary (`invalid_package` at `/semantic_graph/operand_rows/`, exit 20, `refusal`, with or without `items`), which the package tests assert exactly. It labels the route's own refusals as source behavior that no public request reaches. AC-10 is unchanged and open. Not complete. |

Disposition round 4 is the narrow final round and covers FND-011..013 only, at fe674b00c1e8a8dae308ad9fcb5194973aedaa9e (19ff988c follow-through, 92726f02 fixture, fe674b00 mechanical main merge). Spec addendum SR-202611211049161 is CLEAR in round 2: all 7 findings are fixed, there is no new finding (comment 4456c1c5), and FR-1044 is identical to the reviewed 19ff. AC-7, AC-10 (with the admitted-view guard evidence) and text mode stay deferred and open on QSL-2739 by binding ruling. Runtime at fe674b00 is queued and not green.

| FND | outcome | reason |
| --- | --- | --- |
| FND-011 | fixed | `#[trace("FR-1044-AC-9")]` is removed from `a_provider_proof_is_refused_over_a_refuted_component_guarantee`, and its doc now names FR-1044 "Behavior" (the provider proof gate). AC-9 keeps its correct source binder `a_refuted_premise_keeps_its_own_record`. |
| FND-012 | fixed | `#[trace("FR-1044-AC-5")]` is removed from `a_provider_proof_is_refused_over_an_unsupported_component_guarantee`, with the doc relabelled the same way. AC-5 keeps `a_premise_under_assumptions_is_not_run_as_the_clause_alone`. |
| FND-013 | fixed | `#[trace("FR-1044-AC-8")]` is removed from `an_unassumed_proof_does_not_discharge_an_assumed_guarantee`, with the doc relabelled the same way. AC-8 keeps `a_shared_premise_is_written_once`. |
