---
id: SR-202611210022442313144108539
title: "spec-review/ears-conformance of FR-1036 plan candidate correction (QSL-1978)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language; PR3279 head e28930774c7f3c2bf2c8665bd6ca03f24383a43e; FR-1036 candidate Behavior and AC2"
review_set: subset
---

## Summary

FR-1036 correction names generated plan-record candidate typing while preserving actual grammar G and domain D. The approved FSpec786 candidate 5932a58cea11de821e19c231a1eb9ff67bfcec85 supplies owning contract context despite its ordered later merge. No assurance profile applies to this requirement correction.

## Verdict

PASS: the changed writer statement names the writer and concrete emitted references. Exact FR-1036 validation reports 1/1 docs grammar-clean, zero grammar findings. No event/state trigger ambiguity or vague response was found in the changed statement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-1036
  path: spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md
  role: examined
  excerpt: "- The writer SHALL write `candidate` as a reference to the `value`/`parameter`\n\
    \  node of the declaration's own name at level `0`, typed `D` for a value\n  synthesis\
    \ and the generated FR-800 plan-record type for a state-model or\n  hybrid plan\
    \ synthesis,\n  the binder S3 bound inside the block and the objective (FR-754).\
    \ The plan\n  candidate's `semantic_type` SHALL reference that generated plan-record\n\
    \  `composite_type`/`record` node defined by FSpec FR-322 \"Plan-record type\"\
    ,\n  with that section's field schema, applicable FR-819 extensions and\n  ordinary\
    \ generated-record identity, ownership and s"
- id: FR-1036-AC-2
  path: spec/functional/FR-1036-emit-a-checked-synthesis-declaration-as-a-v2-synthesis-request-node.md
  role: examined
  excerpt: The plan synthesis `Reach` over `Config::ConfigVersion` with domain `Lengths`
    emits a node whose `grammar` is the actual `model`/`object_type` node of `ConfigVersion`,
    whose `domain` is the `bounded_domain`/`integer_range` node of `Lengths`, and
    whose `candidate` references the `value`/`parameter` node named `Reach` at level
    `0`, with `semantic_type` referencing the generated FR-800 plan-record type node;
    the emitted package is admitted by FSL's reader, which returns that same candidate
    and plan-record type reference, grammar, domain, objective and block. Retyping
    the candidate to the `ConfigV
- id: FR-322
  path: spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: context_only
  excerpt: The plan candidate is not typed by `G`; the `grammar` binding
```

## Evidence limits

Exact-head runtime remains pending. No cargo/runtime run. Base checklist applied only to the changed candidate schema and AC2; original surrounding Verification cells and dependencies were context, not a fresh baseline audit. No structure, relationship or domain-object change requires additional lenses. Criterion-strength unavailable without Jev; skipped. Native model ID unavailable.

## Final disposition round 2 — static checkpoint

Frozen fix-round-2 head 89de0992412ab37f79c9e71e5616f62ea69e6e8b. This is the same independent reviewer, the second and final disposition round, not a new initial review. Original 29 findings and round-1 history remain unchanged. The only pending finding before this round was gap FND-009. Native model ID unavailable. No cargo, build, runtime, broad matrix, baseline diagnostic, code/spec edit, branch mutation, push or merge was run by reviewer.

No previously closed finding is reopened and no new finding is added. Prior accepted scope deferrals remain required on QSL-1978.

The seven native fixtures consistently use legal field number through declaration, contract, snapshot and readback. Counter.value_field defaults to legacy value, and native Counter consumers explicitly choose number. Declared intervals, canonical order, numeric expected values, receiver/ordinal, replay, certificate and accounting assertions are preserved. The arithmetic fixture uses the existing explicit Integer conversion; no grammar/profile widening.

Evidence at this static checkpoint: author-supplied exact-head corrective cargo check exited 0 in 1m57s; earlier failed compile is not a pass. Old e289 focused set completed 50 passed/7 native parser failures/0 ignored. The requested affected 10 at 89de099 have no result supplied yet. Historical 50 is not a full current-head pass. Actual new-main conflict integration is separately pending architecture repair ruling: only frames.rs conflict is reported; reviewer must inspect the final integration delta/head and affected runtime evidence before any final merge verdict. No main integration/build diagnosis was attempted by reviewer.

