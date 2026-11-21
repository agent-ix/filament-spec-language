---
id: SR-202611210022442313144108539
title: "QSL-2691 explicit-limit implementation gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository spec/, source and tests; QSL-2691 PR3364 frozen39b70ba1916200d129d45084f7bbaafb93d5d6b6"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-531", type: references}
---

## Summary

Repository-driven, planless matrix/reverse-gap/stub audit, with direct ownership and assertion checks on the PR's changed surface. FR-531-AC-41 is now statically tagged and has actual focused execution evidence, but the repository-wide audit retains baseline traceability gaps and incomplete reverse mapping; it is not a global PASS.

## Verdict

**FAIL** — the global untagged, ignored-only and incomplete reverse-audit findings remain. They are repository-level baseline findings, not proof of a new gap in this six-file change. Only the lead may accept them with an explicit scoped disposition. The changed behavior maps to FR-531-AC-41/TC-3133, existing FR-264/TC-738 output-limit rules and FR-531's identities/5 contract; the separate code-review artifact records two stale existing rules/oracles requiring alignment with the already approved amendment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The full-repository static matrix reports 1143 untagged criteria, including live FR-001-AC-7; therefore this audit cannot establish repository-wide traceability. | spec/functional/FR-001-read-exact-source.md::FR-001-AC-7; /private/tmp/qsl2691-review-quire-matrix.json |
| FND-002 | high | The full-repository static matrix reports 81 criteria bound only by ignored tests, including FR-056-AC-60; ordinary test execution supplies no coverage for those rows. | spec/functional/FR-056-admit-domain-package-model-declarations.md::FR-056-AC-60; fsl-eval/tests/it/model_simulation.rs::a_float_field_model_explores_by_its_ieee_bits |
| FND-003 | high | The full-repository reverse audit is incomplete: 8036 public-surface candidates were inventoried but were not exhaustively mapped to owning requirements; extraction diagnostics and unmatched tags also remain globally unclassified. | /private/tmp/qsl2691-review-public-surface.txt; /private/tmp/qsl2691-review-coverage.json |

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2) — no run evidence read by the matrix. Quoin 0.36.7 `matrix --repo /Users/peter/dev/qsl-2691-code --json` exited 1: `Error: native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. This exact failure was recorded once; the lead's approved static fallback ran once and exited 0. No old binary, retry loop, hand-written matrix or grep-based coverage percentage was used.

The computed 6221 criteria have statuses: tagged 4878; untagged 1143; tagged-by-ignored-test 81; method-without-symbol 106; retired 13. These are engine statuses, not a claim that all 1143 untagged rows are live: some textually retired rows are still minted untagged, so FR-001-AC-7 is the concrete live example used for FND-001. Qualified-record: none reported. Zero criteria was not the result. FR-531 retains 44 criteria; AC-41 has three binders: two ordinary small tests and the named ignored large test. The latter actually ran explicitly once with 1 passed/0 failed/0 ignored, so it is not counted as a passing ignore. The small helper assertions alone do not qualify the 100000-step size.

Quire coverage exited 0. Its Rust census is 13674 candidates, 13104 tagged, 12583 bound; it reports 5834 untracked-symbol records and 22767 diagnostics: trace-id-outside-marker 22272, untracked-id-has-minted-children 465, uncatalogued-verification-method 22, section-matches-nothing 6, archetype-matches-nothing 1, catch-all-universal 1. These are distinct reasons, not one missing-test class. New external FSpec-FR-322-AC-510 tags are intentionally prefixed under AGENTS.md and have no local minted criterion; they are not a local typo. Global classification of all remaining unmatched tags/diagnostics is incomplete (FND-003). Hand-written matrix-row status fields were not audited or edited.

A whole-source candidate inventory found 8036 unindented public item/function declarations; it is not a complete behavior count and does not include every impl method. The changed production behavior, three new tests, module declaration, manifest/lock edge and handoff oracle have explicit owners and real assertions. A whole-source stub-pattern scan yielded 19 candidates; all 19 were inspected and are doctest examples or scanner fixtures containing inert todo!/unimplemented! text, not new live stubs. No scoped source or test stub was confirmed. This pattern scan does not exhaustively prove global absence of stubs, vacuous returns or untraced behavior.

Optional repository-wide semantic review: skipped (not requested). Direct test-intent/source checks required by code-review did run. No plan was selected or manufactured, no requirements/trace tags were authored, and no Cargo/tests were rerun by this reviewer.

## Scoped execution evidence

Independently read large raw log SHA256 c6793ae3e61ef79c4ef8ad7c8ac79514a0e31877fcbb32a7788a99cc82082594: one exact test, 1 passed/0 failed/0 ignored, 1070 filtered, 506.19 s at frozen source. Sum bytes216255903/rows100000/entries199999/strictwork4200075/cursor1862149; lets bytes499862214/rows100000/entries299999/strictwork15206928/cursor3231080. Both strict admissions, complete current checker-path comparisons and default16MiB first-unpaid16777217 denials execute. Two small cases pass at the same source, and the assigned identities/5 handoff test passes at f948f44fd3720d8d7e89dd6206274a4bc54eef77; only the new fixture's storage/literal setup changed afterward.

## Review data

```json
{
  "clean": false,
  "scope": [
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "For two separate units, one holding `function d(x: Integer): Integer pure { x + x + ... + x }` with 100,000 `+` applications (100,001 terms), the other holding 100,000 nested `let`s whose first binding is `x + 1`, each subsequent binding is the previous binder plus 1 and the result is the last binder, S4 writes complete packages and their `body_locations` on a 512 KiB stack under finite explicit caller ceilings chosen before emission from the checked input and the declared linear bounds above. The actual complete canonical byte lengths, relative entry counts and charged native strict",
      "excerpt_part": "1/3"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "-read and lending-cursor work are recorded separately for the two units and do not exceed those bounds. The rows use FSpec FR-322's relative form (FSpec FR-322-AC-510's BL-D1 and BL-D2 rows exactly); the strict reader admits each package, and each typed path lent in body-walk order equals the checker's path without retaining all expanded paths. With the same checked units and other stage ceilings unchanged, setting only `emit.output_bytes` to the published default 16,777,216 instead refuses `OutputLimit` with setting `emit.output_bytes`, configured bound 16,777,216 and first unpaid b",
      "excerpt_part": "2/3"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "excerpt": "yte 16,777,217, and no partial package. A row written with a whole `path` is refused `malformed_wire` at its `location`.",
      "excerpt_part": "3/3"
    },
    {
      "id": "explicit-limit-fixtures",
      "path": "fsl-package/src/emit/tests/deep_bodies.rs; fsl-package/src/emit/tests/deep_bodies/explicit_limit.rs",
      "role": "examined",
      "excerpt": "    on_small_stack(100_000, true);"
    },
    {
      "id": "FR-001-AC-7",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`. From initial `a` `3ff0000000000000` (1.0) it reaches exactly the states 1.0 and `bff0000000000000` (-1.0), with one candidate per application. From initial `a` `0000000000000000` (+0) it reaches exactly +0 and `8000000000000000` (-0), two states with distinct keys, and each application has two candidates, because both zeros are `numericEqual` to the solved value (FR-120-AC-34). A simulation ",
      "excerpt_part": "1/2"
    },
    {
      "id": "FR-056-AC-60",
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "role": "examined",
      "excerpt": "run applying `flip()` twice from 1.0 ends at 1.0 with no flag.",
      "excerpt_part": "2/2"
    },
    {
      "id": "spec/spec.md",
      "path": "spec/spec.md",
      "role": "context_only",
      "excerpt": "name: filament-spec-language\norg: agent-ix\ncomponent_type: rust-lib"
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-001-AC-7",
      "related_ids": [],
      "path": "spec/functional/FR-001-read-exact-source.md",
      "lines": "Acceptance Criteria",
      "excerpt": "Package declarations holding one record `Point` with field `x: Int[0, 9]`, checked under the source reference of bytes `b` admitted as authority `a`, identity `u`, under the reference of bytes `b'` admitted as `a`, `u`, under the reference of `b` admitted as authority `c`, identity `u`, and under the reference of `b` admitted as authority `a`, identity `v`, give `Point` the same node key `{package, path: [\"Point\"], ordinal: 0}` each time: the source reference is no part of a node key.",
      "finding": "The full-repository static matrix reports 1143 untagged criteria, including live FR-001-AC-7; therefore this audit cannot establish repository-wide traceability."
    },
    {
      "fnd": "FND-002",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-056-AC-60",
      "related_ids": [],
      "path": "spec/functional/FR-056-admit-domain-package-model-declarations.md",
      "lines": "Acceptance Criteria",
      "excerpt": "Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()` with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state checker returns no `RequiresBound`. From initial `a` `3ff0000000000000` (1.0) it reaches exactly the states 1.0 and `bff0000000000000` (-1.0), with one candidate per application. From initial `a` `0000000000000000` (+0) it reaches exactly +0 and `8000000000000000` (-0), two states with distinct keys, and each application has two candidates, because both zeros are `numericEqual` to the solved value (FR-120-AC-34). A simulation ",
      "finding": "The full-repository static matrix reports 81 criteria bound only by ignored tests, including FR-056-AC-60; ordinary test execution supplies no coverage for those rows."
    },
    {
      "fnd": "FND-003",
      "severity": "high",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "spec/spec.md",
      "related_ids": [],
      "path": "spec/spec.md",
      "lines": "1-6",
      "excerpt": "name: filament-spec-language\norg: agent-ix\ncomponent_type: rust-lib",
      "finding": "The full-repository reverse audit is incomplete: 8036 public-surface candidates were inventoried but were not exhaustively mapped to owning requirements; extraction diagnostics and unmatched tags also remain globally unclassified."
    }
  ],
  "bindings": [
    {
      "test_id": "small_explicit_limit_families_check_actual_paths_and_relative_rows",
      "ac_id": "FR-531-AC-41",
      "trace": "correct"
    },
    {
      "test_id": "row_preflight_names_the_first_unpaid_byte_and_legacy_whole_path_refuses",
      "ac_id": "FR-531-AC-41",
      "trace": "correct"
    },
    {
      "test_id": "two_100000_step_families_measure_complete_wire_and_native_work",
      "ac_id": "FR-531-AC-41",
      "trace": "correct"
    }
  ]
}
```

## Dispositions

Round1 — frozen `ffccf83dbf62a1929115bc236dd7c22d06317356`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | The lead explicitly accepts the existing1143 untagged repository criteria for this scoped PR only. No new untagged criterion is introduced by this fix. Global gap verdict remains FAIL. |
| FND-002 | accepted-no-change | The lead explicitly accepts the existing81 ignored-only baseline criteria for this scoped PR only. Correcting AC8's binding makes the current engine count82; that added row is the explicitly qualified one-time100k lane, not ordinary default coverage. Its actual1PASS is retained; global gap verdict remains FAIL. |
| FND-003 | accepted-no-change | The lead explicitly accepts the incomplete global reverse audit/extraction classification for this scoped PR only. Scoped owned behavior and fixes were checked directly; no global assurance PASS is claimed. |

## Disposition round1 evidence

Verified the fix diff from39b70 toffcc, raw focused log at91bff9f682817fb6e2cff5772344087719470981 (3 passed,0 failed,0 ignored,1068filtered,rc0; SHA2565fd0dae3899bd7b6c0b0b3fbaed122a0ee95ce42ed370decf064743dd4d578d7), and final trace-only delta. Production emit.rs and B/W/R/L/program/oracle code of the large case are unchanged from the actual one-run qualification. No Cargo invocation was run by this reviewer. Targeted Quire validation reports2/2docs grammar-clean,0findings; direct changed-test rustfmt and gitdiff whitespace checks pass.

A single computed trace refresh for the changed binding reports4877tagged/1143untagged/82ignored-only/106method-without-symbol/13retired. FR264AC6 andAC7 are tagged by their real focused tests. AC8's sole binder is the actual100k large test and is correctly classified tagged-by-ignored-test for ordinary runs. The actual explicit1PASS at39b70 qualifies this delivery without pretending the default suite executes it. No full reverse audit or initial review was repeated.

```json
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "accepted-no-change",
      "reason": "The lead explicitly accepts the existing1143 untagged repository criteria for this scoped PR only. No new untagged criterion is introduced by this fix. Global gap verdict remains FAIL."
    },
    {
      "fnd": "FND-002",
      "outcome": "accepted-no-change",
      "reason": "The lead explicitly accepts the existing81 ignored-only baseline criteria for this scoped PR only. Correcting AC8's binding makes the current engine count82; that added row is the explicitly qualified one-time100k lane, not ordinary default coverage. Its actual1PASS is retained; global gap verdict remains FAIL."
    },
    {
      "fnd": "FND-003",
      "outcome": "accepted-no-change",
      "reason": "The lead explicitly accepts the incomplete global reverse audit/extraction classification for this scoped PR only. Scoped owned behavior and fixes were checked directly; no global assurance PASS is claimed."
    }
  ]
}
```
