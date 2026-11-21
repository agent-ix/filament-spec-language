---
id: SR-547
title: "QSL-2128 HP-1 certificate contract integrity review"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; QSL-2128; spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md; spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md"
review_set: subset
---

## Summary

Frozen PR filament-spec-language#3321, commit 39f284fdea379417f151d2b0da861d30492ea1a3; QSL-2128. Reviewed only the FR-163/FR-184 contract diff (108 additions, 8 deletions), with their existing rules and named dependencies as context. No applicable AssuranceProfile was found under spec/. No enforced review policy applies.

The installed SpecReview template and schema were fetched with quoin write . --types SpecReview. Targeted quire validation passed for both changed documents: 2/2 grammar-clean, no grammar findings. git diff --check passed. Existing module-discovery warnings are not semantic evidence.

Implementation, test bindings, computed matrix, engine execution, driver acceptance and package FR-940-AC-18 third-pin qualification were not assessed. Matrix/advisor, Cargo/Python, package changes and full gates were excluded by dispatch. This is contract readiness only, not an implementation or assurance completion claim.

## Review

The same ordered sequence is required for one or several aliases; no second single-subject representation remains. FR-184 explicitly makes engine/checker counters independent and grants each pass the caller value, consistent with FR-163-AC-13/FR-184-AC-4. The in-process Stopped result can retain count and tuple, while the settled limit cause keeps its existing wire shape. FSpec FR-331 was read at Cargo.toml revision 247a394157b58843643a61dfde16280c25d733c0 and compared with local origin/main 9f5b30a525a8729ddb836c8eb9b3821d30d0de4f: existing hyper state/transition loci and typed results agree; main additionally generalizes certificate-unchecked settlement. No dependency update is claimed. Scoped trace path: US-021 -> FR-163/FR-184 -> Test; full stakeholder traversal and test matrix were not assessed.

## Verdict

PASS for the frozen specification diff. No contract defect found; implementation and assurance remain outstanding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

Excerpts over 600 characters are verbatim initial excerpts; the complete source units were read.

