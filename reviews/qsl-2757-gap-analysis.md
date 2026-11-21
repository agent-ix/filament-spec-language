---
id: SR-2757002
title: "QSL-2757 bounded gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2757; changed scheduling request resolver and focused tests; repository extraction unavailable"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language", type: references}
---
## Summary

Bounded manual inspection of changed behavior, owning FR-633/FR-962 contracts and focused tests. This is an incomplete gap-analysis method run, with no engine-backed repository-wide assurance conclusion.

## Verdict

**CONDITIONAL** — whole-repository matrix, coverage and advisory extraction remain unavailable from the previously observed hangs. The brief explicitly prohibits repeating those attempts. No full-repository PASS or runtime acceptance is claimed. The code-review artifact SR-2757001 records the provider-extent and false binding defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Ticket: QSL-2757; candidate identity follows the assigned real QSL team, branch qsl-2757-sched-negotiation and local QSL repository history. Tracker identity/body validation is deferred by the explicit user cooldown; no tracker query or write occurred. GitHub draft PR3375 and local clean HEAD both name 26d77a8965db025e6dcfb073d820b8f5f68676f0. Base 0b82e56f16ea8cdacaf5c64be7a86f61452e4fa3; three Rust files, +140/-1. Prior PR3372 merge 9421660c03 is in local ancestry.

All three changed files were examined, including their clean parts. Source and checked-package selection converge on resolve_schedule_claim. EN-7-only and the exact EN-6/EN-7 unnamed tie yield EN-7 with backend absent; explicit EN-6 remains bounded. Mixed three-provider sets remain whole because resolve_tie is unchanged. Default items re-enter resolve_one. Other claim families, manifests, pins, dependencies, budgets and CI are unchanged. No applicable AssuranceProfile was found under spec/. No new public type, unsafe operation, asynchronous work, input decoder or dependency is introduced.

The new decorator asserts extent before delegating to the existing Fr290 double and checks the real Engines::fsl manifest. Replacing the production extent with its old Bounded value makes that assertion fail. Eight requested source/package combinations inspect proof certification or concrete miss evidence; they have not executed. This establishes a useful static test design, not real CG negotiation or production CLI acceptance. The immutable driver pin is outside this PR and cannot qualify this head.

The implementation clippy log was read: four E0599 errors in unchanged fsl-package/src/bare.rs:218,219,222,229. No repository Cargo checks were repeated against this known prerequisite failure. Zero runtime cases executed. Existing commands.txt names the two focused tests to run after the owning repair and rebase. git diff --check origin/main...HEAD passed independently. Formatter success is implementation-reported, not independently rerun. Full QSL-2757 ticket acceptance remains unassessed until its body is available.

- Plan completion: not assessed
- Reconciliation: no current engine-backed matrix/coverage result. Prior matrix/coverage/advisory extraction hangs are inherited limitations; not retried as explicitly instructed. Manual tag inspection is provisional and is not a computed matrix.
- Criteria: repository-wide tagged/untagged/ignored counts unknown. No counts inferred from grep.
- Evidence: no runtime evidence for this candidate; no evidence store read.
- Bounded reverse inventory: one changed production behavior, schedule request extent resolution, owned by FR-633 with FR-962's default tie context; no newly unspecified behavior found within that inventory. Repository-wide public-surface inventory incomplete.
- Bounded source/test stubs: zero found among the one changed resolver and one new regression; modified existing control also contains behavioral assertions. No repository-wide stub count claimed.
- Hollow evidence: pre-negotiation decorator checks the production item; proof/miss checks follow actual fsl_command analysis. It delegates settlement to Fr290, so it is not real CG acceptance. False FR-633-AC-1 binding is recorded in code-review SR-2757001 rather than counted twice.
- Semantic review: optional whole-repository semantic pass skipped; bounded code/test faithfulness is part of the required code review.
- Spec-review and sub-analyses: not applicable; there are no spec/plan changes.
- AssuranceProfile: none applicable found; no measurement requested or performed.

## Examined scope

