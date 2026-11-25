---
id: SR-2026112521829029
title: "code-review of QSL-3138 FSL owner-operand clause key"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3920 at 1dce281b (base 651e6e09); fsl-semantics/src/check/lowering.rs, fsl-package/src/emit/tests/owner_model.rs, fsl-package/src/emit/tests/protocol_recursion.rs, fsl-package/tests/checked_term_protocol_rows.rs"
review_set: subset
---

## Summary

Ticket: QSL-3138. Code review with the rust-review lane, read only. Nothing was compiled or run (owner rule). Spec basis: FSL FR-383 as merged by #3913 (AC-1, AC-24, AC-26, AC-27) and FSpec FR-322 "Protocol clause owner identity", FR-322-AC-538 and FR-322-AC-521 at filament-specification origin/main 7de9a209.

## Examined scope

- `declaration_key` (lowering.rs:1257-1283): the declaration's text segments, then one `PathSegment::Node(owner)` when `owner_model` is set. A clause without the edge keeps its declaration path. This matches FSpec FR-322 "Protocol clause owner identity". The emitter turns the node segment into an operand slot through the clause's own row, and the owner is always a local node, so the row entry is a graph index.
- `admitted_node_key` and `admitted_group_keys` (lowering.rs:2785-2890). The sole declared member of a group passes its whole path, operand included, to its anonymous members, as FSpec FR-322 requires. With several declared members, the anonymous members take the least member's declaration segments without an operand. The least key is chosen by `NodeKey` order, so the result does not depend on declaration order. Ordinals come from `mint_key`, so keys do not collide within a group or between groups.
- Compile reading: the new types and signatures check out (`Option<Vec<Identifier>>`, `filament_exact::PathSegment::{Text, Node(NodeKey)}`, `InvalidNodeKey::Allocation { requested }`, `bool::then_some(..).flatten()`). I predict no compile error in the four files.
- Tests: the owner_model.rs path and row assertions; the distinct-key assertion; the recursion tests; checked_term_protocol_rows.rs, which asserts the literal code, cause, pointer and locus for the AC-27 stale retarget.
- Other key derivations: fsl-eval, fsl-analyze, fsl-command, fsl-replay and fsl-inspect mint no protocol-clause key from its name. filament-contract-codegen origin/main 7f6c250b derives none either.

## Verdict

Not mergeable. The producer key rule is right for local clauses. Two high defects remain: a dependency protocol clause named by an external refinement key keeps a nested node id, and the PR's own contract-ir pin refuses every operand path. A new fixture is also unproven.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | An external refinement key to a dependency's owned protocol clause keeps a nested node id. `CheckedRefinement::v2_emission` writes `protocol: RefinementKey::External { package, node: structural_id(abstract_protocol) }`. `abstract_protocol` is the dependency's clause key, which is now `[Twice, Node(owner)]`. The refinement body is not in `reachable_ids`. The external clause is not one of the refinement's `dependencies` (`.filter(\|_\| !protocol_is_external)`), so `write_index_segments` never records a change for it, and `replace_node_id` never rewrites it. The nested `node_id` object reaches IR's encoder, which FSpec FR-322 refuses ("A path never holds a node_id object"). An external `{package, node}` key must instead hold index segments that resolve against the dependency's graph. Predicted failure: `admission_corpus::a_protocol_refinement_selects_a_dependency_protocol_by_identity` at `emit_checked(&root).expect("root emits")`, and probably `a_dependency_protocol_node_row_checks_its_receiver_argument`. The PR body leaves this as an "open check". Fix: write the external clause key with the owner's index segment in the dependency's emitted graph, and assert it in the test. | fsl-semantics/src/check/refinement.rs:606-612,788-793,808-819; fsl-semantics/src/check/mod.rs:6771-6782; fsl-package/src/emit.rs:10721-10783,11064-11080 |
| FND-002 | high | The head is red on its own pin. Cargo.toml pins filament-contract-ir at dc9b220b92. That revision's `validate_declaration_names` requires `path.len() == qualified_name.len()` with every segment a name. So every owned clause the producer now writes is refused `invalid_package`/`declaration-nominal-mismatch`, including at the emitter's own read-back (`own_evidence`). Every package with an owned protocol fails to emit until the pin moves. The PR body makes merge depend on #3800, an unrelated FR-563 parity-runner PR, for that pin. Fix: bump the pin in this PR to contract-ir b81baf144 or later (IR main fa9ed76e53 accepts both path forms), so this PR does not depend on #3800. | Cargo.toml:30; filament-contract-ir dc9b220b92 crates/filament-contract-model/src/checked_package/v2/mod.rs:3639 |
| FND-003 | medium | `a_clause_that_is_its_groups_only_declaration_reads_back` depends on a fixture nothing else uses: `on each (seen: Boolean) when (protocol Flow)`, a self-reference to a protocol in its own activation guard over a Boolean trigger. Nothing in the repository writes a protocol reference in an activation guard. If the checker refuses it, or the cycle pulls in another declared member such as `Main`, the test panics at `checked` or at `let [clause] = declared[..]`, and the sole-declared-member path rule stays unproven. Fix: run it before merge, or build the group from a shape already known to check. | fsl-package/src/emit/tests/protocol_recursion.rs:270-321 |
