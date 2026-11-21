---
id: SR-17952575110002
title: "QSL-2691 compact producer gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2691; frozen PR 3292; Cargo.lock, Cargo.toml, fsl-analyze/tests/it/monitor_root.rs, fsl-eval/tests/it/ieee_ordering_route_parity.rs, fsl-package/src/emit.rs, fsl-package/src/emit/body_locations.rs, fsl-package/src/emit/monitor_plan.rs, fsl-package/src/emit/protocol_locations.rs, fsl-package/src/emit/tests.rs, fsl-package/src/emit/tests/deep_bodies.rs, fsl-package/src/emit/tests/protocol_recursion.rs, fsl-package/src/reader_protocol_projection.rs, fsl-package/src/wire_form.rs, tests/it/dispatch_emission.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/filament-spec-language/FR-531, type: references }
---

## Summary

Independent gap-analysis of the first FSL compact producer candidate. One unmet 100k acceptance obligation remains; the compact encoding and focused controls pass.

## Verdict

FAIL against full QSL-2691 acceptance: FR-531-AC-41 remains unmet. This is not a failure of the focused compact-path controls. A partial-land scope decision must explicitly preserve that remaining work.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-531-AC-41 has no bound test; existing stress tests do not exercise its required 100k admission and reconstructed paths. The repository computed matrix reports FR-531-AC-41 untagged with binders []. The changed deep stress case asserts sum OutputLimit at 128 MiB; its nested lets bind the same x and contain no addition rows. The compact delta test uses 128 applications. No scoped witness covers 100,000 additions plus incrementing 100,000 nested lets, default-limit success, strict read and typed reconstruction of their checker paths. Add that behavior and trace its real assertions without raising the defaults, or explicitly defer the unmet criterion. | spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md:1052 |

## Coverage

Frozen head 0c9f5dc4880a926841dc4990745a82f7f5d0610d, fresh origin/main 32afa435df1f6d7ffb0b9df7cea58eea48c45928, 14 changed files. Real manifest/lock IR pin 20a4dfbf29e979da4326ac81f85e8266e464519f. No source edits, Cargo, new worktree, commit or push by reviewer. No applicable AssuranceProfile found.

Inspected retained authentic checker captures; canonical least-up path interning with retained chains; pre-sort body-walk deltas; record/tuple/shared reaches; persistent native paths; lazy lending cursor; checked finite structural allowance and overflow/exhaustion errors; three propagating projection callers; projected function closure; all four write_index_segments callers reusing StructuralIds::key_reader; no limit increase. No additional behavior/resource/identity defect identified in that scope. Allowance is graph-derived, not a new caller-owned execution budget; setup scans the package per projected function, so no package-global linear-time claim is made.

Inspected final handoff /private/tmp/qsl-2691-compact-producer-handoff.md and logs: pinned compact 9 PASS (only 6 are compact body producer/projection cases; the other 3 are existing identity tests); pinned fsl-package lib Clippy PASS; pinned full monitor PASS at 1,000,000 read work; operand slot 2 PASS; repeated reordered record identity PASS; exact output/one-short PASS; actual callee body and missing-row dispatch/IEEE tests PASS. Final-pin compact/monitor logs directly cover final source. Large tests ran on the prior local integration stack, whose emit.rs/body_locations.rs/deep_bodies.rs/protocol_recursion.rs have no diff against frozen HEAD (git diff 810c7071ed HEAD over these paths empty). Later closure-only projection change is exercised by the actual IEEE consumer tests.

100k producer Rust test PASS 226.08s on 512 KiB stack, but sum asserts 128 MiB output denial; else-if/constant lets emit. Restored 100k protocol-cycle Rust test PASS 107.08s, 0 ignored. Both external time wrappers exit 1 after test success due to denied sysctl kern.clockrate; instrumentation is incomplete, test outcomes remain PASS. Canonical writer sampling reports 7.7 GiB peak, so bounded output and compact paths do not establish small process memory. No aggregate gate was run or claimed. git diff --check passed; CI workflows unchanged.

No spec/plan changes: spec-review and optional spec subanalyses do not apply. Intent/code/test review within ordinary code-review was authorized by the dispatch; no separate optional gap semantic fanout was run. No plan supplied.

