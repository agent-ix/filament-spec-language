---
id: SR-1795267647659877508
title: "Base review of FR-517 Await clock selections"
type: SpecReview
analysis: base
scope: "FR-517 Await clock selections amendment and FR-517-AC-29 through FR-517-AC-33"
review_set: base
---

## Summary

PASS for the bounded specification amendment in `/tmp/protocol-batch-2-await-contract.patch`. The four-field Await row, native declaration identity and occurrence, caller-supplied binding authority, refusal axes, global binding accounting, optional format extension and source replay obligations are consistent and sufficiently concrete for implementation. This is a contract review, with no implementation or runtime acceptance claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No defects found in the changed contract under the base checklist. Implementation coverage remains unverified by this review. | FR-517 Await clock selections; FR-517-AC-29–33 |

## Scope and policy

Reviewed only the FR-517 amendment: its FR-508 relationship, selection-input extension, new Await section, binding-count clarification and five new criteria. Existing dependency artifact closure, witnesses, old Verification cells and unrelated requirements are outside the review. No source, Cargo, runtime, matrix, gate, tracker, PR or push actions were performed. No optional analysis lenses or additional agents were used.

The installed catalog resolves AssuranceProfile to `/home/peter/.ix/filament/modules/engineering-assurance`. No instantiated AssuranceProfile exists in this repository's specification tree or project configuration: matches elsewhere in repository reviews are historical prose, and installed module skeletons are templates rather than applicable profiles. Therefore no applicable `review_policy.mode: require` operation or enforced human acceptance gate was identified. This direct review does not assert human acceptance. The dispatch selected the bounded base review; repository AGENTS.md permits applicable selected analyses and keeps incomplete assurance separate from engineering dependencies.

## Contract evidence

1. Native authority: `fsl-package/src/emit.rs:9314` onwards obtains Await selections from the checked source and whole declaration span, constructs `await <Protocol>::<AwaitName>` at line 9336, joins the checked protocol node key to the native node id, and emits each actual Declaration ordinal. Its binding retains `awaited.clock` and the selected configuration, with `source_unit: None` and `trace: None`. The amendment matches these authorities and expressly forbids row-index ordinals and emitted-package readback as the expectation oracle.
2. Admission remains owned by FR-508: its admitted identities include event-position, fixed-sample, timestamped finite-window and timed; InfiniteTrace remains refused. FSpec FR-464, Profiles and laws, requires a temporal-profile selection for Await. The amendment neither broadens those admissions nor creates a new use site. A null binding is conditional on an existing profile/request genuinely requiring no configured binding; it cannot swallow a required-selection refusal.
3. Error behavior distinguishes malformed sidecar rows from package comparison. Duplicate keys and unknown keys retain existing refusals; ambiguous selectors refuse rather than choose a declaration. Comparison matches the native node and actual Declaration occurrence, rejects other loci and surplus/missing bindings, retains Await selector context, and uses existing `clause.node`, `clause.binding` and `clock.*` axes.
4. Global accounting includes all expected configured Await bindings and every actual package binding, preserving temporal-clause, compensation and hyper-trace accounting. No-Await units omit the new member and retain prior bytes under selection/1. Explicit member order and source order make the extension deterministic.
5. Acquisition, sidecar reconstruction and source replay retain the actual supplied selections. FR-517-AC-29 and AC-33 make a non-default caller authority observable across write, compile and analyze/replay. This closes the default-clock and empty-selection escapes without changing generic dependency closure or witness ownership.

## Base checklist disposition

The changed artifact retains FR-517 and its existing US link. New criteria are unique, sequential FR-517-AC-29 through FR-517-AC-33; their Verification cells name Test without TC ids. The new FR-508 link resolves locally; FSpec FR-464 was read as the native protocol contract. No new US, option or constraint identifiers are introduced. Inputs, outputs, exact row members, validation, selection ambiguity, missing clocks and comparison errors are defined. The amendment adds no performance or security promise requiring an invented threshold or separate acceptance condition.

The six coverage rules were examined at contract level: AC-29 supplies a positive oracle and recompilation; AC-30 covers multiple declarations, mixed binding counts, order, determinism and zero-Await omission; AC-31 covers admitted adverse node, occurrence, surplus/missing binding and clock differences; AC-32 covers malformed rows, duplicate/unknown/ambiguous selectors and missing required configuration; AC-33 covers source replay propagation. Additional normative structure and locus refusals remain explicit in the new Behavior section. No new state machine or numeric capacity boundary is introduced. Tests must implement the existing profile-conditioned behavior rather than invent a no-binding profile. The implementation test matrix and trace coverage were deliberately not run in this specification-only scope, so this PASS does not assert tagged or executed coverage.

The changed text introduces no file-tracking hash, digest, pin, manifest, inventory, ledger or provenance-record requirement. Native node identity and configuration authority are domain contract semantics. Existing dependency manifests and inventory behavior were not changed or re-approved by this bounded review.

## Validation

`quoin write . --types SpecReview` was fetched once from the target repository and its installed skeleton/schema guided this single artifact. Actual tools were `/home/peter/.npm-global/bin/quoin` 0.36.8 and `/home/peter/.npm-global/bin/quire` 0.52.0, engine 0.62.2.

`quire validate --scope /home/peter/dev/worktrees/protocol-batch-2-qsl-2271-await-contract "spec/**/*.md"` exited 0 with grammar warnings; no broad fixes were made. The focused FR-517 validation with the same explicit repository scope exited 0; stdout/stderr are retained beside this artifact. The artifact itself is validated with that explicit repository scope, as recorded in the handoff.

The standalone `quire schema SpecReview --module /home/peter/.ix/filament/modules/spec-artifacts-process` command exited 0 and returned the schema, but reported UnknownEdgeType for ADR `realizes_requirement` and TestMatrix/TestMatrixIndex `covers`, plus UnknownRole for ADR `FR`/`NFR`. Exact fresh schema stdout and stderr are retained beside this artifact and reported to the parent. Repository-scoped composed validation is used explicitly; the warning is not silently discarded or interpreted as a runtime result.
