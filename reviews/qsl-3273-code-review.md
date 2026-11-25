---
id: SR-20101
title: "code-review of QSL-3273/QSL-3295 field-member node identity (filament-spec-language PR 3923)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@cf69938b7357ec6a3cac2669223b89829c0fa798 (base 43d14c36dd); fsl-semantics/src/check/lowering.rs, fsl-semantics/src/check/abstraction.rs, fsl-package/src/emit/monitor_plan.rs, fsl-inspect/src/render.rs, fsl-semantics/tests/it/refinement_declarations.rs, fsl-semantics/tests/it/refinement_protocol.rs, fsl-semantics/src/check/refinement/internal_list.rs, fsl-semantics/src/check/refinement/protocol/tests.rs, fsl-package/src/emit/tests.rs, fsl-package/src/emit/tests/abstraction.rs, fsl-package/src/emit/tests/admission_corpus.rs, fsl-package/src/emit/tests/population_field_edge.rs, fsl-package/src/emit/tests/retained_rows.rs"
review_set: subset
---

# Code review of QSL-3273/QSL-3295 field-member node identity (PR 3923)

## Summary

Ticket: QSL-3273 (with QSL-3295). Head cf69938b73, base 43d14c36dd, 13 files. Code review with the rust-review lane. FSpec FR-322 was read at filament-specification origin/main. No cargo command was run (the brief forbids it). The gates are another machine's job.

What the change does: `walked_operands` (fsl-semantics/src/check/lowering.rs:8459-8478) now writes a `Member::Field` application's declaring node and field name into the derived path. It writes the read form only when present and the population only when present. This covers `filament.op.record.project`, `filament.op.model.field` and `filament.op.model.reaches_field`. `emit_monitor_plan` (fsl-package/src/emit/monitor_plan.rs:431-443) re-keys a derived node whose projection field it renames, then applies the change with the record-value re-keys.

Examined and consistent:
- The position order matches FR-322 "Application result and node type positions": arguments, operation identity, member operands, laws, operator, result type, semantic type.
- Within the member, the declaration comes before the name, as "Named type arguments" orders the declaration operand before the alias.
- `mint_key` still gives each key once per graph, so the change creates no duplicate and no missing node. `node_index` deduplicates by content, so the node set is unchanged and only the keys of field-member applications change.
- Recursion-group members never take operand paths (`admitted_group_keys`), so groups are untouched.
- The member `declaration` must already be a graph node: the reader's stage 4 and FR-322-AC-34 require it. The new operand segment therefore always resolves in `write_index_segments`. A dependency node is never written as a `Node` segment here.
- In monitor_plan, the wire form's operand segments are graph indices, so a re-keyed projection changes no other node's id. `renamed_id` touches only Name segments. The only renames in use, `WIRE_FIELD_NAMES` (`origin_tick`→`origin`, `amount`→`value`), cannot equal any other text segment of a projection path.
- Consumers outside the producer (fsl-eval, fsl-analyze, fsl-replay, fsl-inspect, fsl-command export) read projection bodies, not path positions. Every positional read of a path (`analysis_view.rs:306`, `protocol_locations.rs:592`, `model_based_test.rs:2980`) is about other node kinds. The filament-contract-ir reader does not derive derived paths (FR-322: "A reader does not derive a complete identifier from arbitrary node bodies"). filament-contract-codegen origin/main hand-builds the old projection key in `tests/it/v2_clause_obligation_identity.rs:500-507`, and that file already fails under QSL-2955.
- The `read_form` text is the pre-existing `"true"`/`"false"`. FR-322, FSL FR-092 and FR-093 state no path spelling for it, so it is not a finding.
- The test changes are literal and stronger, not weaker: TRAIL_REACHES_KEY, the retained_rows count 5→6 plus a field-name spelling assertion, and the inspect fixture. FR-135-AC-4, FR-137-AC-1, FR-147-AC-9 and FR-306-AC-2 now assert an equal key and an unequal body, as merged in #3883. The `FSpec-` prefixes are correct: FSL has no FR-375 or TC-310.
- `git merge-tree` against origin/main 7665f0fd03 exits 0. Against draft #3920 (QSL-3138, 1dce281be3) it also exits 0. #3920 changes only declared keys (`declaration_key`, `admitted_group_keys`), not `walked_operands`, so the two do not conflict in text or in meaning.

