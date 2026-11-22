---
id: SR-967175026302197768900474181941935084721
title: "gap-analysis of replay and grammar red-main repairs"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2822/QSL-2823/QSL-2812; PR 3476"
review_set: subset
---

## Summary

Frozen candidate: `f11f8fd554609c447b3677f0b88061d62a3118c0`, PR #3476. Tickets: QSL-2822, QSL-2823 and QSL-2812 (including latest assigned comment). No branch edits or repository builds/tests/scanners/jobs were performed. Named titan verification remains pending with the lead. `git diff --check origin/main...HEAD` is clean. No specification text changes require a separate spec-review method.

Plan completion: not assessed. This is a bounded PR forward/reverse gap review, not a full repository assurance pass. No new scoped implementation, test stub or unowned behavior found. Product changes implement the existing FR-910, FR-060 and FR-397 obligations; the new parser vectors exercise the production parser and retain typed located-negative controls. Existing six product regression oracles were not weakened. Existing trace attributes were retained.

`quoin matrix --repo /home/peter/dev/worktrees/qsl-2822-replay --json`, using `/home/peter/.npm-global/bin/quoin` 0.36.8 on terra, exited 1: `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. Reported to the lead; no downgraded binary or altered limits used. Static `quire matrix --scope /home/peter/dev/worktrees/qsl-2822-replay --format json` and `quire coverage --scope /home/peter/dev/worktrees/qsl-2822-replay --json` completed successfully using quire 0.52.0 (engine 0.62.2). Their JSON is retained beside this review. No execution evidence was read.

All 21 examined local ACs are tagged in the static matrix. The added test binds FR-397-AC-16/25. External FSpec tags are not resolved by this local-only matrix (including the new test's FSpec-FR-339-AC-1); those requirement texts were read in the sibling specification and remain a declared coverage limitation, not an assertion of external binding. Global coverage reports 5101 backed/6220 total, 6241 criteria and 23495 diagnostics, including pre-existing symbol collisions and unresolved tags. These figures establish neither repository-wide cleanliness nor a passing coverage percentage.

The reverse inspection covers every changed production branch and its named consumers: real CST spans, refusal-carrier classification, and bounded empty-call grammar control. No source/test stub, unconditional skip, mock replacing the unit under test or omitted new public behavior was found. Static tags are evidence claims only; final execution checks and the large-input memory result remain pending on titan.

Scoped matrix rows:

- FR-002-AC-7: tagged (1 binders)
- FR-060-AC-6: tagged (4 binders)
- FR-104-AC-15: tagged (2 binders)
- FR-109-AC-2: tagged (7 binders)
- FR-218-AC-33: tagged (5 binders)
- FR-263-AC-1: tagged (2 binders)
- FR-286-AC-26: tagged (7 binders)
- FR-286-AC-28: tagged (1 binders)
- FR-302-AC-4: tagged (2 binders)
- FR-354-AC-4: tagged (3 binders)
- FR-397-AC-6: tagged (2 binders)
- FR-397-AC-16: tagged (5 binders)
- FR-397-AC-25: tagged (3 binders)
- FR-492-AC-8: tagged (2 binders)
- FR-492-AC-13: tagged (3 binders)
- FR-663-AC-1: tagged (2 binders)
- FR-763-AC-2: tagged (3 binders)
- FR-767-AC-11: tagged (2 binders)
- FR-880-AC-8: tagged (7 binders)
- FR-910-AC-2: tagged (1 binders)
- FR-940-AC-9: tagged (1 binders)

## Examined scope

Changed files: fsl-cst/src/{grammar.rs,grammar/generate.rs,parser.rs}; fsl-cst/tests/it/production_value_forms.rs; fsl-replay/src/{execute.rs,execute/tests.rs,execute/tests/composite.rs,outcome.rs,spine/lifecycle.rs,spine/lifecycle/tests.rs}; fsl-replay/tests/dispatch_link_compile.rs; fsl-replay/tests/it/{compile_ieee_profile.rs,format_outcome_facade.rs,integer_division_check.rs,parse_outcome_facade.rs,probability_certificate.rs,refinement_query_facade.rs,replay_result_rows.rs,source_diagnostic_outcome.rs}. External FSpec paths below identify the sibling specification repository, not an included local matrix root.

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
      "id": "FR-060-AC-6",
      "path": "spec/functional/FR-060-check-fsl-api-surface-boundary.md",
      "role": "examined",
      "excerpt": "`fsl_replay::compile_package` refuses as the spine does: a malformed source refuses `ReplayRefusal::Recompile` with the spine refusal's catalog code and stage; a library whose source has the unit's owner refuses `ReplayRefusal::DependencyInput`; an S1 `text_input_bytes` limit below the source's size refuses `stage_limit_exceeded`; and an S1 limit above the reader limit refuses `LimitAboveReader`."
    },
    {
      "id": "FR-104-AC-15",
      "path": "spec/functional/FR-104-check-state-clauses.md",
      "role": "examined",
      "excerpt": "`spine::check_with_execution_points` accepts `StateClauseExecutionPoints`; the native `CheckBindings.clauses` vector is projected only after its existing complete/no-extra validation, without weakening that contract. A supplied exact authored clause name retains its accepted point unchanged; an absent entry has no point, and none is inferred. An unknown or non-clause key refuses `missing_declaration`/`missing-name` at the check unit's source-start region before lowering or identity minting; duplicate `CheckBindings` names/identities refuse at the native check source-start region. Existing clau"
    },
    {
      "id": "FR-109-AC-2",
      "path": "spec/functional/FR-109-run-a-state-clause-through-the-spine.md",
      "role": "examined",
      "excerpt": "missing-model (no package supplied) reports stage `compile`, `refusal`, `missing_import`/`missing-selection`, exit 20, with no `package_id`; a `Clause` selection naming `Absent`, and one naming the function `sameIdentity`, each report stage `select`, `missing_declaration`/`missing-name`."
    },
    {
      "id": "FR-218-AC-33",
      "path": "spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md",
      "role": "examined",
      "excerpt": "A repeat owned by role `c` whose body contains `attempt Load by c as (loaded: Boolean)` checks with `visible (loaded)` and `while { not loaded }`; its first body pass is unconditional, each later guard reads the most recently completed body's binder, and at least one iteration completes. The same post-test repeat with `max 0` refuses `invalid_package`/`invalid-value` at the declaration. A repeat whose guard reads no body binder retains pre-test behavior, so `while { false }` skips its body. The same binder produced by a role in a nested body branch is readable only after that branch is in the "
    },
    {
      "id": "FR-263-AC-1",
      "path": "spec/functional/FR-263-replay-at-any-depth-under-the-request-limits.md",
      "role": "examined",
      "excerpt": "On a thread with a 512 KiB stack, a request whose source holds a function with a 100,000-term sum over a recursive list parameter, whose assignment carries a 100,000-long recursive list value, whose `stage_limits` carries the proving run's raised `s1.*` and `s3.*` settings, and whose `replay.input_bytes` is raised to fit the request, replays to the same verdict as the proving run, and its recompiled `package_id` equals the request's."
    },
    {
      "id": "FR-286-AC-26",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "The bytes of every golden document, the `analyze` documents `fsl-analyze` assembles and the `monitor` and `execute` documents `fsl-command` writes included, equal their own RFC 8785 re-encoding through `quire-canonical`. FR-286-AC-2's diagnostic is written with its members in the order `cause`, `code`, `details`, `hint`, `locus`, `message`, and an `unsupported_construct`/`declaration-form` diagnostic carrying `construct`, `declaration`, `profile` and `text` in the order `cause`, `code`, `construct`, `declaration`, `details`, `hint`, `locus`, `message`, `profile`, `text`. The canonical check re"
    },
    {
      "id": "FR-286-AC-28",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "FR-286-AC-2's `ill_typed`/`type-mismatch` diagnostic carries `hint` equal to `hint_for(Code::IllTyped, Some(\"type-mismatch\"))`, a nonempty sentence ending in `.`; the `io-error` diagnostic of FR-865-AC-4's `fsl run nosuch.json` carries `hint` equal to `hint_for(Code::IoError, None)`. FR-286-AC-23's `output-failure`/`output-failure` diagnostic, the cause-less `invalid-request` diagnostic of a malformed request document, and FR-286-AC-15's `invalid_source_identity`/`blank-label` diagnostic, pairs with no catalog row, have no `hint` member; the blank-label diagnostic has none through `from_source"
    },
    {
      "id": "FR-302-AC-4",
      "path": "spec/functional/FR-302-link-dispatch-during-unit-compile.md",
      "role": "examined",
      "excerpt": "A unit with dispatched calls on two operations, each with one failing subtype, refuses once and names both failures."
    },
    {
      "id": "FR-354-AC-4",
      "path": "spec/functional/FR-354-admit-extensions-from-declarative-grammar-and-typed-node-schemas.md",
      "role": "examined",
      "excerpt": "A unit declaring the `retention` form without selecting the extension refuses `unsupported_construct`/`declaration-form` naming `retention`, its span and the unit's selections, and refuses identically over a catalog with and without the `retention` extension. A unit that selects the extension, names a parameter `retention` inside a function body and declares a record member named `retention` compiles, with `retention` an ordinary identifier in both places."
    },
    {
      "id": "FR-397-AC-6",
      "path": "spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md",
      "role": "examined",
      "excerpt": "`+`, `-`, `*` and `/` over unlike float widths, over unlike rounding modes (`Float64[nearest-even]` and `Float64[toward-zero]`), or over a float and an `Int[..]`, `Rational[..]` or `Decimal[..]` parameter refuse `ill_typed`/`type-mismatch`; `a = b`, `a != b`, `a < b`, `a <= b`, `a > b` and `a >= b` over the same pairs, in either order, refuse `ill_typed`/`type-mismatch` at the operator."
    },
    {
      "id": "FR-397-AC-16",
      "path": "spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md",
      "role": "examined",
      "excerpt": "In a unit that also declares a float parameter, which admits the profile, `filament::value::ieee::sqrt(a, b)`, `fma(a, b)` and `numericEqual(a)` refuse `ill_typed`/`type-mismatch` at the call, while `sqrt()` and `bitIdentical()` refuse `invalid_syntax`/`unexpected-token` at the closing `)` before the checker runs; `sqrt(i)` over an `Int[0, 10]` parameter, `totalOrder(f32, f64)` over a `Float32` and a `Float64`, `numericEqual(a, b)` over `Float64[nearest-even]` and `Float64[toward-zero]` parameters and `fma(a, b, i)` refuse `ill_typed`/`type-mismatch` at the argument (each of those rows is in a"
    },
    {
      "id": "FR-397-AC-25",
      "path": "spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md",
      "role": "examined",
      "excerpt": "In a unit whose only parameters are `i: Int[0, 10]` and `a: Boolean` (no float form), `filament::value::ieee::sqrt(i)`, `sqrt(i, i)` and `numericEqual(i)` each refuse `invalid_package` with the cause `IeeeProfileNotAdmitted` at the call, and not `ill_typed`/`type-mismatch`; adding one parameter `f: Float64[nearest-even]` to the unit makes `sqrt(i)` and `sqrt(i, i)` refuse `ill_typed`/`type-mismatch` as FR-397-AC-16 states; `sqrt()` refuses `invalid_syntax`/`unexpected-token` at `)` in both units."
    },
    {
      "id": "FR-492-AC-8",
      "path": "spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md",
      "role": "examined",
      "excerpt": "Over a replay-root request for the seven-row refinement, the CG-format Base and Step handoff assembles in row order in either input order, with exact query/proof bytes preserved; the zero-row refinement accepts Base only. No caller-provided count defines the selected refinement."
    },
    {
      "id": "FR-492-AC-13",
      "path": "spec/functional/FR-492-expose-the-refinement-simulation-query-through-the-replay-facade.md",
      "role": "examined",
      "excerpt": "The public refinement_key accessor returns the exact key selected from the retained closure, and step_count returns the authoritative declaration-order row count for seven-row and zero-row selections; an unrelated key faults rather than giving a count. The driver needs no private request type or caller row-count authority."
    },
    {
      "id": "FR-663-AC-1",
      "path": "spec/functional/FR-663-report-the-parsed-unit-and-the-locus-of-every-refusal.md",
      "role": "examined",
      "excerpt": "`OutcomeDocument::from_parse` over the `parse` operation's result for `tests/fixtures/spine-compile.native` with identity `agent-ix`/`test:parent` has `result` of kind `parsed` whose `source` is those labels, `path` the fixture path, `bytes` the fixture's byte count, and `usage` the four S1 settings in order with each `bound` the default and each `used` the count the parse reached, `s1.input_bytes` used equal to `bytes`. A unit holding `a < b < c` has a `parsed` result beside its `invalid_syntax` diagnostic. A parse with a blank label has `result` `null`."
    },
    {
      "id": "FR-763-AC-2",
      "path": "spec/functional/FR-763-name-the-construct-in-every-forms-stage-refusal.md",
      "role": "examined",
      "excerpt": "S2 builds function bodies `null` and `xs[0]` as `Null` and `Index`, retaining the index's operand, selector and span; their parse outcome documents have no diagnostics and exit 0."
    },
    {
      "id": "FR-767-AC-11",
      "path": "spec/functional/FR-767-supply-the-integer-division-law-from-a-command-option-or-request-member.md",
      "role": "examined",
      "excerpt": "`spine::check_with_integer_division` given the floor law over FR-670-AC-1's unit without its law header checks, its `div` and `rem` nodes typed under floor; the same unit through `spine::check` with no selection refuses `missing_declaration`/`missing-selection` at each operator; a Markdown document of two `ix:native` fences, each declaring a `div` function with no law header, checked through `check_native_fences` with the floor law reports both fences checked."
    },
    {
      "id": "FR-880-AC-8",
      "path": "spec/functional/FR-880-write-every-checked-protocol-construct-at-s4.md",
      "role": "examined",
      "excerpt": "`repeat Loop by Seller visible (got.ok) while { not got.ok }` over a body that binds `got` compiles with `test` `post`. A `repeat` that steps only through a nested `repeat` compiles. Both packages are admitted. A post-test `repeat` whose body has a cycle through no step is still refused at S3 by FR-218's progress rule."
    },
    {
      "id": "FR-910-AC-2",
      "path": "spec/functional/FR-910-check-and-compile-a-unit-that-declares-only-aliases-or-dimensions.md",
      "role": "examined",
      "excerpt": "A unit whose only declaration is `type I = Integer;`, a unit of `type I = Integer;` and `type J = Boolean;`, and a header-only unit each refuse at S3 check with code `missing_declaration` and cause `empty-package`, located at the unit's `language` header declaration (from byte 0 to its closing `;`), retaining the package name `fixture` and the unit's source identity (authority `a`, identity `u`), and compile through the spine refuses with the same code and cause; neither refusal has code `unsupported_projection`, and neither message contains `no node of the package`."
    },
    {
      "id": "FR-940-AC-9",
      "path": "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md",
      "role": "examined",
      "excerpt": "A unit declaring `probabilistic Stable ... { probability >= 0.5 [ holds(true) ] }` and `probabilistic Stable ... { probability >= 0.25 [ holds(true) ] }` is refused `ambiguous_declaration`/`ambiguous-name` at the second `Stable`, naming both; with the second named `Other` it checks, and its package carries one `claim_names` row per claim. The package of a unit whose operation has two postconditions `Q1` and `Q2` constraining one field (a derived conjunction, FR-105-AC-14) is admitted by the strict reader, and `claim_name(\"Q1\")` returns `Q1`'s own node, never the derived node, over the source r"
    },
    {
      "id": "FSpec-FR-352-AC-22",
      "path": "../filament-specification/spec/functional/foundation/FR-352-mint-native-run-result-witness-wire.md",
      "role": "examined",
      "excerpt": "A `window-limit-reached` disposition is written at stage `evaluate` as `{\"kind\": \"window-limit-reached\", \"limit\": \"max_window_values\", \"value\": \"2\", \"setting\": \"monitor.max_window_values\"}`, its `value` a decimal string, and reads back equal; the same row with `value` the JSON number 2, with `value` `\"02\"` or with `value` `\"\"` refuses with a located wire diagnostic naming `value`."
    },
    {
      "id": "FSpec-FR-339-AC-1",
      "path": "../filament-specification/spec/functional/source-package/FR-339-define-complete-native-grammar.md",
      "role": "examined",
      "excerpt": "Every production in the base and complete-V1 facet grammar has positive, boundary and located-negative corpus vectors."
    },
    {
      "id": "FR-059",
      "path": "spec/functional/FR-059-check-backend-dependency-direction.md",
      "role": "examined",
      "excerpt": "and its normal dependency on `fsl-package` for the types that layer owns,\n  such as `ObligationIdentity`."
    },
    {
      "id": "FR-286",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "examined",
      "excerpt": "`OutcomeDiagnostic` takes its `hint` from\n`hint_for` when it is built, through every constructor; no producer,\nconsumer or driver writes, edits or removes a hint."
    }
  ]
}
```

## Verdict

CONDITIONAL: no scoped findings. Required named titan runtime evidence remains pending. No merge recommendation until those results pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Fix-commit disposition — round 1

CONDITIONAL: no remaining scoped finding found in the confined delta `f11f8fd554609c447b3677f0b88061d62a3118c0..cee01a2176443872658839217ad11171b75a670a`. This disposition examines only the two changed test source files and initial review-artifact custody. Product fixes are untouched. The original Findings table above remains unchanged; its sole placeholder is not a finding and there are no finding dispositions to enumerate.

The parse-outcome fixture now explicitly types the matched tuple as `(&CompileRefusal, Option<&fsl_replay::spine::ParseReport>)`. `StageFailure::RefusedWithPayload` contains `Box<C>` while `Refused` contains `C`; the additional `.as_ref()` in the payload arm reaches the same borrowed `CompileRefusal` as the non-payload arm. The S1/S2 carrier handling, source cases, code/stage/category/exit assertions, original refusal locus and complete document/report comparisons are unchanged. This addresses the lead-reported E0308 at lines 159–162 by type coercion, not by weakening an oracle. No remaining defect was found in that source change; compilation and all 16 previously blocked integration cases still need the lead's titan result.

The large 100000-term replay test was restored exactly to `origin/main`, including the original 256 MiB setting. Its file SHA-256 is `c99d6a72d2856f28537bbd2d7eab9a15cad64d3ee36dfd6624ce62ab0f8558a8` on both sides. Arch's ruling removes the earlier limit change from #3476 after a 12 GB watchdog kill. R11/OOM-S2 remains a known main red owned by QSL-2825, outside this PR; it is not resolved or waived by this review. The source/list size, assertions and execution are not shrunk, ignored or retuned, and this case must not be rerun for #3476. The original review's discussion of the 512 MiB change records its historical candidate only; it does not describe the current head.

Lead-supplied titan evidence, not independently executed by this reviewer: CST 4 PASS; compile facade 3 PASS; dispatch 2 PASS; library 5 PASS; doctests 2 PASS. The 16 integration cases were blocked by E0308 before their assertions could run. Their corrected rerun remains pending. The large case's watchdog kill is not a PASS. No repository build, test, scanner or job was run or queued by this reviewer. `git diff --check` for the confined delta passed. The original native-matrix size refusal/static-matrix caveats remain; no matrix or full audit was repeated.

Initial SR artifacts were copied unchanged into the frozen candidate by the coder. These appended scratchpad dispositions retain the same IDs and original findings, for the lead to copy later. Both original review methods retain a CONDITIONAL verdict pending the 16 named titan checks, with QSL-2825 explicitly outside #3476 scope. No new findings or additional review methods were added.
