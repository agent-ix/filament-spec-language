---
id: SR-1547
title: "Independent production code/Rust review found two functional regressions and one admission-order defect in the frozen diff."
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; PR3404; QSL-2758/QSL-2760; frozen 0d77f99a3895a12015850da6837e991e76c4a013; command/run, replay reference/certificate helpers, model owners, scoped FR-650/FR-753 additions"
review_set: subset
---

## Summary

Independent production code/Rust review found two functional regressions and one admission-order defect in the frozen diff.

Frozen source head: `0d77f99a3895a12015850da6837e991e76c4a013`; PR: agent-ix/filament-spec-language#3404; integrated review base: `21edb2ead1c8b69768beedaab003e077ce4ed042`. Tickets: QSL-2758 and QSL-2760. The review is diff-scoped; accepted prior specification/reference work was not repeated. No production code or frozen-branch file was changed.

## Verdict

**FAIL** — Fix the confined findings before merge.

The canonical-only command selection, exact-value reuse, coordinator entry, root named-backend propagation and unknown-backend refusal follow the promised design. The source and package paths retain one supplied run for membership and execution. Source normalization uses the original SelectedModels owner; snapshot rows retain individual NormalizedModel owners, and the result writer selects a declaration owner before reference encoding. The Mutex is justified by borrowed immutable owner views and fixed charging operations; its deep-clone behavior is preserved rather than used to reset the invocation counter. These clean observations do not discharge the findings below.

## Assurance Context

Repository AGENTS.md/CLAUDE.md and code-review/Rust review/style rules govern this pass. No applicable AssuranceProfile was identified by the scoped filename lookup; no profile/enforced-run conformance is claimed. The explicit no-rerun, bounded-scope, unchanged outside-run behavior and admission-before-allocation constraints from the dispatch apply.

## Examined Scope

