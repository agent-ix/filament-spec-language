---
id: SR-202611210022442313144108538
title: "QSL-2691 explicit-limit implementation code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3364; QSL-2691; Cargo.toml, Cargo.lock, fsl-package/src/emit.rs, fsl-package/src/emit/tests/deep_bodies.rs, fsl-package/src/emit/tests/deep_bodies/explicit_limit.rs, tests/it/bare_package.rs"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-531", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-264", type: references}
---

## Summary

One independent code-review/Rust pass over frozen source `39b70ba1916200d129d45084f7bbaafb93d5d6b6`, against the approved FR-531-AC-41/TC-3133 amendment. The actual large qualification is sound; two stale existing requirement/oracle statements must be aligned with that already approved rule before merge. No source was edited and no Cargo invocation was run by this reviewer.

## Verdict

**PASS at ffccf83dbf62a1929115bc236dd7c22d06317356** — both findings are fixed in disposition round1 below; no new scoped defect was found. The original review was CONDITIONAL. First-unpaid semantics, explicit caller limits and the unchanged16MiB default are preserved. The one large qualification remains used once, without rerun.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The existing remembered-emission contract and zero-bound oracle still require actual N, but the authorized first-unpaid helper now returns bound + 1; at bound 0 the existing assertion expects N and receives 1. | fsl-package/src/emit/tests/output_limit.rs:126; spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:85; fsl-package/src/emit.rs:470 |
| FND-002 | medium | FR-264-AC-8 and TC-738 step 5 still say the deep sum emits under the 16 MiB default as FR-531-AC-41 states, although the approved AC-41 now requires predeclared explicit limits and exact refusal under that unchanged default. | spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:86; spec/test-cases/TC-738-a-deep-package-emits-reads-back-and-verifies.md:36 |

## Implementation and Rust evidence

The two source families are authentic and independent: the sum creates 100,000 additions/100,001 parameter terms; the let family checks every next local slot, previous-local-plus-one value and final binder read. Real S1/S3 checking and S4 emission run on a 512 KiB thread; each family joins before the next. Term references and one current expected path are retained, rather than a collection of expanded paths. Shared-ancestor induction ties the expected vector to the checker's persistent location; every complete lent path and each exact relative row is asserted. This tests genuine strict I2 and the public lending cursor, without a duplicate serializer/reader, private upstream API or alternate wire representation.

The fixed B/W/R/L formulas use source byte length and checked node/edge counts before emission. Their coefficients describe the closed families' fixed keys, references, rows and native walks; they do not use observed bytes/work, depth-dependent coefficients or u64::MAX. Native strict-read, read-storage and cursor counters are independently measured. The full-path test-oracle comparisons are not a production-time linearity claim, and cumulative read-storage requests are not RSS. S4 exposes no emitter-work meter. The production helper widens the u64 bound before adding one; inclusive boundaries and typed refusal/no package return are preserved. The conflict is the unreconciled existing contract/oracle, not a request to undo the approved first-unpaid behavior.

Only the root test observation dev-dependency and matching lock dependency-list edge are added; pins and production dependencies are unchanged. The handoff change updates a stale /4 assertion to the writer/strict reader's existing /5, preserving deterministic read/re-encode checks. No public API, schema/version, unsafe path, workflow or build gate is changed. No applicable AssuranceProfile is declared under spec/.

## Validation

- Independently read and hashed `/private/tmp/qsl2691-measure-39b70-once.log`: SHA256 `c6793ae3e61ef79c4ef8ad7c8ac79514a0e31877fcbb32a7788a99cc82082594`. Actual exact large result: 1 passed, 0 failed, 0 ignored, 1070 filtered, 506.19 s; source candidate above, one invocation, no pause/retry per execution handoff.
- Sum: 216255903 complete bytes, 100000 rows, 199999 entries, strict work 4200075, charged read storage 7823148825, logical storage 0, cursor work 1862149. Lets: 499862214 bytes, 100000 rows, 299999 entries, strict work 15206928, read storage 18211567567, logical storage 0, cursor work 3231080. Both default denials: bound 16777216, first unpaid 16777217.
- Read the small-c5 log: 2 passed/0 failed/0 ignored at the same source. Read handoff-c1: 1 passed/0 failed/0 ignored, 1366 filtered at f948f44fd3720d8d7e89dd6206274a4bc54eef77. Git confirms all later changes affect only the new fixture's read-storage/literal oracle.
- Direct `rustfmt --check --config skip_children=true` on the four changed Rust files: exit 0. `git diff --check`: exit 0. An initial recursive formatter inspection also found an unchanged protocol_view.rs:1225 formatting difference; this is outside the diff, not an edited-file failure.
- Static trace of output_limit -> remembered_within and the existing bound-0 assertion establishes FND-001; that existing test was not executed here. No broad tests, Clippy, deny or aggregate gate were run, per the lead's focused-only/no-Cargo review instruction.

