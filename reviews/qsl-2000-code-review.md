---
id: SR-2026111612172400000040
title: "Code and Rust review of QSL-2000 slice A probabilistic node emission (filament-spec-language PR 2921)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@5f99c3c4a051689566803d8792a83db84905de5f; git diff origin/main...HEAD (PR 2921, branch qsl-2000-random-parameter-workload, 17 files); focus: fsl-package/src/emit/probabilistic.rs, fsl-package/src/probabilistic_read.rs, fsl-semantics/src/check/probabilistic/model.rs, fsl-semantics/src/check/probabilistic/model/package.rs, fsl-semantics/src/check/lowering/probabilistic.rs, fsl-eval/src/simulation/model/input.rs, tests in fsl-package/src/emit/tests/probabilistic.rs, fsl-eval/src/simulation/model/probabilistic/tests.rs, fsl-semantics/tests/it/probabilistic_declarations.rs"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2000 (slice A). The PR emits checked random parameters and workloads as
FSpec FR-640 `probabilistic` nodes, reads them back through `BarePackage::probabilistic()`
and rebuilds S3 records through `StateModel::probabilistic_model`, which
`PackageModelInput::of` now carries. Rust lane (rust-review) folded in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `workload_weight_order_changes_no_bytes_and_weight_zero_is_emitted` emits the identical unit text twice and compares bytes; nothing is reordered (one-operation fixture), so FR-740-AC-3 / FSpec FR-640-AC-3 weight-order independence is untested while the test is now un-ignored and tagged as covering it | fsl-package/src/emit/tests/probabilistic.rs:821-847 |
| FND-002 | medium | Tests that assert a refusal are tagged to ACs that require emission: `a_checked_reward_refuses_emission_beside_an_emitted_workload` newly tagged FR-185-AC-7; `service_refuses_emission_naming_its_boolean_random_parameter`, `an_enum_valued_random_parameter_refuses_emission_with_its_locus`, `a_workload_over_unrelated_types_refuses_with_its_locus` tagged FR-740-AC-1. The matrix shows AC-1/AC-7 covered while the only full-Service test is ignored (QSL-2440) | fsl-package/src/emit/tests/probabilistic.rs:30-35,655-660,792-794; fsl-eval/src/simulation/model/probabilistic/tests.rs:733-736 |
| FND-003 | medium | A workload over a model whose domain package has unrelated object types (no lineage-covering type) refuses at S4; FR-740 says S4 SHALL emit a workload node for each Workload. This refusal is not one of the stated deferrals (QSL-2439/2440/2441, IR Boolean/enum) and has no ticket; root cause is FR-640's single `model` reference | fsl-package/src/emit/probabilistic.rs:316-320; fsl-semantics/src/check/probabilistic/model.rs:756-788 |
| FND-004 | low | Node-id operand spells a text support value with Rust `{:?}` (Debug escaping), which is not a specified or stable canonical spelling; identity of a text-valued random parameter depends on std's Debug escape rules | fsl-package/src/emit/probabilistic.rs:125 |
| FND-005 | low | `value_of` accepts an integer outside `Int[lo, hi]`, a rational outside `Rational` bounds and ignores `BoundedQuantity` bounds while `ValueOutsideType` documents the check; text admission uses a `Meter` with every limit `u64::MAX`. Safe today only because IR's `not_member` checks ranges first | fsl-semantics/src/check/probabilistic/model/package.rs:918-960 |
| FND-006 | low | `support_spelling` arms for `Boolean`, `Enum` and `_` are unreachable: `random_parameter_node` refuses those values before spelling them | fsl-package/src/emit/probabilistic.rs:120-129,265-290 |
| FND-007 | low | Stale ignore reason: delay emission test still cites QSL-2000 although the PR moves delay work to QSL-2439 | fsl-analyze/tests/it/digital_clocks.rs:848 |
| FND-008 | low | Operation resolution now fails `Unresolved::Missing` when the declaring type is not among `model_objects[alias]` (new S3 failure path for an operation inherited from a type outside the alias's object set) | fsl-semantics/src/check/probabilistic/model.rs:689,722-726 |

## Scope and evidence

- Read every changed production file in full; checked FR-740 (FSL), FR-640 and FR-322
  "Probabilistic-model nodes" / "Structural node identity" (FSpec origin/main 7e10eddb;
  local FSpec checkout is stale).
- Exact rationals: `spelling` writes `n` or `n/d` of `filament_exact::Rational` (reduced);
  workload `1.5` reads back `3/2` (workload_round_trips). Support kept in declared order.
  Workload weights sorted by (declaring node id, operation); zero weights emitted.
- One `declaration` occurrence per node at `declaration_location()` (label span),
  asserted by `declaration_region` in the round-trip tests.
- Refusals: `refusal()` always yields `ProbabilisticModelNotEmitted` with a region once
  `region_of` succeeded; `SourceMapInvariant` only on a missing region (invariant).
- Read-back differences documented in package.rs module doc (model alias = domain package
  identity; `declared` = package order). Consumers checked: `ModelSystem` tables,
  `weight_of` (fsl-eval probabilistic.rs, timed/race.rs, fsl-analyze digital.rs) look
  weights up by operation text and random parameters by (declaring, name, position);
  none reads `model()` or `declared`. `Workload::model()` is compared with a claim alias
  only in S3 claim checking (claim.rs:1011), not on the package route; becomes relevant
  when QSL-2441 lands.
- New reader code: no `unwrap`/`expect`/indexing on package input; positional bindings via
  `get`; faults typed. No new `#[allow]`. Clippy (fsl-package, fsl-semantics, fsl-eval,
  all targets): clean.
- Ignores: only QSL-2440 / QSL-2441 / pre-existing QSL-1991 and the delay test; no test
  weakened beyond the retargeting of the TC-620 refusal tests from workload/random to
  reward (consistent with the PR).
- Focused tests on 5f99c3c4: fsl-package `probabilistic` 15 pass/2 ignored,
  `admission_corpus` 33/2, fsl-eval `probabilistic` 13/2, fsl-semantics it
  `probabilistic_declarations` 76/2. Full fsl-eval lib 234 pass/0 fail. Full fsl-package
  lib 855 pass/36 fail: the 11 known emit tests (protocol_locations, protocol_controls,
  refinement_adverse), checked_v2 conformance_* and 10 further checked_v2/timed_order
  tests, each of which also fails on pristine merge-base d3c2fcf51 (stale local FSpec
  checkout); no new failure.

## Verdict

Implementation is sound and conforms to FR-640 body order, lowest terms, support order,
weight sort, zero weights, single declaration occurrence and deterministic emission
ordering. No high findings. FND-001 (tautological determinism test) and FND-002
(trace inflation) should be fixed in this PR; FND-003 needs a ticket or a stated
deferral. Mergeable after those.

## Dispositions

Round 1, head 3936e36cabda798c65231072ef4f0fdad4562b87 (fix commit aeec40ac1).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `workload_weight_order_changes_no_bytes_and_weight_zero_is_emitted` now emits the two-operation lineage workload with its weight lines in both orders and compares bytes |
| FND-002 | fixed | the refusal tests are retagged TC-2062 / FR-185-AC-12 (S4 refusal carries the declaration's locus), which is what they assert; the message half of AC-12 is the pre-existing fsl-command `probabilistic_package_locus` test (ignored QSL-1996 on main) |
| FND-003 | deferred | QSL-2444 (Backlog) filed; referenced at emit/probabilistic.rs:333 and in the test name `a_workload_over_unrelated_types_refuses_with_its_locus_qsl_2444` |
| FND-004 | fixed | text support spelled with `serde_json::to_string`, documented as the literal's JSON string encoding |
| FND-005 | fixed | `value_of` admits through `ty.admits(&value)`; the text Meter is sized to the literal (bytes, 18x NFKC expansion, bounded work) |
| FND-006 | fixed | `support_spelling` returns `Option`, Boolean/Enum arms removed, `None` refuses with the declaration's locus |
| FND-007 | fixed | digital_clocks.rs:848 ignore now cites QSL-2439 |
| FND-008 | still-open | The `Unresolved::Missing` path was replaced by an untested fallback chain: the alias's own objects, then any `object_types` entry with the same EffectiveId (which can name a type of another package), then `key.clone()`, which is the operation's declaration key used as a type key. The comment says a key without a node "is refused at emission with the declaration's locus", but `Lowering::probabilistic_declarations` runs first in the same check and calls `model_node` on that key: `model_owner` faults `UnadmittedPackage` for another package's key, and `records.get` faults `UnknownDeclaration` for an operation key. Either one becomes an S3 InternalFault refusal of the whole unit, or roots a mislabelled declaring node. Required: make `declaring_key` an `Option`, skip rooting on `None` and refuse at emission with the locus (as `model_key` already does), or restore a typed S3 refusal; in both cases add a test that reaches the case |
