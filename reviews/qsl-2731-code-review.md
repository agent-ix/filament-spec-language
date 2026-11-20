---
id: SR-2026112013172122111220105
title: "code-review of QSL-2731 located refusal of a probabilistic claim over a source-object package"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2731; PR3203 head 4e94b698c1c310b3572a1a467171e9681571616f; git diff origin/main...HEAD (base 1af5220358); fsl-analyze/src/analyze/document/package_run.rs; fsl-command/src/command.rs; fsl-command/src/command/analyze.rs; fsl-analyze/tests/it/{package_probabilistic_source.rs,prob_world.rs,main.rs}; fsl-command/tests/it/probabilistic_package_locus.rs"
review_set: subset
---

## Summary

Ticket: QSL-2731, PR filament-spec-language#3203, frozen head 4e94b698c1c310b3572a1a467171e9681571616f, scope `git diff origin/main...HEAD` over base 1af5220358. Code-review with the rust-review lane folded in.

`settle_package_run` now calls `probabilistic_source_refusal` before `resolve_names`. When the package's first unread model form is a source object and an item's `claim` names a probabilistic claim, the run is refused `unsupported_construct`/`not-yet-implemented` at the claim, with an occurrence locus in words. The family is `ProbabilisticSourceModel` when `PackageModelInput::source_object_probabilistic_model` rebuilds the model, and `ProbabilisticClaimUnresolved` otherwise. `RunCause::AnalyzeNotYetImplemented` gains an optional `locus`.

## Verdict

MERGEABLE. No high finding. FND-001 (medium, low confidence) is a refusal-classification inconsistency on a path not shown reachable; it does not produce a verdict.

What holds:

- **Soundness, no wrong verdict.** On base, a source-object package refuses every runnable item: `PackageModelInput::of` returns `Form { SourceObject }` before reading anything, and a probabilistic claim item resolves to `Pending(Declaration)` in `resolve_names`. So no probabilistic claim item could settle on base, and none can now. The new check only turns that unlocated refusal into a located one. Mixed selections refuse the whole run, as `resolve_names`'s `collect` already did on base. Hybrid items, derived-kind items, items with no `claim`, and the default-items rule all keep the base path. A name that `declared_names` maps to a non-claim node falls through to the base path, which still refuses.
- **Packages without source objects.** `first_unread_model_form` returns `SourceObject` only when no unread node form exists and the supply has a source-object row, so other packages return `None` before any item is read. `analyze_package_selection` (23), `model_check_input` (12) and `analyze_family_cli` (13) pass.
- **Unresolved branch.** The Rational test reaches `SourceModel(UnreadParameter)` for `O::odd`'s parameter `r`, named by the workload weight. That is the reader's real gap, so the not-yet-implemented refusal is accurate there.
- **Wire.** `OutcomeLocus::Occurrence` is an existing variant. `filament-outcome-1.schema.json` `$defs/locus/oneOf/2` types `occurrence` as a plain string, so the locus is schema-valid and additive. Every other pending family keeps `locus: None` (null). `wire_schema` (2), `inspect_handoff_locus` (1) and `probabilistic_package_locus` (2) pass.
- **Stale test.** The renamed locus test now reads a refusal that S4 still produces (random parameter over `String`). The new delay test pins #3171's emission, so the test that was red on base is green.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unresolved branch classifies every `PendingPackageForm` refusal from `source_object_probabilistic_model` as `ProbabilisticClaimUnresolved` (`unsupported_construct`/`not-yet-implemented`). The same refusals from `PackageModelInput::of`, 20 lines above, are mapped to `AnalyzeDocumentError::Fault`: `World`, `Model`, `Operations` and `ProbabilisticNodes`, which mean an unreadable package or selected domain packages that do not admit. On a source-object package with a probabilistic claim, an internal inconsistency or an inadmissible model is now reported as a feature gap ("not yet implemented ... does not resolve"), not as a fault. Only `SourceModel` and `Probabilistic`, which describe gaps in the source-object rebuild, are genuine not-yet-implemented causes. Reachability is not demonstrated (low confidence). Fix: map `World`/`Model`/`Operations`/`ProbabilisticNodes` to `Fault`, as the `of` path does | fsl-analyze/src/analyze/document/package_run.rs:564-574, fsl-analyze/src/analyze/document/package_run.rs:596-617, fsl-analyze/src/analyze/document/package_run.rs:297-306 |
