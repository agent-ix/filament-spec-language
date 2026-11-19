---
id: SR-2026111911313600000019
title: "code-review of QSL-2043 protocol source corpus"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2043; PR 3079; tests/corpus/protocol/**; tests/it/protocol_source_routes.rs; tests/it/main.rs"
review_set: subset
---

## Summary

Diff-scoped code-review, including the Rust/style lane for code-review. Independent original sources exercise public compile/run/analyze and strict emitted handoff reading. No checked-package bypass, sibling copy, compatibility reader, new production helper, manifest or CI changes were found.

## Verdict

**CONDITIONAL for this partial diff.** Resolve the listed test defects. This is not QSL-2043 completion or repository assurance approval.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The negative test requires seven authoritative parts-saga programs to remain ill_typed. Correcting their attempt reads to Boolean under FR-218-AC-44 makes this new test fail despite correct FSL behavior. Replace this legacy-source dependency with an original owned mismatched-result negative and retain canonical journeys as positive completion work. | tests/it/protocol_source_routes.rs:614 |
| FND-002 | medium | The local-order test compares an edge with the returned events array and checks only correspondence cardinality. Reversing the global-to-local mappings, or mapping both global events to one local event, still passes while violating induced source order/total one-to-one correspondence. Resolve Sent/Next and Seen/Delivered checked nodes, then assert their actual local mappings and directed edge; also verify peer/message identity for the send/receive pair. | tests/it/protocol_source_routes.rs:316 |

## Coverage

All 68 changed paths examined, including all source, request and expectation records and 13 route tests. The existing source_corpus runner is context only, unchanged. Its directory traversal excludes original/ and staged/; original files run through explicit route tests; staged await retains intended S4 success and is not counted as passing coverage. Compensation compile admission does not establish a successful strict handoff or journey. Await selection and compensation handoff blockers remain under QSL-2225; intended canonical parts-saga journeys remain incomplete.

Accepted reported evidence without repeating: 19 standalone corpus programs passed and 13 route tests passed across qsl-2043-{changed,original,focused-2,deadlock-2,projection-order-2,corpus-4}.log. No tests, builds, computed matrix or full gate run by this reviewer. Lead owns the final applicable gate. Frozen head 295007777ec0ae110fe08ad6963f4f7aba741fa2.

Saved matrix at /home/peter/dev/.agent/fsl-matrix.json and summary is accepted as baseline: 5808 total, 4323 tagged, 308 ignored-test-only, 1074 untagged, 103 method-without-symbol. No new repository-wide matrix claim is made. Reverse trace review is confined to new test behavior; production code is unchanged. All new route tests execute real command/analysis paths; no ignored test was added. Canonical model and requests are read from explicit FSL_EXAMPLES_DIR, without copying; a missing root fails. Unsafe/concurrency and production API changes are absent. Test-only unwrap/panic follows established fixture style.

No applicable AssuranceProfile selected in the established repository context. No repository-wide profile inventory or optional broader semantic audit performed. Requested assertion/oracle review is included within the diff.

Plan completion: not assessed

## Examined Scope

```json
{
  "scope": [
    {
      "id": "tests/corpus/protocol/await-unknown-profile.expected.json",
      "path": "tests/corpus/protocol/await-unknown-profile.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"missing_declaration\",\"cause\":\"missing-name\"}\n"
    },
    {
      "id": "tests/corpus/protocol/await-unknown-profile.native",
      "path": "tests/corpus/protocol/await-unknown-profile.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile t = \"filament.temporal.timestamped-event.finite-window/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premis"
    },
    {
      "id": "tests/corpus/protocol/await-unknown-profile.request.json",
      "path": "tests/corpus/protocol/await-unknown-profile.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"await-unknown-profile.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[],\"clocks\":{\"timer\":{\"kind\":\"timestamped_event\",\"timestamp_unit\":\"s\"}}}}\n"
    },
    {
      "id": "tests/corpus/protocol/choice-model-read.expected.json",
      "path": "tests/corpus/protocol/choice-model-read.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"ill_typed\",\"cause\":\"operator-ineligible\"}\n"
    },
    {
      "id": "tests/corpus/protocol/choice-model-read.native",
      "path": "tests/corpus/protocol/choice-model-read.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/choice-model-read.request.json",
      "path": "tests/corpus/protocol/choice-model-read.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"choice-model-read.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}}}}\n"
    },
    {
      "id": "tests/corpus/protocol/choice-overlap.expected.json",
      "path": "tests/corpus/protocol/choice-overlap.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"undefined_expression\",\"cause\":\"unproved-exhaustiveness\"}\n"
    },
    {
      "id": "tests/corpus/protocol/choice-overlap.native",
      "path": "tests/corpus/protocol/choice-overlap.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/choice-overlap.request.json",
      "path": "tests/corpus/protocol/choice-overlap.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"choice-overlap.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/choice-visibility.expected.json",
      "path": "tests/corpus/protocol/choice-visibility.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"wrong_snapshot\",\"cause\":\"wrong-observation\"}\n"
    },
    {
      "id": "tests/corpus/protocol/choice-visibility.native",
      "path": "tests/corpus/protocol/choice-visibility.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/choice-visibility.request.json",
      "path": "tests/corpus/protocol/choice-visibility.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"choice-visibility.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/choice.expected.json",
      "path": "tests/corpus/protocol/choice.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/choice.native",
      "path": "tests/corpus/protocol/choice.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/choice.request.json",
      "path": "tests/corpus/protocol/choice.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"choice.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/duplicate-protocol.expected.json",
      "path": "tests/corpus/protocol/duplicate-protocol.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"ambiguous_declaration\",\"cause\":\"ambiguous-name\"}\n"
    },
    {
      "id": "tests/corpus/protocol/duplicate-protocol.native",
      "path": "tests/corpus/protocol/duplicate-protocol.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/duplicate-protocol.request.json",
      "path": "tests/corpus/protocol/duplicate-protocol.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"duplicate-protocol.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}}}}\n"
    },
    {
      "id": "tests/corpus/protocol/duplicate-role.expected.json",
      "path": "tests/corpus/protocol/duplicate-role.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"ambiguous_declaration\",\"cause\":\"ambiguous-name\"}\n"
    },
    {
      "id": "tests/corpus/protocol/duplicate-role.native",
      "path": "tests/corpus/protocol/duplicate-role.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine; role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequenc"
    },
    {
      "id": "tests/corpus/protocol/duplicate-role.request.json",
      "path": "tests/corpus/protocol/duplicate-role.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"duplicate-role.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/effect-of-send.expected.json",
      "path": "tests/corpus/protocol/effect-of-send.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"ill_typed\",\"cause\":\"type-mismatch\"}\n"
    },
    {
      "id": "tests/corpus/protocol/effect-of-send.native",
      "path": "tests/corpus/protocol/effect-of-send.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/effect-of-send.request.json",
      "path": "tests/corpus/protocol/effect-of-send.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"effect-of-send.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/exchange.expected.json",
      "path": "tests/corpus/protocol/exchange.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/exchange.native",
      "path": "tests/corpus/protocol/exchange.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/exchange.request.json",
      "path": "tests/corpus/protocol/exchange.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"exchange.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/missing-finish.expected.json",
      "path": "tests/corpus/protocol/missing-finish.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S1\",\"code\":\"invalid_syntax\",\"cause\":\"unexpected-token\"}\n"
    },
    {
      "id": "tests/corpus/protocol/missing-finish.native",
      "path": "tests/corpus/protocol/missing-finish.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/missing-finish.request.json",
      "path": "tests/corpus/protocol/missing-finish.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"models\":[],\"program\":{\"source\":{\"file\":\"missing-finish.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}}}}\n"
    },
    {
      "id": "tests/corpus/protocol/object-message.expected.json",
      "path": "tests/corpus/protocol/object-message.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"invalid_model_binding\",\"cause\":\"malformed-declaration\"}\n"
    },
    {
      "id": "tests/corpus/protocol/object-message.native",
      "path": "tests/corpus/protocol/object-message.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Job\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via "
    },
    {
      "id": "tests/corpus/protocol/object-message.request.json",
      "path": "tests/corpus/protocol/object-message.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"object-message.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/original/commit.native",
      "path": "tests/corpus/protocol/original/commit.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nmodel Factory = \"example/parts\";\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Factory::Job) on origin {\n  role Sender on Factory::PlanningService;\n  role Receiver on Factory::MachiningStation;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via Queue as (sent: Note) { true };\n    receive Seen via Queue of Sent as (got: Note)"
    },
    {
      "id": "tests/corpus/protocol/original/compensation.native",
      "path": "tests/corpus/protocol/original/compensation.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nprofile t = \"filament.temporal.timestamped-event.finite-window/v1\";\nmodel Factory = \"example/parts\";\nrecord Completion { ok: Boolean; }\nrecord Trigger { fire: Boolean; }\nprotocol Recovery using p over (job: Factory::Job) on origin {\n  role Worker on Factory::MachiningStation;\n  compensate Undo for Main::Prepared as (forward: Completion)\n    by Worker on Factory::MachiningStation::release using t clock \"timer\" {\n    activate first (trigger: Trigger) when { trigger."
    },
    {
      "id": "tests/corpus/protocol/original/exchange.native",
      "path": "tests/corpus/protocol/original/exchange.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nmodel Factory = \"example/parts\";\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Factory::Job) on origin {\n  role Sender on Factory::PlanningService;\n  role Receiver on Factory::MachiningStation;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via Queue as (sent: Note) { true };\n    receive Seen via Queue of Sent as (got: Note)"
    },
    {
      "id": "tests/corpus/protocol/original/hidden-choice.native",
      "path": "tests/corpus/protocol/original/hidden-choice.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nmodel Factory = \"example/parts\";\nrecord Note { ok: Boolean; }\nprotocol HiddenChoice using p over (job: Factory::Job) on origin {\n  role Source on Factory::PlanningService;\n  role Picker on Factory::MachiningStation;\n  role Target on Factory::AssemblyStation;\n  channel Decision from Source to Picker carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  channel Notice from Picker to Target carries Note\n    ordering unordered "
    },
    {
      "id": "tests/corpus/protocol/original/parallel-any.native",
      "path": "tests/corpus/protocol/original/parallel-any.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nmodel Factory = \"example/parts\";\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Factory::Job) on origin {\n  role Sender on Factory::PlanningService;\n  role Receiver on Factory::MachiningStation;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\nrun parallel Both { branch A sequence First { commit One by Sender as (one: Boolean) { true }; } branch B sequence Second"
    },
    {
      "id": "tests/corpus/protocol/original/repeat.native",
      "path": "tests/corpus/protocol/original/repeat.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nmodel Factory = \"example/parts\";\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Factory::Job) on origin {\n  role Sender on Factory::PlanningService;\n  role Receiver on Factory::MachiningStation;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via Queue as (sent: Note) { true };\n    receive Seen via Queue of Sent as (got: Note)"
    },
    {
      "id": "tests/corpus/protocol/parallel-all.expected.json",
      "path": "tests/corpus/protocol/parallel-all.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/parallel-all.native",
      "path": "tests/corpus/protocol/parallel-all.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\nrun parallel Both { branch A sequence F"
    },
    {
      "id": "tests/corpus/protocol/parallel-all.request.json",
      "path": "tests/corpus/protocol/parallel-all.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"parallel-all.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/parallel-any.expected.json",
      "path": "tests/corpus/protocol/parallel-any.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/parallel-any.native",
      "path": "tests/corpus/protocol/parallel-any.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\nrun parallel Both { branch A sequence F"
    },
    {
      "id": "tests/corpus/protocol/parallel-any.request.json",
      "path": "tests/corpus/protocol/parallel-any.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"parallel-any.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/parallel-missing-branch.expected.json",
      "path": "tests/corpus/protocol/parallel-missing-branch.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"invalid_package\",\"cause\":\"invalid-value\"}\n"
    },
    {
      "id": "tests/corpus/protocol/parallel-missing-branch.native",
      "path": "tests/corpus/protocol/parallel-missing-branch.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\nrun parallel Both { branch A sequence F"
    },
    {
      "id": "tests/corpus/protocol/parallel-missing-branch.request.json",
      "path": "tests/corpus/protocol/parallel-missing-branch.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"parallel-missing-branch.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/record-role.expected.json",
      "path": "tests/corpus/protocol/record-role.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":20,\"category\":\"refusal\",\"last_stage\":\"S3\",\"code\":\"invalid_model_binding\",\"cause\":\"malformed-declaration\"}\n"
    },
    {
      "id": "tests/corpus/protocol/record-role.native",
      "path": "tests/corpus/protocol/record-role.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Note;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via Qu"
    },
    {
      "id": "tests/corpus/protocol/record-role.request.json",
      "path": "tests/corpus/protocol/record-role.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"record-role.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/repeat-no-progress.expected.json",
      "path": "tests/corpus/protocol/repeat-no-progress.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":21,\"category\":\"unsupported\",\"last_stage\":\"S3\",\"code\":\"unsupported_construct\",\"cause\":\"declaration-form\"}\n"
    },
    {
      "id": "tests/corpus/protocol/repeat-no-progress.native",
      "path": "tests/corpus/protocol/repeat-no-progress.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/repeat-no-progress.request.json",
      "path": "tests/corpus/protocol/repeat-no-progress.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"repeat-no-progress.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/repeat-zero.expected.json",
      "path": "tests/corpus/protocol/repeat-zero.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/repeat-zero.native",
      "path": "tests/corpus/protocol/repeat-zero.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity 1 overflow block;\n  run sequence Main {\n    send Sent via"
    },
    {
      "id": "tests/corpus/protocol/repeat-zero.request.json",
      "path": "tests/corpus/protocol/repeat-zero.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"repeat-zero.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/source-objects.expected.json",
      "path": "tests/corpus/protocol/source-objects.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/source-objects.native",
      "path": "tests/corpus/protocol/source-objects.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Worker { alive: Boolean; population workers; }\nprotocol Empty using p over (job: Job) on origin {\n  role Agent on Worker;\n  run sequence Start {}\n  finish Done as (done: Boolean) { done };\n}\n"
    },
    {
      "id": "tests/corpus/protocol/source-objects.request.json",
      "path": "tests/corpus/protocol/source-objects.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"source-objects.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/corpus/protocol/staged/await.expected.json",
      "path": "tests/corpus/protocol/staged/await.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/staged/await.native",
      "path": "tests/corpus/protocol/staged/await.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile t = \"filament.temporal.timestamped-event.finite-window/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premis"
    },
    {
      "id": "tests/corpus/protocol/staged/await.request.json",
      "path": "tests/corpus/protocol/staged/await.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"await.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[],\"clocks\":{\"timer\":{\"kind\":\"timestamped_event\",\"timestamp_unit\":\"s\"}}}}\n"
    },
    {
      "id": "tests/corpus/protocol/symbolic-capacity.expected.json",
      "path": "tests/corpus/protocol/symbolic-capacity.expected.json",
      "role": "examined",
      "excerpt": "{\"verb\":\"compile\",\"exit_code\":0,\"category\":\"success\",\"last_stage\":\"S4\"}\n"
    },
    {
      "id": "tests/corpus/protocol/symbolic-capacity.native",
      "path": "tests/corpus/protocol/symbolic-capacity.native",
      "role": "examined",
      "excerpt": "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.model.complete/v1\";\nprofile p = \"filament.protocol.finite-global/v1\";\nobject Job { ready: Boolean; population jobs; }\nobject Planner { alive: Boolean; population planners; }\nobject Machine { busy: Boolean; population machines; }\nrecord Note { ok: Boolean; }\nprotocol Exchange using p over (job: Job) on origin {\n  role Sender on Planner;\n  role Receiver on Machine;\n  channel Queue from Sender to Receiver carries Note\n    ordering unordered delivery exactly-once-premise capacity symbolic(QueueSlots) overflow block;\n  run sequence Main "
    },
    {
      "id": "tests/corpus/protocol/symbolic-capacity.request.json",
      "path": "tests/corpus/protocol/symbolic-capacity.request.json",
      "role": "examined",
      "excerpt": "{\"format\":\"native-compile/1\",\"request\":{\"program\":{\"source\":{\"file\":\"symbolic-capacity.native\",\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\"document\":\"ProtocolCorpus\",\"formal_revision\":1}},\"models\":[]}}\n"
    },
    {
      "id": "tests/it/main.rs",
      "path": "tests/it/main.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! Single integration-test binary. Every module below was\n//! formerly its own `tests/<name>.rs` cargo test target, statically linking\n//! the whole `filament-spec-language` crate plus its dependencies on its own\n//! (measured, #306: 426 test executables across 109 targets took 106.6 GiB\n//! in one worktree's target dir, against 2.7 GiB for every compiled\n//! library). Each is now a `mod` of this one `it` target instead, so\n//! `cargo test` pays one link step, not ~105 of them.\n//!\n//! Shared helpers live in `tests/support/` and are reached as\n//!"
    },
    {
      "id": "tests/it/protocol_source_routes.rs",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! Original protocol source programs and authoritative parts-saga journeys.\n\nuse filament_exact::Cancel;\nuse fsl_command::command;\nuse ix_trace_rs::trace;\nuse serde_json::{json, Value};\nuse std::path::{Path, PathBuf};\n\nfn saga() -> PathBuf {\n    let root = std::env::var_os(\"FSL_EXAMPLES_DIR\")\n        .map(PathBuf::from)\n        .expect(\"FSL_EXAMPLES_DIR must point to the authoritative filament-examples checkout\");\n    root.join(\"examples/parts-saga\")\n}\n\nfn corpus(file: &str) -> PathBuf {\n    Path::new(env!(\"CARGO_MANIFEST_DIR\"))\n        .join(\"tes"
    },
    {
      "id": "saga",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn saga() -> PathBuf {\n    let root = std::env::var_os(\"FSL_EXAMPLES_DIR\")\n        .map(PathBuf::from)\n        .expect(\"FSL_EXAMPLES_DIR must point to the authoritative filament-examples checkout\");\n    root.join(\"examples/parts-saga\")\n}"
    },
    {
      "id": "corpus",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn corpus(file: &str) -> PathBuf {\n    Path::new(env!(\"CARGO_MANIFEST_DIR\"))\n        .join(\"tests/corpus/protocol\")\n        .join(file)\n}"
    },
    {
      "id": "source",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn source(file: &Path) -> Value {\n    json!({\"file\":file,\"authority\":\"test\",\"identity\":\"test:protocol-corpus\",\n        \"document\":\"ProtocolCorpus\",\"formal_revision\":1})\n}"
    },
    {
      "id": "compile_request",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn compile_request(file: &Path) -> Value {\n    let text = std::fs::read_to_string(file).unwrap();\n    let clocks = [\"timer\", \"line-clock\"]\n        .into_iter()\n        .filter(|name| text.contains(&format!(\"clock \\\"{name}\\\"\")))\n        .map(|name| {\n            (\n                name.to_owned(),\n                json!({\"kind\":\"timestamped_event\",\"timestamp_unit\":\"s\"}),\n            )\n        })\n        .collect::<serde_json::Map<_, _>>();\n    json!({\"format\":\"native-compile/1\",\"request\":{\n        \"program\":{\"source\":source(file)},\"clocks\":clocks,\n        \"models\":[{\"format\":\"semantic-ir/2.0.0\",\n"
    },
    {
      "id": "write",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn write(directory: &Path, request: &Value) -> PathBuf {\n    let path = directory.join(\"request.json\");\n    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();\n    path\n}"
    },
    {
      "id": "compile",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn compile(directory: &Path, file: &Path) {\n    let output = command::compile_outcome(&write(directory, &compile_request(file)))\n        .unwrap_or_else(|error| panic!(\"{}: {error}\", file.display()));\n    assert!(!output.package_bytes().is_empty(), \"{}\", file.display());\n}"
    },
    {
      "id": "refusal",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn refusal(directory: &Path, file: &Path, code: &str, cause: &str) {\n    let error = match command::compile_outcome(&write(directory, &compile_request(file))) {\n        Ok(_) => panic!(\"{} must refuse {code}/{cause}\", file.display()),\n        Err(error) => error,\n    };\n    let artifact = error.artifact_value().unwrap();\n    assert_eq!(artifact[\"code\"], code, \"{}: {artifact}\", file.display());\n    assert_eq!(artifact[\"cause\"], cause, \"{}: {artifact}\", file.display());\n}"
    },
    {
      "id": "original_protocol_declarations_communications_and_refusals",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn original_protocol_declarations_communications_and_refusals() {\n    let directory = tempfile::tempdir().unwrap();\n    let original = std::fs::read_to_string(corpus(\"original/exchange.native\")).unwrap();\n    compile(directory.path(), &corpus(\"original/exchange.native\"));\n    // Each mutation retains the same accepted model, subject and surrounding source.\n    for (name, from, to, code, cause) in [\n        (\"duplicate-role\", \"role Receiver on Factory::MachiningStation;\",\n         \"role Receiver on Factory::MachiningStation; role Receiver on Factory::MachiningStation;\",\n         \"ambiguous_decl"
    },
    {
      "id": "authoritative_parts_saga_protocol_constructs_compile_from_their_own_repository",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn authoritative_parts_saga_protocol_constructs_compile_from_their_own_repository() {\n    let directory = tempfile::tempdir().unwrap();\n    for file in [\n        \"protocols/declaration/declaration-minimal.fsl\",\n        \"protocols/declaration/declaration-on-each.fsl\",\n        \"protocols/declaration/declaration-terminal-scheduling.fsl\",\n        \"protocols/roles-and-channels/roles-replicated.fsl\",\n        \"protocols/roles-and-channels/channels-loss-head.fsl\",\n        \"protocols/roles-and-channels/channels-fifo-by-part.fsl\",\n    ] {\n        compile(directory.path(), &saga().join(\"rules\").join(file"
    },
    {
      "id": "authoritative_parts_saga_rejects_invalid_protocols_at_the_right_boundary",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn authoritative_parts_saga_rejects_invalid_protocols_at_the_right_boundary() {\n    let directory = tempfile::tempdir().unwrap();\n    for (file, code, cause) in [\n        (\n            \"protocols/declaration/declaration-no-finish.fsl\",\n            \"invalid_syntax\",\n            \"unexpected-token\",\n        ),\n        (\n            \"protocols/roles-and-channels/channels-object-payload.fsl\",\n            \"invalid_model_binding\",\n            \"malformed-declaration\",\n        ),\n        (\n            \"protocols/steps/steps-effect-of-send.fsl\",\n            \"ill_typed\",\n            \"type-mismatch\",\n    "
    },
    {
      "id": "authoritative_request",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn authoritative_request(path: &Path) -> Value {\n    let mut request: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();\n    let base = path.parent().unwrap();\n    let program = &mut request[\"request\"][\"program\"][\"source\"][\"file\"];\n    *program = json!(base.join(program.as_str().unwrap()));\n    for model in request[\"request\"][\"models\"].as_array_mut().unwrap() {\n        let file = &mut model[\"source\"][\"file\"];\n        *file = json!(base.join(file.as_str().unwrap()));\n    }\n    for file in request[\"request\"][\"clause\"][\"observations\"]\n        .as_object_mut()\n        .unwrap("
    },
    {
      "id": "parts_saga_observations_distinguish_tracked_untracked_and_third_rework",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn parts_saga_observations_distinguish_tracked_untracked_and_third_rework() {\n    let directory = tempfile::tempdir().unwrap();\n    for (name, exit, result) in [\n        (\"jobs-rework-tracked\", 0, \"tested\"),\n        (\"jobs-rework-untracked\", 10, \"refuted\"),\n        (\"jobs-third-rework\", 10, \"refuted\"),\n    ] {\n        let request = authoritative_request(&saga().join(format!(\"requests/{name}.json\")));\n        let output = command::run(\n            &write(directory.path(), &request),\n            &crate::negotiation::Fr290,\n        )\n        .unwrap_or_else(|error| panic!(\"{name}: {error}\"));\n   "
    },
    {
      "id": "assessed_root",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn assessed_root() -> fsl_replay::protocol_analysis::AssessedRootDraft {\n    use filament_exact::{\n        EffectiveId, NodeKey, ObjectId, ObjectReference, OriginalDeclarationKey, PathSegment,\n        UniverseId,\n    };\n    fsl_replay::protocol_analysis::AssessedRootDraft {\n        observation: Some(fsl_replay::ObservationIdentity {\n            authority: \"example\".into(),\n            identity: \"ix://example/parts/data/saga-start\".into(),\n        }),\n        object: Some(ObjectReference::new(\n            UniverseId::new(\n                NodeKey::new(\n                    \"example/parts\",\n      "
    },
    {
      "id": "handoff",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn handoff(directory: &Path, file: &Path) -> fsl_command::checked_handoff::CompiledHandoff {\n    let mut request = compile_request(file);\n    request[\"request\"][\"handoff\"] = json!({\"directory\":\"handoff\",\"package_name\":\"protocol-corpus\",\n        \"native_join\":{\"work_units\":1_u64<<28,\"storage\":1_u64<<24}});\n    command::compile_outcome(&write(directory, &request))\n        .expect(\"compile and emit the source handoff\");\n    fsl_command::checked_handoff::compile(&directory.join(\"handoff\"))\n        .expect(\"read and compile the real handoff\")\n}"
    },
    {
      "id": "project",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn project(\n    handoff: &fsl_command::checked_handoff::CompiledHandoff,\n    role: &str,\n) -> fsl_analyze::protocol_analysis::ProjectionOutcome {\n    use fsl_replay::protocol_analysis::{ProjectionBoundsDraft, RunBudget};\n    let package = handoff.admitted();\n    let protocol = package\n        .resolved_graph()\n        .nodes\n        .iter()\n        .find(|node| &*node.semantic_form == \"protocol_clause\")\n        .expect(\"source protocol\");\n    let role = package\n        .resolved_graph()\n        .nodes\n        .iter()\n        .find(|node| {\n            node.declaration.as_ref().is_some_and(|dec"
    },
    {
      "id": "source_projection_preserves_send_receive_direction_and_local_order",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn source_projection_preserves_send_receive_direction_and_local_order() {\n    use fsl_replay::protocol_analysis::{AnalysisOutcome, LocalEventKind};\n    let directory = tempfile::tempdir().unwrap();\n    let original = std::fs::read_to_string(corpus(\"original/exchange.native\")).unwrap();\n    let file = directory.path().join(\"two-exchanges.native\");\n    std::fs::write(\n        &file,\n        original.replace(\n            \"receive Seen via Queue of Sent as (got: Note) { true };\",\n            \"receive Seen via Queue of Sent as (got: Note) { true };\\n\\\n         send Next via Queue as (next: Note) { "
    },
    {
      "id": "analysis_request",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn analysis_request(file: &Path) -> Value {\n    let mut request = compile_request(file);\n    request[\"request\"].as_object_mut().unwrap().remove(\"clocks\");\n    request[\"format\"] = json!(\"native-analyze/1\");\n    request[\"request\"][\"documents\"] = json!([{\"file\":saga().join(\"data/saga-start.json\")}]);\n    let binding = json!({\"type\":\"model-binding\",\"selection\":\"example/parts\"});\n    request[\"request\"][\"subject\"] = json!({\"protocol\":{\"protocol\":\"Exchange\",\n        \"over\":{\"reference\":{\"population\":\"ix://example/parts/jobs\",\"key\":\"j1\"}},\n        \"roles\":{\"Sender\":binding,\"Receiver\":binding}}});\n    "
    },
    {
      "id": "public_source_analysis_distinguishes_consumed_and_stranded_messages",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn public_source_analysis_distinguishes_consumed_and_stranded_messages() {\n    let directory = tempfile::tempdir().unwrap();\n    let original = std::fs::read_to_string(corpus(\"original/exchange.native\")).unwrap();\n    let stranded = directory.path().join(\"stranded.native\");\n    std::fs::write(\n        &stranded,\n        original.replace(\n            \"receive Seen via Queue of Sent as (got: Note) { true };\",\n            \"send Second via Queue as (second: Note) { true };\",\n        ),\n    )\n    .unwrap();\n    for (file, expected) in [\n        (corpus(\"original/exchange.native\"), \"proved\"),\n      "
    },
    {
      "id": "original_choice_parallel_and_repeat_have_positive_negative_and_zero_bound_controls",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn original_choice_parallel_and_repeat_have_positive_negative_and_zero_bound_controls() {\n    let directory = tempfile::tempdir().unwrap();\n    let original = std::fs::read_to_string(corpus(\"original/exchange.native\")).unwrap();\n    let marker = \"receive Seen via Queue of Sent as (got: Note) { true };\";\n    for (name, control) in [\n        (\"exclusive-choice\", \"choice Decision by Receiver visible (got.ok) { case yes when { got.ok } sequence Yes {} case no when { not got.ok } sequence No {} }\"),\n        (\"zero-repeat\", \"repeat Again by Receiver visible (got.ok) max 0 while { got.ok } sequence C"
    },
    {
      "id": "source_projection_names_each_construct_outside_bounded_static",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn source_projection_names_each_construct_outside_bounded_static() {\n    use fsl_replay::protocol_analysis::AnalysisOutcome;\n    for (file, role, slug) in [\n        (\n            saga().join(\"rules/protocols/declaration/declaration-on-each.fsl\"),\n            \"Planning\",\n            \"activation-on-each\",\n        ),\n        (\n            saga().join(\"rules/protocols/roles-and-channels/roles-replicated.fsl\"),\n            \"Worker\",\n            \"dynamic-role-population\",\n        ),\n        (\n            corpus(\"original/parallel-any.native\"),\n            \"Sender\",\n            \"unsupported-join\",\n  "
    },
    {
      "id": "source_projection_refutes_a_choice_hidden_from_the_target",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn source_projection_refutes_a_choice_hidden_from_the_target() {\n    use fsl_replay::protocol_analysis::AnalysisOutcome;\n    let directory = tempfile::tempdir().unwrap();\n    let package = handoff(directory.path(), &corpus(\"original/hidden-choice.native\"));\n    let target = project(&package, \"Target\");\n    assert!(\n        matches!(target.outcome, AnalysisOutcome::Refuted(_)),\n        \"{target:?}\"\n    );\n    let owner = project(&package, \"Picker\");\n    let AnalysisOutcome::Proved(witness) = owner.outcome else {\n        panic!(\"the deciding role knows its own choice: {owner:?}\");\n    };\n    ass"
    },
    {
      "id": "root_wire",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn root_wire() -> Value {\n    use filament_exact::{NodeKey, PathSegment};\n    let universe = NodeKey::new(\n        \"example/parts\",\n        vec![\n            PathSegment::Text(\"model\".into()),\n            PathSegment::Text(\"object_type\".into()),\n            PathSegment::Text(\"ix://example/parts/Job\".into()),\n        ],\n        0,\n    )\n    .unwrap();\n    json!({\"observation\":{\"authority\":\"example\",\"identity\":\"ix://example/parts/data/saga-start\"},\n        \"subject\":{\"tag\":\"object\",\"universe\":fsl_foundation::node_wire::node_key_to_text(&universe).unwrap(),\n            \"type\":serde_json::to_strin"
    },
    {
      "id": "public_source_projection_realizability_and_deadlock_agree_on_bounded_static_programs",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn public_source_projection_realizability_and_deadlock_agree_on_bounded_static_programs() {\n    let directory = tempfile::tempdir().unwrap();\n    for (file, protocol, roles) in [\n        (\n            corpus(\"original/exchange.native\"),\n            \"Exchange\",\n            vec![\"Sender\", \"Receiver\"],\n        ),\n        (\n            saga().join(\"rules/protocols/declaration/declaration-minimal.fsl\"),\n            \"PartFlow\",\n            vec![\"Planning\"],\n        ),\n    ] {\n        let mut request = analysis_request(&file);\n        let bindings = roles\n            .iter()\n            .map(|role| {"
    },
    {
      "id": "original_compensation_has_an_accepting_template_and_zero_attempt_refusal",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn original_compensation_has_an_accepting_template_and_zero_attempt_refusal() {\n    let directory = tempfile::tempdir().unwrap();\n    let file = corpus(\"original/compensation.native\");\n    compile(directory.path(), &file);\n    let source = std::fs::read_to_string(&file).unwrap();\n    let zero = directory.path().join(\"zero-attempts.native\");\n    std::fs::write(\n        &zero,\n        source.replace(\"attempts 1 of Completion\", \"attempts 0 of Completion\"),\n    )\n    .unwrap();\n    refusal(directory.path(), &zero, \"ill_typed\", \"type-mismatch\");\n}"
    },
    {
      "id": "original_symbolic_capacity_is_admitted_and_projection_names_the_capacity",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn original_symbolic_capacity_is_admitted_and_projection_names_the_capacity() {\n    use fsl_replay::protocol_analysis::AnalysisOutcome;\n    let directory = tempfile::tempdir().unwrap();\n    let source = std::fs::read_to_string(corpus(\"original/exchange.native\")).unwrap();\n    let file = directory.path().join(\"symbolic.native\");\n    std::fs::write(\n        &file,\n        source.replace(\"capacity 1\", \"capacity symbolic(QueueSlots)\"),\n    )\n    .unwrap();\n    let package = handoff(directory.path(), &file);\n    let result = project(&package, \"Receiver\");\n    let AnalysisOutcome::Unsupported(cause)"
    },
    {
      "id": "parts_saga_attempt_views_cannot_be_read_as_boolean_operation_results",
      "path": "tests/it/protocol_source_routes.rs",
      "role": "examined",
      "excerpt": "fn parts_saga_attempt_views_cannot_be_read_as_boolean_operation_results() {\n    let directory = tempfile::tempdir().unwrap();\n    for file in [\n        \"part-flow-untimed.fsl\",\n        \"part-flow.fsl\",\n        \"inspection.fsl\",\n        \"protocols/steps/steps-all-kinds.fsl\",\n        \"protocols/control-flow/control-parallel-all.fsl\",\n        \"protocols/control-flow/control-parallel-any-cancel.fsl\",\n        \"protocols/control-flow/control-repeat-progress.fsl\",\n    ] {\n        refusal(\n            directory.path(),\n            &saga().join(\"rules\").join(file),\n            \"ill_typed\",\n            "
    },
    {
      "id": "FR-218",
      "path": "spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md",
      "role": "context_only",
      "excerpt": "When S3 checks a protocol declaration, the `ProtocolClause` family SHALL\ncheck `parallel` with its branches and its join policy and `outstanding`\nrule, `choice`, `repeat`, `await`, `check`, `send`, `receive`, `effect`,\n`event` and `commit` nodes, channels with their premises, relationship\ndeclarations and `related by` clauses, `requires temporal` requirements,\nreplicated roles, the owner and visible facts of each `choice` and `repeat`,\nand node bodies, and SHALL produce\nan in-process `CheckedProtocolClause` holding the control tree, the\ncompiled causal edge relation and the scopes the protocol"
    },
    {
      "id": "FR-218-AC-2",
      "path": "spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md",
      "role": "examined",
      "excerpt": "A `choice` guard that reads `k.v` refuses `ill_typed`/`operator-ineligible` at the read. A two-case `choice` whose `visible` entry `m.n` is an individual non-Boolean fact, admitted as a decision fact, and whose guards are `m.n > 0` and `m.n > 1`, refuses `undefined_expression`/`unproved-exhaustiveness`. `join all [left, middle]` in `Fill` refuses `missing_declaration`/`missing-name` at `middle`."
    },
    {
      "id": "FR-218-AC-5",
      "path": "spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md",
      "role": "examined",
      "excerpt": "A `repeat` whose body is one attempt checks with a maximum, with an invariant and variant and no maximum, with both, and with neither, and the checked clause records the maximum, invariant and variant each has. A `repeat` whose body is only a `check` refuses `unsupported_construct`/`declaration-form` at the `repeat`, naming it, and so does a `repeat` whose body is a `choice` with one attempt in one case and an empty `sequence` in the other."
    },
    {
      "id": "FR-218-AC-44",
      "path": "spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md",
      "role": "examined",
      "excerpt": "Over the Cell fixture, whose `flag` returns `Boolean` and whose `touch` returns nothing: `attempt T by w on M::Cell::flag contracts [FlagPre, FlagPost] as (t: Boolean) { true }` followed by `check Gate using v { t }` checks and the check reads the result; the same attempt with `as (t: Integer)` and no read of `t`, and the `touch` attempt with `as (t: Boolean)` and no read, each check; with `as (t: Integer)` and `check Gate using v { t = 1 }`, and with the `touch` attempt and `check Gate using v { t }`, each unit refuses exactly once, `ill_typed`/`type-mismatch` at the read `t` inside the check"
    },
    {
      "id": "FR-219",
      "path": "spec/functional/FR-219-declare-and-select-the-memory-model-of-a-parallel.md",
      "role": "context_only",
      "excerpt": "S3 SHALL check a `parallel` control's optional `memory M` clause, `M` one of\n`sc`, `tso` and `ra`, with default `sc` when absent, and SHALL refuse a\nnested `parallel` whose default differs from its outermost `parallel`'s and\na weak default on a `parallel` whose join is not `all` with no `outstanding`\nclause (ADR-025 MM-1, MM-3). The model checker SHALL resolve each outermost\n`parallel`'s model from the request's `memory` member when it names that\n`parallel`, else from its source default, and SHALL apply it to every\n`parallel` nested inside (ADR-025 MM-2, MM-3). The resolved models SHALL\nenter "
    },
    {
      "id": "FR-220",
      "path": "spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md",
      "role": "context_only",
      "excerpt": "S3 SHALL check an attempt's optional `ordering o` (`relaxed`, `acquire`,\n`release`, `acq_rel`, `seq_cst`) and the `fence Name ordering o;` control\n(`acquire`, `release`, `acq_rel`, `seq_cst`) (ADR-025 MM-6); the shared\ngrammar's `fence-order` does not admit `relaxed`, so S1 refuses a fence\nwritten with it. S3 SHALL record each attempt's ordering and mark the attempt atomic when it has an\nordering and non-atomic otherwise, and SHALL refuse an ordering that no access\nkind the attempt's operation leaves possible takes (ADR-025 MA-3). EN-1 SHALL\nclassify each instantiated attempt as a load, store,"
    },
    {
      "id": "FR-224",
      "path": "spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md",
      "role": "context_only",
      "excerpt": "Under a weak model, `Tso` and `Ra` SHALL explore a non-atomic access and\ncarry its race summary as FSpec FR-438 states (ADR-025 DR-1, DR-3). The\nrequest writer SHALL add one derived **race-freedom item**,\n`always holds(not raced)`, for each distinct subject among a request's\nitems whose resolved model is weak and that has a non-atomic shared access\n(ADR-025 DR-4). EN-1 SHALL decide it and report a violation as a\ncounterexample of kind `Race{location, earlier, later}` (ADR-025 DR-5)."
    },
    {
      "id": "FR-225",
      "path": "spec/functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md",
      "role": "context_only",
      "excerpt": "`ModelCheckRequest` SHALL carry `max_store_buffer` and `max_messages` (FR-126)\nbeside `max_depth`\nas method bounds, like the `max_depth` search horizon: bounds on the\nexplored state space that every result states, not resource limits. The\nrequest sets them with no ceiling, and their published default is 4\n(ADR-025 MB-2). The model checker SHALL apply them and settle under them as\nFSpec FR-439 states (ADR-025 MB-3, MB-4), and every terminal record of an\nitem over a subject with a weak `parallel` SHALL state the bound used and\nwhether it was reached.\n\n### Decision: `max_messages` also bounds eac"
    },
    {
      "id": "FR-226",
      "path": "spec/functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md",
      "role": "context_only",
      "excerpt": "A proof under resolved model `M` transfers to concurrent code only under the\npreconditions MX-1 to MX-4 below, which FSL states as preconditions of the\ncode claim (ADR-025 §8). For an item resolved to `ra`, EN-1's pre-check SHALL\nwarn of each load-buffering shape of its subject (ADR-025 MX-4). EN-1's\nprovider manifest SHALL advertise the\nmemory models it decides, and FSL's `route` candidate step (FR-075) SHALL\ngive an item over a subject with a weak `parallel` only candidates that\nadvertise its resolved model (ADR-025 MX-7)."
    },
    {
      "id": "FR-227",
      "path": "spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md",
      "role": "context_only",
      "excerpt": "A counterexample over a subject with a weak `parallel` SHALL be ADR-027\nPX-1's protocol counterexample whose steps include memory steps, whose `ra`\nloads and stores name their message or slot by depth, and whose every step\ncarries its post-state's memory component in canonical form; it SHALL name\nthe resolved model of each outermost `parallel` and list the memory\nconstraints with their `memory` origin (ADR-025 MK-1). Replay SHALL\nre-execute each step through the protocol system under the resolved models,\nrecompute the memory component, check a lasso against the memory\nconstraints, and replay a"
    },
    {
      "id": "FR-230",
      "path": "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md",
      "role": "context_only",
      "excerpt": "FSL's S3 checker SHALL admit the timed members of a state model (ADR-026\nCK-1 to CK-9): one `time` member naming the model's time source, `Clock`\nfields under a dense source, clock resets in postconditions, atomic clock\nconstraints in guards, time invariants and urgency declarations. It records\nthem on the checked state model so that the timed subject (FR-231), the zone\nengine (FR-239) and replay (FR-237) read one checked form. S3 confines every\nclock read to an atomic clock constraint, so no real value reaches the value\nkernel. Clock members are admitted only at package positions admitted by "
    },
    {
      "id": "FR-233",
      "path": "spec/functional/FR-233-bind-a-model-claim-to-model-time-or-model-steps.md",
      "role": "context_only",
      "excerpt": "FSL's S3 checker SHALL give every temporal claim over a model subject one\nclock binding, `model-steps` or `model-time` (ADR-026 CB-1 to CB-5). Under\n`model-steps` an interval counts steps; under `model-time` it counts time\nunits of the model's time source. A claim that writes no `clock` binds the\ndefault of its profile: `model-time` under the timed profile and\n`model-steps` under every other profile (FR-942)."
    },
    {
      "id": "FR-330",
      "path": "spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md",
      "role": "context_only",
      "excerpt": "`fsl_replay::spine::run_clause` (FR-109) SHALL accept a `Temporal` selection\nthat names one temporal clause, a supplied trace and the clause's `over`\nbinding. It compiles the unit through the spine, admits each observation of\nthe trace (FR-106), evaluates the clause at S6a through the TemporalTrace\nevaluator (FR-327, FR-328, FR-329), and returns one `ClauseRunReport`. With\nit, every admitted clause kind executes through the spine, as the reference\nruntime's exit asks (ADR-011 §5)."
    },
    {
      "id": "FR-570",
      "path": "spec/functional/FR-570-project-a-global-protocol-onto-one-role.md",
      "role": "context_only",
      "excerpt": "FSL SHALL decide the `local-projection` capability kind for a checked global\nprotocol and one role, in the `bounded-static` profile (FR-575), and return\neither the role's local graph with the evidence that it is sound or a minimal\nglobal-path counterexample. It implements ADR-036 PA-9 to PA-12 and PA-25\n(projection artifacts) and FSpec FR-174-AC-1 to AC-5. The key of the result\nis FR-572's; its settlement is FR-573's."
    },
    {
      "id": "FR-570-AC-1",
      "path": "spec/functional/FR-570-project-a-global-protocol-onto-one-role.md",
      "role": "examined",
      "excerpt": "For a projectable protocol the local graph's events and causal order equal the global event structure restricted to the role's events through the correspondence table, every local event maps to exactly one global event, and the local order is the induced order of any causal path (implements FR-174-AC-1)."
    },
    {
      "id": "FR-570-AC-2",
      "path": "spec/functional/FR-570-project-a-global-protocol-onto-one-role.md",
      "role": "examined",
      "excerpt": "Every global communication projects to exactly one local send and one local receive, the correspondence table is total over the role's global events, and the send/receive pair is matched by message type and peer (implements FR-174-AC-4)."
    },
    {
      "id": "FR-570-AC-4",
      "path": "spec/functional/FR-570-project-a-global-protocol-onto-one-role.md",
      "role": "examined",
      "excerpt": "A choice branch merges for the role only by the PA-10 rule: owner, an earlier message the role took part in that carries the whole decision, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5)."
    },
    {
      "id": "FR-570-AC-5",
      "path": "spec/functional/FR-570-project-a-global-protocol-onto-one-role.md",
      "role": "examined",
      "excerpt": "A choice the role cannot distinguish returns a counterexample naming the choice, owner, role, the two paths and the two actions, of smallest length with the PA-11 tie-break, and returns no `local_graph` and no substitute graph; for two failing choices the counterexample names the one the tie-break orders first (implements FR-174-AC-2)."
    },
    {
      "id": "FR-571",
      "path": "spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md",
      "role": "context_only",
      "excerpt": "FSL SHALL decide the `realizability` capability kind for a checked global\nprotocol in the `bounded-static` profile (FR-575) as a game in which the\nenvironment chooses admitted external events and channel outcomes and each\nrole chooses from its local history only, and return a replayable strategy\nwitness, a counterexample, or a non-conclusive result. It implements ADR-036\nPA-7, PA-8, PA-16 to PA-19 and PA-25 (realizability artifacts) and FSpec\nFR-175-AC-1 to AC-5. The key is FR-572's; settlement is FR-573's."
    },
    {
      "id": "FR-574",
      "path": "spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md",
      "role": "context_only",
      "excerpt": "Every `decisive-counterexample` result of FR-570 or FR-571 SHALL carry exactly\none FSpec FR-351 separating witness record, with the trace position of the\nstep at which the divergence or violation occurs, and its `value_path`\nderived from FSpec FR-207 as below. It implements ADR-036 PA-24 (ADR-031\nSW-5, SW-7 as amended there). filament-protocol FR-302 imports the record\nbyte-exactly and refuses a decisive realizability result without it."
    },
    {
      "id": "FR-575",
      "path": "spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md",
      "role": "context_only",
      "excerpt": "FSL SHALL admit two projection and realizability profiles. Under `bounded`\nit SHALL decide every construct a checked protocol holds (FR-890, FR-891,\nADR-036 §7). Under `bounded-static` it SHALL decide only the fragment below,\nand SHALL answer `unsupported` for every construct outside it, naming a\nlocated construct cause under the existing `unsupported_projection` code,\nwith no substitute graph and no guess. It implements ADR-036 PA-21, PA-22\nand PA-26 and FSpec FR-174-AC-3, and fixes the open item that ADR-036 left\nto the requirements: the profile words and the cause slugs.\n\n### Decision: the "
    },
    {
      "id": "FR-575-AC-1",
      "path": "spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md",
      "role": "examined",
      "excerpt": "For each construct row other than `evaluator-unsupported`, a protocol using it yields `unsupported`, `unsupported_projection` with exactly that slug and the construct's checked node id, for projection and for realizability, and returns no local graph, strategy or counterexample (implements FR-174-AC-3)."
    },
    {
      "id": "FR-575-AC-2",
      "path": "spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md",
      "role": "examined",
      "excerpt": "A construct `fsl-eval` refuses (an unenumerable step binder, FR-882) yields `evaluator-unsupported` for realizability carrying the `fsl-eval` cause, except that a refused construct that has a row above (`activation on each`, compensation) reports that row's slug. A symbolic capacity is no `fsl-eval` refusal: `fsl-eval` steps it at the protocol subject's bound value (FSpec FR-785), and `bounded-static` answers it `symbolic-queue-capacity`, bound or not."
    },
    {
      "id": "FR-575-AC-3",
      "path": "spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md",
      "role": "examined",
      "excerpt": "A request with profile `bounded-static` over a protocol using only the profile's constructs is decided, a request with profile `bounded` over the same protocol gives the same result in every member but the key's profile, and a request with any other profile word is refused `invalid_runtime_input`/`invalid-value` naming `profile` before any analysis."
    },
    {
      "id": "FR-575-AC-4",
      "path": "spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md",
      "role": "examined",
      "excerpt": "A `NegotiationCause` has exactly its three existing variants and a kind with no candidate still negotiates `unsupported_projection`/`unsupported-requested-capability`; the slugs above arise only from an engine result and add no code family."
    },
    {
      "id": "FR-575-AC-5",
      "path": "spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md",
      "role": "examined",
      "excerpt": "For each construct row other than `evaluator-unsupported`, the protocol of AC-1 under profile `bounded` yields no `unsupported` result for projection or realizability; a protocol whose binder type has a `String` field and one whose resolved memory model the system does not implement each yield `evaluator-unsupported` under `bounded`, carrying `UnenumerableRecord` and `WeakMemory`."
    },
    {
      "id": "FR-636",
      "path": "spec/functional/FR-636-request-protocol-analysis-items-and-replay-supplied-protocol-traces.md",
      "role": "context_only",
      "excerpt": "A `native-analyze/1` item naming a protocol clause SHALL run on the protocol\nanalysis engine, deciding over the protocol system (FR-205): deadlock\nfreedom against declared terminal states (FR-211), liveness and bounded\nlassos under scheduler fairness (FR-212), replicated roles and runtime role\ninstances (FR-209), identities, causal edges and delivery, per-instance\nobligations, compensation and retry or commit identities, channel capacity\nand overflow, and parallel joins. Local projection (FR-570) and\nrealizability (FR-571) are items of their own kinds. Results are keyed and\nsettled as FR-572 a"
    }
  ]
}
```
