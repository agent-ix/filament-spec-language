---
id: SR-202611210022442313144108556
title: "QSL-2753 IEEE repository gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; repository spec, source and tests; QSL-2753 changed IEEE collector behavior"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---

## Summary

Repository-driven, planless matrix audit plus reverse-gap and stub inventory. Global assurance fails on existing missing and ignored-only bindings; no new production behavior without an owner or hollow IEEE regression was found.

## Verdict

FAIL for global repository assurance. The two representative baseline findings below retain the failure; the full criterion inventory is the accompanying computed matrix, not a claim of only two missing criteria. Production behavior review is sound; SR-202611210022442313144108555 requires one narrow local trace normalization before scoped clearance. No baseline waiver or disposition is invented; parent may explicitly accept no change for this bounded fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Baseline FR-001-AC-7 has no bound test; the repository matrix has 1164 untagged criteria. | FR-001-AC-7; spec/functional/FR-001-read-exact-source.md |
| FND-002 | high | Baseline FR-056-AC-60 is bound only to ignored tests; the repository matrix has 74 ignored-only criteria. | FR-056-AC-60; spec/functional/FR-056-admit-domain-package-model-declarations.md |
| FND-003 | medium | The new package regression has no computed local FR-531-AC-39 binding: normalize its qualified trace attribute to the extracting trace form. | fsl-package/src/emit/body_locations.rs:336; FR-531-AC-39 |

## Local finding

The new test's qualified macro tag does not produce a local matrix binding. Root decision: normalize only this new attribute using the same imported trace macro, preserving all assertions and the FSpec IDs. Existing sibling control extraction remains baseline. No full IEEE rerun is requested for this trace-only repair.

## Coverage

Plan completion: not assessed

Reconciliation: quoin 0.36.7 matrix --repo /Users/peter/dev/qsl-2753-code --json exited 1 with `native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size`. Recorded once; no retry or older binary. Parent-authorized static fallback: quire 0.52.0 (engine 0.62.2) matrix --scope /Users/peter/dev/qsl-2753-code --format json exited 0. Coverage --scope /Users/peter/dev/qsl-2753-code --json exited 0. Raw files remain alongside this artifact.

Computed matrix: 6254 criteria: tagged 4895; untagged 1164; tagged-by-ignored-test 74; method-without-symbol 106; retired 15. No run evidence read by the static fallback. Coverage census: 13780 Rust candidates, 13207 tagged, 12666 bound; untracked_symbols 5886, unmatched_tags 6727. Diagnostics: trace-id-outside-marker 22360; untracked-id-has-minted-children 482; section-matches-nothing 6; archetype-matches-nothing 1; uncatalogued-verification-method 22; catch-all-universal 1. These are distinct engine outputs, not all missing tests.

External FSpec tags on the public tests are expected to be untracked by this single-repository specification scope; local FSL FR-148 is an unrelated protocol requirement and must not replace them. The existing fully qualified ix_trace_rs::trace idiom on body_locations tests, including the new test's local FR-531-AC-39 tag, appears under unmatched_tags rather than this criterion's matrix binders. No clean engine-backed binding is claimed for it; actual test execution is separately evidenced. This extraction mismatch also affects unchanged sibling controls and remains an explicit limitation. The declared module's engine did extract 12666 bound symbols, so no grep fallback was used.

Reverse inventory: compiler intake/check/package, public command compile/run/analyze/monitor/replay/output, package bytes-only read/evaluation and captured body/protocol locations. Existing owners include FR-278/FR-279/FR-312/FR-285, FR-531, FR-383 and the FSpec IEEE and checked-package contracts. No new public endpoint, command, configuration field or operation is introduced. Changed meaningful behaviors inventoried: 3 (parent-directed selection, public Boolean regression checks, native shared-location admission regression), all owned; new untraced behavior 0, changed source stubs 0, changed test stubs 0. Whole-tree todo/unimplemented scan mostly identifies compile-fail string fixtures or docs, with a tracked QSL-1978 todo in fsl-analyze/tests/it/hybrid_requests.rs outside the diff; no claim of complete global stub elimination is made. A global reverse audit is necessarily incomplete at this scale, so FAIL remains; this is not a repository PASS.

