---
id: SR-2026111919340478491681405
title: "Code and Rust review of QSL-2040 context and runtime corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2040; PR 3089; tests/corpus/collections/contexts/**; tests/it/source_corpus_collection_contexts.rs; tests/it/source_corpus.rs additive registration"
review_set: subset
---

## Summary

Reviewed the frozen d17ad5331f23071bb4808633d0ff667dee602124 diff: 34 original programs, requests and exact expectations, one snapshot, and the small Rust route. Code review includes the Rust/style lane. No findings.

## Verdict

**PASS for static review of the changed group.** Runtime readiness remains conditional on the coder's existing focused process, which was still compiling at dispatch. No pass is inferred, no tests or builds were repeated, and no repository-wide or whole-ticket completion is asserted. Lead owns the final applicable gate.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

All changed files examined. Existing strict source-corpus execution performs public command intake, matches exact stage/category/exit/code/cause and compares complete returned JSON values. Every new test runs the existing harness and panics on errors; the test attribute precedes the trace attribute. No production code, dependency, CI, unsafe code, public API, permissive reader, ignored test, helper framework or model copy is added. The model is referenced from the authoritative existing same-repository fixture; the snapshot is original test input.

The pure-function/profile cases follow FR-110, FR-230-AC-64 and FR-361 item 12's derived model-body exception. Population-name refusal and sequence clause context follow FR-391-AC-8 and AC-10. Runtime values and refusals are subsets of FR-650, not claims of full criterion coverage. Checked the root/child field values, complete-population dangling reference, missing snapshot, wrong argument kind, forward/reverse/reflexive reachability, explicit versus ambient population, population count true/false controls, contextual seq versus ordinary seq function, finite sequences with unbounded Integer element/cardinality types including empty and negative cases, literal cardinality minimum/maximum failures and successful empty/one-element boundaries, and guarded call outcomes at 0, 8 and 9. The state-core mod case observes declaration gating; it does not claim isolated modulo-expression gating.

No applicable AssuranceProfile in the established repository context. Spec review is inapplicable because no spec or plan changed. No standalone Rust artifact: it is folded into this code-review artifact. No tests, builds, full gates, discovery runs or computed matrix were executed by this reviewer.

## Examined Scope

