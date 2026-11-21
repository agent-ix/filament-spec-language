---
id: SR-202611210022442313144108541
title: "QSL-2691 round1 targeted spec alignment integrity"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; QSL-2691 PR3364 round1; FR-264 Behavior5/AC6/AC7/AC8 and TC-738; FR-531-AC41 context"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-264", type: references}
  - {target: "ix://agent-ix/filament-spec-language/TC-738", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-531", type: references}
---

## Summary

Targeted integrity analysis of the spec alignment in disposition round1. No new finding.

## Verdict

**PASS** for the changed alignment scope at the frozen fix source. This does not claim repository-wide coverage or a second large execution.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Analysis

The edited Behavior5, AC6, AC7, AC8 and TC738 procedures/expected results agree on cached, fresh and row-preflight refusals. The apparent cached-N versus B+1 conflict is removed while exact-N byte-equal success remains. The default-success versus explicit-positive/default-negative conflict is removed. Fixed schema/walk coefficients and measured charged work retain the distinction between a structural linear bound and success at one size; no output-derived ceiling or RSS/emitter-work claim is added. Existing relationship edges and requirement owners are unchanged; no new US/StR/NFR mapping is minted or inferred. Extension points and purity are unchanged; deep topology and bounded work are explicitly retained. This is a consistency check of the fix, not a fresh whole-repository completeness audit.

## Coverage

Frozenfixsource `ffccf83dbf62a1929115bc236dd7c22d06317356`; same reviewer and disposition round1, not a new independent review. Targeted Quire validation of FR264 and TC738: exit0,2/2docs grammar-clean,0findings. No applicable AssuranceProfile requires an enforced run; direct subset base/integrity/EARS was selected under the reviewer role. No Cargo invocation or large rerun was performed.

Computed trace verification is exact: AC6/AC7 are tagged by real limit tests; AC8 is tagged-by-ignored-test with only the qualified true100k case. The current82ignored-only total comprises81baseline rows plus that explicit manual lane. Its actual1PASS at39b70 and the3focused small PASS at91bff are retained separately from ordinary default-run coverage. Code/formulas of the measured large case are unchanged except its trace marker. These statements do not upgrade the global gap FAIL.

Plan completion: not assessed.

## Review data

```json
{
  "clean": true,
  "scope": [
    {
      "id": "FR-264-AC-6",
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "role": "examined",
      "excerpt": "A package whose canonical emitted bytes number `N` is emitted at `emit.output_bytes` `N`, with bytes equal to the bytes emitted under a larger bound. At bound `N - 1` it returns `EmitRefusal::OutputLimit` with code `stage_limit_exceeded`, cause `input-bytes-exceeded`, kind input bytes, bound `N - 1`, actual first unpaid byte `N` and setting `emit.output_bytes`, and returns no partial package. At bound 0 it returns the same refusal with bound 0 and actual first unpaid byte 1.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-264-AC-7",
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "role": "examined",
      "excerpt": "A package emitted once with canonical byte length `N` and then offered again to an emit entry at a bound `B < N` returns the same `OutputLimit` refusal with configured bound `B` and actual first unpaid byte `B + 1`, including actual 1 at bound 0; at bound `N` it returns the remembered emission unchanged. The compile lifecycle given `emit.output_bytes` `N - 1` returns `CompileRefusal::Limit` naming `emit.output_bytes`, and given `N` emits.",
      "excerpt_part": "1/1"
    },
    {
      "id": "FR-264-AC-8",
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "role": "examined",
      "excerpt": "A sum `x + x + ... + x` with 100,000 `+` applications (100,001 terms) nested on the left emits a complete package on a thread with a 512 KiB stack under finite explicit caller ceilings derived before emission from the checked input and fixed schema/body-walk coefficients, as FR-531-AC-41 states. Its `body_locations` rows have at most 200,000 relative entries (one `up` per row plus its `down` entries; FSpec FR-322-AC-510 BL-D1), and its actual complete bytes and charged native strict-read and lending-cursor work are recorded against those bounds. With the same checked input and other ",
      "excerpt_part": "1/2"
    },
    {
      "id": "FR-264-AC-8",
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "role": "examined",
      "excerpt": "stage ceilings unchanged, setting only `emit.output_bytes` to the published default 16777216 instead returns `EmitRefusal::OutputLimit` with code `stage_limit_exceeded`, setting `emit.output_bytes`, configured bound 16777216 and actual first unpaid byte 16777217, and no partial package.",
      "excerpt_part": "2/2"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "context_only",
      "excerpt": "For two separate units, one holding `function d(x: Integer): Integer pure { x + x + ... + x }` with 100,000 `+` applications (100,001 terms), the other holding 100,000 nested `let`s whose first binding is `x + 1`, each subsequent binding is the previous binder plus 1 and the result is the last binder, S4 writes complete packages and their `body_locations` on a 512 KiB stack under finite explicit caller ceilings chosen before emission from the checked input and the declared linear bounds above. The actual complete canonical byte lengths, relative entry counts and charged native strict",
      "excerpt_part": "1/3"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "context_only",
      "excerpt": "-read and lending-cursor work are recorded separately for the two units and do not exceed those bounds. The rows use FSpec FR-322's relative form (FSpec FR-322-AC-510's BL-D1 and BL-D2 rows exactly); the strict reader admits each package, and each typed path lent in body-walk order equals the checker's path without retaining all expanded paths. With the same checked units and other stage ceilings unchanged, setting only `emit.output_bytes` to the published default 16,777,216 instead refuses `OutputLimit` with setting `emit.output_bytes`, configured bound 16,777,216 and first unpaid b",
      "excerpt_part": "2/3"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "context_only",
      "excerpt": "yte 16,777,217, and no partial package. A row written with a whole `path` is refused `malformed_wire` at its `location`.",
      "excerpt_part": "3/3"
    },
    {
      "id": "FR-264-Behavior-5",
      "path": "spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md",
      "role": "examined",
      "excerpt": "   fragment would cross the bound by several bytes. The same first-unpaid\n   count applies to a refused remembered emission; its complete remembered\n   length is not the refusal's actual count. No refusal returns a partial package."
    },
    {
      "id": "TC-738",
      "path": "spec/test-cases/TC-738-a-deep-package-emits-reads-back-and-verifies.md",
      "role": "examined",
      "excerpt": "- Step 4: the remembered emission is refused at each `B < N` with configured\n  bound `B` and actual first unpaid byte `B + 1` (including actual 1 at bound 0),\n  and returned unchanged at `N`; the\n  lifecycle returns `CompileRefusal::Limit` at `N - 1` and emits at `N`."
    }
  ],
  "findings": [],
  "bindings": [
    {
      "test_id": "a_package_is_emitted_at_its_exact_length_and_refused_one_below_and_at_zero",
      "ac_id": "FR-264-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "a_remembered_emission_is_held_to_the_bound_of_the_later_call",
      "ac_id": "FR-264-AC-7",
      "trace": "correct"
    },
    {
      "test_id": "two_100000_step_families_measure_complete_wire_and_native_work",
      "ac_id": "FR-264-AC-8",
      "trace": "correct"
    }
  ]
}
```
