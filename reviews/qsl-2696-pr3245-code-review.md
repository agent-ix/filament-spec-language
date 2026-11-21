---
id: SR-2026112102482701
title: "QSL-2696/QSL-2697 PR 3245 code review: operation namespaces and frame collapse at intake"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@a4de58ba954a55ff537d5a0dfcd2b358e5c513f6; fsl-semantics/src/model/intake.rs, fsl-semantics/src/model/refusal.rs, fsl-semantics/tests/it/operation_uniqueness.rs"
review_set: subset
---

## Summary

Ticket: QSL-2696 (primary), QSL-2697. PR 3245 adds `ModelRefusalCause::AmbiguousName`, refuses a repeated operation name before an owner's operations are read (object types, processes, persistence interfaces), refuses a parameter name that differs from its identity tail or equals `result`, and collapses repeated `modifies`/`creates`/`deletes` entries. The Rust-review lane is folded in. The main path matches FR-512 AC-5, 9, 10, 11, 12, 13 and 15. There are three medium defects at the edges. State machines skip the name check. Parameter checks preempt the record and event owner refusal. The `refusal_node` wildcard drops the new variant.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | State machines skip the operation-name check. `refuse_repeated_operation_name` is not called in `read_record_value_type`, which is also the operation reader for state machines (state_machine.rs:177). A state machine is an admitted owner under FR-512, so two of its operations with equal `name` and distinct identities are admitted instead of refusing `ambiguous_declaration`/`ambiguous-name`. | fsl-semantics/src/model/intake.rs:3707-3718 |
| FND-002 | medium | A record or event operation that has a parameter named `result`, or a parameter whose name differs from its identity tail, now refuses at intake (`ambiguous-name` or `malformed-declaration` at `params[i].name`). The owner refusal for that type runs later, in normalization (normalize.rs:1061, 1108), so it never runs first. FR-512 Order puts the owner rule before the parameter and result-name checks. The same reasoning the PR uses to exempt records and events from the name check applies to these parameter checks. | fsl-semantics/src/model/intake.rs:2771-2791 |
| FND-003 | medium | `refusal_node` has a `_ => None` arm that catches the new `AmbiguousName` variant, although it carries a `key` just as `ConflictingBinding` does. AmbiguousName refusals therefore sort ahead of every keyed refusal, not in ascending node order (intake.rs:6065, 6223; normalize.rs:1715, 2987). They are also never matched against `semantic_refused_nodes`/`refused_nodes`, so one can be reported beside a retained reader diagnostic for the same node. FR-512 says such a check does not run. | fsl-semantics/src/model/intake.rs:6291-6300 |
| FND-004 | low | `member_identity_name(owner_identity, node)` is computed twice: once by the `is_none()` guard and again with `.unwrap_or_default()`. A `let Some(tail) = ... else { return Err(..) }` makes the invariant explicit and removes the default-string path. | fsl-semantics/src/model/intake.rs:2765-2771 |

## Scope and evidence

- Read the full diff `origin/main...a4de58ba95` (3 commits, 3 files). Cargo.lock, CI workflows and gate scripts are unchanged, and there is no unrelated reformatting.
- FR-512-AC-11: the spec refuses a `result` parameter whether or not `returns` is declared ("whether or not the operation declares `returns`"). The code and tests do this. The ticket text that says AC-11 admits it is wrong.
- A missing `name`: the semantic-IR reader (agent-ix-semantic-ir c23980e, schema.rs `operation_schema` require_members and `FIELD_REQUIRED`) requires a non-empty `name` on operations and parameters. The `opt_str_field` skip is unreachable behind the reader, so no finding.
- `AmbiguousName`: `ambiguous_declaration`/`ambiguous-name` is the code and cause pair already used across the spec (FR-380, FR-087, FR-898, ADR-012), and FR-512 itself assigns it. FR-272 is about canonical-type copies and has no cause list, so no catalog row is owed. The new variant is added to every exhaustive match in refusal.rs, and to the catalog-payload `None` list, the fixtures and both tag/code tests. The `_` wildcard in `refusal_node` (FND-003) is the only site where the variant is mismatched.
- Panics: none new on untrusted input. `refuse_repeated_operation_name` is O(n^2) over one owner's operations. Within its bound, that is fine.
- Frame collapse keeps the first-occurrence position (`!keys.contains`), as FR-512 requires. The `modifies`/`modifies_receiver` conflict check still reads the raw lists, so AC-14 is unaffected.
- No build was run. The in-file intake unit tests and the refusal.rs exhaustive tests were checked by reading only.

## Verdict

CONDITIONAL. The production change is correct for object types, processes and persistence interfaces, and the un-ignored tests have strong oracles (code, cause tag, key node, member path, and both colliding identities). Fix FND-001 to FND-003 before merge. FND-004 is optional.