## Coverage

All six changed files and the relevant existing emitter paths/oracles were inspected. Code-review includes the Rust API, implementation, testing, safety/resource and wire/error lanes. Spec review subanalyses are not applicable to the initial six-file diff because it changes no specification; targeted spec alignment can be assessed in the same review's disposition. Global gap findings are in the separate gap-analysis artifact. Optional repository-wide semantic gap analysis was not requested. Plan completion: not assessed.

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
      "id": "FR-264-AC-7",
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "role": "examined",
      "excerpt": "A package emitted once and then offered again to an emit entry at a bound below its remembered byte length returns the same `OutputLimit` refusal with actual `N`; at bound `N` it returns the remembered emission. The compile lifecycle given `emit.output_bytes` `N - 1` returns `CompileRefusal::Limit` naming `emit.output_bytes`, and given `N` emits.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-264-AC-8",
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "role": "examined",
      "excerpt": "A 100,000-term sum `x + x + ... + x` nested on the left, whose `body_locations` rows hold at most 200,000 `up` and `down` entries together (FSpec FR-322-AC-510 BL-D1), is emitted under the default `emit.output_bytes` 16777216 on a thread with a 512 KiB stack, as FR-531-AC-41 states; at a bound one byte below its emitted length it returns `EmitRefusal::OutputLimit` with code `stage_limit_exceeded`, setting `emit.output_bytes`, that bound and an actual count above it, charged for its rows (Behavior 5), and writes no bytes.",
      "excerpt_part": "1/1"
    },
    {
      "id": "emit::output_limit",
      "path": "fsl-package/src/emit.rs",
      "role": "examined",
      "excerpt": "actual.min(u128::from(bound) + 1)"
    },
    {
      "id": "root-observation-test-edge",
      "path": "Cargo.toml",
      "role": "examined",
      "excerpt": "filament-observation = { workspace = true }"
    },
    {
      "id": "explicit-limit-fixtures",
      "path": "fsl-package/src/emit/tests/deep_bodies.rs; fsl-package/src/emit/tests/deep_bodies/explicit_limit.rs",
      "role": "examined",
      "excerpt": "    on_small_stack(100_000, true);"
    },
    {
      "id": "identities-format-oracle",
      "path": "tests/it/bare_package.rs",
      "role": "examined",
      "excerpt": "    assert_eq!(file[\"format\"], \"filament.checked-handoff.identities/5\");"
    },
    {
      "id": "TC-738",
      "path": "spec/test-cases/TC-738-a-deep-package-emits-reads-back-and-verifies.md",
      "role": "examined",
      "excerpt": "- Step 5: the sum emits under the default bound with at most 200,000 location entries; one byte below its length it refuses `OutputLimit` naming `emit.output_bytes` and writes no bytes."
    },
    {
      "id": "root-lock-observation-edge",
      "path": "Cargo.lock",
      "role": "examined",
      "excerpt": " \"filament-observation\","
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "test-intent",
      "artifact_id": "FR-264-AC-7",
      "related_ids": [
        "FR-531-AC-41",
        "FR-264",
        "TC-738"
      ],
      "path": "fsl-package/src/emit/tests/output_limit.rs",
      "lines": "126-149",
      "excerpt": "A package emitted once and then offered again to an emit entry at a bound below its remembered byte length returns the same `OutputLimit` refusal with actual `N`; at bound `N` it returns the remembered emission. The compile lifecycle given `emit.output_bytes` `N - 1` returns `CompileRefusal::Limit` naming `emit.output_bytes`, and given `N` emits.",
      "finding": "The existing remembered-emission contract and zero-bound oracle still require actual N, but the authorized first-unpaid helper now returns bound + 1; at bound 0 the existing assertion expects N and receives 1."
    },
    {
      "fnd": "FND-002",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "other",
      "artifact_id": "FR-264-AC-8",
      "related_ids": [
        "FR-531-AC-41",
        "TC-738"
      ],
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "lines": "86",
      "excerpt": "A 100,000-term sum `x + x + ... + x` nested on the left, whose `body_locations` rows hold at most 200,000 `up` and `down` entries together (FSpec FR-322-AC-510 BL-D1), is emitted under the default `emit.output_bytes` 16777216 on a thread with a 512 KiB stack, as FR-531-AC-41 states; at a bound one byte below its emitted length it returns `EmitRefusal::OutputLimit` with code `stage_limit_exceeded`, setting `emit.output_bytes`, that bound and an actual count above it, charged for its rows (Behavior 5), and writes no bytes.",
      "finding": "FR-264-AC-8 and TC-738 step 5 still say the deep sum emits under the 16 MiB default as FR-531-AC-41 states, although the approved AC-41 now requires predeclared explicit limits and exact refusal under that unchanged default."
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
    },
    {
      "test_id": "a_remembered_emission_is_held_to_the_bound_of_the_later_call",
      "ac_id": "FR-264-AC-7",
      "trace": "correct"
    }
  ]
}
```

## Dispositions

Round1 — frozen `ffccf83dbf62a1929115bc236dd7c22d06317356`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-264 Behavior5/AC6/AC7 and TC738 now consistently specify first unpaid B+1 for fresh, row-preflight and remembered refusals; cached zero-bound oracle asserts B+1 and exact-N success remains byte-equal. Actual changed-head small qualification passed3/0/0. |
| FND-002 | fixed | FR-264-AC8/TC738 now require the approved true100000-addition explicit-limit positive and unchanged16MiB exact denial. FullAC8 trace moved to the actual qualified large case, not small helper or historical99999-add fixture. Measured program/formulas/production helper unchanged. |

## Disposition round1 evidence

Verified the fix diff from39b70 toffcc, raw focused log at91bff9f682817fb6e2cff5772344087719470981 (3 passed,0 failed,0 ignored,1068filtered,rc0; SHA2565fd0dae3899bd7b6c0b0b3fbaed122a0ee95ce42ed370decf064743dd4d578d7), and final trace-only delta. Production emit.rs and B/W/R/L/program/oracle code of the large case are unchanged from the actual one-run qualification. No Cargo invocation was run by this reviewer. Targeted Quire validation reports2/2docs grammar-clean,0findings; direct changed-test rustfmt and gitdiff whitespace checks pass.

A single computed trace refresh for the changed binding reports4877tagged/1143untagged/82ignored-only/106method-without-symbol/13retired. FR264AC6 andAC7 are tagged by their real focused tests. AC8's sole binder is the actual100k large test and is correctly classified tagged-by-ignored-test for ordinary runs. The actual explicit1PASS at39b70 qualifies this delivery without pretending the default suite executes it. No full reverse audit or initial review was repeated.

```json
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "A package emitted once with canonical byte length `N` and then offered again to an emit entry at a bound `B < N` returns the same `OutputLimit` refusal with configured bound `B` and actual first unpaid byte `B + 1`, including actual 1 at bound 0; at bound `N` it returns the remembered emission unchanged. The compile lifecycle given `emit.output_bytes` `N - 1` returns `CompileRefusal::Limit` naming `emit.output_bytes`, and given `N` emits.",
      "reason": "FR-264 Behavior5/AC6/AC7 and TC738 now consistently specify first unpaid B+1 for fresh, row-preflight and remembered refusals; cached zero-bound oracle asserts B+1 and exact-N success remains byte-equal. Actual changed-head small qualification passed3/0/0."
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "With the same checked input and other stage ceilings unchanged, setting only `emit.output_bytes` to the published default 16777216 instead returns `EmitRefusal::OutputLimit` with code `stage_limit_exceeded`, setting `emit.output_bytes`, configured bound 16777216 and actual first unpaid byte 16777217, and no partial package.",
      "reason": "FR-264-AC8/TC738 now require the approved true100000-addition explicit-limit positive and unchanged16MiB exact denial. FullAC8 trace moved to the actual qualified large case, not small helper or historical99999-add fixture. Measured program/formulas/production helper unchanged."
    }
  ]
}
```