```yaml
scope: [
  {
    "id": "FR-633",
    "path": "spec/functional/FR-633-request-schedulability-and-response-time-items.md",
    "role": "examined",
    "excerpt": "A `native-analyze/1` item naming a checked schedulability or response claim\n(FR-246) SHALL run on EN-7, which `Engines::fsl()` registers under `en-7`\n(FR-620), and be decided in closed form: fixed-priority response-time\nanalysis with its busy period, EDF with the processor-demand test and QPA,\nand AMC-rtb (FR-247)."
  },
  {
    "id": "FR-633-AC-1",
    "path": "spec/functional/FR-633-request-schedulability-and-response-time-items.md",
    "role": "examined",
    "excerpt": "ADR-026 §12's `Ctl` task set with `schedulable CtlFp on Ctl under fixed_priority;` settles `proved` with basis `closed-form` `fixed-priority`, certification `certified`, on backend `en-7`."
  },
  {
    "id": "FR-633-AC-2",
    "path": "spec/functional/FR-633-request-schedulability-and-response-time-items.md",
    "role": "examined",
    "excerpt": "A response claim that holds at 12 ms settles `proved`, and the same claim at 11 ms settles `refuted` with the task, response time 12 and deadline 11 in its counterexample."
  },
  {
    "id": "FR-633-AC-3",
    "path": "spec/functional/FR-633-request-schedulability-and-response-time-items.md",
    "role": "examined",
    "excerpt": "`schedulable CtlEdf on Ctl under edf;` settles `proved` with basis `closed-form` `edf`."
  },
  {
    "id": "FR-962-AC-6",
    "path": "spec/functional/FR-962-resolve-an-unnamed-multi-candidate-item-by-the-production-tie-rule.md",
    "role": "examined",
    "excerpt": "An unnamed `schedulability` item whose matching set is `[en-6, en-7]` gets the candidate set `[en-7]`, writes `candidates` `{type: \"set\", backends: [\"en-7\"]}` with `backend` absent, and settles through EN-7, never `ambiguous-backend`; over a registry that also holds a process provider advertising `schedulability`, the three-member set stays whole and settles `ambiguous-backend`."
  },
  {
    "id": "resolve_items",
    "path": "fsl-analyze/src/analyze/document.rs",
    "role": "examined",
    "excerpt": "Resolve `selections` (FR-620 \"Resolving items\") over `package`, the\nrequest's default `subject` and the caller's `registry`"
  },
  {
    "id": "FR-246",
    "path": "spec/functional/FR-246-check-task-sets-and-schedulability-claims.md",
    "role": "context_only",
    "excerpt": "FSL's S3 checker SHALL admit a package's task sets for one preemptive\nuniprocessor and the schedulability and response claims over them, and\nroute each claim to the closed-form engine EN-7, module `schedulability`\nin layer A, crate `fsl-analyze`,"
  },
  {
    "id": "FR-247",
    "path": "spec/functional/FR-247-analyse-a-task-set-in-closed-form.md",
    "role": "context_only",
    "excerpt": "EN-7 SHALL decide schedulability and response claims in exact rational\narithmetic by fixed-priority response-time analysis, the EDF\nprocessor-demand test with Quick Processor-demand Analysis, and AMC-rtb\nfor dual-criticality task sets (ADR-026 RT-4 to RT-6)."
  },
  {
    "id": "FR-248",
    "path": "spec/functional/FR-248-settle-a-closed-form-verdict-after-recomputing-its-evidence.md",
    "role": "context_only",
    "excerpt": "`fsl_replay::check_closed_form`, a layer-6 facade entry, SHALL recompute\nEN-7's evidence in exact rationals before a schedulability verdict settles,"
  },
  {
    "id": "closed_form_schedule_requests_match_en7_before_negotiation",
    "path": "fsl-analyze/tests/it/analyze_family_cli.rs",
    "role": "examined",
    "excerpt": "fn closed_form_schedule_requests_match_en7_before_negotiation()"
  },
  {
    "id": "a_release_backed_claim_routes_to_en_6_only_when_named",
    "path": "fsl-analyze/tests/it/task_automaton_lowering.rs",
    "role": "examined",
    "excerpt": "fn a_release_backed_claim_routes_to_en_6_only_when_named()"
  }
]
```

## Deferred publication

Marked comment prepared locally only. Tracker identity/body validation, dedup, posting, has-review and read-back remain deferred under the user instruction.


## Disposition pass 1 evidence

Frozen disposition head f35c2326cdfc297a7ccfeec1dacafcf3c9a8c8ca, repaired base 5deeba7d06bfd9b99b9874a3766a302be9b0c3e2. Local clean HEAD and GitHub PR3375 head matched. Five changed files inspected; +217/-2.

The code now classifies only a sole resolved EN-7 candidate as Unbounded(false). The actual FR-962 tie runs first: exactly [en-6,en-7] becomes [en-7], therefore unbounded as intended. EN-7-only registries also become unbounded. An EN-6-only registry, explicit EN-6, unknown/empty sets and unrecognized mixed sets keep their previous Bounded classification. In particular a third schedulability provider is not removed or selected: resolve_tie and registry matching are unchanged, so a multi-member set retains its prior ambiguous negotiation behavior and prior extent. Other claim-family paths are untouched.

The new EN-6 control uses the real descriptor from Engines::fsl and public resolve_items over explicit, unnamed and default selections. Each selection yields exactly one PairFp claim, bounded extent, sole en-6 candidate, and the expected presence/absence of backend. It does not call the EN-6 proof engine. The five source EN-7 tests assert before Fr290 settlement, then verify actual certified proofs or exact miss values. The new CtlFp uses the ADR-026 task parameters 1/4, 2/6 and 5/12 under fixed priority.

Two authorized constructor additions are faithful to the fixtures: observation.rs has empty relationships and required_relationships, max_relationships 0, and no later relationship mutation, so relationship_extents: vec![] is appropriate. The monitor_root fixture starts from the ConfigVersion domain, adds Tier as an enumeration and a tier field, and reaches that enum; it adds no model union. model_unions: &[] therefore supplies the absent model-union rows without replacing any real producer getter. Existing model tuple/source union/enum rows continue to use their owning getters. No production change or weakening of assertions occurs in these two lines.

Evidence inspected: final2-compile.rc=0; compiler-artifact JSON names the integration test executable; final2-runtime.source equals the frozen full head; final2-runtime.binary records executable SHA-256 0175f0ccdcda7968448ad4b7d741f513539d79ba6660e715f9816dc006cd74e3. final2-runtime.rc=0 and log reports the exact six scoped tests passed, zero failed/ignored/measured, 2695 filtered. Runtime was externally capped at 15 seconds. These are read-back of implementation-owned local logs, not an independent rerun; no extra Cargo or baseline run occurred. git diff --check on the repaired-base diff passed independently. The eight inherited off-diff strict-clippy lints remain a disclosed, owner-accepted scoped limitation; no lint allows or threshold changes appear in this diff.

Correction to the initial static coverage inference: checked-package selection does NOT currently reach this schedule resolver for these declarations; it explicitly refuses the unsupported declaration-family/package form before negotiation. The earlier source+package convergence/positive package expectation is withdrawn. The final candidate scopes its regressions and claims to source routes and preserves the existing package refusal. This PR does not add package-form support.

Owner-supplied bounded acceptance is the four staged source examples reaching their expected ready exits/verdicts, with one focused FSL test per route and a reviewed merge. The four separate source regressions meet the in-repository test part. Real CG negotiation and actual driver CLI example acceptance remain unverified here and require consumption of the merged FSL pin. The old immutable CLI is not evidence for this head. Full tracker-body acceptance remains unavailable during the no-Linear period.

The source fix introduces no additional unspecified behavior within the bounded reverse inventory. Both new constructor fields describe empty sets already true of their fixtures. The new trace defect is recorded once in code-review SR-2757001 FND-003. No findings were added to this gap artifact. Prior whole-repository extraction and inventory limitations remain; no full repository PASS is claimed.

## Disposition pass 1 examined scope

```yaml
scope: [
  {
    "id": "resolve_schedule_claim",
    "path": "fsl-analyze/src/analyze/document.rs",
    "role": "examined",
    "excerpt": "let extent = if matches!(candidate_backends.as_slice(), [backend]\n        if backend.id().as_str() == crate::schedulability::manifest::BACKEND_ID)\n    {\n        ExtentClassification::Unbounded {\n            finite_bound_available: false,\n        }\n    } else {\n        ExtentClassification::Bounded\n    };"
  },
  {
    "id": "fixed_priority_schedulable_matches_en7_before_negotiation",
    "path": "fsl-analyze/tests/it/analyze_family_cli.rs",
    "role": "examined",
    "excerpt": "#[trace(\"TC-1593\", \"FR-633-AC-1\")]\n#[test]\nfn fixed_priority_schedulable_matches_en7_before_negotiation() {\n    closed_form_schedule_request(\"CtlFp\", \"proved\", 0, \"fixed-priority\");\n}"
  },
  {
    "id": "FR-633-AC-3",
    "path": "spec/functional/FR-633-request-schedulability-and-response-time-items.md",
    "role": "examined",
    "excerpt": "`schedulable CtlEdf on Ctl under edf;` settles `proved` with basis `closed-form` `edf`."
  },
  {
    "id": "FR-247-AC-2",
    "path": "spec/functional/FR-247-analyse-a-task-set-in-closed-form.md",
    "role": "examined",
    "excerpt": "`Ctl` with the third WCET 6 returns, under fixed priority, `Miss` for the third task, job 0, iterates `6, 10, 13`. A set of `a` (`C = 1`, `T = D = 4`, priority 1) and `b` (`C = 2`, `T = D = 6`, jitter 1, blocking 3, priority 2) returns `Miss` for `b`, job 0, iterates `5, 7` (response `8 > 6`), and `check_closed_form` accepts that evidence; under EDF, `Miss(Utilization(13/12))`."
  },
  {
    "id": "observation fixture constructor",
    "path": "fsl-analyze/tests/it/monitor_authority/observation.rs",
    "role": "examined",
    "excerpt": "relationship_extents: vec![],"
  },
  {
    "id": "model enum fixture constructor",
    "path": "fsl-analyze/tests/it/monitor_root.rs",
    "role": "examined",
    "excerpt": "model_unions: &[],"
  },
  {
    "id": "an_en6_only_registry_keeps_unnamed_schedule_requests_bounded",
    "path": "fsl-analyze/tests/it/task_automaton_lowering.rs",
    "role": "examined",
    "excerpt": "fn an_en6_only_registry_keeps_unnamed_schedule_requests_bounded()"
  }
]
```

## Disposition pass 1 verdict

**CONDITIONAL** — scoped evidence has improved to six logged passes at the repaired frozen head, but repository matrix/coverage/advisory extraction remains unavailable and was not repeated. Plan completion: not assessed. No dispositions tracker comment is required for the placeholder-only gap artifact; its initial marked local draft is retained unposted.

## Disposition pass 2 evidence

Final frozen head b8ef1b9553af3822c353882658f50c2fd7569129 verified against clean local HEAD and GitHub PR3375. The complete diff from runtime-qualified f35c2326cdfc297a7ccfeec1dacafcf3c9a8c8ca removes only the false FR-633-AC-3 attribute argument from the EDF-over refutation test and retains TC-1593 / FR-247-AC-2. The separate EDF-full test retains FR-633-AC-3. No test body, assertion, production code or fixture changed. git diff --check passed independently. No new finding.

The six focused passing tests remain tied to f35c2326cdfc297a7ccfeec1dacafcf3c9a8c8ca and its recorded executable hash; runtime was not rerun on b8ef1b9553af3822c353882658f50c2fd7569129 because the only change is the trace attribute correction. This is not a claim of a runtime execution at the final head. The repaired base remains 5deeba7d06bfd9b99b9874a3766a302be9b0c3e2. The eight owner-accepted inherited off-diff clippy errors and whole-repository extraction limitations remain disclosed.

Driver/CG consumption and actual four staged CLI-case acceptance remain external dependencies. The immutable old driver cannot execute this FSL fix, and no old binary result is attributed to it. Checked-package declaration-family support remains explicitly unsupported and outside this change. No Linear query/write, comment dedup, label operation or tracker read-back occurred. These operations remain deferred until the user's permitted time and subsequent authorization/workflow. Initial findings, initial marked comments and prior disposition rows remain unchanged.

This final tag correction removes the previously recorded false proved-result binding; the gap artifact has no new finding or disposition row. Code-review SR-2757001 records FND-003 fixed. No additional source behavior was introduced, and there is no reason to repeat matrix/extraction or Cargo checks for the attribute removal.

## Final gap-analysis verdict

**CONDITIONAL** — the bounded changed-behavior inventory and manual test/code inspection are clear; whole-repository matrix, coverage, advisory extraction and reverse inventory remain incomplete as explicitly accepted for this scoped review. Plan completion: not assessed. No full repository PASS is claimed. No dispositions tracker comment is needed for this artifact's placeholder-only finding table.
