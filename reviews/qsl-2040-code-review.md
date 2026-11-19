---
id: SR-2026111911313600000007
title: "code-review of QSL-2040 collection source corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3056; tests/it/source_corpus.rs; tests/corpus/collections/**"
review_set: subset
---

## Summary

Ticket QSL-2040, PR 3056, reviewed head 881338f5da16052589e69b978944f98918aa728e. Code review includes the Rust review/style lane and test-oracle inspection. No defects found in this delivered slice.

## Verdict

**PASS for the reviewed diff.** The real public entry points and explicit result/refusal assertions provide behavioral tests; no production path or CI gate is changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Reviewed the complete PR diff: tests/it/source_corpus.rs and all added files under tests/corpus/collections/, including native programs, request documents, expected records, and the two pending reproducers. The corpus covers collection kinds, filtering/flatMap, aggregates, quantifiers, named calls, recursion, collection indexing, map presence, conversions, options, type/range refusals, and scalar runtime outcomes. The runner uses existing public compile/run entry points; the changed test no longer silently skips a missing collections directory. Expected runtime values, categories, lifecycle stages, and refusal codes/causes make the cases sensitive to real language defects.

The delivered slice is 107 runnable cases (78 admission and 29 run requests). A run request may deliberately refuse before runtime, so this is not a claim of 29 successful evaluations. In particular, r-mod-zero checks the settled S3 unproved-nonzero refusal; it does not establish the older ticket's runtime Undefined behavior. Successful evaluations compare their actual integer/Boolean results. Cardinality and narrow-count requests check runtime refusals. Existing comparison logic and request/source pairing were read as context, not changed or requalified.

Trace bindings FR-093-AC-59 and FR-680-AC-1 correspond to exercised collection operations/indexing portions. They do not establish all graph-shape, metering, operand, or profile clauses of those compound criteria. No full-criterion or repository-coverage completion is claimed.

Pending map-literal-key-guard and fact-guard-sum-2var sources remain in a nested pending directory outside runnable traversal, with intended success preserved; required-admission fixes are owned by QSL-2572 and QSL-2573. No refusal for either has been blessed as conforming. Remaining old probe cases and broader QSL-2040 scope stay open; QSL-2041 is not delivered by this PR.

Reported coder evidence accepted without repetition: focused source_corpus::corpus_collections passed 107 cases, exit 0; rustfmt check passed; /home/peter/dev/.agent/qsl-2040-fix-focused.log. No tests, builds, aggregate gate, matrix computation, or preexisting peer results were rerun. The code lead owns the one final aggregate gate after review. No applicable AssuranceProfile was found under the local spec tree.

Plan completion: not assessed

## Examined Scope

```json
{
  "scope": [
    {
      "id": "corpus_collections",
      "path": "tests/it/source_corpus.rs",
      "role": "examined",
      "excerpt": "fn corpus_collections() {\n    let directory = corpus_root().join(\"collections\");\n    let report = run_directory(&directory).unwrap_or_else(|error| panic!(\"{error}\"));\n    println!(\"{report}\");\n}"
    },
    {
      "id": "mismatch",
      "path": "tests/it/source_corpus.rs",
      "role": "examined",
      "excerpt": ".result\n            .as_ref()\n            .is_some_and(|value| actual.result.as_ref() != Some(value))"
    },
    {
      "id": "FR-093-AC-59",
      "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
      "role": "context_only",
      "excerpt": "One-occurrence collection forms (QSL-1488; FSpec FR-145-AC-10 and AC-11, TC-190). Each of these checks, lowers and evaluates:"
    },
    {
      "id": "FR-680-AC-1",
      "path": "spec/functional/FR-680-check-indexing-tuple-positions-maps-option-construction-and-quantity-scaling.md",
      "role": "context_only",
      "excerpt": "`s[0]` over a parameter `s: Sequence<Integer>[1, 3]`, `r.xs[0]` over a field `xs: Sequence<Integer>[1, 2]`, and `o[1]` over `o: OrderedSet<Integer>[0, 3]` under the guard `size(o) >= 2` check with result type `Integer`"
    },
    {
      "id": "r-sum",
      "path": "tests/corpus/collections/r-sum.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"18\"}}"
    }
  ]
}
```

