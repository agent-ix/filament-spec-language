---
id: SR-20261125155846
title: "code-review (rust) of PR 3945 operator-based classification of recursion-group source-map terms"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR 3945 head 464132bd7ae7cd710e050cd578d33147bdfc683d; git diff origin/main...HEAD (fsl-package/src/emit/protocol_locations.rs, emit/body_locations.rs tests, emit/tests.rs, tests/it/source_corpus.rs)"
review_set: subset
---

## Summary

Ticket: QSL-3217. Code review with the rust-review lane folded in. `term_application` and the former `is_deref_application` in `fsl-package/src/emit/protocol_locations.rs` now classify a candidate application by its node's `Operator` (`has_operator`) instead of by an `expression/<form>` node-key prefix. A recursion-group member is keyed under its group's declaration path (FSpec FR-322-AC-274), so the prefix test skipped it and emission refused `Emit(SourceMapInvariant)`.

Specs read (FSpec at 57c662b1): FR-146-AC-1, FR-146-AC-4, FR-322-AC-274, FR-322-AC-510.

## Verdict

PASS for the code. The operator-based classification cannot newly include or exclude a non-recursive application:

- A non-group application is keyed by `derived_node_key(package, NodeTag::Expression, operator.semantic_form(), ...)` (`fsl-semantics/src/check/lowering.rs` `typed_application` and `insert`). Every `Operator::Quantify`, `Operator::Collection` and `Operator::Deref` application is built through `typed_application` (also `timed_clause.rs:512`) with `NodeTag::Expression`.
- `Operator::semantic_form` yields `quantify`, `collection` and `deref` for exactly these three operators. No other `NodeTag::Expression` insert uses those forms; the other direct inserts use `reference`.
- So outside a recursion group, `expression/quantify|collection|deref` prefix holds if and only if the node's operator is that class. Emitted bytes, node ids, ordinals and row order are unchanged for every package that emitted before. Only packages holding a group-member quantify, collection or deref change, and those refused before.
- `has_operator` reads `graph.semantic_graph().node(key).body()`; a group member's body keeps its `Application` with its operator (`SemanticNode::body`, map_refs only rewrites references).

Dependents: `term_rows` serves both function-body rows (`body_locations.rs`) and protocol/slot rows (`protocol_locations.rs`). The strict reader (`read_checked_package`) and FSL's I2 read (`read_v2`) are exercised by the new tests. No other production code classifies by the node-key prefix for these forms; `fsl-inspect/src/render.rs:743` and `is_expression_node` (render.rs:784) do so for `expression/binary` comparison operand spelling, a diagnostic-only path with the same blind spot for group members (out of this diff; draft ticket in the report). The `#[ix_trace_rs::string_edge]` marker was correctly dropped: `has_operator` does no string compare.

Tests run at the head under lock C, name-filtered, all passing: see the review comment on QSL-3217.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The two unit tests carry `FSpec-FR-322-AC-510` (relative, linear body-location encoding and its BL vectors) but assert only that a row exists for each query, collection or deref term; no up/down form is checked | fsl-package/src/emit/body_locations.rs:522 |
| FND-002 | low | Reader coverage is split: the deref case is read only by FSL's I2 read, the query and collection cases only by the strict contract-ir reader; neither case is read by both | fsl-package/src/emit/tests.rs:3437 |
