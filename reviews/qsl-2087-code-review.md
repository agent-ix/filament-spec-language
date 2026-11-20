---
id: SR-1491
title: "Code review — certificate and witness boundary cases"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2087; .agent/receipts/qsl-2087/{scope,validation}.md; fsl-analyze/tests/it/{closure_certificate,exact_probabilistic_item,hyper_replay,probabilistic_replay}.rs and each certificate_boundary_cases.rs child"
review_set: subset
---

## Summary

Reviewed the ten-file frozen QSL-2087 diff at `716f44b511daec8cf8028b7f8abd7094be078d68`, applying code-review with the Rust lane. Eight new tests use real EN-1/EN-5 evidence and actual replay/checking APIs, with independently expected outcomes and typed adverse results.

## Verdict

**PASS** — no defect found in the changed tests, registrations or scoped receipts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review evidence

The EN-1 cases preserve the actual admitted model state while challenging noncanonical instance context, and distinguish equal model-state sets under different instance bindings. The EN-5 Bellman mutation uses the normative 4/5 to 9/10 change and expects the first affected canonical key, the exact rejection rule, and an inconclusive terminal with no successful evidence. The transport case checks canonical byte equality and independently checks the returned certificate to the golden 24/25 AtLeast result. Rank corruption keeps all value rows intact and expects RankBroken at the first live rank. The byte-boundary case admits exactly the produced length and rejects one byte below it with exact bound and actual fields. Hyper binding changes retain actual refuting traces; a successful original replay is required before foreign, absent and duplicate instance cases. The probabilistic initial-state mutation uses an actual distinct terminal state/key and cannot pass by rejecting the original witness.

Existing parent helpers supply real checked subjects, engine outcomes, and replay facades. The only helper added is the local bounded document-limit fixture. No external fixture, schema, binary, production fallback, alternate evaluator, warning suppression, unsafe code, nondeterministic timing oracle, gate weakening or test-only production bypass is introduced. Expectations and indexing intentionally fail when fixture or outcome invariants cease to hold; they are not production panic paths.

## Evidence and limits

Accepted the author's eight focused passes and original pre-execution tuple-field compile correction without rerunning them. No tests, builds, lint, matrix, whole-repository inventory, main-refresh check or old Done-ticket audit was run by this reviewer. Existing analyze/semantics lint limitations remain disclosed; this is not a green lint claim. One final affected-component gate remains with the lead. Review scope is the diff and exact current criteria, with no assurance-profile inventory or qualification claim.

## Examined scope

