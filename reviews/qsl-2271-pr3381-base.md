---
id: SR-20261121003381002
title: "PR3381 package analyze clocks base review"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language; QSL-2271; spec/functional/FR-850-analyze-a-selected-checked-package-from-its-bytes.md:318-322; FR-640 package-run context; existing run/analyze intakes"
review_set: subset
---

## Summary

Ticket: QSL-2271. PR: filament-spec-language#3381. Independent reviewer of the complete one-line diff at frozen head `9f41ad372d1b6309cbb6e9bdaa894ed5b5e5730c`, base `fd0794de74a9f98f9b22ba3d557eb638ca4340cd`.

No `type: AssuranceProfile` artifact was found under `spec/`; no enforced review policy applies. This is a direct subset review selected by the dispatching lead. Scope is the inserted `clocks` list member and its existing contract context, not the whole FR or repository. No source edits, build, Cargo, test execution, aggregate gates, or runtime PASS claims.

## Verdict

PASS — adding `clocks` to the existing compile-only list is a concrete and testable normative correction. A nonempty clock map alongside `package` must refuse the whole request with `invalid-request`, request stage, exit 20, naming `clocks`, before analysis; no per-item result or new refusal category is introduced. The existing non-empty qualifier remains intact: absence and `{}` supply no selection, consistent with run's guard and both request wires' default empty clock maps. FR-640's package-run rule supplies the clock-ownership rationale; FR-850 remains the owner of package-analyze admission. This inserts one member into that existing rule, without introducing a competing carrier or restating FR-640's configuration rules.

Existing FR-850-AC-4 maps the compile-only refusal family to TC-2410; its current examples/test body cover program, libraries and integer_division, and do not assert a clock case. Existing FR-640-AC-4 names package+clock refusal. Neither AC is edited or newly claimed to verify analyze clocks. The subsequent guard work needs its focused clock regression evidence; this review approves the normative clarification only. Adverse cases examined: nonempty valid map, absent map, empty object, program+package, malformed clock configuration (existing request decoder responsibility). No new ID, relationship or acceptance criterion is introduced, so there is no changed-ID/link or new criterion-binding obligation. No full matrix was invoked or coverage completion claimed.

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

Linear operations are prohibited by dispatch cooldown through 19:45 UTC. Exact marked comment is queued beside this artifact; no tracker reads, posts, label updates or read-back have occurred. Publication/read-back and `has-review` remain pending for a resumed pass. The dispatching lead owns committing this exact artifact under `reviews/qsl-2271-pr3381-base.md` before merge; the reviewer does not mutate the frozen branch.
