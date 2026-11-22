---
id: SR-967175026302197768900474181941935084700
title: "Code and Rust review of QSL-2812 stage and outcome test corrections"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2812; PR 3467; tests/it/call_requirements_items.rs, compile_command.rs, diagnostic_golden.rs, native_boundaries.rs, observation_model_header.rs, parse_operation.rs, plan_export_surface.rs, refinement_remaining_source_routes.rs, value_invariants_run.rs; fix disposition: fsl-command/src/command/items.rs and clause.rs, tests/it/run_items.rs, tests/support/mod.rs"
review_set: subset
---

## Summary

Reviewed the frozen nine-file test-only change in PR 3467 against the current requirements and the production paths those tests exercise. All 18 assigned test function names remain. The source review found no defects; compilation and runtime acceptance have not been established.

## Verdict

**CONDITIONAL** — no source-review findings. The named titan tests remain mandatory before acceptance; all builds and tests are held by the owner. This is not a merge-ready verdict and does not claim that the red tests now pass.

## Scope

The following criteria were examined, including the clean units. Long excerpts retain the relevant initial 600 characters verbatim; the full rows were read.

```yaml
{
  "scope": [
    {
      "id": "FR-002-AC-7",
      "path": "spec/functional/FR-002-parse-native-units.md",
      "role": "examined",
      "excerpt": "`OutcomeDocument::from_parse` over the `parse` operation's result for `tests/fixtures/spine-compile.native` is the document with operation `parse`, `last_stage` `S2`, category `success`, empty `items`, `diagnostics` and `artifacts`, and `result` FR-663's `parsed` report (FR-663-AC-1), and its category's FR-285 exit code is 0. Over a unit holding `a < b < c` it is the document with category `refusal`, `last_stage` `S1` and one `invalid_syntax` diagnostic at the second `<`, and its exit code is 20. `make ci-clean-build` runs, in its no-default-features build, a test that calls the `parse` operat"
    },
    {
      "id": "FR-027-AC-8",
      "path": "spec/functional/FR-027-export-compiled-native-package.md",
      "role": "examined",
      "excerpt": "A `1-draft` source refused at each of `source`, `assembly`, `check` and `emit` returns the refusing stage's cause code and reports that stage, with no package bytes. Any actual `forms` refusal retains the same stage-and-cause propagation. [FR-763](FR-763-name-the-construct-in-every-forms-stage-refusal.md)'s admitted constructs, including `synthesis` and function-body `null`, pass S2 and are not forms-refusal fixtures; its AC-5 exercises `UnrepresentedConstruct` directly because no source reaches it, so this criterion requires no source fixture for that cause."
    },
    {
      "id": "FR-096-AC-12",
      "path": "spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md",
      "role": "examined",
      "excerpt": "Exactly `Code::RuntimeInvariant`, the code of `InternalFault` (T-4), and `Code::OutputFailure` ([FR-286-AC-23](FR-286-serialize-every-outcome-as-one-json-outcome-document.md)) have the O-16 internal-failure category and resolve to FR-301 exit status 30 (tool failure) through the category's FR-285 exit code; every other `Code` resolves to 20, 21 or 22. A native `run` whose evaluation refuses with `runtime_invariant` exits 30. A report holding a `runtime_invariant` or `output-failure` diagnostic beside invalid, unsupported or incomplete ones exits 30."
    },
    {
      "id": "FR-1020-AC-2",
      "path": "spec/functional/FR-1020-transfer-previous-through-a-step-for-step-refinement.md",
      "role": "examined",
      "excerpt": "Over a refinement whose step map sends a concrete operation to `stutter`, the same clauses settle `inconclusive`, `transfer-not-preserving`, naming `once[1,1]` and `eventually[0,2]`; one with an `any` arm or a `time dense` concrete subject is not step-for-step."
    },
    {
      "id": "FR-1030-AC-3",
      "path": "spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md",
      "role": "examined",
      "excerpt": "With `premises.refinement` and `premises.claims.PrevSafe` supplied as complete proved FSpec FR-492 premise results marked `certified`, FSL settles neither premise again and the transfer is `proved`, `uncertified`; with the supplied claim premise `refuted`, the transfer settles `inconclusive`, `premise-unproved` naming slot 1 and `refuted`."
    },
    {
      "id": "FR-1030-AC-4",
      "path": "spec/functional/FR-1030-request-a-refinement-transfer-from-a-native-run.md",
      "role": "examined",
      "excerpt": "FSpec FR-380-AC-8's liveness clause, with no `premises`, is settled over the abstract stutter closure and transfers through `CasRefinesCounter` `proved`."
    },
    {
      "id": "FR-1031-AC-4",
      "path": "spec/functional/FR-1031-refuse-a-refinement-transfer-request-by-stage.md",
      "role": "examined",
      "excerpt": "A supplied claim premise whose request item is over another abstract package refuses at stage `admit`, `invalid_runtime_input`/`invalid-value` at `/refinement_transfer/premises/claims/PrevSafe`, settling no transfer; `CasRefinesCounter`, which is not step-for-step, is never refused."
    },
    {
      "id": "FR-172-AC-9",
      "path": "spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md",
      "role": "examined",
      "excerpt": "Through `command::compile` (`native-compile/1`) with the counter-step model selected, the source clause `hyper IgnoreTick using infinite over behaviours (item: CounterModel::Counter) forall trace left of CounterModel forall trace right of CounterModel align skip {} match { left.step.op = right.step.op } { always holds(item.value @ left = item.value @ right) }` gives a `filament-outcome/1` document with `last_stage` `S3`, category `unsupported`, exit code 21, and one diagnostic `unsupported_construct` with cause `expression-form` whose locus is the span of `align skip {}`; no diagnostic has cod"
    },
    {
      "id": "FR-185-AC-12",
      "path": "spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md",
      "role": "examined",
      "excerpt": "An S4 refusal concerning a random parameter, a workload, a reward or a delay carries that declaration's source reference and label span in its locus member (never `null`), and its message names the declaration in words and holds no debug rendering of a span or source reference."
    },
    {
      "id": "FR-354-AC-4",
      "path": "spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md",
      "role": "examined",
      "excerpt": "A unit declaring the `retention` form without selecting the extension refuses `unsupported_construct`/`declaration-form` naming `retention`, its span and the unit's selections, and refuses identically over a catalog with and without the `retention` extension. A unit that selects the extension, names a parameter `retention` inside a function body and declares a record member named `retention` compiles, with `retention` an ordinary identifier in both places."
    },
    {
      "id": "FR-513-AC-4",
      "path": "spec/functional/FR-513-name-the-successor-or-keep-of-each-retained-criterion-that-rides-a-retired-path.md",
      "role": "examined",
      "excerpt": "A self-calling predicate and a two-predicate cycle with no measure each refuse at check with the FR-146 `missing-measure` obligation (FR-091-AC-30); the same cycles with a measure FSpec FR-146 accepts (`predicate P using v(x: Int[0, 9]): Boolean decreases(x) { ... }`, FR-091 \"Predicate form\") admit, and a measure that does not decrease refuses with the `decrease` obligation (a widening of the legacy always-refuse rule, which FR-046-AC-2 is amended to state; the planner ruled for it); a call with the wrong argument count or type or unit refuses `ill_typed` at the call; a predicate whose result "
    },
    {
      "id": "FR-621-AC-17",
      "path": "spec/functional/FR-621-write-the-analyze-result-as-one-outcome-document-per-request.md",
      "role": "examined",
      "excerpt": "A composition's component-guarantee premise item writes a `package` member naming its dependency and node ids that index that dependency's graph, so that item holds no node id object whose path holds a node id; the composition item writes no `package` member, holds the node ids of each component-guarantee premise key in the graph of that slot's component package (so each key's `claim` equals its premise item's `request.claim` as written, and the item holds no node id object whose path holds a node id) and every other node id in the request package's graph; and a run whose package has no admitt"
    },
    {
      "id": "FR-652-AC-3",
      "path": "spec/functional/FR-652-select-state-machine-contexts-and-value-invariants-from-a-run-request.md",
      "role": "examined",
      "excerpt": "Through `run`, over FR-106-AC-48's TC-1051 K2 fixture, an `items` request with the selection `{\"value_invariants\": {\"snapshot\": \"s1\", \"invocation\": \"i1\"}}` returns one item per `evaluate_value_invariants` result in FR-366's order, each with `invariant` and `path`; the item for `Money`'s non-negative invariant at `i1`'s parameter `amount` (`amount_minor` `-7`) reports `violation`, and the document's `category` is `violation`, exit 10. The same selection with a `pre` snapshot as `snapshot` gives one item refused `wrong_snapshot`/`wrong-observation` at stage `admit`, with no `invariant` or `path`"
    },
    {
      "id": "FR-660-AC-6",
      "path": "spec/functional/FR-660-supply-extension-definitions-through-the-request-extensions-member.md",
      "role": "examined",
      "excerpt": "The `parse` operation over the AC-1 unit with a catalog holding `ext.retention` returns an admissible `ParsedSource` holding one `retention` form; without the catalog the form refuses as an unselected extension form."
    },
    {
      "id": "FR-663-AC-1",
      "path": "spec/functional/FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md",
      "role": "examined",
      "excerpt": "`OutcomeDocument::from_parse` over the `parse` operation's result for `tests/fixtures/spine-compile.native` with identity `agent-ix`/`test:parent` has `result` of kind `parsed` whose `source` is those labels, `path` the fixture path, `bytes` the fixture's byte count, and `usage` the four S1 settings in order with each `bound` the default and each `used` the count the parse reached, `s1.input_bytes` used equal to `bytes`. A unit holding `a < b < c` has a `parsed` result beside its `invalid_syntax` diagnostic. A parse with a blank label has `result` `null`."
    },
    {
      "id": "FR-762-AC-6",
      "path": "spec/functional/FR-762-report-every-independent-syntax-error-of-a-unit.md",
      "role": "examined",
      "excerpt": "A unit of 150 `record` declarations, each missing a field type, under the default limits gives 101 diagnostics: 100 `unexpected-token` diagnostics, then one `invalid_syntax`/`diagnostic-list-cut` diagnostic located at the 101st record's offending token whose message names `s1.diagnostics` and the bound 100. The outcome is a refusal, not `stage_limit_exceeded`."
    },
    {
      "id": "FR-763-AC-5",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "examined",
      "excerpt": "The `FormsCause::UnrepresentedConstruct` refusal for the `declaration` row, built directly because no source reaches the row, has catalog code `unsupported_construct` and construct spelling `declaration`; a 200-byte `text` is cut to at most 123 bytes ending with `...` and a short `text` is kept whole; its message is `` `<text>` declarations are not supported yet `` followed by ` (declaration <declaration>)` when an owner is set."
    },
    {
      "id": "FR-763-AC-6",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "examined",
      "excerpt": "A function body `null` passes S2 and its check outcome document has one diagnostic with `code` `ill_typed`, `cause` `type-mismatch`, the exact span of `null`, and a message naming `null`, `none` and `absent` (FR-986); the document's category is `refusal`, exit 20."
    },
    {
      "id": "FR-868-AC-3",
      "path": "spec/functional/FR-868-refuse-a-version-or-digest-in-an-observation-document-s-model-header.md",
      "role": "examined",
      "excerpt": "An invocation document whose `model` carries `version` refuses the same way, naming `model.version`; and a compile request's `models` entry and an observation's `model` member each refuse an unknown member, so neither input accepts a member the other refuses."
    }
  ]
}
```

## Review evidence

- Read the complete diff in all nine changed files. No production code, specification, manifest, workflow, source scanner or allow-list changes are in this PR.
- FR-096-AC-12 and FR-027-AC-8 were amended and merged in PR 3463 before these test edits. The two code-category tests now require exactly RuntimeInvariant and OutputFailure to exit 30; every other code remains bounded to 20 through 22.
- The replacement unselected-retention fixture is the existing FR-354/FR-660 parser refusal. Its unsupported category, code, exit 21 and S1 stage remain explicit assertions. The malformed-source case still asserts S1 and exit 20.
- Only the unreachable synthesis/forms and null/forms fixtures were removed. The PR quotes FR-763's statement that no source reaches UnrepresentedConstruct and amended FR-027-AC-8. The source, assembly, check and emit cases retain exact stage, code, exit, stdout and locus checks. The existing named null check/locus regression remains in the pending titan command.
- The declaration-locus replacement reuses the repository's existing abstraction model builder through its file module; it introduces no copied fixture. The mixed-owner workload is the same real S4 refusal already exercised in fsl-package's probabilistic emission tests. The helper explicitly requires CompileRefusal::Emit, the new test requires unsupported_construct, and its existing region and plain-message checks remain.
- The complete parsed success document matches OutcomeDocument::from_parse and ParsedOutcome: exact source labels, path, byte count, four settings in order, default bounds and actual CST/work counters. The test reads the real parsed artifact, not a copied serializer or synthetic report.
- Both typed check-refusal carriers expose the same CheckRefusal list, and the self-cycle and mutual-cycle missing-measure assertions remain. The outcome exit accessor, value-invariant disposition path and VersionAdvances selection match their current owner APIs and fixture.
- All six transfer tests use the existing emittable fixture that omits only unrelated, unconfigured BoundedTwo. Every existing transfer result, admission, premise identity/result, preservation, certification and cause assertion remains unchanged.
- Source inspection found no new mocks of the unit under test, unconditional skips, no-op checks, swallowed failures, copied implementation, unsafe code, or nondeterministic resource selection.
- Computed quire matrix exited 0 and reports all 19 examined criteria tagged. The command also emitted repository-wide symbol-collision, unresolved/unmarked-trace and section-selection diagnostics, including cfg-alternative symbols and documents outside this PR. Those diagnostics were not treated as proof of a clean repository. Matrix JSON remains in the scratchpad; this scoped test review makes no repository-wide gap-audit claim.
- No applicable AssuranceProfile is declared for this changed test scope.
- git diff --check for the frozen diff passed. No cargo command, build, test, compiled repository scanner or titan job ran or was queued. Formatting was reported by the coder; this reviewer did not repeat it or treat that report as runtime evidence.

## Method selection and limitations

Ran code-review with its Rust lane, loading rust-review, rust-style and the API, implementation, safety, wire and testing references. The change is tests only, so the reviewer role's production-code gap-analysis condition and specification-change review condition do not apply. The gap-analysis skill was read to confirm that it is an end-to-end repository assurance audit; no such audit or optional semantic gap-analysis was claimed. Requirement-to-test alignment and computed traceability for the changed scope were checked as part of code-review.

Quoin 0.36.8 and Quire 0.52.0 (engine 0.62.2) supplied the installed SpecReview schema and matrix. Exact runtime model identity is unavailable in this harness and is recorded as unavailable in tracker metadata.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Disposition pass 1 — scoped fix commit

This section records the later production fix, separately from the original test-only review above. The inspected delta is the one fix round from 24a0382a4d to 9f4272343398320cb921822f55ae9503314fb54c. The original Findings table is unchanged; it contained no real findings to disposition.

**PASS for the scoped fix disposition.** No new source/Rust defect was found in the fix delta. Arch reports that all eight tests owned by this change now pass on titan at the frozen fix head. The ninth requested test, run_items::function_selection_admits_an_identity_reference, failed in the canonical-value reader/test area owned by QSL-2810 and PR 3470; arch explicitly excluded that reader/test from this PR. Neither was changed or bypassed here. The final workspace gate and merge remain arch-owned.

### Additional scope examined

- fsl-command/src/command/clause.rs: ObservationInputs, read_observation_inputs and the delegation from read_with_source.
- fsl-command/src/command/items.rs: actual value-invariant preparation, shared source/program compilation and batch dispatch, including format/key checks, bounded intake, accounting/observation overrides and preservation of raw observation roles and identities.
- tests/it/compile_command.rs: the S4 workload refusal, exit 21 and exact declaration-span assertion; tests/support/mod.rs and diagnostic_golden.rs share the one existing model builder.
- tests/it/run_items.rs: the new current/pre/post/invocation function-reader control; tests/it/value_invariants_run.rs: the retained invocation-parameter violation plus explicit pre-snapshot item stage/cause.
- FR-652 Behavior requires a pre/post snapshot of the correct document format to reach FR-106, then refuse wrong_snapshot/wrong-observation at admit; FR-652-AC-3 requires the invocation-parameter violation and one item for the pre-snapshot refusal without invariant/path. FR-106-AC-48 owns the actual no-clause admission. FR-312/FR-109 retain the function selection's current-snapshot rule.

### Fix evidence

- The emit-stage fixture now selects the actual unrelated-type model and mixed-owner workload already used by the declaration-locus test. It reaches the genuine emitter refusal instead of relying on a type declaration the emitter now supports. Exact S4, unsupported_construct, exit 21, empty stdout and whole workload-region checks remain.
- Value-invariant items no longer synthesize a function selection. Their actual ValueInvariantsBatchRequest uses the shared model/library/observation reader and existing bounded Intake, preserving caller source identity, path, bytes, document references and roles. FR-106's admit_value_invariant_selection remains the admission authority.
- The shared clause reader's existing model, dependency, observation and override semantics are retained. The function-reader role check in fsl-replay/src/spine/request.rs is byte-unchanged across the fix delta. No dummy function name, role mutation, fabricated current snapshot or compatibility bypass is used.
- Source inspection checked missing observation keys, snapshot/invocation format separation, request/override observation-limit conflict, original accounting override behavior, file-count/dependent-byte admission and batch compilation identity. No new panic path, incomplete adapter, meaningful unowned behavior or test stub was found in the changed surface.
- Arch-reported passing exact tests: compile_command::each_spine_stage_refusal_reports_its_stage_and_code; value_invariants_run::value_invariants_selection_checks_invocation_parameters; diagnostic_golden::random_parameter_refusal_has_its_recorded_declaration_locus; run_items::function_selection_requires_a_current_snapshot; run_items::mixed_call_and_value_invariants_share_one_items_request; run_items::malformed_context_and_value_invariant_selections_refuse; run_items::value_invariants_selection_is_an_items_selection; value_invariants_run::a_false_type_invariant_is_a_violation_through_run.
- This reviewer ran no builds or tests. git diff --check over the fix delta passed. Newly applicable gap-analysis is recorded separately as SR-967175026302197768900474181941935084710, including the evidence-backed matrix refusal and documented static fallback.

The later metadata-only commit 4fd8d974ae99f3060cc2532d298c6401014226c6 changes only the new control's trace attribute to the minted FR-312-AC-4 criterion. The test body and all production source are byte-unchanged from the eight-green behavioral head. Gap FND-001 is now explicitly fixed in its own artifact; no code-review finding or extra source fix round was introduced.
