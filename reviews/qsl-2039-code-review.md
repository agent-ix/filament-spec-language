---
id: SR-2026111612172400000005
title: "Code review — QSL-2039 values and types source corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2039; PR #2772; tests/it/source_corpus.rs, tests/corpus/values/*"
review_set: subset
---

## Summary

Reviewed the QSL-2039 acceptance criteria and PR #2772 at the original review head against current main `87d15204d`. The review found a stale modulo run expectation, incomplete pinning of named cases and exact flip counts, and missing `flips_with` sentinels on three pinned cases. At current PR head `ce6df44d8d6c0b566835b8252e601fb4e2d1681e`, all three findings are fixed. The focused source-corpus test passes 22/22.

## Verdict

**FAIL at initial review; all findings fixed at disposition.**

## Scope

| ID | Role | Excerpt |
| --- | --- | --- |
| QSL-2039-AC-1 | examined | source_corpus::corpus_values passes over all listed programs; accepting programs reach S4 and rejecting programs match their listed code and cause. |
| QSL-2039-AC-2 | examined | dec-numeral-1965 uses 0.9 and pins the current S1 refusal with flips_with: QSL-1965. |
| QSL-2039-AC-3 | examined | Named run requests return the listed completed values, including 81, 5, 7, 0 and Boolean true. |
| QSL-2039-AC-4 | examined | int-arith-outside exits 20 with invalid_runtime_input. |
| QSL-2039-AC-5 | examined | Every pinned program carries flips_with; the report lists all named keys and their counts. |
| QSL-2039-AC-6 | examined | At least 40 accepted and 40 rejected programs exist, with both outcomes in every value-family prefix. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `mod-zero-2002` expectation is stale against current main; focused corpus test fails | tests/corpus/values/mod-zero-2002.expected.json:1 |
| FND-002 | medium | Required named examples and exact flip counts are not pinned by the runner | tests/it/source_corpus.rs:510-520, 606-624 |
| FND-003 | medium | `qty-carried`, `qty-carried-order`, and `qty-affine` omit required `flips_with` | tests/corpus/values/qty-carried.expected.json:1 |

## Original finding details

- FND-001: The case is documented as an accepted check followed by runtime division-by-zero, but on current main 87d15204d the corpus test reports check exit 20 at S3 with undefined_expression/unproved-nonzero. QSL-2002 has landed, so this pinned expectation must be updated and flips_with removed.
- FND-002: The current fixture has the named examples, but this test does not assert the required program/request names or expected flip counts. Deleting a named case such as float-to-rat and its paired files, or removing one of several QSL-2004 flip entries, can leave the minimum counts and key substring check passing. Assert the required names and flip-count map.
- FND-003: AC-5 says every program in the pinned table carries flips_with. This expectation omits the field; qty-carried-order and qty-affine do as well. The table calls these current-rule cases with no flip ticket, so encode that explicitly (for example flips_with: none) and have the test validate it.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Current check refusal is pinned; the obsolete run request was removed. |
| FND-002 | fixed | Exact case/request inventories and exact flip-count assertions are now exercised. |
| FND-003 | fixed | The three no-ticket cases explicitly carry `flips_with: none`. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      {"verb":"check","exit_code":20,"category":"refusal","last_stage":"S3","code":"undefined_expression","cause":"unproved-nonzero"}
  - fnd: FND-002
    outcome: fixed
    after_excerpt: '    assert_eq!(actual_counts, expected_counts, "pinned flip counts changed");'
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |-
      {"verb":"check","exit_code":0,"category":"success","last_stage":"S4","flips_with":"none"}
```

## Verification

- PR head: `ce6df44d8d6c0b566835b8252e601fb4e2d1681e`.
- Focused command: `CARGO_TARGET_DIR=/Users/peter/dev/qsl-2039-code/target cargo test --locked --test it source_corpus:: -- --nocapture`; 22 passed, 0 failed, 0 ignored.
- Exact inventory: 162 native case stems match the required set; 48 request stems match the required set; all 156 distinct named cases are present.
- Exact flip counts match the fixture report; QSL-2002 is absent after its behavior landed.
- `cargo fmt --check` passed in the earlier review round.
- Initial review tool evidence: quoin 0.35.0; quire 0.45.0 (engine 0.60.0). `quoin gap-analysis --help` was an invalid CLI invocation (exit 2, command not found); the reviewer performed the method's manual acceptance check and the separate gap-analysis artifact records it.