```yaml
{
  "scope": [
    {
      "id": "FR-650-AC-18",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "Through the public run command, canonical Integer, Boolean, Text, Sequence and Set arguments complete with canonical values; a 23-digit integer remains exact, `4/6` completes as `2/3`, an empty sequence and one at its maximum retain order, and a set supplied as 3 then 1 completes as 1 then 3. A call with no arguments also writes its result canonically."
    },
    {
      "id": "FR-650-AC-19",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "The public run command refuses an untagged integer, a bare integer, a tagged integer with noncanonical decimal spelling, an unknown member, and an out-of-domain integer, each with exit 20. Request and dependent-byte exhaustion remain incomplete with no completed value."
    },
    {
      "id": "FR-650-AC-20",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "Genuine canonical argument membership and execution each fit a supplied admission-work limit independently, but their combined work on the same supplied run exceeds it and returns the typed supplied-value limit refusal. Reference codec conversion retains its separately cumulative conversion-work counter on that same run."
    },
    {
      "id": "FR-650-AC-21",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "A genuine checked-call reference catalog exhausts its original normalization owner's remaining work allowance and returns `NormalizationIncomplete` at `reference.catalog`, retaining the configured limit, prior consumption and denied charge."
    },
    {
      "id": "FR-650-AC-22",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "A checked-package call over a genuine admitted snapshot accepts a canonical reference and returns that same canonical reference; feeding the result into another package call completes unchanged after the original program file is removed. The catalog and writer use the same snapshot admission's retained normalization owners."
    },
    {
      "id": "FR-650-AC-23",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "A call with no models and no snapshot completes with its canonical value when `model.work_units` is zero; canonical output does not introduce model work into that value-only call."
    },
    {
      "id": "FR-650",
      "path": "spec/functional/FR-650-carry-every-argument-and-result-kind-of-a-run-in-the-native-value-grammar.md",
      "role": "examined",
      "excerpt": "The run value form has no depth limit. Admission reads an argument\n  value at any nesting depth through the shared reader (FR-261), bounded\n  only by the request's byte limit and the values-per-document setting, and\n  the writer writes a completed value at any depth in full, without a\n  recursion limit, truncation or a depth refusal."
    },
    {
      "id": "FR-753",
      "path": "spec/functional/FR-753-build-check-emit-and-execute-a-verification-plan.md",
      "role": "examined",
      "excerpt": "A run request's existing `requirements.backend` selector and its options SHALL apply to that caller negotiation for every negotiated plan step."
    },
    {
      "id": "TaskTransitionTemplate::to_wire",
      "path": "fsl-replay/src/certificate/task_product/wire.rs",
      "role": "examined",
      "excerpt": "pub fn to_wire(&self) -> Result<Value, TaskProductWireError> {"
    },
    {
      "id": "Forms::read_canonical",
      "path": "fsl-replay/src/spine/run_value/canonical.rs",
      "role": "examined",
      "excerpt": "        let normalized = self.canonical_input(\n            form,\n            declared,\n            &mut Path::new(),\n            &mut Position { argument, next: 0 },\n        )?;\n        self.read(&normalized, declared).map_err(|stop| match stop {"
    }
  ]
}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Context-free certificate APIs now reject references before existing analyze callers migrate. | fsl-replay/src/certificate/task_product/wire.rs:466 |
| FND-002 | high | Canonical completed-value output introduces fixed JSON and typed-value depth ceilings. | fsl-replay/src/reference_codec.rs:1612 |
| FND-003 | medium | Canonical argument adaptation builds a complete legacy JSON tree before supplied-run admission. | fsl-replay/src/spine/run_value/canonical.rs:19 |

### FND-001

The imported typed_value reference arm now always returns ReferenceContextRequired (line 342), and source_transition_wire newly applies that refusal to every receiver/argument. Consequently an existing native TaskTransitionTemplate with a reference can no longer use to_wire, and the old reader also rejects references. The new codec test explicitly asserts this refusal at reference_codec.rs:2166-2175. Existing fsl-analyze/src/zone_check/task_certificate.rs:501-515 still calls the old ZoneCertificate read/write APIs; analyze/task_product_item.rs:616-627 maps the failed roundtrip to fault. Thus a previously serializable reference-bearing task/source certificate faults on an unmigrated live path. New with_references APIs alone do not migrate that path. Fix: trim the unready carrier refusal hunks or preserve the unchanged outside-run behavior in this bounded run PR. New scoped helper APIs may remain for later adoption; no compatibility shim or wait for the entire QSL-2726 lane is needed.

### FND-002

write_run_value always routes the borrowed writer bytes through parse_text, including the no-snapshot admit_native_value_bytes path. The new serde_json::from_str uses its default recursion ceiling and maps a depth error to MalformedWire. A valid result with, for example, 100 nested Sequence levels can finish evaluation and then refuse output as invalid runtime input even when request-byte/value budgets admit it. With a catalog, value at lines 888-892 also clamps the requested usize::MAX depth to 64. FR-650 Depth and null explicitly requires writing at any depth without a recursion limit or depth refusal. Fix: use the existing depth-safe shared JSON admission path and an explicit work stack for conversion as needed, retaining the actual caller budgets/cancellation without importing the certificate-only depth policy into run output. Add a focused public-command deep-value regression on the repaired path.

### FND-003

read_canonical first recursively constructs normalized JSON and then constructs the typed Value through self.read. The new collection/map/composite branches allocate vectors/maps and clone payloads during this pass (for example lines 159-182 and 293-315), with no supplied run or admission budget available to the adapter. The supplied membership charge occurs only afterward in spine/call.rs:1520-1528. A large canonical collection therefore incurs the complete additional native-form allocation/traversal even when the invocation has no remaining supplied allowance. This is the preallocated legacy JSON then postcharge pattern excluded by the dispatch, not a request to change the existing kernel type rules. Fix: remove the retained legacy JSON staging tree, or make the adapter borrow the same supplied run and perform the applicable conversion/storage admission before the corresponding traversal/allocation. Preserve real argument/preorder loci and cumulative execution membership; do not add a fresh allowance.

## Evidence and Limits

Accepted evidence: 27 focused named passes reported by the author, with the focused-receipts document and original logs retained beside the PR body. Actual selected-plan and package-reference pass receipts were read, as was the shallow canonical-values pass receipt. The original borrowed-writer compile failure remains retained and is not counted as passing. No Cargo, build, test, lint, full gate, whole-repository matrix or peer-check rerun occurred. Root owns the final affected-crate plus fsl-analyze/filament-spec-language check and exact-head integration. The known reused held-claim failure remains owned by QSL-2744 / PR3403; it is not duplicated here. QSL-2760 remains open for real filament-driver adoption.


## Dispositions

### Round 1

Fix round 1 inspected only the deltas addressing the prior findings at frozen head `cc2e8b67e7af38e9b091dbfb8063bfb51579b0e5`. Accepted evidence: `review_fix_` 3/3 and the public depth-100 test 1/1, single-threaded, both exit 0, in `.agent/logs/review-fix-round1-replay-2.log` and `.agent/logs/review-fix-round1-command.log`. The preceding compile failure remains retained in `review-fix-round1-replay.log`; it is not counted as a pass. The original 27 passes and inherited held-claim owner repair `be3ad51571bb5e7b9c0e86588cfa32611053ed3d` were accepted without reruns. No review sweep, test, build, lint, matrix or gate was run. Spec reviews SR-1549/SR-1550 remain unchanged. QSL-2760 still requires actual driver adoption. Root retains the current-base/final-gate/arch-hold merge decision.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The imported unconditional reference refusal and extra writer scan were removed. Old native reference shape validation and direct task-source output are restored, so unchanged zone/analyze callers retain their prior behavior. The genuine-reference old-API roundtrip test passes; the accepted alternative of trimming the unready carrier behavior removes the caller gap without requiring analyze migration. |
| FND-002 | fixed | parse_text now uses the shared stack-backed reader through Admission::json_value; typed reference conversion uses explicit Visit/Finish tasks and compares depth only to the supplied max_depth. The hard-coded min(64) clamp is removed. The reported tests pass explicit depth-63 refusal, usize::MAX depth-100 acceptance, and real public-command depth-100 results both without models and with an authentic ConfigVersion reference. |
| FND-003 | fixed | Position borrows the same invocation SuppliedRun. WitnessPreflight and payload materialization charges precede canonical node staging; child scheduling precedes fallible vector reservation and record staging, with the actual argument and preorder locus retained. Post-staging typed membership and execution continue on that run. The new reader-path test passes a zero Conversion allowance on a 50,000-element argument at parameter 1, retains consumed=0 and admission_spent=0, and admits the allowed small counterpart. |

```yaml
{
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "after_excerpt": "pub(crate) fn source_transition_wire(\n    transition: &ModelTransition,\n) -> Result<Value, TaskProductWireError> {\n    crate::task_source_transition_wire(transition).map_err(|_| TaskProductWireError)\n}",
      "reason": "The imported unconditional reference refusal and extra writer scan were removed. Old native reference shape validation and direct task-source output are restored, so unchanged zone/analyze callers retain their prior behavior. The genuine-reference old-API roundtrip test passes; the accepted alternative of trimming the unready carrier behavior removes the caller gap without requiring analyze migration."
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "after_excerpt": "fn parse_text(text: &str, scope: &mut Scope<'_, '_>) -> Result<Value, ReferenceCodecError> {\n    match scope.with_admission(|admission| admission.json_value(text.as_bytes())) {\n        Err(ReferenceCodecError::Key {\n            source: ModelKeyError::Unsupported(json::MALFORMED),\n            ..\n        }) => Err(scope.fail(ReferenceFailure::MalformedWire)),\n        other => other,\n    }\n}",
      "reason": "parse_text now uses the shared stack-backed reader through Admission::json_value; typed reference conversion uses explicit Visit/Finish tasks and compares depth only to the supplied max_depth. The hard-coded min(64) clamp is removed. The reported tests pass explicit depth-63 refusal, usize::MAX depth-100 acceptance, and real public-command depth-100 results both without models and with an authentic ConfigVersion reference."
    },
    {
      "fnd": "FND-003",
      "outcome": "fixed",
      "after_excerpt": "        let node = position.next;\n        position.preflight(form, node)?;\n        position.next = position\n            .next\n            .checked_add(1)\n            .ok_or(ReadStop::Fault(\"canonical-preorder-fits-u64\"))?;",
      "reason": "Position borrows the same invocation SuppliedRun. WitnessPreflight and payload materialization charges precede canonical node staging; child scheduling precedes fallible vector reservation and record staging, with the actual argument and preorder locus retained. Post-staging typed membership and execution continue on that run. The new reader-path test passes a zero Conversion allowance on a 50,000-element argument at parameter 1, retains consumed=0 and admission_spent=0, and admits the allowed small counterpart."
    }
  ]
}
```

## Current Verdict

**PASS after fix round 1** — all three original findings are fixed; no new findings were identified in these confined fix deltas. Review permits merge once root's final integration gate and external hold are satisfied. The original Findings and original review verdict above remain historical and unchanged.