```json
{
  "scope": [
    {
      "id": "tests/corpus/collections/contexts/clock-temporal.expected.json",
      "path": "tests/corpus/collections/contexts/clock-temporal.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"check\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/clock-temporal.native",
      "path": "tests/corpus/collections/contexts/clock-temporal.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile t = \"filament.temporal.complete/v1\";\nfunction f using t(c: Clock): Boolean pure { true }"
    },
    {
      "id": "tests/corpus/collections/contexts/clock-value-refused.expected.json",
      "path": "tests/corpus/collections/contexts/clock-value-refused.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"check\",\"exit_code\":21,\"category\":\"unsupported\",\"last_stage\":\"S3\",\"code\":\"unsupported_construct\",\"cause\":\"declaration-form\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/clock-value-refused.native",
      "path": "tests/corpus/collections/contexts/clock-value-refused.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(c: Clock): Boolean pure { true }"
    },
    {
      "id": "tests/corpus/collections/contexts/data/current.json",
      "path": "tests/corpus/collections/contexts/data/current.json",
      "role": "examined",
      "excerpt": "{\"format\":\"filament.state.snapshot/v1\",\"identity\":{\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:current\"},\"observation\":\"current\",\"anchor\":{\"kind\":\"handler\",\"name\":\"validate\"},\"model\":{\"identity\":\"example/config-version\"},\"populations\":[{\"population\":\"ix://example/config-version/config_history\",\"complete\":true,\"objects\":[{\"key\":\"root\",\"type\":\"ix://example/config-version/ConfigVersion\",\"fields\":{\"versionNumber\":{\"integer\":\"1\"},\"parent\":{\"absent\":{}}}},{\"key\":\"child\",\"type\":\"ix://example/config-version/ConfigVersion\",\"fields\":{\"versionNumber\":{\"integer\":\"2\"},\"parent\":{\"present\":{"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-child.expected.json",
      "path": "tests/corpus/collections/contexts/deref-child.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"2\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-child.native",
      "path": "tests/corpus/collections/contexts/deref-child.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using m(a: Reference<M::ConfigVersion>): Integer pure { deref(a).versionNumber }"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-child.request.json",
      "path": "tests/corpus/collections/contexts/deref-child.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"reference\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"}}}],\"snapshot\":\"data/current.json\"},\"program\":{\"source\":{\"file\":\"deref-child.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:deref-child\",\"document\":\"deref-child\",\"form"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-dangling.expected.json",
      "path": "tests/corpus/collections/contexts/deref-dangling.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"dangling_reference\",\"cause\":\"absent-target-in-complete-population\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-dangling.native",
      "path": "tests/corpus/collections/contexts/deref-dangling.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using m(a: Reference<M::ConfigVersion>): Integer pure { deref(a).versionNumber }"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-dangling.request.json",
      "path": "tests/corpus/collections/contexts/deref-dangling.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"reference\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"ghost\"}}}],\"snapshot\":\"data/current.json\"},\"program\":{\"source\":{\"file\":\"deref-dangling.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:deref-dangling\",\"document\":\"deref-dangli"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-derived-body-layer.expected.json",
      "path": "tests/corpus/collections/contexts/deref-derived-body-layer.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"2\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-derived-body-layer.native",
      "path": "tests/corpus/collections/contexts/deref-derived-body-layer.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using v(a: Reference<M::ConfigVersion>): Integer pure { deref(a).versionNumber }"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-derived-body-layer.request.json",
      "path": "tests/corpus/collections/contexts/deref-derived-body-layer.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"reference\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"}}}],\"snapshot\":\"data/current.json\"},\"program\":{\"source\":{\"file\":\"deref-derived-body-layer.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:deref-derived-body-layer\",\"doc"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-root.expected.json",
      "path": "tests/corpus/collections/contexts/deref-root.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"1\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-root.native",
      "path": "tests/corpus/collections/contexts/deref-root.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using m(a: Reference<M::ConfigVersion>): Integer pure { deref(a).versionNumber }"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-root.request.json",
      "path": "tests/corpus/collections/contexts/deref-root.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"reference\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"root\"}}}],\"snapshot\":\"data/current.json\"},\"program\":{\"source\":{\"file\":\"deref-root.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:deref-root\",\"document\":\"deref-root\",\"formal_r"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-value-layer.expected.json",
      "path": "tests/corpus/collections/contexts/deref-value-layer.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":21,\"category\":\"unsupported\",\"last_stage\":\"S3\",\"code\":\"unsupported_construct\",\"cause\":\"expression-form\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-value-layer.native",
      "path": "tests/corpus/collections/contexts/deref-value-layer.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using v(a: Reference<M::ConfigVersion>): Integer pure { deref(a).versionNumber }"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-value-layer.request.json",
      "path": "tests/corpus/collections/contexts/deref-value-layer.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"program\":{\"source\":{\"file\":\"deref-value-layer.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:deref-value-layer\",\"document\":\"deref-value-layer\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-without-snapshot.expected.json",
      "path": "tests/corpus/collections/contexts/deref-without-snapshot.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"missing-member\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-without-snapshot.native",
      "path": "tests/corpus/collections/contexts/deref-without-snapshot.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using m(a: Reference<M::ConfigVersion>): Integer pure { deref(a).versionNumber }"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-without-snapshot.request.json",
      "path": "tests/corpus/collections/contexts/deref-without-snapshot.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"reference\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"}}}]},\"program\":{\"source\":{\"file\":\"deref-without-snapshot.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:deref-without-snapshot\",\"document\":\"deref-without-snapshot\",\"fo"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-wrong-kind.expected.json",
      "path": "tests/corpus/collections/contexts/deref-wrong-kind.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"invalid_runtime_input\",\"cause\":\"wrong-value-kind\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-wrong-kind.native",
      "path": "tests/corpus/collections/contexts/deref-wrong-kind.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using m(a: Reference<M::ConfigVersion>): Integer pure { deref(a).versionNumber }"
    },
    {
      "id": "tests/corpus/collections/contexts/deref-wrong-kind.request.json",
      "path": "tests/corpus/collections/contexts/deref-wrong-kind.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"integer\":\"1\"}}],\"snapshot\":\"data/current.json\"},\"program\":{\"source\":{\"file\":\"deref-wrong-kind.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:deref-wrong-kind\",\"document\":\"deref-wrong-kind\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-else-boundary.expected.json",
      "path": "tests/corpus/collections/contexts/fact-ac19-else-boundary.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"0\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-else-boundary.native",
      "path": "tests/corpus/collections/contexts/fact-ac19-else-boundary.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction inc using v(a: Int[0,8]): Int[1,9] pure { a+1 }\nfunction f using v(a: Int[0,9]): Integer pure { if a<9 then inc(a) else 0 }"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-else-boundary.request.json",
      "path": "tests/corpus/collections/contexts/fact-ac19-else-boundary.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"integer\":\"9\"}}]},\"program\":{\"source\":{\"file\":\"fact-ac19-else-boundary.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:fact-ac19-else-boundary\",\"document\":\"fact-ac19-else-boundary\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-guard-limit.expected.json",
      "path": "tests/corpus/collections/contexts/fact-ac19-guard-limit.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"9\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-guard-limit.native",
      "path": "tests/corpus/collections/contexts/fact-ac19-guard-limit.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction inc using v(a: Int[0,8]): Int[1,9] pure { a+1 }\nfunction f using v(a: Int[0,9]): Integer pure { if a<9 then inc(a) else 0 }"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-guard-limit.request.json",
      "path": "tests/corpus/collections/contexts/fact-ac19-guard-limit.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"integer\":\"8\"}}]},\"program\":{\"source\":{\"file\":\"fact-ac19-guard-limit.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:fact-ac19-guard-limit\",\"document\":\"fact-ac19-guard-limit\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-minimum.expected.json",
      "path": "tests/corpus/collections/contexts/fact-ac19-minimum.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"1\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-minimum.native",
      "path": "tests/corpus/collections/contexts/fact-ac19-minimum.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction inc using v(a: Int[0,8]): Int[1,9] pure { a+1 }\nfunction f using v(a: Int[0,9]): Integer pure { if a<9 then inc(a) else 0 }"
    },
    {
      "id": "tests/corpus/collections/contexts/fact-ac19-minimum.request.json",
      "path": "tests/corpus/collections/contexts/fact-ac19-minimum.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"integer\":\"0\"}}]},\"program\":{\"source\":{\"file\":\"fact-ac19-minimum.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:fact-ac19-minimum\",\"document\":\"fact-ac19-minimum\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/function-state-core.expected.json",
      "path": "tests/corpus/collections/contexts/function-state-core.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"check\",\"exit_code\":21,\"category\":\"unsupported\",\"last_stage\":\"S3\",\"code\":\"unsupported_construct\",\"cause\":\"declaration-form\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/function-state-core.native",
      "path": "tests/corpus/collections/contexts/function-state-core.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile s = \"filament.state.core/v1\";\nfunction f using s(a: Integer): Integer pure { a }"
    },
    {
      "id": "tests/corpus/collections/contexts/integer-type-is-not-domain.expected.json",
      "path": "tests/corpus/collections/contexts/integer-type-is-not-domain.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"check\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S1\",\"code\":\"invalid_syntax\",\"cause\":\"unexpected-token\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/integer-type-is-not-domain.native",
      "path": "tests/corpus/collections/contexts/integer-type-is-not-domain.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Boolean pure { forall(x in Integer: true) }"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-above.expected.json",
      "path": "tests/corpus/collections/contexts/literal-above.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"cardinality_out_of_bound\",\"cause\":\"above-maximum\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-above.native",
      "path": "tests/corpus/collections/contexts/literal-above.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Sequence<Integer>[0,1] pure { sequence[1,2] }"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-above.request.json",
      "path": "tests/corpus/collections/contexts/literal-above.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"call\":{\"function\":\"f\",\"arguments\":[]},\"program\":{\"source\":{\"file\":\"literal-above.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:literal-above\",\"document\":\"literal-above\",\"formal_revision\":1}},\"models\":[]}}"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-below.expected.json",
      "path": "tests/corpus/collections/contexts/literal-below.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S6a\",\"code\":\"cardinality_out_of_bound\",\"cause\":\"below-minimum\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-below.native",
      "path": "tests/corpus/collections/contexts/literal-below.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Sequence<Integer>[1,2] pure { sequence[] }"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-below.request.json",
      "path": "tests/corpus/collections/contexts/literal-below.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"call\":{\"function\":\"f\",\"arguments\":[]},\"program\":{\"source\":{\"file\":\"literal-below.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:literal-below\",\"document\":\"literal-below\",\"formal_revision\":1}},\"models\":[]}}"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-empty.expected.json",
      "path": "tests/corpus/collections/contexts/literal-empty.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"sequence\":[]}}"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-empty.native",
      "path": "tests/corpus/collections/contexts/literal-empty.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Sequence<Integer>[0,1] pure { sequence[] }"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-empty.request.json",
      "path": "tests/corpus/collections/contexts/literal-empty.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"call\":{\"function\":\"f\",\"arguments\":[]},\"program\":{\"source\":{\"file\":\"literal-empty.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:literal-empty\",\"document\":\"literal-empty\",\"formal_revision\":1}},\"models\":[]}}"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-maximum.expected.json",
      "path": "tests/corpus/collections/contexts/literal-maximum.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"sequence\":[{\"integer\":\"1\"}]}}"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-maximum.native",
      "path": "tests/corpus/collections/contexts/literal-maximum.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Sequence<Integer>[0,1] pure { sequence[1] }"
    },
    {
      "id": "tests/corpus/collections/contexts/literal-maximum.request.json",
      "path": "tests/corpus/collections/contexts/literal-maximum.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"call\":{\"function\":\"f\",\"arguments\":[]},\"program\":{\"source\":{\"file\":\"literal-maximum.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:literal-maximum\",\"document\":\"literal-maximum\",\"formal_revision\":1}},\"models\":[]}}"
    },
    {
      "id": "tests/corpus/collections/contexts/mod-state-core.expected.json",
      "path": "tests/corpus/collections/contexts/mod-state-core.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"check\",\"exit_code\":21,\"category\":\"unsupported\",\"last_stage\":\"S3\",\"code\":\"unsupported_construct\",\"cause\":\"declaration-form\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/mod-state-core.native",
      "path": "tests/corpus/collections/contexts/mod-state-core.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile s = \"filament.state.core/v1\";\nfunction f using s(): Integer pure { 8 mod 3 }"
    },
    {
      "id": "tests/corpus/collections/contexts/mod-value.expected.json",
      "path": "tests/corpus/collections/contexts/mod-value.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"2\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/mod-value.native",
      "path": "tests/corpus/collections/contexts/mod-value.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Integer pure { 8 mod 3 }"
    },
    {
      "id": "tests/corpus/collections/contexts/mod-value.request.json",
      "path": "tests/corpus/collections/contexts/mod-value.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[]},\"program\":{\"source\":{\"file\":\"mod-value.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:mod-value\",\"document\":\"mod-value\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/population-count-clause-false.expected.json",
      "path": "tests/corpus/collections/contexts/population-count-clause-false.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":10,\"category\":\"violation\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":false}}"
    },
    {
      "id": "tests/corpus/collections/contexts/population-count-clause-false.native",
      "path": "tests/corpus/collections/contexts/population-count-clause-false.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\ninvariant Q using m on M::ConfigVersion at current { count(x in M::config_history where x.versionNumber >= 2) = 2 }"
    },
    {
      "id": "tests/corpus/collections/contexts/population-count-clause-false.request.json",
      "path": "tests/corpus/collections/contexts/population-count-clause-false.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"clause\":{\"selection\":{\"clause\":{\"name\":\"Q\",\"observation\":\"current\",\"self\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"},\"anchor\":{\"kind\":\"handler\",\"name\":\"validate\"}}},\"observations\":{\"current\":\"data/current.json\"}},\"program\":{\"source\":{\"file\":\"population-count-clause-false.native\",\"authority\":\"agent-ix\",\""
    },
    {
      "id": "tests/corpus/collections/contexts/population-count-clause.expected.json",
      "path": "tests/corpus/collections/contexts/population-count-clause.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":true}}"
    },
    {
      "id": "tests/corpus/collections/contexts/population-count-clause.native",
      "path": "tests/corpus/collections/contexts/population-count-clause.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\ninvariant Q using m on M::ConfigVersion at current { count(x in M::config_history where x.versionNumber >= 1) = 2 }"
    },
    {
      "id": "tests/corpus/collections/contexts/population-count-clause.request.json",
      "path": "tests/corpus/collections/contexts/population-count-clause.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"clause\":{\"selection\":{\"clause\":{\"name\":\"Q\",\"observation\":\"current\",\"self\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"},\"anchor\":{\"kind\":\"handler\",\"name\":\"validate\"}}},\"observations\":{\"current\":\"data/current.json\"}},\"program\":{\"source\":{\"file\":\"population-count-clause.native\",\"authority\":\"agent-ix\",\"identi"
    },
    {
      "id": "tests/corpus/collections/contexts/population-function-ambient.expected.json",
      "path": "tests/corpus/collections/contexts/population-function-ambient.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"missing_declaration\",\"cause\":\"missing-name\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/population-function-ambient.native",
      "path": "tests/corpus/collections/contexts/population-function-ambient.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using m(): Boolean pure { size(allInstances<M::ConfigVersion>(M::config_history)) >= 0 }"
    },
    {
      "id": "tests/corpus/collections/contexts/population-function-ambient.request.json",
      "path": "tests/corpus/collections/contexts/population-function-ambient.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"program\":{\"source\":{\"file\":\"population-function-ambient.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:population-function-ambient\",\"document\":\"population-function-ambient\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/population-function-explicit.expected.json",
      "path": "tests/corpus/collections/contexts/population-function-explicit.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"2\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/population-function-explicit.native",
      "path": "tests/corpus/collections/contexts/population-function-explicit.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\ntype Total = Integer;\nfunction f using m(a: Population<M::ConfigVersion>[2]): Integer pure { count<Total>(x in allInstances<M::ConfigVersion>(a): deref(x).versionNumber >= 1) }"
    },
    {
      "id": "tests/corpus/collections/contexts/population-function-explicit.request.json",
      "path": "tests/corpus/collections/contexts/population-function-explicit.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"population\":\"ix://example/config-version/config_history\"}}],\"snapshot\":\"data/current.json\"},\"program\":{\"source\":{\"file\":\"population-function-explicit.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:population-function-explicit\",\"document\":\"population-f"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-parent.expected.json",
      "path": "tests/corpus/collections/contexts/reaches-clause-parent.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":true}}"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-parent.native",
      "path": "tests/corpus/collections/contexts/reaches-clause-parent.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\ninvariant Q using m on M::ConfigVersion at current { if present(self.parent) then reaches(self,value(self.parent),parent) else false }"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-parent.request.json",
      "path": "tests/corpus/collections/contexts/reaches-clause-parent.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"clause\":{\"selection\":{\"clause\":{\"name\":\"Q\",\"observation\":\"current\",\"self\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"},\"anchor\":{\"kind\":\"handler\",\"name\":\"validate\"}}},\"observations\":{\"current\":\"data/current.json\"}},\"program\":{\"source\":{\"file\":\"reaches-clause-parent.native\",\"authority\":\"agent-ix\",\"identity"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-reverse.expected.json",
      "path": "tests/corpus/collections/contexts/reaches-clause-reverse.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":10,\"category\":\"violation\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":false}}"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-reverse.native",
      "path": "tests/corpus/collections/contexts/reaches-clause-reverse.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\ninvariant Q using m on M::ConfigVersion at current { if present(self.parent) then reaches(value(self.parent),self,parent) else false }"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-reverse.request.json",
      "path": "tests/corpus/collections/contexts/reaches-clause-reverse.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"clause\":{\"selection\":{\"clause\":{\"name\":\"Q\",\"observation\":\"current\",\"self\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"},\"anchor\":{\"kind\":\"handler\",\"name\":\"validate\"}}},\"observations\":{\"current\":\"data/current.json\"}},\"program\":{\"source\":{\"file\":\"reaches-clause-reverse.native\",\"authority\":\"agent-ix\",\"identit"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-self.expected.json",
      "path": "tests/corpus/collections/contexts/reaches-clause-self.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":true}}"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-self.native",
      "path": "tests/corpus/collections/contexts/reaches-clause-self.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\ninvariant Q using m on M::ConfigVersion at current { reaches(self,self,parent) }"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-clause-self.request.json",
      "path": "tests/corpus/collections/contexts/reaches-clause-self.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"clause\":{\"selection\":{\"clause\":{\"name\":\"Q\",\"observation\":\"current\",\"self\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"},\"anchor\":{\"kind\":\"handler\",\"name\":\"validate\"}}},\"observations\":{\"current\":\"data/current.json\"}},\"program\":{\"source\":{\"file\":\"reaches-clause-self.native\",\"authority\":\"agent-ix\",\"identity\":"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-function-context.expected.json",
      "path": "tests/corpus/collections/contexts/reaches-function-context.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"ill_typed\",\"cause\":\"operator-ineligible\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-function-context.native",
      "path": "tests/corpus/collections/contexts/reaches-function-context.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\nfunction f using m(a: Reference<M::ConfigVersion>): Boolean pure { reaches(a,a,parent) }"
    },
    {
      "id": "tests/corpus/collections/contexts/reaches-function-context.request.json",
      "path": "tests/corpus/collections/contexts/reaches-function-context.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"program\":{\"source\":{\"file\":\"reaches-function-context.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:reaches-function-context\",\"document\":\"reaches-function-context\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-clause-context.expected.json",
      "path": "tests/corpus/collections/contexts/sequence-clause-context.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"kind\":\"boolean\",\"value\":true}}"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-clause-context.native",
      "path": "tests/corpus/collections/contexts/sequence-clause-context.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile m = \"filament.model.complete/v1\";\nmodel M = \"example/config-version\";\ninvariant Q using m on M::ConfigVersion at current { size(seq(i in 0 .. 1: i)) = 2 }"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-clause-context.request.json",
      "path": "tests/corpus/collections/contexts/sequence-clause-context.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[{\"format\":\"semantic-ir/2.0.0\",\"source\":{\"file\":\"../../../../examples/config-version/model.semantic-ir.json\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:model\",\"document\":\"model\",\"formal_revision\":1}}],\"clause\":{\"selection\":{\"clause\":{\"name\":\"Q\",\"observation\":\"current\",\"self\":{\"population\":\"ix://example/config-version/config_history\",\"key\":\"child\"},\"anchor\":{\"kind\":\"handler\",\"name\":\"validate\"}}},\"observations\":{\"current\":\"data/current.json\"}},\"program\":{\"source\":{\"file\":\"sequence-clause-context.native\",\"authority\":\"agent-ix\",\"identi"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-function-form-refused.expected.json",
      "path": "tests/corpus/collections/contexts/sequence-function-form-refused.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"check\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S1\",\"code\":\"invalid_syntax\",\"cause\":\"unexpected-token\"}"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-function-form-refused.native",
      "path": "tests/corpus/collections/contexts/sequence-function-form-refused.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(): Integer pure { size(seq(i in 0 .. 1: i)) }"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-function-name.expected.json",
      "path": "tests/corpus/collections/contexts/sequence-function-name.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"integer\":\"3\"}}"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-function-name.native",
      "path": "tests/corpus/collections/contexts/sequence-function-name.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction seq using v(a: Integer): Integer pure { a+1 }\nfunction f using v(): Integer pure { seq(2) }"
    },
    {
      "id": "tests/corpus/collections/contexts/sequence-function-name.request.json",
      "path": "tests/corpus/collections/contexts/sequence-function-name.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[]},\"program\":{\"source\":{\"file\":\"sequence-function-name.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:sequence-function-name\",\"document\":\"sequence-function-name\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-all-positive.expected.json",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-all-positive.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"boolean\":true}}"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-all-positive.native",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-all-positive.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Sequence<Integer>): Boolean pure { forall(x in a: x >= 0) }"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-all-positive.request.json",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-all-positive.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"sequence\":[{\"integer\":\"0\"},{\"integer\":\"1208925819614629174706176\"}]}}]},\"program\":{\"source\":{\"file\":\"unbounded-integer-elements-all-positive.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:unbounded-integer-elements-all-positive\",\"document\":\"unbounded-integer-elements-all-positive\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-empty.expected.json",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-empty.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"boolean\":true}}"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-empty.native",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-empty.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Sequence<Integer>): Boolean pure { forall(x in a: x >= 0) }"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-empty.request.json",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-empty.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"sequence\":[]}}]},\"program\":{\"source\":{\"file\":\"unbounded-integer-elements-empty.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:unbounded-integer-elements-empty\",\"document\":\"unbounded-integer-elements-empty\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-negative.expected.json",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-negative.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"run\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S6a\",\"result\":{\"boolean\":false}}"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-negative.native",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-negative.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\nfunction f using v(a: Sequence<Integer>): Boolean pure { forall(x in a: x >= 0) }"
    },
    {
      "id": "tests/corpus/collections/contexts/unbounded-integer-elements-negative.request.json",
      "path": "tests/corpus/collections/contexts/unbounded-integer-elements-negative.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-run/1\",\"request\":{\"models\":[],\"call\":{\"function\":\"f\",\"arguments\":[{\"parameter\":\"a\",\"value\":{\"sequence\":[{\"integer\":\"0\"},{\"integer\":\"-1\"}]}}]},\"program\":{\"source\":{\"file\":\"unbounded-integer-elements-negative.native\",\"authority\":\"agent-ix\",\"identity\":\"test:collections:contexts:unbounded-integer-elements-negative\",\"document\":\"unbounded-integer-elements-negative\",\"formal_revision\":1}}}}"
    },
    {
      "id": "tests/it/source_corpus.rs",
      "path": "tests/it/source_corpus.rs",
      "role": "examined",
      "excerpt": "\n#[path = \"source_corpus_collection_contexts.rs\"]\nmod collection_contexts;"
    },
    {
      "id": "tests/it/source_corpus_collection_contexts.rs",
      "path": "tests/it/source_corpus_collection_contexts.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! Source cases for pure call contexts, layer gates and runtime bounds.\n\nuse ix_trace_rs::trace;\n\n#[test]\n#[trace(\"FR-110\", \"FR-230-AC-64\", \"FR-391-AC-8\", \"FR-391-AC-10\", \"FR-650\")]\nfn collection_contexts_and_runtime_boundaries() {\n    let directory = super::corpus_root().join(\"collections/contexts\");\n    let report = super::run_directory(&directory).unwrap_or_else(|error| panic!(\"{error}\"));\n    println!(\"{report}\");\n}"
    }
  ],
  "clean": true,
  "findings": [],
  "bindings": []
}

```
