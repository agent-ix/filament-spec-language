---
id: SR-202611212158130340200000001
title: "WorkScope cohort pin code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2145; Cargo.toml, fsl-command/Cargo.toml, Cargo.lock"
review_set: subset
relationships:
  - { target: "ix://agent-ix/filament-spec-language", type: references }
---

## Summary

Ticket: QSL-2145. Independent initial combined code-review with rust-review lane at frozen HEAD 0516015174ef2cab09b2861102c047f550c97f94, base cf8d85826db560569cd8806cce07007d4b493e0c. The entire diff is seven Git dependency source rows across six repositories and their matching lock updates (14 insertions/14 deletions); no implementation, spec, test, workflow or gate changes.

## Verdict

**PASS** for the assigned dependency readiness chore. No introduced defect found. This makes the accepted WorkScope APIs available coherently; it does not implement observation ledger migration or cumulative probability and does not accept their client features. Reviewed accepted revisions agree with the dispatcher's merged cohort; upstream review/reachability is supplied by root, not independently re-reviewed here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Read AGENTS.md, CLAUDE.md, CONTRIBUTING.md, deny.toml, root lints, code-review, rust-review, rust-style and its testing reference. No applicable AssuranceProfile found: the only textual matches are prior review prose stating no profile applies. No profile was created. API, boundary, safety and concurrency changes are absent; no new Rust implementation requires further language-specific analysis.

All seven manifest revisions equal the seven lock source revisions and accepted cohort. Features, both FSpec qualification aliases f6b0480cd90f5d1b5c5ea64f694895bb7b5ce2e0, TL 06cda7776a3566ae90e223c3bc286c254618d39a, walk v0.1.1, core-data and unrelated sources remain unchanged. No copied implementation, new helper, vendoring, new source override, lowered gate, warning suppression or public JSON member/version change is introduced. Existing fsl-foundation patch is unchanged.

Inspected root-supplied locked workspace/all-feature reverse graph receipts /private/tmp/qsl-workscope-pin-exact-tree.txt and -sv-tree.txt: one package named filament-exact, one named filament-semantic-value at the assigned sources. The separately named dev-only fsv-failing-allocator remains at f5c913bd4fc8880b45e02e3918e02360c530320c; allocator-tree.txt has its edge under fsl-semantics dev-dependencies. This is not a second typed semantic-value package.

Read five current-head raw logs and .out receipts: package11 11, analyze17 17, eval3 3, command1 1, replay1 1; all rc=0, 33 distinct passes, zero failures/ignored. Exact invocations are /private/tmp/qsl-workscope-pin-qualification.txt. Current cycle selection is a_mutual_dependency_is_a_cycle_that_derives_no_slot. Reviewed temporal_owner_types_share_one_dependency_cohort: typed owner relationships plus real TL trace derive/read and byte/document assertions. These are focused compatibility receipts, not full runtime or consumer acceptance. Root supplied normal Cargo lock generation provenance; no Cargo invocation was repeated by this reviewer.

git diff --check passed independently. Reported cargo fmt baseline failure and unused sample_fingerprint warning are unchanged source; neither is introduced here. No Cargo/build/tests/clippy/full gate/deny command was run by this reviewer, per dispatch. Root must still perform its authorized final locked workspace/all-target check after review artifacts and fresh rebase, then check actual head/main and merge.

Plan completion: not assessed. Spec review skipped because no spec/plan changes. Runtime model id is unavailable. No source edits, commit, push, merge, new worktree, relay message or tracker action.

## Scope units

