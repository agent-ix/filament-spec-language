---
id: SR-1492
title: "Gap analysis — certificate and witness boundary bindings"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language; QSL-2087; .agent/receipts/qsl-2087/{scope,validation}.md; fsl-analyze/tests/it/{closure_certificate,exact_probabilistic_item,hyper_replay,probabilistic_replay}.rs and each certificate_boundary_cases.rs child"
review_set: subset
---

## Summary

Reviewed the new QSL-2087 test-to-current-requirement bindings and their real evidence routes at `716f44b511daec8cf8028b7f8abd7094be078d68`. The eight boundary tests back concrete admitted behavior; absent product-closure support remains explicitly unimplemented.

## Verdict

**PASS** — no gap or inflated binding found within the changed scope. This is a diff-scoped acceptance and trace review, not a whole-repository matrix verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

The examined FR-338 instance-context cases exercise canonical-shape rejection and separation of instance-bound initial product states. FR-201 cases cover its concrete Link Bellman mutation, checked-subject rule/locus, lossless closed-profile transport, positive-rank progress independent of Bellman equality, and a real exhausted-byte boundary. These are supported facets, not claims that each multi-part criterion is exhausted by one test. FR-183/TC-608 binding changes exercise the actual hyper replay admission path, with original reproduced false truth required. FR-202's full-key authentication refuses an actual foreign initial key/state and preserves the golden original witness mass.

Every new assertion can fail on a real behavioral mutation: accepting malformed context, conflating instance initials, accepting the Bellman/rank changes, changing canonical transport, mishandling the exact byte bound, or accepting foreign witness bindings/initial states. No placeholder checker, fabricated proof success, source-grep producer assertion, ignored passing-coverage claim or altered implementation is used.

The parent reports the current FR-163 product-closure checker unavailable, with existing QSL-964 linked as a blocker. That trusted boundary is recorded in the submitted scope and remains untagged; this review did not rerun it or audit the Done specification deliveries. Hyper certified proof settlement is likewise not claimed. Historical ticket counts and its matrix method were not treated as execution instructions.

Per user/lead scope, matrix computation, repository-wide reverse-gap/stub scans, unrelated requirements, old coverage inventories and feature-completion bookkeeping were not repeated. Reverse-gap and hollow-evidence checks were confined to all changed test bodies and their immediate helper routes; no unspecified production behavior is introduced. The author’s execution receipts were accepted. No full repository assurance or exhaustive acceptance-criterion coverage claim is made.

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

