---
id: SR-967175026302197768900474181941935084710
title: "Gap analysis of the QSL-2812 value-invariant admission fix"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2812; bounded fix disposition 24a0382a4d..9f4272343398320cb921822f55ae9503314fb54c; fsl-command/src/command/items.rs and clause.rs; changed test consumers"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

The production addition to PR 3467 makes gap-analysis applicable during its single fix disposition. This artifact records the caller-bounded changed-surface audit and full-repository computed traceability input; it does not claim a second full repository review or a repository-wide assurance PASS.

Plan completion: not assessed

## Verdict

**CONDITIONAL** — one medium trace-binding defect was found in the new function-role control. No new requirement, reverse-gap, source stub or behavioral defect was found in the changed production surface. Arch reports eight owned reruns green at the fix head. The external canonical-value reader failure belongs to QSL-2810/PR 3470 and was explicitly excluded by arch. The evidence-backed matrix was unavailable because its request exceeded the native runtime's accepted size; static traceability cannot replace that execution evidence. The final workspace gate remains arch-owned.

## Scope

The explicit caller instruction limits this disposition to the one fix delta and newly affected production surface. Therefore the full-source inventory required for a repository-wide gap-analysis PASS was not repeated. Repository identity is agent-ix/filament-spec-language, with spec/ as the document root; no plan was supplied. The full computed matrix is still read as the traceability input, with broader baseline diagnostics disclosed rather than silently interpreted as clean.

| Unit | Role | Requirement text examined |
| --- | --- | --- |
| FR-652 Behavior | examined | A snapshot naming a pre or post snapshot is a document of the right format and reaches FR-106, which refuses it wrong_snapshot/wrong-observation at stage admit. |
| FR-652-AC-3 | examined | The invocation-parameter item reports violation with document violation/exit 10; the same selection with a pre snapshot gives one item refused wrong_snapshot/wrong-observation at admit, without invariant or path. |
| FR-106-AC-48 | examined | ValueInvariants admits the K2 current snapshot and invocation parameter; a pre document selected as snapshot refuses wrong_snapshot/wrong-observation. |
| FR-312 / FR-109 | context_only | Function selection reads a current snapshot; the existing request reader retains its format and current-role check. |
| FR-027-AC-8 / FR-185-AC-12 | examined | A real emit refusal preserves stage/cause with no package bytes, and a workload's S4 refusal carries its source declaration locus and a plain declaration message. |

## Reverse-gap and stub inspection

| Changed behavior | Owning requirement | Evidence inspected |
| --- | --- | --- |
| Prepare the actual no-clause value-invariant item | FR-652 Behavior and AC-3, FR-106-AC-48 | prepare_value_invariants constructs ValueInvariantsBatchRequest, selected references and unmodified role-bearing bytes; program batch dispatch reaches admit_value_invariant_selection. |
| Share bounded model/library/observation reading with clause preparation | FR-312, FR-106, existing intake bounds | read_observation_inputs contains the existing real readers and byte/file admission; read_with_source delegates to it, retaining its selection-specific mapping. |
| Keep function snapshot-role admission independent | FR-312 / FR-109 | The existing current-role request check is unchanged; the new function control covers current success and pre/post/invocation request refusals. |

Inventoried changed meaningful behaviors: 3. Unowned changed behaviors: 0. Confirmed source stubs: 0. Confirmed changed test stubs: 0. All inspected paths execute real owner APIs; no selected result or document role is fabricated. The remaining preflight population-placeholder literal is existing structural decoding support outside the changed value-invariant path, not a synthesized function or execution result.

The compile and diagnostic tests share the existing unrelated-type model builder rather than introduce a second fixture definition. The strengthened value-invariant test retains both the invocation-parameter violation and pre-snapshot admission refusal. No benchmark or coverage percentage was supplied or used as completion evidence.

## Coverage

Plan completion: not assessed

Evidence-backed command on terra: /home/peter/.npm-global/bin/quoin 0.36.8, quoin matrix --repo /home/peter/dev/worktrees/qsl-2812-stage-tests --json. It exited 1 with empty stdout and stderr: Error: native runtime matrix.build failed: CORE_REFUSED: request exceeds the accepted size. The smallest reproduction is that command and scope; the failure was reported to the lead. No old binary or cached matrix was substituted.

The documented static fallback uses /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2), quire matrix --scope /home/peter/dev/worktrees/qsl-2812-stage-tests --format json. Current-head results and source-binding diagnostics are summarized below. Static tagging alone never establishes runtime success.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new function snapshot-role control tags FR-109 and FR-312, which the computed coverage reports as untracked symbols, so the test binds no owning criterion. Replace the document-id tags with the owning function-reader criterion, such as FR-312-AC-4, while retaining the behavioral control. | tests/it/run_items.rs:846; FR-312-AC-4 |

### Current-head static results

Reconciliation: quire matrix 0.52.0 (engine 0.62.2), exit 0 — no run evidence read. The current-head full matrix contains 6,291 criteria: 5,020 tagged, 1,081 untagged, 68 tagged-by-ignored-test, 107 method-without-symbol and 15 retired. Those broader baseline gaps prevent any repository-wide assurance PASS; they are disclosed, not folded into this caller-bounded source-fix review as newly introduced defects. FR-652-AC-1, AC-2 and AC-3 are all tagged; FR-027-AC-8 and FR-185-AC-12 remain tagged.

The current-head quire coverage command also exited 0. Its reported Rust binding census is 14,441 candidates, 13,866 tagged and 13,268 bound. Its changed-source diagnostic names the newly added function_selection_requires_a_current_snapshot test: FR-109 and FR-312 are both reported as untracked_symbols at tests/it/run_items.rs:846. The control ran successfully on titan, but those two document-id tags bind no acceptance criterion. This is the one changed traceability defect recorded below; it does not invalidate the observed function-reader behavior.

The static commands emitted symbol-id-collision diagnostics, including cfg alternatives and deliberate duplicate native fixtures, plus unmatched/unmarked traces and section-selection diagnostics outside this delta. No silent clean interpretation was made. The matrix and coverage JSON are retained only in this review scratchpad. Optional semantic gap-analysis and plan assistance were not requested and were not run.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The sole change in 4fd8d974ae99f3060cc2532d298c6401014226c6 replaces the two document-id trace attributes with the existing owning criterion FR-312-AC-4. The test body and production reader remain unchanged. |

### Latest bounded verdict

**PASS for the bounded fix source and changed trace binding.** No change finding remains open. The exact after text is `#[trace("FR-312-AC-4")]`; that criterion is minted and tagged in the computed current-source matrix. The metadata correction uses the same declared Rust trace-attribute form, so a new full matrix or runtime execution was not required to establish this local disposition. The eight passing titan results remain evidence for the unchanged behavioral source at 9f427234; the final workspace gate at the final PR head remains arch-owned. Broader repository assurance and the unavailable evidence-backed matrix remain outside this bounded PASS claim.
