---
id: SR-2026112404075000000001
title: 'Code review of PR #3647: Every-scheduler long-run certificates, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/filament-spec-language@1ff3d3869e; PR #3647 (branch draco/qsl-2778-longrun): fsl-replay long_run.rs, fsl-analyze exact/long_run.rs and exact_probabilistic; tickets QSL-2778, QSL-2779'
review_set: subset
date: '2026-11-24'
---

<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.10 module=spec-artifacts-process@installed id=SR-2026112404075000000001 method=code-review lang=rust pr=filament-spec-language#3647 model=claude-opus-5-5 run=29578B1B-7F04-463C-9EFC-366EED9D25E5 -->

PR #3647 head 1ff3d3869ede9cbc208f5f58958630955c2122a9. Methods: code-review with rust-review lane, plus a manual AC-to-test check (gap-analysis was not run as a full audit; this was a read-only review with no builds). Verdict: NOT MERGEABLE (FND-001).

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | soundness | The every-scheduler holds-side checker fixes each MEC state's value at its own component's ρ and ignores actions that leave the MEC, so it accepts a false `proved` |
| FND-002 | medium | coverage | No test has a non-policy action that violates an inequality, and no test has an MEC with an exit, so FND-001 and AC-26's "both actions" go untested |
| FND-003 | low | trace | Unprefixed `"FR-411-AC-18"` tags bind to the local FSL FR-411 (terminal emission, which has no AC-18), not to FSpec FR-411-AC-18 |
| FND-004 | low | code-bug | Two new lines exceed rustfmt width (`exact_probabilistic.rs:120`, `fsl-replay/src/lib.rs:498`), so `cargo fmt --check` fails |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FSpec-FR-412-AC-26, path: "spec/objects/interfaces/FR-412-probability-certificate.md", role: examined, excerpt: "a maximum long-run certificate whose component row carries value, gain, bias and a policy with one row per state is accepted when the equalities hold under that policy and the inequalities hold for both actions at each state; changing one bias by 1/1000 is rejected inequality-fails ...; removing the component's policy, adding a row for a state outside C, duplicating a row, or naming a choice that is no action of its state is rejected malformed at policy; ... under workload with a policy is rejected malformed"}
  - {id: FSpec-FR-412-AC-27, path: "spec/objects/interfaces/FR-412-probability-certificate.md", role: examined, excerpt: "Under minimum over every scheduler with no fairness set, a refutation ... is accepted when its policy rows are deterministic and induce a chain whose recomputed long-run value is 1/2; a witness whose induced value is 3/4 is rejected inequality-fails at the initial state; a randomised row, a row naming no action, or a missing row ... malformed at policy. Under a fairness set ... accepted only when every bottom component of its induced chain is fair"}
  - {id: FSpec-FR-412-AC-28, path: "spec/objects/interfaces/FR-412-probability-certificate.md", role: context_only, excerpt: "initial-state verdict refuted by the lower certificate on the absorbed initial state; not touched by this diff"}
  - {id: FSpec-FR-411-AC-18, path: "spec/functional/analysis/FR-411-settle-probabilistic-claims-exactly.md", role: examined, excerpt: "Over Pair ... >= 3/4 under Every settles refuted with the deterministic scheduler witness of FR-412-AC-27 ..., and <= 3/4 settles proved with the per-component policy certificate of FR-412-AC-26 ... No Every-scheduler long-run request settles unsupported for naming Every"}
  - {id: FSpec-FR-1041, path: "spec (FSL FR-1041 fair-minimum)", role: context_only, excerpt: "fair holds side needs a progress ranking; accepted gap (Unsupported)"}
  - {id: FSL-FR-202, path: "spec/functional/FR-202-replay-a-probabilistic-witness.md", role: examined, excerpt: "settle the item refuted only when it agrees (WS-5)"}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: code-review
    check: soundness
    artifact_id: FSpec-FR-412-AC-26
    related_ids: [FSpec-FR-411-AC-18, FSpec-FR-411]
    path: fsl-replay/src/probability_certificate/document/long_run.rs
    lines: "2121-2128, 2151-2160 (check_mdp); analyzer: fsl-analyze/src/exact_probabilistic/unbounded.rs decide_long_run_mdp fixed[state]=rho + policy_iteration skips fixed states"
    excerpt: |-
      // The value of a state outside every component is its optimal value over
      // the actions; inside one it is the component's.
      ... Some(mec) => document.components[entry_of[mec]..].value.clone(),
    finding: "check_mdp requires values[s] == rho_M for every state s in MEC M, and it checks inequalities only for actions that stay inside M. A scheduler can leave M for a better MEC, so the optimum at s is opt(rho_M, opt_a sum P*v), not rho_M. Counterexample: phase 0 has `stay0` (self-loop) and `go` (to phase 1); phase 1 has `stay`. The claim is `every scheduler long_run_fraction holds(phase=1) <= 1/2`. MECs are {0} with rho 0 and {1} with rho 1. The analyzer (decide_long_run_mdp pins MEC states via `fixed`) gives v(0)=0, and the checker accepts it, so the claim is proved. The true maximum is 1. Min with >= is wrong the same way. The refutation side is sound because it recomputes over the induced chain. Fix: in check_mdp, require for every state (MEC states included) v(s) >=/<= rho_M(s) and v(s) >=/<= sum P*v over every action, with attainment by some MEC stay or action. Fix the analyzer to solve the de Alfaro quotient instead of pinning MEC states. FSpec FR-412 'Long-run' should state this cross-component value condition; 'inequalities over every action' cannot be read per component without reading other components' biases."
  - fnd: FND-002
    severity: medium
    confidence: medium
    method: code-review
    check: coverage
    artifact_id: FSpec-FR-412-AC-26
    related_ids: [FSpec-FR-411-AC-18]
    path: fsl-analyze/tests/it/long_run_certificate.rs
    lines: "1137-1244 (diff)"
    excerpt: |-
      ... accepted when the equalities hold under that policy and the inequalities hold for both actions at each state
    finding: "The AC-26 tests use drift() (one-state component, a single action) and availability (bias perturbation only). No document has policy rows that satisfy the equalities while another action breaks an inequality (the AC's u/v model with actions a and b), and no fixture has an MEC with an exiting action. AC-18's Pair model with <= 3/4 proved is not built either. A test with both would have caught FND-001."
  - fnd: FND-003
    severity: low
    confidence: high
    method: code-review
    check: trace
    artifact_id: FSpec-FR-411-AC-18
    related_ids: []
    path: fsl-analyze/tests/it/exact_probabilistic_item/subsystem_refutation.rs
    lines: "1406, 1452"
    excerpt: |-
      #[trace("FSpec-TC-362", "FSpec-FR-412-AC-27", "FR-411-AC-18")]
    finding: "The tag lacks the FSpec- prefix. It resolves to this repo's spec/functional/FR-411 (terminal-declaration emission), which has no AC-18, so the tag dangles. long_run_certificate.rs uses the correct form, FSpec-FR-411-AC-18."
    test_id: a_fairness_set_accepts_a_deterministic_witness_only_when_every_bottom_component_is_fair
    claimed_ac: FR-411-AC-18
    correct_ac: FSpec-FR-411-AC-18
  - fnd: FND-004
    severity: low
    confidence: medium
    method: code-review
    check: code-bug
    artifact_id: fsl-analyze/src/exact_probabilistic.rs
    related_ids: [fsl-replay/src/lib.rs]
    path: fsl-analyze/src/exact_probabilistic.rs
    lines: "120"
    excerpt: |-
      decide_fair_reachability, deterministic_policy_is_fair, maximal_fair_end_components, ActionChoice, FairBoundedResult,
    finding: "The new re-export lines run past rustfmt's 100-column width and are not wrapped (also fsl-replay/src/lib.rs:498), so cargo fmt --check would flag them."
bindings:
  - {test_id: an_every_scheduler_long_run_refutation_is_replayed_over_the_chain_its_policy_induces, ac_id: FSpec-FR-412-AC-27, trace: correct}
  - {test_id: a_fairness_set_accepts_a_deterministic_witness_only_when_every_bottom_component_is_fair, ac_id: FR-411-AC-18, trace: wrong}
  - {test_id: an_every_scheduler_long_run_claim_settles_refuted_with_its_scheduler_witness, ac_id: FR-411-AC-18, trace: wrong}
  - {test_id: every_scheduler_long_run_claims_settle_instead_of_unsupported, ac_id: FSpec-FR-411-AC-18, trace: correct}
  - {test_id: an_every_scheduler_long_run_proof_carries_per_component_policy_rows, ac_id: FSpec-FR-412-AC-26, trace: correct}
  - {test_id: an_every_scheduler_long_run_proof_checks_the_gain_bias_inequalities_and_workload_has_no_policy, ac_id: FSpec-FR-412-AC-26, trace: correct}
```

+++
