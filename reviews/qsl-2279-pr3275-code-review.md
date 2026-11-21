---
id: SR-202611210022442313144108500
title: "code-review of QSL-2279 catalog-free division run API"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-replay/src/spine.rs, fsl-replay/src/spine/call.rs, fsl-replay/src/spine/call/tests.rs; FR-767, FR-100, FR-670"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-767
    type: references
---

## Summary

Ticket: QSL-2279. Reviewed frozen PR #3275 at 57a87538bd9df90eda19c4487dc4fdd11cc53ad1 against base 5f968c70b30a74429edc12e0ab0b8ebb82b1946a. The three-file API slice forwards an optional admitted division law through the existing catalog-free parse/check/run path; legacy run forwards None. The exact limit and cancellation inputs are preserved, with no second evaluator, new state, unsafe operation, dependency, wire format or gate change. Three real tests exercise all three laws on positive/negative divisor cases, no-default/header-only behavior and typed header conflicts. The production implementation is correct by inspection; three unrelated trace bindings need removal.

## Verdict

MERGEABLE after disposition round 1, subject to green delegated focused tests. All three false criterion bindings are removed; production and test bodies are unchanged. This is a PR-scoped review, not a repository-wide assurance verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The law-loop test claims FR-100-AC-1, but it calls q/r directly and checks quotient/remainder values; it never exercises the criterion's seven request, output envelope, source labels, authored path or function field. Remove the unrelated FR-100-AC-1 tag; keep FR-767-AC-1. | fsl-replay/src/spine/call/tests.rs:113 |
| FND-002 | low | The no-selection test also claims FR-100-AC-1, but it runs q(-7, 3) through library APIs and checks missing-selection or -3; it never asserts the criterion's seven request/output contract. Remove the unrelated FR-100-AC-1 tag; keep FR-767-AC-4. | fsl-replay/src/spine/call/tests.rs:148 |
| FND-003 | low | The no-selection test claims FR-670-AC-5, whose cases all supply a caller law. This test passes None throughout, so it cannot verify caller/header agreement, caller/header conflict, or a headerless supplied truncating law. Remove that tag here; the adjacent agreement/conflict test correctly retains it. | fsl-replay/src/spine/call/tests.rs:148 |

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix (quire 0.52.0, engine 0.62.2) — no run evidence read. The whole-repository static matrix completed exit 0. All five examined criteria are tagged; the new binders were inspected individually. The source-only coverage report also completed exit 0 and reports no untracked symbol from the three newly added tests. Existing older tests in that same file do have untracked TC tags; the original review wording incorrectly generalized the new-test result to the whole file and is corrected here. Existing repository-wide untagged/ignored rows, unresolved links and symbol-id-collision diagnostics outside this diff were not converted into new defects of this PR; no repository-wide PASS is claimed. Raw matrix, coverage and stderr are retained beside this artifact. The emitted Rust binding census reports candidates 13192, tagged 12631, bound 12153.

The evidence-backed command `/home/peter/.npm-global/bin/quoin matrix --repo /home/peter/dev/worktrees/qsl-2279-catalog-free-api --json` (quoin 0.36.8) produced no result and was interrupted, exit 130, empty stderr; the documented static fallback was then used, with no binary-version fallback. `/home/peter/.npm-global/bin/quire` reports 0.52.0 / engine 0.62.2. The resolved versions differ from the older release notice; this is an existing recorded environment discrepancy.

No Cargo, build, test or aggregate gate was run by this reviewer, per the focused delegated TEST-RUN constraint. Runtime evidence is pending with draco:QSL:code. Semantic requirement/test/code inspection was requested explicitly in the review brief. No applicable AssuranceProfile was found. No explicit plan was supplied.

Changed behavior inventory: two public run entry points (new optional-law wrapper and legacy delegation), zero unowned behaviors; no source stubs, test stubs or mocked production seams in the diff. The reexport is intentional public API glue. The requirements already own law selection, conflict and no-default semantics; adding this entry point does not introduce new semantics.

## Findings data

