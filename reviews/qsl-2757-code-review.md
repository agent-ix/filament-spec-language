---
id: SR-2757001
title: "QSL-2757 code review with Rust lane"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2757; fsl-analyze/src/analyze/document.rs; fsl-analyze/tests/it/analyze_family_cli.rs; fsl-analyze/tests/it/task_automaton_lowering.rs"
review_set: subset
---
## Summary

Static review of the frozen candidate and bounded owned request-routing contracts. Two edge/evidence defects were found; runtime qualification is blocked by an unchanged prerequisite.

## Verdict

**CONDITIONAL (static only)** — two medium findings; no merge/runtime acceptance claimed. Resume on the rebased frozen candidate after repair, checking every finding and the focused qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Extent depends on an explicit backend spelling rather than the resolved provider: a release-backed claim resolved through the public resolve_items API with a caller registry containing only the real EN-6 descriptor and no named backend keeps candidate [en-6], but is now classified Unbounded although that descriptor advertises schedulability only in Bounded mode. Its request therefore mismatches its actual sole candidate before negotiation. Narrow the Unbounded adjustment to a resolved EN-7 selection and preserve the prior classification for other sets; cover the EN-6-only registry control. | fsl-analyze/src/analyze/document.rs:2511 |
| FND-002 | medium | The new regression claims FR-633-AC-1, but its only schedulable claims use EDF; fixed_priority appears only on response claims. It never exercises AC-1's fixed-priority schedulable CtlFp route, so this is an incorrect test-to-criterion binding even though the separate existing Ctl test covers that AC. Remove this tag or exercise the actual AC-1 case; retain the useful EDF/response regression. | fsl-analyze/tests/it/analyze_family_cli.rs:535 |

## Coverage

Ticket: QSL-2757; candidate identity follows the assigned real QSL team, branch qsl-2757-sched-negotiation and local QSL repository history. Tracker identity/body validation is deferred by the explicit user cooldown; no tracker query or write occurred. GitHub draft PR3375 and local clean HEAD both name 26d77a8965db025e6dcfb073d820b8f5f68676f0. Base 0b82e56f16ea8cdacaf5c64be7a86f61452e4fa3; three Rust files, +140/-1. Prior PR3372 merge 9421660c03 is in local ancestry.

All three changed files were examined, including their clean parts. Source and checked-package selection converge on resolve_schedule_claim. EN-7-only and the exact EN-6/EN-7 unnamed tie yield EN-7 with backend absent; explicit EN-6 remains bounded. Mixed three-provider sets remain whole because resolve_tie is unchanged. Default items re-enter resolve_one. Other claim families, manifests, pins, dependencies, budgets and CI are unchanged. No applicable AssuranceProfile was found under spec/. No new public type, unsafe operation, asynchronous work, input decoder or dependency is introduced.

The new decorator asserts extent before delegating to the existing Fr290 double and checks the real Engines::fsl manifest. Replacing the production extent with its old Bounded value makes that assertion fail. Eight requested source/package combinations inspect proof certification or concrete miss evidence; they have not executed. This establishes a useful static test design, not real CG negotiation or production CLI acceptance. The immutable driver pin is outside this PR and cannot qualify this head.

The implementation clippy log was read: four E0599 errors in unchanged fsl-package/src/bare.rs:218,219,222,229. No repository Cargo checks were repeated against this known prerequisite failure. Zero runtime cases executed. Existing commands.txt names the two focused tests to run after the owning repair and rebase. git diff --check origin/main...HEAD passed independently. Formatter success is implementation-reported, not independently rerun. Full QSL-2757 ticket acceptance remains unassessed until its body is available.

Duplication inspection found no new helper duplicating production routing, manifest/catalog snapshot, copied external fixture, gate weakening, warning suppression or stub. Existing package_selection owns the source-to-handoff conversion; the negotiation decorator composes the established double instead of replacing extent resolution. Fr290 does not inspect modes; the added precondition assertion is therefore necessary and meaningful.

The EN-6-only finding is about the caller-registry resolution API and its emitted request, not a claim that unnamed task-product analysis is the default supported CLI path. A complete linked registry still selects EN-7 as intended.

