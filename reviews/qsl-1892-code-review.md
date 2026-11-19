---
id: SR-2026111610160900000001
title: "Code and Rust review of QSL-1892 native enum values in the protocol transition identity (PR #2746)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-1892; PR #2746 head d98ad9452f8926a36c4324e629742aa5f0098e3c; fsl-eval/src/simulation/model.rs; fsl-eval/src/simulation/model/encode.rs; fsl-eval/src/simulation/model/encode/bounded.rs; fsl-eval/src/simulation/model/encode/projection.rs; fsl-eval/src/simulation/protocol/compensation.rs; fsl-analyze/tests/it/protocol_system.rs; spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md"
review_set: subset
---
# Code and Rust review: QSL-1892

## Summary

Ticket: QSL-1892. PR #2746, scoped to `gh pr diff 2746`. Methods: code-review
with the rust-review lane (rust-style baseline). The full gate has not run; no
focused build was needed for these findings.

Examined: `Typing::with_enums` and the new `enums` member; `enum_member` and the
single `Value::Enum` arm of `project_value`; the `ValueSink::variant` signature
change in `Retained` and `Bounded`; `ModelSystem::value_typing`; the
compensation record encoding; the TC-660 test; the FR-215-AC-5 edit.

- One encoder: both sinks reach enums only through `enum_member`; no second
  path, no compat layer. `typed_value_enc` stays only behind
  `typed_value_json` and test helpers.
- `Retained::variant` (`value.case() == case && value.owner() == owner`) is
  equivalent to the old `left == right`, because `VariantId` equality is
  `(owner, case)` and `VariantTypeDeclaration` mints each case's `VariantId`
  from `node` and the case name (`fsl-semantics/src/check/check.rs:246-262`).
- `Bounded::variant` still meters `text_equal` and `node_equal`, and
  `enum_member` charges `visit()` once per declaration in both loops, so the
  budget is still charged.
- `NodeKey` is an `Arc`, so `declared.key()` per binding is cheap.
- `on` is a grammar keyword (`fsl-cst/src/token.rs:158`), so the `up` rename is
  required.
- Domain order: `Mode { slow, fast }` is unordered. FSpec FR-141 makes the
  canonical member list "sorted by case name for an unordered enum", and FSL
  admits only that order (`EnumBinding::members`,
  `fsl-semantics/src/check/check.rs:116-119`). `Space::of` enumerates an enum in
  its shape's rank order (`fsl-eval/src/simulation/model/domain.rs:107-116`).
  So `fast` before `slow` is the specified behavior, not an engine bug. FSpec
  also requires that source order of an unordered enum has no semantic effect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-215's normative statement still says an enum field's domain is "each member in declaration order", while the edited AC-5 now lists `fast` before `slow` (case-name order) for `enum Mode { slow, fast }`. The AC now contradicts its own FR as literally read, which is the reading the original AC author took. The statement should say FR-141 canonical member order (declaration order for an ordered enum, case-name order otherwise) | spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md:166, spec/functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md:194 |

## Verdict

Code: correct and idiomatic. The change has no Rust findings. Spec: one medium
consistency finding on FR-215's domain-order sentence. The AC-5 edit is correct
against FSpec FR-141. Fix the statement in this PR.