Reconciliation: quire matrix 0.52.0 (engine 0.62.2), repository scope, no run evidence read. Whole-repository totals: tagged 4781; untagged 1126; tagged-by-ignored-test 85; method-without-symbol 104; retired 13. These totals describe broader existing repository debt, not defects introduced by this 14-file PR; no repository-wide assurance PASS is asserted. Matrix/coverage diagnostics include existing symbol collisions, unmatched external FSpec tags, and missing NFR acceptance sections; they are retained in matrix.stderr and coverage.stderr and prevent a blanket complete-extraction claim. Scoped FR-531-AC-41 extraction is present and unambiguous. FR-531-AC-39 is tagged by the authentic dispatch test. FR-531-AC-40 also remains untagged in current repository data and was not implemented by this diff.

Reverse inventory covers the emitter carrier, structural index conversion, reader projection/public read access, monitor producer/readback and evaluation dispatch behavior. Changed helpers have FR-531/FR-264/FR-383/FR-251 ownership; no new public API, stub, copied dependency source, coverage suppression or weakened CI lane found. No coverage percentage was inferred from test pass counts.

Plan completion: not assessed

### Examined units

```yaml
scope:
  - id: Cargo.lock
    path: Cargo.lock
    role: examined
    excerpt: "# This file is automatically @generated by Cargo."
  - id: Cargo.toml
    path: Cargo.toml
    role: examined
    excerpt: "[workspace]"
  - id: fsl-analyze/tests/it/monitor_root.rs
    path: fsl-analyze/tests/it/monitor_root.rs
    role: examined
    excerpt: use filament_contract_model::{
  - id: fsl-eval/tests/it/ieee_ordering_route_parity.rs
    path: fsl-eval/tests/it/ieee_ordering_route_parity.rs
    role: examined
    excerpt: use filament_eval::CallFailure;
  - id: fsl-package/src/emit.rs
    path: fsl-package/src/emit.rs
    role: examined
    excerpt: use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
  - id: fsl-package/src/emit/body_locations.rs
    path: fsl-package/src/emit/body_locations.rs
    role: examined
    excerpt: use std::collections::{BTreeMap, BTreeSet, HashMap};
  - id: fsl-package/src/emit/monitor_plan.rs
    path: fsl-package/src/emit/monitor_plan.rs
    role: examined
    excerpt: use std::collections::{BTreeMap, BTreeSet};
  - id: fsl-package/src/emit/protocol_locations.rs
    path: fsl-package/src/emit/protocol_locations.rs
    role: examined
    excerpt: use std::collections::{BTreeMap, HashMap, HashSet};
  - id: fsl-package/src/emit/tests.rs
    path: fsl-package/src/emit/tests.rs
    role: examined
    excerpt: use filament_semantic_value::checking::CheckMode;
  - id: fsl-package/src/emit/tests/deep_bodies.rs
    path: fsl-package/src/emit/tests/deep_bodies.rs
    role: examined
    excerpt: use super::*;
  - id: fsl-package/src/emit/tests/protocol_recursion.rs
    path: fsl-package/src/emit/tests/protocol_recursion.rs
    role: examined
    excerpt: use super::admission_corpus::config_version_unit;
  - id: fsl-package/src/reader_protocol_projection.rs
    path: fsl-package/src/reader_protocol_projection.rs
    role: examined
    excerpt: use std::collections::{BTreeMap, BTreeSet};
  - id: fsl-package/src/wire_form.rs
    path: fsl-package/src/wire_form.rs
    role: examined
    excerpt: '#[cfg(feature = "std")]'
  - id: tests/it/dispatch_emission.rs
    path: tests/it/dispatch_emission.rs
    role: examined
    excerpt: use filament_contract_model::{
  - id: FR-531-AC-41
    path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
    role: examined
    excerpt: "S4 writes the `body_locations` rows of `function d(x: Integer):
      Integer pure { x + x + ... + x }` with 100,000 `+` applications, and of a
      body of 100,000 nested `let`s each binding the previous binder plus 1, in
      FSpec FR-322's relative form (FSpec FR-322-AC-510's BL-D1 and BL-D2 rows
      exactly), on a small stack and within the default `emit.output_bytes`
      limit, with no `OutputLimit` refusal; the strict reader admits each
      package, and the bare route's typed read of each row returns the checker's
      path. A row written with a whole `path` is refused `malformed_wire` at its
      `location`."
  - id: FR-531-AC-39
    path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
    role: examined
    excerpt: "Over the `Config` domain fixture of `tests/it/dispatch_emission.rs`,
      the unit holding its three `size` bodies, `pre rootCall using v on
      Config::ConfigVersion::probe { self.size() > 0 }` (`rootCall`'s call of
      `size` is what makes `sqrtPre` a clause function: the checker builds a
      clause function only for an operation a dispatch candidate reaches, and no
      candidate reaches `probe`) and `pre sqrtPre using v on
      Config::ConfigVersion::size { filament::value::ieee::sqrt(float64(bits:
      0x4000000000000000)) <= filament::value::ieee::sqrt(float64(bits:
      0x4000000000000000)) }` emits, for the precondit"
  - id: FR-251-AC-9
    path: spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md
    role: examined
    excerpt: The admitted plan of AC-1 crosses E5 as one `temporal`/`monitor_plan`
      plan-root node (FSpec FR-487, FR-322-AC-308) whose dependencies include
      the entries `init`, `on_event`, `on_watermark` and `on_gap` (member 15 of
      FSpec FR-487; FR-487-AC-7) as `function` nodes with the signatures of the
      Transitions row, and the `State` type; no entry uses a statement, loop or
      assignment form. Changing any one entry body changes the plan-root's
      content key and so the plan identity. The `init` result is every slot's
      initial value, and an obligation's origin in `State` is the unwrapped tick
      of its activati
  - id: FR-383-AC-25
    path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
    role: examined
    excerpt: "SCC discovery is iterative and O(V + E), does not merge groups
      quadratically, computes each completed group's identity once, and uses no
      global lock on the emit path. There is no fixed recursion depth, group
      size, node count or group count cap: the only applicable work bound is the
      caller's shared `s3.work_units` budget, and exhaustion produces the
      existing typed limit refusal naming `s3.work_units`. A 100,000-node cycle
      group emits and reads back within the supplied budget without native stack
      overflow; the budget refuses at zero and one unit short and succeeds at
      the exact measured work"
  - id: FR-264-AC-6
    path: spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md
    role: examined
    excerpt: A package whose canonical emitted bytes number `N` is emitted at
      `emit.output_bytes` `N`, with bytes equal to the bytes emitted under a
      larger bound. At bound `N - 1` it returns `EmitRefusal::OutputLimit` with
      code `stage_limit_exceeded`, cause `input-bytes-exceeded`, kind input
      bytes, bound `N - 1`, an actual count above `N - 1` and setting
      `emit.output_bytes`, and writes no bytes. At bound 0 it returns the same
      refusal with bound 0.
```

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Accepted only for partial PR3292 at unchanged head 0c9f5dc4880a926841dc4990745a82f7f5d0610d under the explicitly dispatched arch ruling, durably recorded in QSL-2691 comment 62a22bf3-dc5c-4cad-9dca-741ca4e6b8e4. FR-531-AC-41 remains real, unchanged and unmet; QSL-2691 stays OPEN for default-limit emission, strict read and typed checker-path reconstruction at 100k. The true default is 16,777,216 bytes (FR-255-AC-20); 134,217,728 bytes is this fixture's explicit limit, eight times default, and first unpaid 134,217,729 proves denial only. No limit raise, compatibility fallback or acceptance weakening is authorized. Compact representation, exact output denial, real monitor and restored cycle controls may land independently; whole-wire output dominators remain to be measured before the corrective follow-up. No other scoped actionable finding was identified. |

## Disposition verdict

CLEAR for the explicitly authorized partial PR3292 scope only. Original review findings and FAIL against full QSL-2691 acceptance remain unchanged. This disposition does not claim FR-531-AC-41 passes or authorize closing QSL-2691. No source changed; no further Cargo or review round was run.
