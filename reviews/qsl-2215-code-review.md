---
id: SR-2026111612172400000037
title: "Code review — filament-spec-language protocol refusal and progress remainder"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; fsl-command/tests/it, fsl-package/src/emit/tests, fsl-replay/src/spine, fsl-semantics/src/check/lowering, fsl-semantics/src/check/protocol_clause/tests, FR-218, FR-464, FR-865, FR-880, FR-884; QSL-2215 and QSL-2220 acceptance criteria"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-218", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-884", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-880", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-464", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-865", type: references}
---

## Summary

Reviewed frozen PR #2904 at `e8fab7669b6cada8077ddffc319b3398efd18a22`, including its Rust implementation, test changes, both ticket scopes and ticket clarifications. The main paths and targeted tests are sound, with one medium test-oracle gap for the full refusal-message criterion; the change is also a partial delivery of both tickets.

## Verdict

**CONDITIONAL** — one medium test-oracle gap remains for the refusal message variants in FR-884-AC-3.

## Scope examined

- QSL-2215 AC: over-read refusal at the read; non-exhaustive choice at its header; wrong-observation at the decision/read; duplicate role at its name; refusal messages naming the offending item; duplicate protocol names rejected at the second name; same name across units accepted; minimal accepted/rejected protocol-construction corpus with sub-span loci.
- QSL-2215 ticket clarification: each S3 decision refusal names the item from its typed cause and carries that item's span; rendered text shows name and location; source expression text is not required.
- QSL-2220 AC: post-test repeat reading a body binder compiles; repeat nested in repeat compiles; scalar, record, Option and collection `over` inputs compile; terminal `when` reading `over` under `on each` compiles; new forms run through evaluator; post-test repeat without progress remains refused.
- QSL-2220 PR-declared exclusions: non-object `over` package readback and post-test repeat reader schema remain excluded under QSL-2424 / pinned contract reader; repeat invariant/variant typing and attempt-result reads remain excluded under QSL-2222 / QSL-2409.
- Changed code and tests: `fsl-replay/src/spine/lifecycle.rs`, `fsl-semantics/src/check/lowering/protocol.rs`, `fsl-command/tests/it/protocol_refusal_text.rs`, `fsl-package/src/emit/tests/protocol_controls.rs`, `fsl-package/src/emit/tests/protocol_terminal.rs`, and `fsl-semantics/src/check/protocol_clause/tests/{declaration,refusal_corpus,rules}.rs`; plus the changed test module registration.
- Context-only: the existing ignored `fsl-command/tests/it/protocol_refusal_names.rs::each_protocol_refusal_names_its_subject` and FR-218, FR-464, FR-865, FR-880, FR-884 acceptance text.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-884-AC-3 lacks active end-to-end message assertions for wrong-observation and undeclared-owner refusals; the new command test covers only a guard over-read, while the broader named-refusal test remains ignored | fsl-command/tests/it/protocol_refusal_text.rs:45 |

The new command test is a strong oracle for its case: it checks the typed `Pick` cause, exact source span, and rendered subject/location. The semantic tests also assert exact refusal spans and typed causes for the protocol corpus. However, the pre-existing command test that checks rendered messages for the unlisted read and undeclared owner remains ignored (`QSL-2215`), and the added command case does not exercise those branches. The ticket's FR-884-AC-3 message criterion therefore is not fully protected against regressions.

## Rust review

The changed lifecycle path uses typed causes to select a subject span, falling back to the declaration only when the cause has no subject span. The nested-repeat progress change recurses through a nested repeat's body and preserves the refusal path for an empty-progress body. I found no additional correctness or safety defect in the changed Rust code. No CI workflows changed.

`cargo fmt --check` fails on formatting differences, including changed files and unrelated existing files. No formatting edits were made on the frozen branch.

## Check evidence

- Passed: `cargo test --locked -p fsl-command --test it a_protocol_slot_refusal_renders_its_declared_subject_and_location -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated`
- Passed: `cargo test --locked -p fsl-semantics --lib the_over_read_refusal_is_at_the_read -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated`
- Passed: `cargo test --locked -p fsl-semantics --lib minimal_protocol_construct_pairs_check_and_refuse_at_their_subject -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated`
- Passed: `cargo test --locked -p fsl-semantics --lib two_protocols_of_one_name_refuse_at_each_name -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated`
- Passed: `cargo test --locked -p fsl-package --lib an_each_terminal_condition_that_reads_over_is_written -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated`
- Passed: `cargo test --locked -p fsl-package --lib a_nested_repeat_with_a_stepping_body_is_written -- --test-threads=1 --skip two_protocols_declaring_one_channel_name_keep_their_queues_apart --skip a_recovery_predicate_over_the_recovery_view_is_evaluated`
- No aggregate test or clippy gate run, per review brief.

## Dispositions

Not applicable to this initial review pass.
