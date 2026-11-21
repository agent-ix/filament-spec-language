---
id: SR-17953050000007
title: "QSL-2691 explicit-limit contract spec-review/scope-boundary"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/filament-spec-language; QSL-2691; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md; spec/test-cases/TC-3133-function-body-losses-are-located-alike-on-both-routes.md"
review_set: subset
---

## Summary

S4 owns canonical emission/output-byte refusal (core plus cross-cutting limits). The caller owns finite predeclared input-derived ceilings. The existing Contract IR native strict reader and lending cursor own admission and reconstruction charges; their FSpec FR-322 row contract is guaranteed through exact row checks and strict-read/typed-path qualification. No dependency, wire rewrite or emitter-work counter is introduced; EmitLimits remains a byte ceiling. A diagnostic source occurrence is explicitly not wire-member byte attribution.

Ticket: QSL-2691. Frozen head beb57f515ad68bc49b9026328258017b325b95f4; source base 9943d97241ad3faa4c83fd42feea5b899a96cdc1. PR filament-spec-language#3345, SPEC-only readiness. No applicable AssuranceProfile exists: no frontmatter type: AssuranceProfile or review_policy under spec/. Direct subset selected by dispatch; no enforced review run applies. No source edits, Cargo, checkout, commits, extra worktree or large measurement.

## Verdict

PASS for the frozen specification amendment only. No defects found in this method's changed scope. Implementation and AC-41 qualification remain unmet.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Scope examined

Every excerpt below was examined; repeated IDs distinguish contiguous parts of the same long unit.

```yaml
{
  "scope": [
    {
      "id": "FR-531",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "part": 1,
      "excerpt": "hold a number of entries linear in the body, and S4 SHALL charge\n`emit.output_bytes` for the canonical bytes it writes. For AC-41's two\ndeep-body families, the complete canonical artifact size and the charged\nnative strict-read and location-reconstruction work SHALL have linear upper\nbounds derived from the checked input's node and edge counts, source and\nidentifier byte lengths, and the fixed schema and body-walk structure. The\nrelative entry count is one `up` entry per row plus the number of `down`\nentries; it SHALL not exceed the BL-D1 and BL-D2 bounds of FSpec\nFR-322-AC-510. The"
    },
    {
      "id": "FR-531",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "part": 2,
      "excerpt": " caller SHALL\nstate finite byte, work and read-storage ceilings from those bounds before\nemission. The bound coefficients SHALL be independent of body depth and of\nthe term or binding count; neither the ceilings nor their coefficients are\nderived from the completed artifact or its measured consumption. Qualification SHALL record\neach artifact's actual canonical byte length, relative entry count and\ncharged native work against its declared bounds. Success at one size alone\nis not evidence of a linear bound.\n\nThose explicit ceilings do not change the published default. With the"
    },
    {
      "id": "FR-531",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "part": 3,
      "excerpt": " same\nchecked input and the other stage ceilings unchanged, an artifact that\nexceeds the default `emit.output_bytes` SHALL refuse `OutputLimit`, naming\nthat setting, its configured bound and the first unpaid byte, with no\npartial package. The source occurrence attached to that refusal does not\nidentify the wire member or source offset of the denied byte. Default-limit\nrefusal alone does not qualify AC-41's explicit-limit positive case. Every\n"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "part": 1,
      "excerpt": "For two separate units, one holding `function d(x: Integer): Integer pure { x + x + ... + x }` with 100,000 `+` applications (100,001 terms), the other holding 100,000 nested `let`s whose first binding is `x + 1`, each subsequent binding is the previous binder plus 1 and the result is the last binder, S4 writes complete packages and their `body_locations` on a 512 KiB stack under finite explicit caller ceilings chosen before emission from the checked input and the declared linear bounds above. The actual complete canonical byte lengths, relative entry counts and charged native"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "part": 2,
      "excerpt": " strict-read and lending-cursor work are recorded separately for the two units and do not exceed those bounds. The rows use FSpec FR-322's relative form (FSpec FR-322-AC-510's BL-D1 and BL-D2 rows exactly); the strict reader admits each package, and each typed path lent in body-walk order equals the checker's path without retaining all expanded paths. With the same checked units and other stage ceilings unchanged, setting only `emit.output_bytes` to the published default 16,777,216 instead refuses `OutputLimit` with setting `emit.output_bytes`, configured bound 16,777,216 and first"
    },
    {
      "id": "FR-531-AC-41",
      "path": "spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md",
      "role": "examined",
      "part": 3,
      "excerpt": " unpaid byte 16,777,217, and no partial package. A row written with a whole `path` is refused `malformed_wire` at its `location`."
    },
    {
      "id": "TC-3133-step-9",
      "path": "spec/test-cases/TC-3133-function-body-losses-are-located-alike-on-both-routes.md",
      "role": "examined",
      "part": 1,
      "excerpt": "Emit FR-531-AC-41's two separate programs (100,000 `+` applications and 100,000 genuinely incrementing nested `let`s) on a 512 KiB stack under finite explicit caller ceilings and linear bounds chosen from the checked input before emission, record each complete canonical byte length, relative entry count and charged native strict-read and lending-cursor work, compare the rows with FSpec's BL-D1 and BL-D2, admit both packages and compare each lent typed path with the checker's path without retaining all expanded paths; repeat emission with only `emit.output_bytes` changed to its"
    },
    {
      "id": "TC-3133-step-9",
      "path": "spec/test-cases/TC-3133-function-body-losses-are-located-alike-on-both-routes.md",
      "role": "examined",
      "part": 2,
      "excerpt": " published default, and admit one row rewritten with a whole `path`."
    },
    {
      "id": "TC-3133-expected-9",
      "path": "spec/test-cases/TC-3133-function-body-losses-are-located-alike-on-both-routes.md",
      "role": "examined",
      "part": 1,
      "excerpt": "both complete packages emit under the predeclared explicit ceilings, with independently recorded byte, relative-entry and native-work totals within their declared linear bounds, their rows equal BL-D1's and BL-D2's, both strict reads admit and the lent typed paths equal the checker's paths; each default-limit emission instead refuses `OutputLimit` with `emit.output_bytes` bound 16,777,216 and first unpaid byte 16,777,217, with no partial package, and the whole-`path` row is `malformed_wire` at its `location`."
    }
  ]
}
```

## Evidence and limits

Quoin 0.36.7; Quire 0.52.0 (engine 0.62.2); spec-artifacts-process@v0.29.2. Changed-file validation and whitespace checks pass. quoin matrix exits 1 with native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size; the explicitly approved quire matrix fallback succeeds. Its repository-wide baseline diagnostics remain in /private/tmp/qsl-2691-review-quire-matrix.stderr: no global assurance PASS is claimed. Matrix mints 44 FR-531 criteria; AC-41 is Test, untagged, zero binders. This is honest pending qualification, not delivery evidence. Public IR743 APIs read_with_budget/NativeJoinBudget::consumed and body_locations/next_location/work_consumed ground the native charges; S4 has no emitter work counter.

Rust/React/Python lanes and implementation gap analysis are inapplicable to this two-file SPEC-only amendment. Object/dependency analyses are skipped because no domain objects or relationship edges change. Risk-complexity has no new architecture to assess beyond the selected resource/failure lens. Criterion-strength/Jev is not selected; no calibrated semantic verdict claimed. No large test invocation was used; QSL-2691 must remain open for the corrected fixture implementation, derived-bound review and authorized sequential qualification.
