---
id: SR-2026111919340478491681403
title: "code-review of QSL-2040 source call values"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2040; PR 3083; tests/corpus/collections/calls/**; tests/it/source_corpus.rs"
review_set: subset
---

## Summary

Diff-scoped code-review including Rust/style; no defects found.

## Verdict

**PASS for the changed group.** Static diff review is clean; the lead reports the existing focused run passed. This is no repository-wide assurance or QSL-2040 completion claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

All 88 changed files examined: 29 original source/request/expectation triples and one eight-line Rust test route. Native-run/1 calls use public fsl_command command intake and the existing strict result reader; exact serde_json equality checks complete returned values. The test attribute precedes trace, calls the existing runner, and fails on runner errors. Text cases exercise NFC normalization, inclusive bounds, wrong kind and unknown member. Decimal cases exercise retained scalar payloads, membership by value across equivalent scales, exact mixed-scale equality, inclusive bounds, off-scale/noncanonical input and wrong kind. Enum cases exercise two real cases, missing case and nominal type mismatch. Collection returns cover empty/nonempty Sequence, Set sorting, Bag sorting and multiplicity, OrderedSet order, and Map key/value association and sorting. No production code, public API, dependency, model, CI, helper, ignore or compatibility changes. Tests bind existing FSL AC ids; these cases cover subsets and do not claim the full FR-650 fixtures or all refusal detail paths. No applicable AssuranceProfile in established repository context. The runner is context only. No vendored source, model or binary is added.

Frozen head 4fc2af333ba33f4df9d68d6b214c922d3f67cd59. The lead reports the existing focused source_corpus::corpus_collection_call_values run completed: 1 passed, 0 failed, 0 ignored, 865 filtered, 0.27 seconds runtime after 13m57s compilation; log /home/peter/dev/.agent/qsl-2040-call-values-focused.log. This covers the 29 new programs. Evidence accepted without rerun. Lead owns the final applicable changed-group gate. Reviewer ran no tests, builds, list/discovery, computed matrix or full baseline. Prior 107 cases are accepted without rerun. Pending QSL-2572/QSL-2573 guard cases unchanged and excluded.

Plan completion: not assessed

## Examined Scope

```json
{
  "scope": [
    {
      "id": "tests/corpus/collections/calls/decimal-above.expected.json",
      "path": "tests/corpus/collections/calls/decimal-above.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-above.native",
      "path": "tests/corpus/collections/calls/decimal-above.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-above.request.json",
      "path": "tests/corpus/collections/calls/decimal-above.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-above.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-above\",\"document\":\"decimal-above\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"101\",\"scale\":\"2\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-below.expected.json",
      "path": "tests/corpus/collections/calls/decimal-below.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-below.native",
      "path": "tests/corpus/collections/calls/decimal-below.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-below.request.json",
      "path": "tests/corpus/collections/calls/decimal-below.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-below.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-below\",\"document\":\"decimal-below\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"-101\",\"scale\":\"2\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-equivalent-scale.expected.json",
      "path": "tests/corpus/collections/calls/decimal-equivalent-scale.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"decimal\",\"coefficient\":\"10\",\"scale\":\"1\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-equivalent-scale.native",
      "path": "tests/corpus/collections/calls/decimal-equivalent-scale.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-equivalent-scale.request.json",
      "path": "tests/corpus/collections/calls/decimal-equivalent-scale.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-equivalent-scale.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-equivalent-scale\",\"document\":\"decimal-equivalent-scale\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"10\",\"scale\":\"1\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-maximum.expected.json",
      "path": "tests/corpus/collections/calls/decimal-maximum.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"decimal\",\"coefficient\":\"100\",\"scale\":\"2\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-maximum.native",
      "path": "tests/corpus/collections/calls/decimal-maximum.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-maximum.request.json",
      "path": "tests/corpus/collections/calls/decimal-maximum.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-maximum.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-maximum\",\"document\":\"decimal-maximum\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"100\",\"scale\":\"2\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-minimum.expected.json",
      "path": "tests/corpus/collections/calls/decimal-minimum.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"decimal\",\"coefficient\":\"-100\",\"scale\":\"2\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-minimum.native",
      "path": "tests/corpus/collections/calls/decimal-minimum.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-minimum.request.json",
      "path": "tests/corpus/collections/calls/decimal-minimum.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-minimum.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-minimum\",\"document\":\"decimal-minimum\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"-100\",\"scale\":\"2\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-mixed-scale-different.expected.json",
      "path": "tests/corpus/collections/calls/decimal-mixed-scale-different.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"boolean\":false}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-mixed-scale-different.native",
      "path": "tests/corpus/collections/calls/decimal-mixed-scale-different.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 0, 2; exact];\nfunction f using v(a: Number,b: Number): Boolean pure { a=b }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-mixed-scale-different.request.json",
      "path": "tests/corpus/collections/calls/decimal-mixed-scale-different.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-mixed-scale-different.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-mixed-scale-different\",\"document\":\"decimal-mixed-scale-different\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"50\",\"scale\":\"2\"}},{\"parameter\":\"b\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"6\",\"scale\":\"1\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-mixed-scale-equal.expected.json",
      "path": "tests/corpus/collections/calls/decimal-mixed-scale-equal.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"boolean\":true}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-mixed-scale-equal.native",
      "path": "tests/corpus/collections/calls/decimal-mixed-scale-equal.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 0, 2; exact];\nfunction f using v(a: Number,b: Number): Boolean pure { a=b }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-mixed-scale-equal.request.json",
      "path": "tests/corpus/collections/calls/decimal-mixed-scale-equal.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-mixed-scale-equal.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-mixed-scale-equal\",\"document\":\"decimal-mixed-scale-equal\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"50\",\"scale\":\"2\"}},{\"parameter\":\"b\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"5\",\"scale\":\"1\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-noncanonical.expected.json",
      "path": "tests/corpus/collections/calls/decimal-noncanonical.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-noncanonical.native",
      "path": "tests/corpus/collections/calls/decimal-noncanonical.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-noncanonical.request.json",
      "path": "tests/corpus/collections/calls/decimal-noncanonical.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-noncanonical.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-noncanonical\",\"document\":\"decimal-noncanonical\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"01\",\"scale\":\"2\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-off-scale.expected.json",
      "path": "tests/corpus/collections/calls/decimal-off-scale.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-off-scale.native",
      "path": "tests/corpus/collections/calls/decimal-off-scale.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-off-scale.request.json",
      "path": "tests/corpus/collections/calls/decimal-off-scale.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-off-scale.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-off-scale\",\"document\":\"decimal-off-scale\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"1\",\"scale\":\"3\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-retained.expected.json",
      "path": "tests/corpus/collections/calls/decimal-retained.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"decimal\",\"coefficient\":\"50\",\"scale\":\"2\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-retained.native",
      "path": "tests/corpus/collections/calls/decimal-retained.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-retained.request.json",
      "path": "tests/corpus/collections/calls/decimal-retained.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-retained.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-retained\",\"document\":\"decimal-retained\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"50\",\"scale\":\"2\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-wrong-kind.expected.json",
      "path": "tests/corpus/collections/calls/decimal-wrong-kind.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"wrong-value-kind\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-wrong-kind.native",
      "path": "tests/corpus/collections/calls/decimal-wrong-kind.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-wrong-kind.request.json",
      "path": "tests/corpus/collections/calls/decimal-wrong-kind.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-wrong-kind.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-wrong-kind\",\"document\":\"decimal-wrong-kind\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"integer\":\"1\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-zero.expected.json",
      "path": "tests/corpus/collections/calls/decimal-zero.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"decimal\",\"coefficient\":\"0\",\"scale\":\"0\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-zero.native",
      "path": "tests/corpus/collections/calls/decimal-zero.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Number = Decimal[-100, 100; 2, 2; exact];\nfunction f using v(a: Number): Number pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/decimal-zero.request.json",
      "path": "tests/corpus/collections/calls/decimal-zero.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"decimal-zero.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:decimal-zero\",\"document\":\"decimal-zero\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"decimal\",\"coefficient\":\"0\",\"scale\":\"0\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-blue.expected.json",
      "path": "tests/corpus/collections/calls/enum-blue.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"variant\":{\"type\":\"Color\",\"case\":\"blue\"}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-blue.native",
      "path": "tests/corpus/collections/calls/enum-blue.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nenum Color { red, blue }\nfunction f using v(a: Color): Color pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-blue.request.json",
      "path": "tests/corpus/collections/calls/enum-blue.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"enum-blue.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:enum-blue\",\"document\":\"enum-blue\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"variant\":{\"type\":\"Color\",\"case\":\"blue\"}}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-case-missing.expected.json",
      "path": "tests/corpus/collections/calls/enum-case-missing.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-case-missing.native",
      "path": "tests/corpus/collections/calls/enum-case-missing.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nenum Color { red, blue }\nfunction f using v(a: Color): Color pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-case-missing.request.json",
      "path": "tests/corpus/collections/calls/enum-case-missing.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"enum-case-missing.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:enum-case-missing\",\"document\":\"enum-case-missing\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"variant\":{\"type\":\"Color\",\"case\":\"green\"}}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-red.expected.json",
      "path": "tests/corpus/collections/calls/enum-red.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"variant\":{\"type\":\"Color\",\"case\":\"red\"}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-red.native",
      "path": "tests/corpus/collections/calls/enum-red.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nenum Color { red, blue }\nfunction f using v(a: Color): Color pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-red.request.json",
      "path": "tests/corpus/collections/calls/enum-red.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"enum-red.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:enum-red\",\"document\":\"enum-red\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"variant\":{\"type\":\"Color\",\"case\":\"red\"}}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-type-mismatch.expected.json",
      "path": "tests/corpus/collections/calls/enum-type-mismatch.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"wrong-value-kind\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-type-mismatch.native",
      "path": "tests/corpus/collections/calls/enum-type-mismatch.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nenum Color { red, blue }\nfunction f using v(a: Color): Color pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/enum-type-mismatch.request.json",
      "path": "tests/corpus/collections/calls/enum-type-mismatch.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"enum-type-mismatch.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:enum-type-mismatch\",\"document\":\"enum-type-mismatch\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"variant\":{\"type\":\"Other\",\"case\":\"red\"}}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-bag.expected.json",
      "path": "tests/corpus/collections/calls/result-bag.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"bag\":[{\"integer\":\"1\"},{\"integer\":\"2\"},{\"integer\":\"2\"}]}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-bag.native",
      "path": "tests/corpus/collections/calls/result-bag.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Bag<Int[0,9]>[0,3]): Bag<Int[0,9]>[0,3] pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-bag.request.json",
      "path": "tests/corpus/collections/calls/result-bag.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"result-bag.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:result-bag\",\"document\":\"result-bag\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"bag\":[{\"integer\":\"2\"},{\"integer\":\"1\"},{\"integer\":\"2\"}]}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-map.expected.json",
      "path": "tests/corpus/collections/calls/result-map.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"map\":[{\"key\":{\"integer\":\"1\"},\"value\":{\"boolean\":true}},{\"key\":{\"integer\":\"2\"},\"value\":{\"boolean\":false}}]}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-map.native",
      "path": "tests/corpus/collections/calls/result-map.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Map<Int[0,9],Boolean>[0,2]): Map<Int[0,9],Boolean>[0,2] pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-map.request.json",
      "path": "tests/corpus/collections/calls/result-map.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"result-map.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:result-map\",\"document\":\"result-map\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"map\":[{\"key\":{\"integer\":\"2\"},\"value\":{\"boolean\":false}},{\"key\":{\"integer\":\"1\"},\"value\":{\"boolean\":true}}]}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-ordered-set.expected.json",
      "path": "tests/corpus/collections/calls/result-ordered-set.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"ordered-set\":[{\"integer\":\"2\"},{\"integer\":\"1\"}]}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-ordered-set.native",
      "path": "tests/corpus/collections/calls/result-ordered-set.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: OrderedSet<Int[0,9]>[0,3]): OrderedSet<Int[0,9]>[0,3] pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-ordered-set.request.json",
      "path": "tests/corpus/collections/calls/result-ordered-set.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"result-ordered-set.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:result-ordered-set\",\"document\":\"result-ordered-set\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"ordered-set\":[{\"integer\":\"2\"},{\"integer\":\"1\"}]}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-sequence-empty.expected.json",
      "path": "tests/corpus/collections/calls/result-sequence-empty.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"sequence\":[]}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-sequence-empty.native",
      "path": "tests/corpus/collections/calls/result-sequence-empty.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Sequence<Int[0,9]>[0,3]): Sequence<Int[0,9]>[0,3] pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-sequence-empty.request.json",
      "path": "tests/corpus/collections/calls/result-sequence-empty.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"result-sequence-empty.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:result-sequence-empty\",\"document\":\"result-sequence-empty\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"sequence\":[]}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-sequence.expected.json",
      "path": "tests/corpus/collections/calls/result-sequence.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"sequence\":[{\"integer\":\"2\"},{\"integer\":\"1\"}]}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-sequence.native",
      "path": "tests/corpus/collections/calls/result-sequence.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Sequence<Int[0,9]>[0,3]): Sequence<Int[0,9]>[0,3] pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-sequence.request.json",
      "path": "tests/corpus/collections/calls/result-sequence.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"result-sequence.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:result-sequence\",\"document\":\"result-sequence\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"sequence\":[{\"integer\":\"2\"},{\"integer\":\"1\"}]}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-set.expected.json",
      "path": "tests/corpus/collections/calls/result-set.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"set\":[{\"integer\":\"1\"},{\"integer\":\"2\"}]}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-set.native",
      "path": "tests/corpus/collections/calls/result-set.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Set<Int[0,9]>[0,3]): Set<Int[0,9]>[0,3] pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/result-set.request.json",
      "path": "tests/corpus/collections/calls/result-set.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"result-set.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:result-set\",\"document\":\"result-set\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"set\":[{\"integer\":\"2\"},{\"integer\":\"1\"}]}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-above.expected.json",
      "path": "tests/corpus/collections/calls/text-above.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-above.native",
      "path": "tests/corpus/collections/calls/text-above.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Label = Text[1, 4; nfc];\nfunction f using v(a: Label): Label pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-above.request.json",
      "path": "tests/corpus/collections/calls/text-above.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"text-above.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:text-above\",\"document\":\"text-above\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"text\",\"text\":\"abcde\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-empty.expected.json",
      "path": "tests/corpus/collections/calls/text-empty.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-empty.native",
      "path": "tests/corpus/collections/calls/text-empty.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Label = Text[1, 4; nfc];\nfunction f using v(a: Label): Label pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-empty.request.json",
      "path": "tests/corpus/collections/calls/text-empty.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"text-empty.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:text-empty\",\"document\":\"text-empty\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"text\",\"text\":\"\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-maximum.expected.json",
      "path": "tests/corpus/collections/calls/text-maximum.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"text\",\"text\":\"abcd\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-maximum.native",
      "path": "tests/corpus/collections/calls/text-maximum.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Label = Text[1, 4; nfc];\nfunction f using v(a: Label): Label pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-maximum.request.json",
      "path": "tests/corpus/collections/calls/text-maximum.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"text-maximum.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:text-maximum\",\"document\":\"text-maximum\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"text\",\"text\":\"abcd\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-minimum.expected.json",
      "path": "tests/corpus/collections/calls/text-minimum.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"text\",\"text\":\"a\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-minimum.native",
      "path": "tests/corpus/collections/calls/text-minimum.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Label = Text[1, 4; nfc];\nfunction f using v(a: Label): Label pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-minimum.request.json",
      "path": "tests/corpus/collections/calls/text-minimum.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"text-minimum.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:text-minimum\",\"document\":\"text-minimum\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"text\",\"text\":\"a\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-normalize.expected.json",
      "path": "tests/corpus/collections/calls/text-normalize.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"text\",\"text\":\"é\"}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-normalize.native",
      "path": "tests/corpus/collections/calls/text-normalize.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Label = Text[1, 4; nfc];\nfunction f using v(a: Label): Label pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-normalize.request.json",
      "path": "tests/corpus/collections/calls/text-normalize.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"text-normalize.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:text-normalize\",\"document\":\"text-normalize\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"text\",\"text\":\"é\"}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-unknown-member.expected.json",
      "path": "tests/corpus/collections/calls/text-unknown-member.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"unknown-member\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-unknown-member.native",
      "path": "tests/corpus/collections/calls/text-unknown-member.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Label = Text[1, 4; nfc];\nfunction f using v(a: Label): Label pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-unknown-member.request.json",
      "path": "tests/corpus/collections/calls/text-unknown-member.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"text-unknown-member.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:text-unknown-member\",\"document\":\"text-unknown-member\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"text\",\"text\":\"abc\",\"extra\":1}}]}}}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-wrong-kind.expected.json",
      "path": "tests/corpus/collections/calls/text-wrong-kind.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"wrong-value-kind\"}\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-wrong-kind.native",
      "path": "tests/corpus/collections/calls/text-wrong-kind.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\ntype Label = Text[1, 4; nfc];\nfunction f using v(a: Label): Label pure { a }\n"
    },
    {
      "id": "tests/corpus/collections/calls/text-wrong-kind.request.json",
      "path": "tests/corpus/collections/calls/text-wrong-kind.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"text-wrong-kind.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:calls:text-wrong-kind\",\"document\":\"text-wrong-kind\",\"formal_revision\":1}},\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"kind\":\"string\",\"text\":\"abc\"}}]}}}\n"
    },
    {
      "id": "tests/it/source_corpus.rs",
      "path": "tests/it/source_corpus.rs",
      "role": "examined",
      "excerpt": "#[test]\n#[trace(\"FR-650-AC-1\", \"FR-650-AC-3\", \"FR-650-AC-11\")]\nfn corpus_collection_call_values() {\n    let directory = corpus_root().join(\"collections/calls\");\n    let report = run_directory(&directory).unwrap_or_else(|error| panic!(\"{error}\"));\n    println!(\"{report}\");\n}\n\n#[trace(\"FR-106\", \"FR-107\", \"FR-312-AC-1\")]"
    },
    {
      "id": "FR-650-AC-1",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "Through `run`, each row of the identity-fixture table, as a native-run/1 `call` of its function with its argument `value`, returns one `spine-run-result/1` document whose `outcome` is `{\"kind\": \"completed\", \"value\": V}` with `V` the row's completed value, exit 0."
    },
    {
      "id": "FR-650-AC-3",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "Over the identity fixture, each refuses at stage `call`, exit 20, no result document, with `details.position` 0 and the `details.path` given: `id_str` with `{\"kind\": \"text\", \"text\": \"a\"}`, `invalid_runtime_input`/`wrong-value-kind`, `\"\"`; `id_rec` with `fields` lacking `y`, `missing-member`, `/record/fields`; `id_rec` with an extra field `z`, `unknown-member`, `/record/fields/z`; `id_enum` with case `Blue`, `invalid-value`, `/variant/case`; `id_rat` with denominator `\"0\"`, `invalid-value`, `\"\"`; `id_set` with `[{\"integer\": \"1\"}, {\"integer\": \"1\"}]`, `duplicate-member`, `/set/1`; `id_seq` with f"
    },
    {
      "id": "FR-650-AC-11",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "`function id_map using v(a: Map<String, Int[0, 9]>[0, 2]): Map<String, Int[0, 9]>[0, 2] pure { a }` called with the entries for keys `b` then `a` completes with the entries for `a` then `b`, exit 0. It refuses, at stage `call`, exit 20: the key `a` given twice, `invalid_runtime_input`/`duplicate-member` at `/map/1/key`; three entries, `cardinality_out_of_bound`/`above-maximum` at `\"\"`; an entry without `value`, `missing-member` at `/map/0`; an entry with a member `extra`, `unknown-member` at `/map/0/extra`."
    }
  ]
}
```