The new four-case regression uses Full/Over and response bounds 5/4. It is supplementary evidence, not literal execution of Ctl and 12/11 criteria. Existing schedulability_proves_on_en7_and_a_tight_response_refutes was read and does assert CtlFp and 12/11. The AC-1 tag on the new test is specifically wrong because the schedulable fixed-priority form is absent altogether.

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
bindings:
  - {test_id: closed_form_schedule_requests_match_en7_before_negotiation, ac_id: FR-633-AC-1, trace: wrong}
```

## Deferred publication

Both marked method comments are prepared locally. Comment dedup, posting, read-back and has-review label inspection/update are deferred, not passed. No GitHub findings were posted. No branch edits, pushes, merge or relay operations occurred.


## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The refuted EDF-over regression is tagged FR-633-AC-3, which requires a proved EDF claim, while this test selects OverEdf and asserts refuted with utilization 11/10. This negative case cannot verify the criterion's proved result. Remove FR-633-AC-3 from this test's tags and retain its EDF-miss ownership under FR-247-AC-2; the separate EDF-full test covers the proved route. | fsl-analyze/tests/it/analyze_family_cli.rs:597 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: the actual sole resolved candidate selects EN-7 extent; the real EN-6-only control verifies explicit, unnamed and default requests stay bounded. |
| FND-002 | fixed | Round 1: a separate tagged CtlFp test now executes the actual fixed-priority schedulable claim and asserts certified fixed-priority proof. |
| FND-003 | still-open | Round 1: remove the proved-result AC tag from the EDF refutation test; retain the negative-case tag. |
| FND-003 | fixed | Round 2: removed FR-633-AC-3 from the refuted EDF-over test; retained its EDF-miss trace ownership and the separate proved EDF-full criterion binding. |

## Disposition pass 1 evidence

Frozen disposition head f35c2326cdfc297a7ccfeec1dacafcf3c9a8c8ca, repaired base 5deeba7d06bfd9b99b9874a3766a302be9b0c3e2. Local clean HEAD and GitHub PR3375 head matched. Five changed files inspected; +217/-2.

The code now classifies only a sole resolved EN-7 candidate as Unbounded(false). The actual FR-962 tie runs first: exactly [en-6,en-7] becomes [en-7], therefore unbounded as intended. EN-7-only registries also become unbounded. An EN-6-only registry, explicit EN-6, unknown/empty sets and unrecognized mixed sets keep their previous Bounded classification. In particular a third schedulability provider is not removed or selected: resolve_tie and registry matching are unchanged, so a multi-member set retains its prior ambiguous negotiation behavior and prior extent. Other claim-family paths are untouched.

The new EN-6 control uses the real descriptor from Engines::fsl and public resolve_items over explicit, unnamed and default selections. Each selection yields exactly one PairFp claim, bounded extent, sole en-6 candidate, and the expected presence/absence of backend. It does not call the EN-6 proof engine. The five source EN-7 tests assert before Fr290 settlement, then verify actual certified proofs or exact miss values. The new CtlFp uses the ADR-026 task parameters 1/4, 2/6 and 5/12 under fixed priority.

Two authorized constructor additions are faithful to the fixtures: observation.rs has empty relationships and required_relationships, max_relationships 0, and no later relationship mutation, so relationship_extents: vec![] is appropriate. The monitor_root fixture starts from the ConfigVersion domain, adds Tier as an enumeration and a tier field, and reaches that enum; it adds no model union. model_unions: &[] therefore supplies the absent model-union rows without replacing any real producer getter. Existing model tuple/source union/enum rows continue to use their owning getters. No production change or weakening of assertions occurs in these two lines.

Evidence inspected: final2-compile.rc=0; compiler-artifact JSON names the integration test executable; final2-runtime.source equals the frozen full head; final2-runtime.binary records executable SHA-256 0175f0ccdcda7968448ad4b7d741f513539d79ba6660e715f9816dc006cd74e3. final2-runtime.rc=0 and log reports the exact six scoped tests passed, zero failed/ignored/measured, 2695 filtered. Runtime was externally capped at 15 seconds. These are read-back of implementation-owned local logs, not an independent rerun; no extra Cargo or baseline run occurred. git diff --check on the repaired-base diff passed independently. The eight inherited off-diff strict-clippy lints remain a disclosed, owner-accepted scoped limitation; no lint allows or threshold changes appear in this diff.

Correction to the initial static coverage inference: checked-package selection does NOT currently reach this schedule resolver for these declarations; it explicitly refuses the unsupported declaration-family/package form before negotiation. The earlier source+package convergence/positive package expectation is withdrawn. The final candidate scopes its regressions and claims to source routes and preserves the existing package refusal. This PR does not add package-form support.

Owner-supplied bounded acceptance is the four staged source examples reaching their expected ready exits/verdicts, with one focused FSL test per route and a reviewed merge. The four separate source regressions meet the in-repository test part. Real CG negotiation and actual driver CLI example acceptance remain unverified here and require consumption of the merged FSL pin. The old immutable CLI is not evidence for this head. Full tracker-body acceptance remains unavailable during the no-Linear period.

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
bindings:
  - {test_id: fixed_priority_schedulable_matches_en7_before_negotiation, ac_id: FR-633-AC-1, trace: correct}
  - {test_id: schedulability_edf_full_matches_en7_before_negotiation, ac_id: FR-633-AC-3, trace: correct}
  - {test_id: schedulability_edf_over_matches_en7_before_negotiation, ac_id: FR-633-AC-3, trace: wrong}
```