```yaml
{
  "scope": [
    {
      "id": "FR-338-AC-9",
      "path": "spec/functional/FR-338-check-an-en-1-closure-certificate.md",
      "role": "examined",
      "excerpt": "Over populations `P1` with universe `[\"a\"]` and `P2` with universe `[\"b\"]`, the two instances' contexts differ exactly in the key, and a certificate state carrying `a`'s context is not admitted under `b`'s instance (a foreign context, AC-5)."
    },
    {
      "id": "FR-201-AC-1",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "§15.4 at threshold 0.95: the `Exact` certificate with values `24/25`, `4/5`, 1 at delivered states and 0 at the two-loss state is accepted with side `AtLeast`. The same certificate with `4/5` replaced by `9/10` is rejected, naming the first state in canonical order where the minimum over actions differs from the certified value; the item settles `inconclusive`, `CertificateRejected`."
    },
    {
      "id": "FR-201-AC-6",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "Rejection rules and loci: AC-1's certificate with `4/5` replaced by `9/10` is rejected `InequalityFails` at the `ProbabilisticProductState` of that first state;"
    },
    {
      "id": "FR-201-AC-8",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "Over each of those ACs' own model, the checked-subject form gives the verdict, rule and locus that AC states: AC-1's certificate over §15.4's `Link`, including the `4/5` replaced by `9/10` rejection;"
    },
    {
      "id": "FR-201-AC-9",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "For an admitted acyclic eventual claim, writing and reading the closed profile preserves identity, exact rational values, complete keys, targets and required ranks byte for byte. Duplicate/out-of-order rows, unreduced 2/4, denominator 0, missing ranking, extra fields, padded base64url and a model-only key are rejected."
    },
    {
      "id": "FR-201-AC-10",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "A valid document offered for another identity refuses before enumeration; replacing its targets or optimization while retaining valid Bellman rows refuses objective mismatch. A same-model key with a different binding is rejected rather than merged. Altering a value fails equality; removing or breaking positive-value rank progress fails ranking even when equalities hold."
    },
    {
      "id": "FR-201-AC-11",
      "path": "spec/functional/FR-201-check-a-probability-certificate.md",
      "role": "examined",
      "excerpt": "Pre-cancelled input and exhausted byte/state/edge/arithmetic budgets stop without a truth value."
    },
    {
      "id": "FR-202-AC-10",
      "path": "spec/functional/FR-202-replay-a-probabilistic-witness.md",
      "role": "examined",
      "excerpt": "In AC-9, altering the claim occurrence, package or binding refuses; altering an action, draw, post-state or full key cannot produce a token. Duplicate/prefix-overlapping paths, inconsistent scheduler choices, an earlier target on a first-hit path, and a live non-target leaf cannot reproduce a refutation. Mass equal to the relevant threshold yields replay parity."
    },
    {
      "id": "TC-1164",
      "path": "spec/test-cases/TC-1164-an-instance-key-context-binds-its-key.md",
      "role": "examined",
      "excerpt": "- Step 2: two contexts whose octets differ exactly in the key; the state is a foreign context and is not admitted.\n- Step 3: each certificate is rejected `ShapeMismatch` at `ProductState`."
    },
    {
      "id": "TC-1197",
      "path": "spec/test-cases/TC-1197-certificate-rejections-use-the-fr-331-rule-and-locus-pairs.md",
      "role": "examined",
      "excerpt": "- Step 6: every rejection's locus belongs to its checker's row, and FR-331\n  lists its rule at that locus form."
    },
    {
      "id": "TC-608",
      "path": "spec/test-cases/TC-608-replay-a-hyper-counterexample.md",
      "role": "examined",
      "excerpt": "- Step 6: `Accepted`, the body not evaluated; `Refused` with `invalid_runtime_input`/`invalid-value` at joint step 1; equal results; `Refused` with `invalid_runtime_input`/`invalid-value` for the outside key before any re-execution."
    },
    {
      "id": "test-context-shape",
      "path": "fsl-analyze/tests/it/closure_certificate/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "fn noncanonical_instance_bindings_are_rejected_at_the_offered_product_state()"
    },
    {
      "id": "test-context-separation",
      "path": "fsl-analyze/tests/it/closure_certificate/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "fn replacing_one_instances_contexts_by_another_does_not_certify_both()"
    },
    {
      "id": "test-bellman",
      "path": "fsl-analyze/tests/it/exact_probabilistic_item/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "row.value = ratio(9, 10);"
    },
    {
      "id": "test-transport",
      "path": "fsl-analyze/tests/it/exact_probabilistic_item/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "assert_eq!(written, bytes);"
    },
    {
      "id": "test-ranks",
      "path": "fsl-analyze/tests/it/exact_probabilistic_item/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "*rank = 0;"
    },
    {
      "id": "test-byte-budget",
      "path": "fsl-analyze/tests/it/exact_probabilistic_item/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "bound: required - 1,"
    },
    {
      "id": "test-hyper-binding",
      "path": "fsl-analyze/tests/it/hyper_replay/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "ModelTraceRefusal::HyperShape { .. }"
    },
    {
      "id": "test-probability-initial",
      "path": "fsl-analyze/tests/it/probabilistic_replay/certificate_boundary_cases.rs",
      "role": "examined",
      "excerpt": "state_and_key.initial = terminal.post.clone();"
    },
    {
      "id": "FR-183",
      "path": "spec/functional/FR-183-replay-a-hyper-counterexample.md",
      "role": "context_only",
      "excerpt": "It re-executes every trace\nthrough its subject's `ModelSystem`, checks the match and each trace's\nfairness, and evaluates the body over the tuple"
    }
  ],
  "bindings": [
    {
      "test_id": "closure_certificate::certificate_boundary_cases::noncanonical_instance_bindings_are_rejected_at_the_offered_product_state",
      "ac_id": "FR-338-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "closure_certificate::certificate_boundary_cases::noncanonical_instance_bindings_are_rejected_at_the_offered_product_state",
      "ac_id": "TC-1164",
      "trace": "correct"
    },
    {
      "test_id": "closure_certificate::certificate_boundary_cases::replacing_one_instances_contexts_by_another_does_not_certify_both",
      "ac_id": "FR-338-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "closure_certificate::certificate_boundary_cases::replacing_one_instances_contexts_by_another_does_not_certify_both",
      "ac_id": "TC-1164",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::changing_the_one_loss_value_rejects_at_the_first_failed_bellman_state",
      "ac_id": "FR-201-AC-1",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::changing_the_one_loss_value_rejects_at_the_first_failed_bellman_state",
      "ac_id": "FR-201-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::changing_the_one_loss_value_rejects_at_the_first_failed_bellman_state",
      "ac_id": "FR-201-AC-8",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::changing_the_one_loss_value_rejects_at_the_first_failed_bellman_state",
      "ac_id": "TC-1197",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::engine_certificate_transport_preserves_all_rows_targets_ranks_and_identity",
      "ac_id": "FR-201-AC-9",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::correct_bellman_values_cannot_hide_broken_positive_progress_ranks",
      "ac_id": "FR-201-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::correct_bellman_values_cannot_hide_broken_positive_progress_ranks",
      "ac_id": "FR-201-AC-6",
      "trace": "correct"
    },
    {
      "test_id": "exact_probabilistic_item::certificate_boundary_cases::a_real_certificate_obeys_its_exact_byte_ceiling_without_truncation",
      "ac_id": "FR-201-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "hyper_replay::certificate_boundary_cases::genuine_refuting_traces_do_not_authorize_a_foreign_or_missing_instance",
      "ac_id": "FR-183",
      "trace": "correct"
    },
    {
      "test_id": "hyper_replay::certificate_boundary_cases::genuine_refuting_traces_do_not_authorize_a_foreign_or_missing_instance",
      "ac_id": "TC-608",
      "trace": "correct"
    },
    {
      "test_id": "probabilistic_replay::certificate_boundary_cases::a_genuine_terminal_key_and_state_cannot_replace_the_witness_initial_state",
      "ac_id": "FR-202-AC-10",
      "trace": "correct"
    }
  ]
}
```