```yaml
clean: false
scope:
- id: FR-100-AC-1
  path: spec/functional/FR-100-run-a-named-function-through-the-spine.md
  role: examined
  excerpt: 'A native-run/1 request selecting `tests/fixtures/spine-compile.native`
    (`edition "1-draft"`) with `call` `{"function": "seven", "arguments": []}` has
    exit code 0 and an outcome carrying no refusal envelope and one result document
    whose `format` is `spine-run-result/1`, whose `source` names the request''s two
    labels and authored path, whose `function` is `seven`, and whose `outcome` is
    `{"kind": "completed", "value": {"integer": "7"}}`.'
- id: FR-670-AC-5
  path: spec/functional/FR-670-select-the-integer-division-law-from-a-unit-header-profile.md
  role: examined
  excerpt: The AC-1 unit with a floor law header compiled through the public compile
    facade with a caller-supplied floor selection checks with the floor law; with
    a caller-supplied truncating selection it refuses `invalid_package`/`conflicting-definition`
    at the header's identity literal before checking; with no header and a caller-supplied
    truncating selection it checks with the truncating law.
- id: FR-767-AC-1
  path: spec/functional/FR-767-supply-the-integer-division-law-from-a-command-option-or-request-member.md
  role: examined
  excerpt: 'For each law `L` of `euclidean`, `floor` and `truncating`, a `native-compile/1`
    request with `"integer_division": "L"` over FR-670-AC-1''s unit without its law
    header compiles with exit 0 and a package whose selection holds `L`''s integer-division
    role; a `native-run/1` request with the same member calling `q` and `r` at `(-7,
    3)` returns `-2` and `-1` under truncating and `-3` and `2` under floor and Euclidean.'
- id: FR-767-AC-2
  path: spec/functional/FR-767-supply-the-integer-division-law-from-a-command-option-or-request-member.md
  role: examined
  excerpt: 'The AC-1 unit with a floor law header and `"integer_division": "floor"`
    compiles with the floor law; with `"integer_division": "truncating"` it refuses
    `invalid_package`/`conflicting-definition` at the header''s identity literal,
    exit 20, with no package bytes and a message naming `floor`, the alias, `truncating`
    and `integer_division`.'
- id: FR-767-AC-4
  path: spec/functional/FR-767-supply-the-integer-division-law-from-a-command-option-or-request-member.md
  role: examined
  excerpt: The AC-1 unit with no member and no header refuses `missing_declaration`/`missing-selection`
    at each `div` and `rem`, exit 20; a unit using only `mod` compiles with no member.
findings:
- fnd: FND-001
  severity: low
  confidence: high
  method: code-review
  check: trace
  artifact_id: FR-100-AC-1
  related_ids:
  - FR-767-AC-1
  path: fsl-replay/src/spine/call/tests.rs
  lines: '113'
  excerpt: 'A native-run/1 request selecting `tests/fixtures/spine-compile.native`
    (`edition "1-draft"`) with `call` `{"function": "seven", "arguments": []}` has
    exit code 0 and an outcome carrying no refusal envelope and one result document
    whose `format` is `spine-run-result/1`, whose `source` names the request''s two
    labels and authored path, whose `function` is `seven`, and whose `outcome` is
    `{"kind": "completed", "value": {"integer": "7"}}`.'
  finding: The law-loop test claims FR-100-AC-1, but it calls q/r directly and checks
    quotient/remainder values; it never exercises the criterion's seven request, output
    envelope, source labels, authored path or function field. Remove the unrelated
    FR-100-AC-1 tag; keep FR-767-AC-1.
  test_id: catalog_free_integer_division_runs_each_law_without_an_extension_catalog
  claimed_ac: FR-100-AC-1
  correct_ac: FR-767-AC-1
- fnd: FND-002
  severity: low
  confidence: high
  method: code-review
  check: trace
  artifact_id: FR-100-AC-1
  related_ids:
  - FR-767-AC-4
  path: fsl-replay/src/spine/call/tests.rs
  lines: '148'
  excerpt: 'A native-run/1 request selecting `tests/fixtures/spine-compile.native`
    (`edition "1-draft"`) with `call` `{"function": "seven", "arguments": []}` has
    exit code 0 and an outcome carrying no refusal envelope and one result document
    whose `format` is `spine-run-result/1`, whose `source` names the request''s two
    labels and authored path, whose `function` is `seven`, and whose `outcome` is
    `{"kind": "completed", "value": {"integer": "7"}}`.'
  finding: The no-selection test also claims FR-100-AC-1, but it runs q(-7, 3) through
    library APIs and checks missing-selection or -3; it never asserts the criterion's
    seven request/output contract. Remove the unrelated FR-100-AC-1 tag; keep FR-767-AC-4.
  test_id: catalog_free_integer_division_without_a_selection_uses_only_the_header_law
  claimed_ac: FR-100-AC-1
  correct_ac: FR-767-AC-4
- fnd: FND-003
  severity: low
  confidence: high
  method: code-review
  check: trace
  artifact_id: FR-670-AC-5
  related_ids:
  - FR-767-AC-4
  path: fsl-replay/src/spine/call/tests.rs
  lines: '148'
  excerpt: The AC-1 unit with a floor law header compiled through the public compile
    facade with a caller-supplied floor selection checks with the floor law; with
    a caller-supplied truncating selection it refuses `invalid_package`/`conflicting-definition`
    at the header's identity literal before checking; with no header and a caller-supplied
    truncating selection it checks with the truncating law.
  finding: The no-selection test claims FR-670-AC-5, whose cases all supply a caller
    law. This test passes None throughout, so it cannot verify caller/header agreement,
    caller/header conflict, or a headerless supplied truncating law. Remove that tag
    here; the adjacent agreement/conflict test correctly retains it.
  test_id: catalog_free_integer_division_without_a_selection_uses_only_the_header_law
  claimed_ac: FR-670-AC-5
  correct_ac: FR-767-AC-4
bindings:
- test_id: catalog_free_integer_division_runs_each_law_without_an_extension_catalog
  ac_id: FR-767-AC-1
  trace: correct
- test_id: catalog_free_integer_division_runs_each_law_without_an_extension_catalog
  ac_id: FR-100-AC-1
  trace: wrong
- test_id: catalog_free_integer_division_without_a_selection_uses_only_the_header_law
  ac_id: FR-767-AC-4
  trace: correct
- test_id: catalog_free_integer_division_without_a_selection_uses_only_the_header_law
  ac_id: FR-670-AC-5
  trace: wrong
- test_id: catalog_free_integer_division_without_a_selection_uses_only_the_header_law
  ac_id: FR-100-AC-1
  trace: wrong
- test_id: catalog_free_integer_division_preserves_header_agreement_and_conflict
  ac_id: FR-767-AC-2
  trace: correct
- test_id: catalog_free_integer_division_preserves_header_agreement_and_conflict
  ac_id: FR-670-AC-5
  trace: correct
```