```yaml
[
  {
    "id": "filament-exact",
    "path": "Cargo.toml",
    "lines": "16",
    "role": "examined",
    "excerpt": "filament-exact = { git = \"https://github.com/agent-ix/filament-exact\", rev = \"a58474cc105a86f3307a11def3fc001e2459679f\", features = [\"json-object\"] }"
  },
  {
    "id": "filament-semantic-value",
    "path": "Cargo.toml",
    "lines": "17",
    "role": "examined",
    "excerpt": "filament-semantic-value = { git = \"https://github.com/agent-ix/filament-semantic-value\", rev = \"31d807ccb8ffd5a88beb0b1a570b80acb099f5e9\" }"
  },
  {
    "id": "filament-checked",
    "path": "Cargo.toml",
    "lines": "25",
    "role": "examined",
    "excerpt": "filament-checked = { git = \"https://github.com/agent-ix/filament-eval\", rev = \"79b8c42b60f43c936e8e50a7fc6519da030c978d\" }"
  },
  {
    "id": "filament-eval",
    "path": "Cargo.toml",
    "lines": "28",
    "role": "examined",
    "excerpt": "filament-eval = { git = \"https://github.com/agent-ix/filament-eval\", rev = \"79b8c42b60f43c936e8e50a7fc6519da030c978d\" }"
  },
  {
    "id": "filament-contract-model",
    "path": "Cargo.toml",
    "lines": "30",
    "role": "examined",
    "excerpt": "filament-contract-model = { git = \"https://github.com/agent-ix/filament-contract-ir\", rev = \"c46c29a5ccd8d4079b092e37662817bf8cd95c0b\" }"
  },
  {
    "id": "filament-observation",
    "path": "Cargo.toml",
    "lines": "32",
    "role": "examined",
    "excerpt": "filament-observation = { git = \"https://github.com/agent-ix/filament-observation\", rev = \"004c7c4f934d15022d2a80b98c451d436109b44b\" }"
  },
  {
    "id": "filament-mltl",
    "path": "fsl-command/Cargo.toml",
    "lines": "44",
    "role": "examined",
    "excerpt": "filament-mltl = { git = \"https://github.com/agent-ix/filament-mltl\", rev = \"b3881775908e0f17ac32e8a4d13f07e49b82edf9\" }"
  },
  {
    "id": "lock:filament-exact",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "a58474cc105a86f3307a11def3fc001e2459679f"
  },
  {
    "id": "lock:filament-semantic-value",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "31d807ccb8ffd5a88beb0b1a570b80acb099f5e9"
  },
  {
    "id": "lock:filament-checked",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "79b8c42b60f43c936e8e50a7fc6519da030c978d"
  },
  {
    "id": "lock:filament-eval",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "79b8c42b60f43c936e8e50a7fc6519da030c978d"
  },
  {
    "id": "lock:filament-contract-model",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "c46c29a5ccd8d4079b092e37662817bf8cd95c0b"
  },
  {
    "id": "lock:filament-observation",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "004c7c4f934d15022d2a80b98c451d436109b44b"
  },
  {
    "id": "lock:filament-mltl",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "b3881775908e0f17ac32e8a4d13f07e49b82edf9"
  },
  {
    "id": "FSpec-aliases",
    "path": "Cargo.toml",
    "role": "examined",
    "excerpt": "filament-specification = { package = \"filament-specification-qualification\", git = \"https://github.com/agent-ix/filament-specification\", rev = \"f6b0480cd90f5d1b5c5ea64f694895bb7b5ce2e0\" }"
  },
  {
    "id": "TL-pin",
    "path": "fsl-command/Cargo.toml",
    "role": "examined",
    "excerpt": "tl-mltl = { git = \"https://github.com/agent-ix/tl-mltl\", rev = \"06cda7776a3566ae90e223c3bc286c254618d39a\", features = [\"infinite-trace\"] }"
  },
  {
    "id": "walk-pin",
    "path": "Cargo.toml",
    "role": "examined",
    "excerpt": "filament-walk = { git = \"https://github.com/agent-ix/filament-walk\", tag = \"v0.1.1\" }"
  },
  {
    "id": "allocator-helper",
    "path": "Cargo.lock",
    "role": "examined",
    "excerpt": "source = \"git+https://github.com/agent-ix/filament-semantic-value?rev=f5c913bd4fc8880b45e02e3918e02360c530320c#f5c913bd4fc8880b45e02e3918e02360c530320c\""
  },
  {
    "id": "temporal-cohort-test",
    "path": "fsl-command/tests/it/temporal_owner_cohort.rs",
    "role": "context_only",
    "excerpt": "assert_eq!(retained.document(), &document);\n    assert_eq!(retained.bytes(), produced.bytes());"
  }
]
```

## Publication and custody

Private pending method comment is prepared outside the worktree; NOT POSTED, no label change, no comment id or read-back claimed. Root owns publication, actual byte/YAML read-back, and committing this latest artifact to reviews/qsl-2145-workscope-pin-code-review.md. Public artifact contains no private tracker links. No dispositions apply to the placeholder.

