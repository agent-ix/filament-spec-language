---
id: SR-202611210022442313144108558
title: "Gap analysis — bounded Tally projection"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2271 PR3388; changed bounded integer identity and public-route fixture"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language
    type: references
---

## Summary

Performed the requested bounded gap-analysis methods over the changed integer identity behavior, new regression and immediate requirements. No new traceability, reverse-gap or stub defect was identified; this is not an end-to-end repository gap audit.

Frozen candidate: 0641a1f7cf33dcb8c24f6b454d7ee21a7d8f58f0. Base: 736f9f4437936d243f7ba70e544a441eac94c763. Ticket: QSL-2271; PR: filament-spec-language#3388. Review is bounded to this two-file change and its immediate evidence; no whole-repository assurance claim is made.

Actual tools: /home/peter/.npm-global/bin/quoin 0.36.8; /home/peter/.npm-global/bin/quire 0.52.0 (engine 0.62.2); spec-artifacts-process@v0.36.0. The installed SpecReview authoring pack, skeleton and schema were read. No applicable AssuranceProfile was found. AGENTS.md, CLAUDE.md, CONTRIBUTING.md, README local-gate guidance, workspace lint policy and manual-dispatch CI were inspected.

## Verdict

**PASS** — no gap is identified in the requested changed slice; the Tally runtime filter passes at 85f67e6 and the unchanged nondegenerate integer control passed at 0641. The separate code-review finding is fixed. Mandatory exact-final-head compilation after current-main rebase remains a root-owned merge condition. This is no whole-repository gap-audit PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

Reconciliation: quire matrix --scope . --format json, quire 0.52.0 (engine 0.62.2), exit 0; no run evidence read. quire coverage --scope . --json, exit 0. Commands were run from the explicit repository scope and their JSON is retained alongside this artifact. The selected local criterion FR-940-AC-6 is tagged; its binders include package_send_preserves_bounded_tally_projection at line 2210. In-scope criterion status: tagged 1, untagged 0, tagged-by-ignored-test 0, method-without-symbol 0, qualified-record 0, retired 0. These are a bounded selection, not repository totals.

Coverage lists the new test's FR-093 and FR-094 whole-requirement tags as untracked symbols and the external FSpec-FR-322-AC-31 tag as unmatched locally. The former are contextual requirement references, while FR-940-AC-6 is the local criterion binder. The latter is the repository-mandated FSpec namespace, independently checked at the pinned external normative requirement, not an invented local FR-322-AC-31. No local binding or whole-repository coverage is claimed for the external tag. Both commands emit extensive existing document-selection and symbol-collision diagnostics outside this slice; those do not turn this bounded result into a full repository PASS. No fallback or old binary was used.

Reverse inventory: one changed private classification predicate/operand retention rule and one new public-route regression. The predicate implements FSpec FR-322's endpoint rule; the regression binds FR-940-AC-6 and externally documents FSpec FR-322-AC-31. No new endpoint, CLI flag, config, wire form, duplicate semantic implementation or unspecified production behavior is added. In this slice: untraced production behaviors 0, source stubs 0, test stubs 0. No coverage thresholds, ignores, mock boundaries or evidence collection were weakened.

Optional standalone semantic-review fanout was not run. Code-review's required spec/code/test alignment was inspected without launching extra reviewers. Scope excludes unrelated repository criteria and QSL-2271's other protocol gaps.

## Examined Scope