## Dispositions

Round 1 reviewed frozen head `63f45a37abf9f7f838ca461ba40037315dde38b3`. The only test changes since the reviewed head remove the three incorrect tags. Original findings are preserved above.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The each-law test retains FR-767-AC-1 and removes FR-100-AC-1. |
| FND-002 | fixed | The no-selection test retains FR-767-AC-4 and removes FR-100-AC-1. |
| FND-003 | fixed | The no-selection test removes FR-670-AC-5; the actual agreement/conflict test retains it. |

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: '#[trace("FR-767-AC-1")]

    fn catalog_free_integer_division_runs_each_law_without_an_extension_catalog()
    {'
- fnd: FND-002
  outcome: fixed
  after_excerpt: '#[trace("FR-767-AC-4")]

    fn catalog_free_integer_division_without_a_selection_uses_only_the_header_law()
    {'
- fnd: FND-003
  outcome: fixed
  after_excerpt: '#[trace("FR-767-AC-4")]

    fn catalog_free_integer_division_without_a_selection_uses_only_the_header_law()
    {'
```

## Tool limitation reproduction

Machine: terra, Codex sandbox. Resolved executable `/home/peter/.npm-global/bin/quoin`, actual version 0.36.8. Command: `quoin matrix --repo /home/peter/dev/worktrees/qsl-2279-catalog-free-api --json`, with stdout redirected to `review-3275/matrix.json` and stderr to `review-3275/matrix.stderr`. Expected JSON matrix/evidence output and termination. No explicit timeout was set; the running tool session was interrupted with Ctrl-C after it produced no result while independent inspection continued. Exit 130; stdout and stderr files both zero bytes. Exact elapsed time was not captured and is unavailable. Smallest reproduction: from the reviewed worktree, `/home/peter/.npm-global/bin/quoin matrix --repo . --json`. It was not rerun. Documented static fallback completed exit 0.
