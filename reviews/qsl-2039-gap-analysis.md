---
id: SR-2026111612172400000006
title: "Gap analysis — QSL-2039 values and types source corpus"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2039; PR #2772; tests/it/source_corpus.rs, tests/corpus/values/*"
review_set: subset
---

## Summary

Manually mapped QSL-2039's six acceptance criteria to the changed fixtures and runner after the Quoin gap-analysis subcommand was unavailable. The review initially found an unreachable `mod-zero-2002` run expectation, missing exact inventory and flip-count assertions, and three omitted `flips_with` fields. At current PR head `ce6df44d8d6c0b566835b8252e601fb4e2d1681e`, all three findings are fixed; the focused source-corpus test passes 22/22.

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
| FND-001 | medium | `mod-zero-2002` run expectation is unreachable on current main because check refuses it | tests/corpus/values/mod-zero-2002.native:3 |
| FND-002 | medium | Corpus test does not enforce all named examples and exact flip counts | tests/it/source_corpus.rs:510-520, 606-624 |
| FND-003 | medium | Three pinned current-rule cases omit `flips_with` | tests/corpus/values/qty-carried.expected.json:1 |

## Original finding details

- FND-001: Current main rejects this source at S3 with undefined_expression/unproved-nonzero. The paired native-run expectation records the earlier S6a division-by-zero result, and `assert_values_programs_compile_at_s4` stops before the run. Refresh the case to current behavior and remove QSL-2002 from flips_with.
- FND-002: A named acceptance fixture and its request can be removed while the minimum counts and a duplicate flip key remain, so the focused test may still pass without covering every named case. Pin the required names and expected flip-count map.
- FND-003: The acceptance criterion requires every pinned case to carry flips_with. qty-carried, qty-carried-order and qty-affine omit that member, and the runner does not validate its presence.

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

## Coverage

- The six ticket acceptance criteria QSL-2039-AC-1 through QSL-2039-AC-6 were examined against the source-corpus fixtures and `source_corpus::corpus_values`.
- Exact inventory and values: 162 native case stems and 48 request stems match the required names; all 156 distinct named cases are present; AC-3 values are checked by the passing test.
- The focused source-corpus test reports the expected exact flip counts: G-001 (1), G-009 (1), G-022 (1), G-024 (1), G-025 (1), G-026 (2), G-027 (1), G-028 (5), QSL-1965 (1), QSL-1986 (1), QSL-1989 (4), QSL-2004 (4), QSL-2006 (1), QSL-2007 (3), QSL-2008 (1), STD-654 (6).
- Reverse-gap inventory: no new API, stub, or TODO; the changed runner and fixtures are owned by the specced ticket.
- The spec root has no artifact id; no spec-root relationship was asserted. Plan completion and semantic review were not assessed.
- Tooling: resolved quoin version 0.35.0; `quoin gap-analysis --help` exited 2 with `Error: command gap-analysis not found`. The manual acceptance check continued. Quoin matrix and Quire matrix/validation exited 0; Quire matrix reported existing `semantic.unused-import`, `section-matches-nothing`, and `symbol-id-collision` diagnostics.
