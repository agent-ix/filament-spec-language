---
id: SR-2026111911313600000007
title: "Code and Rust review of native collection element admission"
type: SpecReview
analysis: code-review
scope: "QSL-1068; changes after 8f2ba1883 in fsl-semantics/src/check/assemble.rs, src/model/domain_package.rs, src/model/observation.rs, src/model/observation/{document,encode,tree}.rs, document/tests/collection_elements.rs and tests/it/native_snapshot_values.rs"
review_set: subset
---

## Summary

Reviewed the ticket diff with the code-review, rust-review and rust-style
checklists against FSL FR-106 and FSpec FR-144/FR-181. No specification change
is required by the architecture ruling. This is a partial delivery for the
existing draft PR, not an acceptance verdict for the entire ticket.

## Verdict

CONDITIONAL: focused tests pass; the focused Clippy check is blocked by
existing diagnostics outside the changed behavior. The owner excluded an
aggregate gate from this session.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Focused Clippy cannot establish a clean lint result: the library reports 565 errors and its unit-test target 589, predominantly existing result_large_err diagnostics, plus existing large_enum_variant, too_many_arguments, match_single_binding and question_mark diagnostics. | /tmp/qsl-1068-final-clippy.log; fsl-semantics/src/check/check/modulo_interval.rs:79; fsl-semantics/src/check/footprint.rs:189; fsl-semantics/src/check/probabilistic/claim.rs:1541 |

## Evidence and limits

- The original uncommitted Option/Decimal, scalar Float, fixture lookup and
  Duration tests were retained. Test API errors were corrected before the
  intended red was established.
- Admission uses the kernel's Decimal membership, exact Rational and Float
  constructors, collection construction and membership relation. The public
  equality key excludes Float leaves; the fallback uses kernel member_equal,
  including through Options, and retains supplied indices for duplicates.
  That fallback scans pairs; keyable elements retain the existing sort path.
- The heap-stack admission flag normalizes Decimal representations throughout
  set/bag elements while preserving declared types and ordered representations.
  Encoding and frame identity are checked separately. Envelope names retain
  document order for missing/unknown/wrong-kind classification.
- No dependencies, workflows, AGENTS.md or CLAUDE.md changed. No applicable
  AssuranceProfile was found in spec/. No new production panic or lint
  suppression was introduced. No aggregate, whole-workspace test suite,
  dependency audit or independent reviewer run is claimed.
- Focused command results and acceptance-vector limits are recorded in the
  accompanying gap analysis and draft PR body.
- Final focused compilation passes (`cargo check --locked -p fsl-semantics
  --lib --tests`, with the PR's build flags), /tmp/qsl-1068-final-check.log.
  The 11 named unit tests and three named integration tests pass after the
  cardinality-precedence fix. Formatting, whitespace and both review
  artifacts validate; the exact changed parser test passes separately.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | Existing lint debt is outside this ticket's admission change; retain the failing command and log for the code lead. |