```yaml
scope:
  - id: FR-163
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: |-
      When EN-1 returns `Holds` for an HP-1, HP-2, HP-3 or HP-6 item, or
      `Witnessed` for an HP-5 item (FR-181), the layer-A engine in `fsl-analyze` SHALL hand over a
      `ProductClosureCertificate`, and FSL's layer-6 crate `fsl-replay` SHALL hold
      `check_product_closure`, which accepts the certificate only when the
      certified set holds every initial product state, is closed under the
      recomputed successors, holds no undefined, refused or incomplete letter or
      match, and holds no violation by the form's rule (ADR-023 HX-7). The
      certificate reuses ADR-018's closure certificate (PC-3) and component
      certificate
  - id: FR-163-AC-1
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      Accepted vectors: §8.1's secure `NonInterference` certificate (8 product states, every component
      `MissingAcceptance` or `Trivial`) is `Accepted`; the copy-swap certificate (6 states) is `Accepted`;
      §8.2's secure `Opaque` certificate is `Accepted`; FR-179-AC-1's `Det` certificate is `Accepted`;
      FR-181-AC-1's HP-5 certificate is `Accepted`.
  - id: FR-163-AC-2
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      Rejected vectors over the AC-1 `NonInterference` certificate: one member removed is `Rejected` with rule
      `SuccessorMissing` naming the member whose successor is missing; its keys unsorted is `Rejected` with
      rule `Malformed` before any recomputation; an added member whose `l` components differ, in a component
      marked `MissingAcceptance`, is `Rejected` with rule `WitnessFails`; one member listed in two components
      is `Rejected` with rule `NotPartition`; two components joined by an edge listed in the reverse order is
      `Rejected` with rule `BackwardEdge`; the initial product state removed is `Rejecte
  - id: FR-163-AC-3
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      A certificate for the leaky vault's `NonInterference` built by listing its 14 product states with the
      components of the secure run, which do not cover the leaky states, is `Rejected` with rule
      `NotPartition`, so a false proof is never accepted; a certificate for FR-181-AC-5's body listing the
      reached product states is `Rejected` with rule `UndefinedMember` at the first state with `l = 1`.
  - id: FR-163-AC-4
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      The AC-1 `NonInterference` certificate checked with `max_states` 1 returns `Stopped` with `{MaxStates,
      1}` naming the limit and its value; checking it twice gives equal results.
  - id: FR-163-AC-5
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      HP-2 and HP-3 certificates with absent `components`, including one with an empty closure, reject
      `Malformed` at `CertificateMember("components")` before any successor recomputation, without selecting a
      state key.
  - id: FR-163-AC-6
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      An HP-1 certificate for FR-179-AC-2's nondeterministic `Det` subject rejects `ViolatingMember` at the
      first complete labelled edge tuple in canonical order; equal pre-state keys and arguments with different
      post-states remain distinct. FR-179-AC-4's undefined body rejects `UndefinedMember` at its complete edge
      tuple.
  - id: FR-163-AC-7
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      For ADR-023 HC-2 with zero universal variables, a supplied key with an empty universal tuple and its
      actual witness set retains both on serialization and reading; the empty witness set remains distinct
      from an absent witness set. No component is added to satisfy a nonempty-array constraint.
  - id: FR-163-AC-8
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      **RETIRED (staleness check withdrawn).** The certificate carries no `package_id` and the checker
      compares no package identity.
  - id: FR-163-AC-9
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      Production order. §8.1's secure `NonInterference` proof run twice writes byte-equal RFC 8785
      certificates, whose `closure` keys are ascending, whose components are in FR-126's component order over
      the self-composed product's canonical breadth-first indices, and whose components each list their states
      ascending in the closure's key order. The same certificate with two components that no edge joins listed
      in the other order is still `Accepted`.
  - id: FR-163-AC-10
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      The HP-1 `Det` certificate of FR-179-AC-1 produced twice has byte-equal RFC 8785 serialization and one
      subject closure for its repeated alias, with ascending state keys and no automaton key; the independent
      checker accepts it after evaluating 64 tuples. Removing its initial key rejects `InitialMissing`;
      removing a noninitial key required as a successor rejects `SuccessorMissing` at the source member;
      reversing two keys or duplicating one rejects `Malformed` at the first offending state key before
      recomputation. Each state rejection serializes without an automaton key.
  - id: FR-163-AC-11
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      Over two secure-vault model aliases declared `V` then `N` and execution bindings `(y: N::Vault::step, x:
      V::Vault::step, z: V::Vault::step)`, a relation with body `true` has exactly the subject-closure alias
      sequence `[V, N]`, with execution entries `[x, z]` and `[y]` respectively, retaining both alias entries
      even for equal state keys. Each execution contains its complete typed applicable edges and the
      certificate is accepted. Omitting `N`, duplicating `V`, adding a third alias, reversing the alias
      entries or removing or altering a supplied execution edge refuses the certificate request with 
  - id: FR-163-AC-12
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      FR-163-AC-6's false and undefined tuple rejections serialize the complete labelled edges in
      execution-binding order. Edges with equal pre-state and arguments but unequal post-states remain
      distinguishable; edges with equal pre-state, transition and post-state but unequal typed results remain
      distinguishable. A no-result edge serializes `result: null`, while an edge returning a typed null
      retains that typed value. Neither tuple acquires an automaton key.
  - id: FR-163-AC-13
    path: spec/functional/FR-163-check-a-hyper-item-s-product-closure-certificate.md
    role: examined
    excerpt: >-
      Checking FR-179-AC-1's certificate independently with `max_relation_tuples` 63 stops before evaluating
      tuple 64, naming `MaxRelationTuples`, value 63, count 64 and the complete labelled tuple 64 as its first
      unpaid tuple; settlement is incomplete with setting `model_check.max_relation_tuples`, no counterexample
      and the unchanged limit-cause shape, never `CertificateUnchecked` or proved. With 64 it accepts,
      including when the engine already evaluated its 64 tuples under the same request setting. A second
      independent check with 64 also accepts.
  - id: FR-184
    path: spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md
    role: examined
    excerpt: |-
      `ModelCheckLimits` SHALL gain two budgets for hyper items (ADR-023 HC-10):
      `max_witness_set`, the members one HP-3 witness set may reach, and
      `max_relation_tuples`, the tuples one HP-1 run may evaluate. Like every
      EN-1 limit they are ADR-014 B-5 budgets of FSL's own provider that the
      request sets, each with a published default that applies only when the
      request sets none. They are resource budgets, never modelling limits: no
      form, formula size, arity or depth is capped. Reaching one stops the run
      and settles V-7, naming the limit, its value and the request setting that
      raises it.
  - id: FR-184-AC-1
    path: spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md
    role: examined
    excerpt: >-
      A request that sets neither member runs with `max_witness_set` 65,536 and `max_relation_tuples`
      16,777,216, and its terminal record reports those values, marked as published defaults, for every run: a
      proved run, a refuted run and a stopped run alike.
  - id: FR-184-AC-2
    path: spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md
    role: examined
    excerpt: >-
      ADR-023 §8.2's leaky `Opaque` with `max_witness_set` 0 stops `Stopped(ResourceExhausted, MaxWitnessSet)`
      with value 0, count 1 and no counterexample; with `max_witness_set` 1 it completes with FR-177-AC-1's
      outcome.
  - id: FR-184-AC-3
    path: spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md
    role: examined
    excerpt: >-
      FR-179-AC-1's `Det` with `max_relation_tuples` 63 stops `Stopped(ResourceExhausted, MaxRelationTuples)`
      with value 63 and count 64; with 64 it completes `Holds`.
  - id: FR-184-AC-4
    path: spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md
    role: examined
    excerpt: >-
      FR-179-AC-1's engine run with `max_relation_tuples` 64 evaluates its 64 tuples and the resulting
      certificate's independent check under the same setting evaluates its own 64 tuples and accepts.
      Rechecking it starts from zero and accepts again. Checking that certificate under 63 stops with value 63
      and count 64 before the 64th body evaluation and identifies the complete labelled tuple 64 as the first
      unpaid tuple; the record names `model_check.max_relation_tuples`, retains the existing limit-cause shape
      and carries no counterexample.
  - id: FR-179
    path: spec/functional/FR-179-check-a-step-relation-over-reachable-transitions.md
    role: context_only
    excerpt: |-
      FSL's layer-A `fsl-analyze` engine EN-1 (ADR-029 CB-3) SHALL decide the model claim of an HP-1 step
      relation (ADR-023 HM-9, HC-3) by exploring each subject's reachable graph
      once, retaining edges, and evaluating the relation's body over every tuple
      of reachable transitions, one per execution variable, each labelled with
      that variable's operation. The first tuple, in canonical order, on which
      the body is false or undefined is the counterexample (ADR-018 UE-1,
      ADR-023 HV-8). `max_relation_tuples` bounds the
      number of tuples a run evaluates (FR-184).
  - id: FR-338
    path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
    role: context_only
    excerpt: |-
      `fsl-replay`'s `check_closure` SHALL accept a `ClosureCertificate` that EN-1
      returns with a safety proof exactly when the certificate's product states
      contain every initial product state, are closed under expansion, and hold
      no bad state, and its checker-owned non-progress check below finds no
      admitted violating lasso, recomputing every successor with the core's own code
      (ADR-018 PC-3, PC-5, LA-3). A proof whose certificate it accepts settles
      `proved`, `Certified` (FR-127); one it rejects settles `inconclusive`,
      `CertificateRejected`.
  - id: FR-182-AC-17
    path: spec/functional/FR-182-settle-a-hyper-verdict.md
    role: context_only
    excerpt: Test |
  - id: FR-276
    path: spec/functional/FR-276-cancel-a-lifecycle-operation.md
    role: context_only
    excerpt: |-
      `Cancel` is a cloneable handle the caller owns (ADR-029 LC-3). Every FSL
      lifecycle operation (FR-275) SHALL poll it wherever it charges a work meter
      or a stage limit, so a cancelled run stops within one charge. A cancellation
      carries a `CancelCause`: `Requested`, when the caller asks for it, or
      `Deadline`, when the frontend that owns the timer reaches its deadline. The
      library reads no clock.

      `StageFailure` gains the variant `Cancelled(CancelCause)` and `CallFailure`
      gains the variant `Cancelled(CancelCause)` (ADR-029 Amendments, ADR-013 T-4).
  - id: FSpec-FR-331
    path: filament-specification/spec/objects/interfaces/FR-331-backend-provider-envelope.md
    role: context_only
    excerpt: >-
      | Hyper transition tuple | `{transitions}`: the ordered, nonempty tuple of labelled edges `{pre_state,
      transition, post_state, result}` defined below | FSL FR-163, FR-179, FR-120 | `undefined-member`,
      `violating-member` |
```