## Integrated final-head checkpoint

Final integrated frozen head e13100e4abbd4cb73b9029d300255a1ec7e43cca has parents 89de0992412ab37f79c9e71e5616f62ea69e6e8b and repaired main 8ba8b1bdd5fc4df61caf7ddd9ac1422203c30278; repair 5deeba7d06bfd9b99b9874a3766a302be9b0c3e2 is an ancestor. Integration review is part of this same final disposition round. The sole manual conflict preserves upstream EnabledVector.transition and its complete invocation identity in the shared apply path. Against 89, frames.rs changes only those three lines; native outcome retention and continue remain before legacy FirstResult/AllResults, with recommendations/enabled shortcuts absent in the native caller. The reader fix, negative regression and seven native fixtures are byte-unchanged; the checked_v2 canonical_bytes accessor and other accepted upstream automatic interface joins preserve this PR's paths. No new finding. No unrelated accepted-main audit. Final merge gate remains pending the one affected root3+native7 result at this exact integrated head.

## Final round 2 — authoritative disposition and gate

This final section supersedes prior checkpoint gate statements, preserving their history.

**FINAL ROUND-2 VERDICT: code approval CONDITIONAL only on the mandatory lead-owned premerge compile/current-main window. No finding-based blocker remains.** Reviewed final body head 0c00aca8d4751048f0494ff1bec379ea2aa1815c. All original 29 findings retain their round-1 outcomes (24 fixed, five accepted-scope deferrals). The sole new gap FND-009 is fixed; no later finding was added. Across all 30 finding records, latest outcomes are 25 fixed and five deferred. Original finding text and prior disposition rows are unchanged.

Actual focused runtime: at a673ebae5ead7378b78df2ad6e9a61cf92b45892, root3 passed and native6 passed, with only the single reserved-clause-name fixture failing before its assertions (combined rc101, 9 passed/1 failed/0 ignored). At 0c00aca8d4751048f0494ff1bec379ea2aa1815c, that one native definition reran with rc0, 1 passed/0 failed/0 ignored. The entire delta is one fixture string, post Set -> post Settles; model, interval, operation, postcondition body and every singleton-ordinal/wrong-receiver/replay/certificate assertion are unchanged. Thus all ten affected definitions have passing evidence, including the actual FR-1036 negative candidate-binding regression. Nine passes remain attributed to a673 and one to 0c00; this is not a full 57/58-test run at the final head. Old e289 50 passes remain historical; all 5795 evidence remains discarded. The e131 compile-stopped attempt executed zero runtime tests and is not a pass.

Mandatory premerge gate remains pending: latest SR custody must be committed first; the lead updates/true-rebases onto current origin/main if it moved, verifies any body-equivalent custody/rebase proof, and runs cargo check --locked --all-targets for filament-spec-language, fsl-analyze, fsl-command, fsl-eval, fsl-package, fsl-replay and fsl-semantics at the EXACT final custody head. Merge within ten minutes of PASS; repeat if main moves. Reviewer runs no cargo. No circular post-check SR commit is required or authorized by this report; final compile/time evidence belongs to the lead gate record.

Scoped code/Rust disposition is clear; scoped FR-1036 base and EARS wording reviews remain PASS. Gap disposition is clear within accepted staging, while full computed-matrix assurance remains CONDITIONAL: the earlier quoin matrix request was refused for accepted size; static extraction diagnostics and three ignored-only separately ordered criteria were not converted into passes. No matrix/baseline repeat or broad runtime gate was run in round2. Full QSL-1978 MUST stay In Progress for finite/reference/Enum/optional arguments, native inline bounds producer, Float/Quantity objectives, native numeric result domains, canonical positive policy monitors, object-creation universe, multi-object projection and remaining adapters. The shared contract-IR plan-record conformance vector remains existing QSL-1978 work; local FSL boundary and refusal locus now have actual negative evidence.