The 39 unattributed failures (fsl-analyze, fsl-eval, fsl-replay at head) were triaged from their panic text and code. None names a field-member key, and no consumer parses a projection path. Five are confirmed pre-existing:
- `equal_threshold_falls_back_to_policy_iteration_and_observes_its_budget`, fixed on main by #3915.
- `protocol_initial_faults_follow_distinct_root_admission`, fixed on main by #3921.
- Three `timed_subject` clock-reference tests, tracked as QSL-3258.

The three fsl-replay lib tests have no captured panic text. The full table is in the reviewer's report to the lead.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A `history` member's declaration and name are still left out of the derived path (`_ => {}`). FSpec FR-322 "Refinement row expressions" says a history read's path holds "the operation identity, the member's declaration key and name, the receiver and the result type". Two refinement rows that read two histories of one type over one receiver (`value = self.seen` against `value = self.count`) therefore give equal history-read keys across packages, and so equal refinement bodies. This is the FR-135-AC-4 defect this ticket fixes for fields, in the ticket's own refinement-row scope. | fsl-semantics/src/check/lowering.rs:8479 |
| FND-002 | medium | A `relationship_end` member adds nothing to the path, neither its declaration and end name nor its `population`. FSpec FR-322 "Population link of a `reaches`" says the population "is part of the application's structural path when present" for the `relationship_end` member of `reaches` as well as for the `field` member of `reaches_field`. The PR does this only for the field member. Two `reaches`/`navigate` applications over different ends, or with and without a population, from the same operands and of the same result type are told apart only by ordinal. | fsl-semantics/src/check/lowering.rs:8479 |
| FND-003 | medium | A `type_argument` member with no `alias` adds no operand. FSpec FR-322 "Named type arguments" puts the alias "as a string segment after the member's declaration operand", so the declaration operand is in the path of every `type_argument` member. `count<Int[0, 4]>(x in s: p)` and `count<Int[0, 9]>(x in s: p)` over one operand therefore share one path and differ only by ordinal. | fsl-semantics/src/check/lowering.rs:8450-8458 |
| FND-004 | medium | `operation`, `position`, `element` and `profile_operator` members still add no member operands. FR-322 "Application result and node type positions" puts the "member operands" in every application path, and says a model member "is named by its declaring node and its name" (operation) and that a string operand is "an operator or member name" (profile operator). Two `dispatch_call`s of two operations, or `t.0` and `t.1` of one tuple of one result type, collide up to ordinal. FR-322 does not state how a position index is spelled, which is an FSpec gap. `element` is redundant with the operand's type. This needs a fix here or a named ticket before merge. The PR body lists it only as "Remaining work". | fsl-semantics/src/check/lowering.rs:8479 |
| FND-005 | low | `a_protocol_over_the_concrete_model_is_not_an_abstract_subject` lost its `FR-147-AC-16` tag, although it asserts AC-16's own clause: with `Spec` local, `abstract Spec::OnImpl` refuses `malformed-declaration` at `OnImpl`. Partial coverage is still a correct binding. The only remaining AC-16 binding (fsl-replay/src/spine/clause/tests.rs:5188) covers the dependency-failure sentence, so the OnImpl clause now has no binding in the matrix. | fsl-semantics/tests/it/refinement_protocol.rs:434-439 |
| FND-006 | low | The change re-keys every clause whose condition reads a field, so `ObligationIdentity` changes downstream (codegen v2_clause_obligation_identity's hand-built `query` key at lines 500-507 lacks the declaring node and field name). No codegen ticket is named, and the PR body states it only as remaining work. Add it to QSL-2955 or file one, so the codegen pin bump is not surprised. | fsl-package/src/emit/tests/population_field_edge.rs:20 |

## Verdict

Not mergeable yet. The field-member change itself is correct and consistent across lowering, monitor_plan re-keying, inspect, consumers, recursion groups and imported references. It merges cleanly with origin/main and with #3920.

FND-001 is the same FR-135-AC-4 defect class in the ticket's own refinement-row scope. FND-002 and FND-003 are explicit FR-322 text the PR did not apply. Fix them here, or have the lead name a ticket for each before merge. FND-004 needs a named ticket, plus an FSpec gap for the spelling of the position index. FND-005 and FND-006 are low and quick.

Mergeability also depends on the remote gate runs (A at base for the 39, B1-B6 at head), which have not run yet.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