## Disposition pass 1 verdict

**CONDITIONAL** — original findings fixed; production source is sound within the bounded reviewed contract, with six focused tests passing in the supplied log. FND-003 remains open in test trace metadata. Tracker publication, dedup, has-review and read-back remain deferred; none was attempted. Initial finding text and initial comment payloads are retained unchanged.

## Disposition pass 2 evidence

Final frozen head b8ef1b9553af3822c353882658f50c2fd7569129 verified against clean local HEAD and GitHub PR3375. The complete diff from runtime-qualified f35c2326cdfc297a7ccfeec1dacafcf3c9a8c8ca removes only the false FR-633-AC-3 attribute argument from the EDF-over refutation test and retains TC-1593 / FR-247-AC-2. The separate EDF-full test retains FR-633-AC-3. No test body, assertion, production code or fixture changed. git diff --check passed independently. No new finding.

The six focused passing tests remain tied to f35c2326cdfc297a7ccfeec1dacafcf3c9a8c8ca and its recorded executable hash; runtime was not rerun on b8ef1b9553af3822c353882658f50c2fd7569129 because the only change is the trace attribute correction. This is not a claim of a runtime execution at the final head. The repaired base remains 5deeba7d06bfd9b99b9874a3766a302be9b0c3e2. The eight owner-accepted inherited off-diff clippy errors and whole-repository extraction limitations remain disclosed.

Driver/CG consumption and actual four staged CLI-case acceptance remain external dependencies. The immutable old driver cannot execute this FSL fix, and no old binary result is attributed to it. Checked-package declaration-family support remains explicitly unsupported and outside this change. No Linear query/write, comment dedup, label operation or tracker read-back occurred. These operations remain deferred until the user's permitted time and subsequent authorization/workflow. Initial findings, initial marked comments and prior disposition rows remain unchanged.

Fixed text:

```rust
#[trace("TC-1593", "FR-247-AC-2")]
#[test]
fn schedulability_edf_over_matches_en7_before_negotiation() {
    closed_form_schedule_request("OverEdf", "refuted", 10, "edf");
}
```

## Final code-review verdict

**PASS (bounded code/Rust review)** — FND-001, FND-002 and FND-003 are explicitly fixed by their latest disposition rows. No open source or test finding remains in the reviewed five-file scope. Six focused runtime passes are recorded at f35c2326, followed solely by the verified trace correction at the final frozen head. This is source/review clearance, not full repository assurance, a clean strict-clippy claim, real CG negotiation, or actual driver CLI acceptance.
