---
id: SR-9892
title: "Code and Rust review of PR #4059: QSL-3522 deeply nested request stack overflow"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@0955f2720ebb46b2513e05beb9d1eda715358b6f; fsl-command/src/unique_json.rs, fsl-command/src/replay_request.rs, fsl-command/src/command/export.rs, fsl-command/tests/it/export_selection.rs, xtask/src/qualification.rs"
review_set: subset
---
# SR-9892: code and Rust review of PR #4059

## Summary

Ticket: QSL-3522. Head contains current origin/main. The envelope_defect fix is correct:
`unique_json::read` returns the owning `Deep`, and `Deep` releases through
`discard_node_json`, which is an explicit-stack loop. Making `read` return the owner is the
right shape. `export` hands the owner on without walking the tree recursively.

The replay route still walks the owned tree recursively after the read. `replay_request::decode`
and `WireGraph` clone or serialize subtrees of it, so `native-replay/1` still aborts. This was
reproduced at this head through `fsl_command::command::replay_with_limits` (a probe linked to the
dev-profile rlib in the PR's target directory), with threads of 2 MiB and 8 MiB. Request files are
about 200 KB, inside the input bound. The same code is on main, so these are not regressions.

Run at head from the existing build: the two `deeply_nested` tests in `it` (2 passed) and the
`replay_request::`, `command::export::` and `unique_json` library tests (18 passed). That the new
test fails without the fix was judged by reading: the old line dropped a bare 100,000-level
`Value` on a 2 MiB stack.

## Verdict

**FAIL (changes requested)** - three recursive walks reachable from a request file still abort the
process (FND-001 to FND-003), and the PR's sweep reports the replay reader as safe.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `decode` runs `rows.clone()` on `/dependencies/N/refinement_internals`. Derived `Clone` recurses, so a deep value aborts: 3,000 levels on 2 MiB, 16,000 on 8 MiB | fsl-command/src/replay_request.rs:421 |
| FND-002 | high | `witness_leaf` copies request members with `json!`, which recurses over a deep value. A text witness whose `value` is a deep array aborts: 3,000 levels on 2 MiB, 16,000 on 8 MiB. The `field["name"]` and `value["member"]` clones have the same shape (read, not run) | fsl-command/src/replay_request.rs:863-904 |
| FND-003 | high | `WireGraph::of_package_value` clones `package.semantic_graph.operand_rows` before reading it. A deep value aborts: 5,000 levels on 2 MiB, 16,000 on 8 MiB | fsl-semantics/src/check/wire_id.rs:1477 |
| FND-004 | medium | `id_of_value` serializes a request node id with `value.to_string()`, which recurses. A deep node id (obligation, occurrence or package node) aborts at 100,000 levels on 2 MiB; 40,000 refuses | fsl-semantics/src/check/wire_id.rs:695-697 |
| FND-005 | medium | The new test sends only requests with absent required members, which stop at `envelope_defect`. No case carries all fourteen replay members, so the typed decode of a deep tree is untested and FND-001 to FND-004 pass it | fsl-command/tests/it/export_selection.rs:208-247 |
| FND-006 | low | `xtask` keeps `DeepValue`, a second owner beside the now public `Deep`, and its new `parse_unique_json` wrapper hands every caller a bare `Value` again, so the PR's statement that no caller can drop a bare deep tree does not hold there | xtask/src/qualification.rs:1093-1097 |
| FND-007 | low | The new test asserts only exit 20, so a refusal for any other reason passes. It also overflows in a thread of the shared `it` process, so a regression aborts the whole binary and hides other results, unlike the child-process test beside it | fsl-command/tests/it/export_selection.rs:216-247 |