```json
[
  {
    "id": "package_send_preserves_bounded_tally_projection",
    "path": "fsl-analyze/tests/it/analyze_package_protocol.rs",
    "role": "examined",
    "excerpt": "fn package_send_preserves_bounded_tally_projection() {"
  },
  {
    "id": "retains_bound_text",
    "path": "fsl-semantics/src/check/lowering.rs",
    "role": "examined",
    "excerpt": "content.node_tag == NodeTag::BoundedDomain\n        && matches!(content.semantic_form, \"integer_range\" | \"decimal_range\")"
  },
  {
    "id": "FSpec-FR-322-AC-31",
    "path": "filament-specification@2bec006/spec/objects/interfaces/FR-322-checked-package-artifact.md",
    "role": "examined",
    "excerpt": "A resolved model member's type follows the declared-type, multiplicity and presence tables."
  },
  {
    "id": "FSpec-FR-322",
    "path": "filament-specification@2bec006/spec/objects/interfaces/FR-322-checked-package-artifact.md:3061",
    "role": "examined",
    "excerpt": "The repeated-operand rule for bounded numeric forms is uniform: each named\nlower and upper endpoint appears once in the body and once in the structural\npath, including `denominator_min` and `denominator_max` for\n`bounded_domain`/`rational_range`, just as `scale_min` and `scale_max` each\nappear for `decimal_range`. No endpoint may be elided because it equals its\npartner."
  },
  {
    "id": "FR-940-AC-6",
    "path": "spec/functional/FR-940-analyze-through-one-sealed-package-read-view.md",
    "role": "examined",
    "excerpt": "For each item family whose evaluator FR-531's inventory lists as taking a source-checked value (the temporal clauses `Bounded` and `AtMostTwo`, the invariant `NotAboveThree`, the deadlock-freedom item and the `SB` litmus claim under `tso`), a `program` request and a `package` request over the handoff directory of the same compiled unit give equal item records, counterexamples and replays, and no package-route record is `unsupported`; a handoff directory whose `checked-package.json` the strict reader refuses, or that holds no `identities.json`, settles no item and compiles no source (FR-850-AC-"
  },
  {
    "id": "FR-093",
    "path": "spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md",
    "role": "context_only",
    "excerpt": "Lower checked value expressions to FR-322 terms"
  },
  {
    "id": "FR-094",
    "path": "spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md",
    "role": "context_only",
    "excerpt": "Key model-owned, reference, population, clause-function and quantity type nodes"
  }
]
```

## Validation Limits

No Cargo, runtime or full gate was executed by the reviewer, as required by the dispatch. Root reports focused fsl-analyze IT cargo check at frozen 0641 exit 0 and rustfmt/diff whitespace checks exit 0. Reviewer independently ran git diff --check, exit 0. Root-owned two-filter runtime evidence is pending. Mandatory current-main rebase and exact final head cargo check --locked --all-targets for both touched crates plus fsl-analyze and filament-spec-language is still pending; no compile waiver or merge approval is granted. Root retains final live-base verification and matched-head admin merge within the required window. QSL-2271 remains In Progress for other protocol gaps.

## Disposition Pass 1

Candidate: 85f67e62b3dedd522dead486232b3cb4784e3d56. Independently inspected the complete 0641..85f67e6 diff: one test file, 6 additions and 14 deletions; production lowering is unchanged. The only change replaces the copied tail with channel_send_parity_using(&ws, &path, "proved", || ...). The closure creates the authentic package, strictly verifies its handoff and returns its request path. The unchanged helper performs Source analysis and requires record/proved before invoking that closure, then removes program.fsl and runs Package with complete item and exit parity. The original Tally and Attempt.n fixture, all eight Cell operations/contracts and all earlier tests are unchanged. No assertion or production path is weakened, and no new finding is identified in this fix.

Root-owned runtime receipt: TitanCode simple3388-0641-c1 at 0641a1f7cf33dcb8c24f6b454d7ee21a7d8f58f0 exited 0, 2 passed, 0 failed, 0 ignored: package_send_preserves_bounded_tally_projection and unchanged package_parallel_send_preserves_weak_model_reads. This supersedes the initial runtime-pending state for 0641. Root reports focused analyzer IT compilation at 85f67e6 exit 0 (27.78 seconds), plus rustfmt/diff checks exit 0. The changed Tally test's exact runtime rerun at 85f67e6 remains pending; the old-head nondegenerate control receipt is retained. Reviewer executed no Cargo or runtime and changed no reviewed worktree.

The mandatory current-main rebase followed by exact final head all-target compilation and timed live-base/matched-head merge remains root-owned and pending. This disposition resolves FND-001 statically; it does not claim those final checks have run.

## Final Bounded Runtime Receipt

Root-owned TitanCode simple3388-85f6-c1 at full head 85f67e62b3dedd522dead486232b3cb4784e3d56 exited 0: 1 passed, 0 failed, 0 ignored for the exact package_send_preserves_bounded_tally_projection filter. This confirms the helper-reuse fixture with genuine Source proof, strict native handoff admission, source-file removal and complete Package item/exit parity. The actual 0641 two-filter pass remains historical evidence; its unchanged package_parallel_send_preserves_weak_model_reads nondegenerate integer control is retained because production is byte-unchanged by the test-only fix.

FND-001 is fixed, with no further finding in disposition pass 1. The bounded review is PASS. Mandatory current-origin/main rebase, exact-final-head cargo check --locked --all-targets for the touched fsl-semantics and fsl-analyze crates plus filament-spec-language, live-base verification and matched-head merge within the required window remain root-owned conditions and have not been performed by this reviewer. This PASS grants no final compile waiver and makes no full repository assurance or QSL-2271 completion claim.