Semantic review: optional gap-analysis semantic fanout skipped; local code-test/spec faithfulness was assessed in code-review. No plan was supplied.

Frozen head: `716f0d44a3d23dfc58e7ec4347ff29fd675f03f5`; base: `5deeba7d06` (#3376). Ticket: QSL-2753; PR: filament-spec-language#3379. Exact diff: three IEEE files, 160 insertions / 30 deletions; no dependency, lock, workflow, or compile-repair edits. Runtime model ID: unknown (not queried or fabricated).

Methods: code-review with rust-review folded into one artifact; gap-analysis separately. No spec/plan change, so spec-review and its subanalyses are not applicable. Tokio is unused in the changed collector. No applicable AssuranceProfile was found. Optional gap semantic fanout was not run; no subagents were permitted. Code-review still checked local intent, tests and implementation.

## Qualification

Inspected retained Titan logs under `/Users/peter/dev/qualification-logs/qsl-2753/`, attributed to this exact frozen head by the dispatch and PR body. `qsl2753-final-run1`: 5 passed, 0 failed/ignored, 1146 filtered. Four exact corpus programs assert Boolean true, exit 0 and empty stderr; the fifth test executes 8 same-bound ordering cases, 8 distinct-name controls and 2 equality controls with their actual expected Booleans. These are 5 tests, not 22 tests. `qsl2753-final-package1`: 7 passed, 0 failed/ignored, 1068 filtered; actual native IR admission, seven lowered terms and three shared-equality occurrence/ordinal pairs, path witnesses, composite/tuple/deep-128 and protocol controls. `qsl2753-clippy-package1`: exit 0. `qsl2753-clippy-run1`: exit 101 in eight unchanged fsl-replay lint failures before the root test binary; root Clippy remains UNQUALIFIED. `git diff 5deeba7d06..HEAD -- fsl-replay .github Cargo.toml Cargo.lock` is empty; this establishes unchanged source, not a globally green baseline.

`qsl2753-red3` at d4dbcbfd3a302113cca0e88a3c249a0b0d8c93b4 compiled and executed four regressions: 0 passed, 4 failed, exit-30 emit/runtime_invariant with missing complete emitted location context (decoded JSON also inspected). Red1/red2 compilation failures are not semantic red evidence. Direct rustfmt --check on the three files and git diff --check passed in this review. No Cargo, full gate, workspace, all-targets, or new runtime build was run. Later main fixture failures and their urgent repair are outside this qualification.

## Scope

Excerpts below are bounded verbatim requirement excerpts where IDs name requirements. Examination of an AC means the changed behavior was checked against it, not that this PR newly qualifies every vector in that AC. The new body regression extends the shared-occurrence carrier contract; it is not the clause fixture of FR-531-AC-39 or the 100,000-node qualification.

```yaml
scope:
- id: FSpec-FR-148-AC-1
  path: "/Users/peter/dev/filament-specification/spec/functional/expressions/FR-148-evaluate-ieee-floating-profiles.md"
  role: examined
  excerpt: NaN, infinity and signed-zero vectors produce the selected equality and
    ordering results.
- id: FSpec-FR-148-AC-12
  path: "/Users/peter/dev/filament-specification/spec/functional/expressions/FR-148-evaluate-ieee-floating-profiles.md"
  role: examined
  excerpt: 'With `p0` `0x0000000000000000`, `n0` `0x8000000000000000`, `one` `0x3ff0000000000000`,
    `pinf` `0x7ff0000000000000`, `qnan` `0x7ff8000000000000` and `snan` `0x7ff0000000000001`:
    `n0 = p0`, `n0 <= p0`, `p0 >= n0`, `one < pinf`, `pinf <= pinf`, `qnan != qnan`
    and `qnan != one` are true; `n0 < p0`, `p0 > n0`, `qnan = qnan`, `qnan < one`,
    `qnan <= one`, `one > qnan`, `one >= qnan`, `snan < one` and `snan = snan` are
    false, with no flag; the binary32 patterns `0x00000000`, `0x80000000`, `0x3f800000`,
    `0x7f800000`, `0x7fc00000` and `0x7f800001` give the same results.'
- id: FSpec-FR-322-AC-490
  path: filament-specification@bca4311db3a4d9d15c6e4e8f9878ce3a3b9a60b1/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: examined
  excerpt: '`checked-term-data-vectors.json` carries `body_locations` vectors over
    the package of `function f(p: Float64[nearest-even], q: Float64[nearest-even],
    r: Float64[nearest-even]): Boolean pure { sqrt(p) <= sqrt(q) }` (`r` is declared
    and never read, so the two `#ieee_operand` `let`s bind at levels 3 and 4). BL-01,
    the positive package, holds one row per term of `f`''s body walk, each with origin
    `{kind: "function_body", function: <f''s NodeRef>, name: "f", index: 0}`, the
    root composition at path `[]`, the first `sqrt` at `[0]` and the second at `[1]`
    (not `[1, 0]`, its position in the lowered body'
- id: FSpec-FR-322-AC-510
  path: filament-specification@bca4311db3a4d9d15c6e4e8f9878ce3a3b9a60b1/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: examined
  excerpt: 'Body locations are relative and linear. In `checked-term-data-vectors.json`,
    BL-03''s rows for the `+` node''s occurrences 0 and 1 are `{up: 0, down: [0]}`
    and `{up: 1, down: [1]}` and its root `and` row `{up: 1, down: []}`; BL-R9, BL-01
    with the second `sqrt` row''s `up` set to `2` (the previous path `[0]` has one
    entry), refuses `invalid_package`/`invalid-value` at `/checked_term_data/body_locations/8/location/up`;
    BL-R10, BL-02 with that row written `{up: 1, down: [0]}`, which rebuilds the same
    path `[0]` with a larger `up` than its least form, refuses the same at the same
    member; BL-S5, a row'
- id: FSpec-FR-322-AC-513
  path: filament-specification@bca4311db3a4d9d15c6e4e8f9878ce3a3b9a60b1/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: examined
  excerpt: 'Composite values preserve the body walk and compact canonicality. In `checked-term-data-vectors.json`,
    BL-04 references BL-01''s body in both stored record fields `z`, then `a`; BL-05
    references that body in both tuple positions 0, then 1. Both are admitted with
    eighteen rows: each application''s first component visit pairs with occurrence
    0, `term_ordinal` 0, and its second with occurrence 0, `term_ordinal` 1. In component
    `c` (0 or 1), the two `sqrt` paths are `[c, 0]` and `[c, 1]`, followed by composition
    paths `[c]`; the composite values have no rows. The first `sqrt` of component
    1 follows '
- id: FR-531-AC-39
  path: spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md
  role: examined
  excerpt: 'Over the `Config` domain fixture of `tests/it/dispatch_emission.rs`, the
    unit holding its three `size` bodies, `pre rootCall using v on Config::ConfigVersion::probe
    { self.size() > 0 }` (`rootCall`''s call of `size` is what makes `sqrtPre` a clause
    function: the checker builds a clause function only for an operation a dispatch
    candidate reaches, and no candidate reaches `probe`) and `pre sqrtPre using v
    on Config::ConfigVersion::size { filament::value::ieee::sqrt(float64(bits: 0x4000000000000000))
    <= filament::value::ieee::sqrt(float64(bits: 0x4000000000000000)) }` emits, for
    the precondition''s'
- id: fsl-package/src/emit/protocol_locations.rs
  path: fsl-package/src/emit/protocol_locations.rs
  role: examined
  excerpt: Select the parent's next direct operand before transitive reach pruning;
    count repeated identity visits.
- id: fsl-package/src/emit/body_locations.rs
  path: fsl-package/src/emit/body_locations.rs
  role: examined
  excerpt: Inspect all changed assertions and their actual production execution seams.
- id: tests/it/spine_run.rs
  path: tests/it/spine_run.rs
  role: examined
  excerpt: Inspect all changed assertions and their actual production execution seams.
```


## Baseline finding scope

```yaml
scope:
- id: FR-001-AC-7
  path: spec/functional/FR-001-read-exact-source.md
  role: examined
  excerpt: 'Package declarations holding one record `Point` with field `x: Int[0,
    9]`, checked under the source reference of bytes `b` admitted as authority `a`,
    identity `u`, under the reference of bytes `b''` admitted as `a`, `u`, under the
    reference of `b` admitted as authority `c`, identity `u`, and under the reference
    of `b` admitted as authority `a`, identity `v`, give `Point` the same node key
    `{package, path: ["Point"], ordinal: 0}` each time: the source reference is no
    part of a node key.'
- id: FR-056-AC-60
  path: spec/functional/FR-056-admit-domain-package-model-declarations.md
  role: examined
  excerpt: Over a model whose `Gauge` has a Float64 field `a` and an operation `flip()`
    with frame `modifies a` and `post F { self.a = -pre(self.a) }`, the explicit-state
    checker returns no `RequiresBound`. From initial `a` `3ff0000000000000` (1.0)
    it reaches exactly the states 1.0 and `bff0000000000000` (-1.0), with one candidate
    per application. From initial `a` `0000000000000000` (+0) it reaches exactly +0
    and `8000000000000000` (-0), two states with distinct keys, and each application
    has two candidates, because both zeros are `numericEqual` to the solved value
    (FR-120-AC-34). A simulation run applyi
```

## Publication

Local only under Peter's Linear hold and parent-only central batch. Findings are not posted, label state is unknown, read-back is pending. Parent owns explicit baseline dispositions and publication.

## Dispositions

Round 1 at frozen head `ee63828c951444597cd30ecdf6ef02a6a8ea19b2`. The original findings above are preserved unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted-no-change | Root explicitly accepts no change for 1164 baseline untagged criteria, represented by FR-001-AC-7, for this bounded IEEE fix; global assurance remains FAIL. |
| FND-002 | accepted-no-change | Root explicitly accepts no change for 74 baseline ignored-only criteria, represented by FR-056-AC-60, for this bounded IEEE fix; global assurance remains FAIL. |
| FND-003 | fixed | New trace attribute normalized; actual local binding and focused passing execution verified. |

## Disposition evidence

The only source changes since the original review import the existing ix_trace_rs::trace macro and change only the new test attribute to #[trace]. All three criterion IDs and the entire test body/assertions remain intact. Existing qualified attributes remain unchanged. Production protocol_locations.rs and public spine_run.rs are byte-identical to the reviewed 716f0d44a3d23dfc58e7ec4347ff29fd675f03f5 head. The two initial committed SRs were checked byte-identical to the original scratch artifacts before appending these dispositions.

In /Users/peter/dev/qualification-logs/qsl-2753/, qsl2753-fix1-trace.json resolves the exact new test, with all three structural claims.verifies IDs; citations are not treated as claims. qsl2753-fix1-coverage.json's actual coverage_matrix gives FR-531-AC-39 status tagged and the exact new body_locations.rs test binder at line 339. The convenience qsl2753-fix1-binding.json has an empty criterion array and was not used as proof. Retained stderr is not empty: trace: 29 symbol-id-collision warnings in existing unrelated fixtures/cfg alternatives; coverage: 29 symbol-id-collision warnings in existing unrelated fixtures/cfg alternatives. None concerns the new test or defeats its resolved claim/binder. The two FSpec near-miss citations remain external-scope extraction limitations. This establishes supplemental carrier coverage, not complete qualification of AC-39's clause-specific fixture.

At this exact head, retained qsl2753-fix1-package.log reports 1 passed, 0 failed, 0 ignored, 1074 filtered for the new package test. Dispatch attributes this Titan run to the frozen head, with -j16 --locked --test-threads=4 and exit 0. The original 5 public and 7 package passing tests remain prior-head evidence, with production collector and public regression code unchanged; they are not relabeled as new runs. Root explicitly accepts the eight unchanged fsl-replay root-Clippy errors for this scoped delivery only; root lint remains unqualified.

No Cargo, gate, repeated public suite or Clippy was run by this reviewer. Direct rustfmt --check on the changed file and git diff --check passed. No source edit, rebase or dependency change was made.

## Latest disposition verdict

Global assurance remains FAIL. MERGEABLE for this bounded IEEE fix: baseline FND-001 and FND-002 accepted-no-change by root, local FND-003 fixed. No finding remains still-open; no new findings in this disposition round. Runtime model ID remains unknown. Publication, tracker dedup, label operations and read-back remain pending root central batch; reviewer made no tracker access.
