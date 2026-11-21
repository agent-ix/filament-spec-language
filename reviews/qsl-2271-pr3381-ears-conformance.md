---
id: SR-20261121003381003
title: "PR3381 package analyze clocks ears-conformance review"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; QSL-2271; spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md:318-322; FR-640 package-run context; existing run/analyze intakes"
review_set: subset
---

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3381. Independent reviewer of the complete one-line diff at frozen head `9f41ad372d1b6309cbb6e9bdaa894ed5b5e5730c`, base `fd0794de74a9f98f9b22ba3d557eb638ca4340cd`.

No `type: AssuranceProfile` artifact was found under `spec/`; no enforced review policy applies. This is a direct subset review selected by the dispatching lead. Scope is the inserted `clocks` list member and its existing contract context, not the whole FR or repository. No source edits, build, Cargo, test execution, aggregate gates, or runtime PASS claims.

## Verdict

PASS — the edited admission obligation has one explicit trigger (a package request carrying a nonempty compile-only member), the request as its subject, and one mandatory refusal response. Adding `clocks` does not change the trigger's nonempty qualification, the SHALL modality, named member, code, request stage or exit. The unchanged explanatory sentence states why compilation choices are already settled. Current Quire validation of FR-850 reports one grammar-clean document and zero grammar findings. FR-640 context validation has three pre-existing grammar warnings at lines 160/166 outside the edited clause; these are not attributed to this insertion.

## Examined scope

```json
[
  {
    "id": "FR-850 Admission compile-only members",
    "path": "spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md",
    "role": "examined",
    "excerpt": "- A request carrying both `package` and `program`, or `package` with a\n  non-empty `libraries`, `clocks`, `refinement_internals` or `integer_division`,\n  SHALL refuse the whole request `invalid-request` at the request stage,\n  exit 20, naming the second member. These members configure a compile, and\n  the package was already compiled."
  },
  {
    "id": "FR-850-AC-4",
    "path": "spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md",
    "role": "examined",
    "excerpt": "AC-1's package request with `program` also present, with a `libraries` entry, and with `integer_division` `floor` each refuse `invalid-request` at the request stage naming that member, exit 20; with `models` empty it refuses `missing_import`/`missing-selection` naming `test/counter`; with an extra `models` entry the package does not record it refuses `invalid_package`/`conflicting-definition` naming that entry."
  },
  {
    "id": "FR-640 package-run boundary",
    "path": "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md",
    "role": "context_only",
    "excerpt": "- A run request that selects a checked package instead of compiling source SHALL\n  NOT carry `clocks`: the package already holds its `temporal_bindings` rows."
  },
  {
    "id": "FR-640-AC-4",
    "path": "spec/functional/FR-640-select-clock-configurations-in-compile-and-run-requests.md",
    "role": "context_only",
    "excerpt": "A `clocks` member with an unknown `kind`, a `fixed_sample` with `period` `{\"kind\": \"integer\", \"decimal\": \"0\"}`, and a request carrying `clocks` with a package selection each refuse `invalid-request` at stage `request`, exit 20, naming the member."
  },
  {
    "id": "package_run::validate_request",
    "path": "fsl-command/src/command/package_run.rs",
    "role": "context_only",
    "excerpt": "if !request.clocks.is_empty() {\n        return Err(invalid(\"clocks\", \"a package already records its clocks\"));\n    }"
  },
  {
    "id": "analyze::admit_package",
    "path": "fsl-command/src/command/analyze.rs",
    "role": "context_only",
    "excerpt": "fn admit_package"
  },
  {
    "id": "AnalyzeRequest.clocks",
    "path": "fsl-command/src/command/wire.rs",
    "role": "context_only",
    "excerpt": "#[serde(default, deserialize_with = \"clocks\")]\n    pub clocks: fsl_replay::spine::RequestClocks,"
  },
  {
    "id": "compile_members_and_mismatched_models_refuse_beside_a_package",
    "path": "fsl-analyze/tests/it/analyze_package_selection.rs",
    "role": "context_only",
    "excerpt": "#[trace(\"TC-2410\", \"FR-850-AC-4\")]"
  }
]
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Method limits

Only code-review, base and ears-conformance apply. Rust review and gap-analysis are not run because no production/test code changed. Integrity/object/dependency lenses have no structural, domain-object or relationship edit. Scope-boundary, failure-domain, evidence and risk-complexity do not warrant separate analyses for the added token; the directly affected ownership, refusal, empty/nonempty boundary and unchanged AC mapping are checked in base. Criterion-strength/Jev and whole-repository matrix are outside the requested bounded review. No unavailable matrix result is represented as evidence.

## Publication and custody

Linear operations are prohibited by dispatch cooldown through 19:45 UTC. Exact marked comment is queued beside this artifact; no tracker reads, posts, label updates or read-back have occurred. Publication/read-back and `has-review` remain pending for a resumed pass. The dispatching lead owns committing this exact artifact under `reviews/qsl-2271-pr3381-ears-conformance.md` before merge; the reviewer does not mutate the frozen branch.
