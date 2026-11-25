---
id: SR-2026112521829029
title: "code-review of QSL-3138 FSL owner-operand clause key"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR #3920 at 1dce281b (base 651e6e09); fsl-semantics/src/check/lowering.rs, fsl-package/src/emit/tests/owner_model.rs, fsl-package/src/emit/tests/protocol_recursion.rs, fsl-package/tests/checked_term_protocol_rows.rs"
review_set: subset
---

## Summary

Ticket: QSL-3138. Code review with the rust-review lane, read only. Nothing was compiled or run for this review. Spec basis: FSL FR-383 as merged by #3913 (AC-1, AC-24, AC-26, AC-27) and FSpec FR-322 "Protocol clause owner identity", FR-322-AC-538 and FR-322-AC-521 at filament-specification origin/main 7de9a209.

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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | Compile error at 94c36876. The fix commit deleted `let invariant = \|_\| EmitRefusal::SourceMapInvariant;` from `write_index_segments`, but four `.map_err(invariant)` calls in that function still use it, and nothing else named `invariant` is in scope. rustc reports E0425 (cannot find value `invariant`), so fsl-package and everything downstream of it fails to build. Fix: restore the closure, or give `write_external_keys` and `write_index_segments` a shared helper. | fsl-package/src/emit.rs:11099-11104,11120,11149,11166 |
| FND-005 | medium | No test pins the external key the producer now writes. `write_external_keys` rewrites `protocol` (and the abstract class `node` and `binder_types`) into the dependency's index form, but no test asserts the written key: the literal assertion is owed until #3923. If `wire_id` picked another graph's index, or a later pass rewrote the key, only a reader refusal elsewhere would show it, if anything did. Fix: in `a_protocol_refinement_selects_a_dependency_protocol_by_identity`, assert the emitted root's refinement `protocol` equals `{"package": "test/protocol-dependency", "node": {"package": <dep name>, "path": ["Twice", {"node": j}], "ordinal": 0}}`, where `j` = `operand_rows[c][k]` of the dependency's own emitted clause `c` (path `["Twice", {"operand": k}]`). Then read the root back with the dependency supplied and require admission. | fsl-package/src/emit/tests/admission_corpus.rs:1932-2140; fsl-package/src/emit.rs:5376-5450 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 94c36876 `write_external_keys` rewrites each external `{package, node}` key in the refinement body (abstract, protocol, and each abstract `Declared` class's node and binder_types) that holds a node segment. It writes the id document-held in the dependency's graph: `WireGraph::of_package_bytes(emit_checked(dependency))` then `wire_id`, so the indices come from the dependency's emitted bytes. `canonical_operand_entries` skips `{package, node}` containers, so these indices never enter the holder's operand row, and IR `replace_key` never matches them again. A wrong index cannot resolve silently: the reader rebuilds the key from the dependency graph, and only Twice's owner gives `[Twice, Node(owner)]`. The fix is proved only by `root emits` until FND-005, and it does not build until FND-004. |
| FND-002 | deferred | Closes when main is merged into this branch after #3800 moves the filament-contract-ir pin to b81baf144 or later; this PR carries no pin change of its own. Until then the pin is dc9b220b92, which refuses every owned clause path as `invalid_package`/`declaration-nominal-mismatch`. |
| FND-003 | still-open | the trigger is now the precedented `on each (t: Tick)` (protocol_captures.rs, protocol_function_bodies.rs), but a protocol referring to itself in its own activation guard (`when (protocol Flow)`) still has no precedent; it resolves when the remote gate runs the test green |
| FND-004 | fixed | 89c30c51 restores `let invariant = \|_\| EmitRefusal::SourceMapInvariant;` in `write_index_segments`. All four `.map_err(invariant)` calls resolve again. Re-reading `write_external_keys` turns up no compile error: `Box::as_mut`, `Option::as_deref_mut`, `r#abstract`, `RefinementAbstractClass::Declared { node: RefinementKey, binder_types: Vec<RefinementKey> }`, `LibraryName::new(impl Into<String>)`, the `BTreeMap` entry API and `WireGraph::wire_id(&NodeKey)` all type-check against contract-ir dc9b220b92 and b81baf144. |
| FND-005 | still-open | Not landed at 89c30c51. The test still asserts `node["body"]["protocol"]` against `root_ids.wire_value(protocol_key)`, which writes the owner with the root graph's index. The emitter writes the dependency graph's index, so the test is predicted to fail unless the two indices happen to match. The held replacement is a correct, independent oracle and closes this when it lands. It reads the dependency's own emitted wire: the `Twice` clause path `["Twice", {"operand": k}]` gives `owner = operand_rows[clause][k]`. It then expects `{"package": "test/protocol-dependency", "node": {"package": "test/protocol-dependency", "path": ["Twice", {"node": owner}], "ordinal": 0}}`. The existing `read_checked_package_v2(root ...).expect(...)` with the dependency supplied already gives the read-back half. |

## Dispositions (pass 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | c395ec3a6e merges main, which carries #3800. The contract-ir pin is now b81baf144 (IR-1614), which admits the operand path and compares the owner edge against it. The coder's gate at 433078677c is green apart from reds that are identical on main 85450e4179. |
| FND-003 | fixed | `a_clause_that_is_its_groups_only_declaration_reads_back` is not among the gate's reds at 433078677c. 33b0b4e105 compares `NodeKey::path()` structurally (`Vec<PathSegment>`) instead of comparing Debug strings. |
| FND-005 | fixed | e660e5b668 asserts the written key against an independent oracle taken from the dependency's own emitted wire. The oracle finds the clause by path `["Twice", {"operand": slot}]`, reads `owner = operand_rows[clause][slot]`, and expects `{"package": "test/protocol-dependency", "node": {"package": "test/protocol-dependency", "path": ["Twice", {"node": owner}], "ordinal": 0}}`. The root read-back with the dependency supplied stays in place. |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The new AC-15 test covers a `timed` selection, no selection, and an event-position selection (profile mismatch). The new branch also runs `validate_temporal_clock` and `check_selected_definition`'s definition check on a timed protocol selection, but no test covers a `timed` selection that fails them: an invalid clock name, an empty `unit`, or a wrong `definition`. A regression that skipped those checks for the `timed` arm, before its `continue`, would pass every test. Fix: add one such case to `a_timed_protocol_clock_selection_is_checked_and_writes_no_row`. | fsl-package/src/emit/tests/protocol_compensation.rs:588-678; fsl-package/src/emit.rs:10606-10615,10694-10701 |

## Dispositions (pass 4)

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | still-open | New this pass. |
