---
id: SR-2026112017124277723988801
title: "code-review of PR 3209 analyze EN-1/EN-6 verdict defects (QSL-2714, QSL-2719, QSL-2689)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language; QSL-2714, QSL-2719, QSL-2689; PR3209 head 293e4ec164febfa6b1d8fe97a164658f458919f1; git diff origin/main...HEAD (base 438a9f3b79); fsl-replay/src/certificate/zone_certificate.rs; fsl-replay/src/certificate/zone_certificate/product.rs; fsl-replay/src/spine/clocks.rs; fsl-analyze/src/zone_check/{live_certificate.rs,liveness.rs,search.rs}; fsl-analyze/src/analyze/{clause_item.rs,document.rs}; fsl-analyze/tests/it/{terminal_two_models.rs,source_model_boundaries.rs,zone_mitl.rs}"
review_set: subset
---

## Summary

Tickets: QSL-2714, QSL-2719, QSL-2689. PR filament-spec-language#3209, frozen head 293e4ec164febfa6b1d8fe97a164658f458919f1, scope `git diff origin/main...HEAD` over base 438a9f3b79. Code-review with the rust-review lane folded in. The main focus is the trusted-base change to the zone-certificate checker: the admission instance in `fsl-replay` `product.rs`.

## Verdict

MERGEABLE. No in-scope finding.

**QSL-2689: checker soundness of the admission instance.**

- `node_ok` now admits `mark.instance <= instances.len()`. Exactly one extra index, `count`, is the admission instance. `count + 1` is still malformed.
- A successor covers an edge only when `successor.mark == target.product`, instance included. `live_successors` keeps the source's instance. So no edge crosses between a claim instance and the admission instance, and claim-instance closure is checked exactly as before.
- Initial coverage still covers every claim instance. It also covers the admission instance once any node carries that index.
- Admission nodes are excluded only from the components numbering requirement, from the non-Zeno graph and acceptance sets, and from fairness `within`. They cannot lie on a claim cycle, so these exclusions cannot hide an accepting cycle.
- When a component reason includes admission nodes, the check only gets stricter.
- `admitted` still recomputes exact zones from an initial node and needs a covered path. A witness in the admission instance proves exactly ADR-026 TD-6: the subject admits a time-divergent behaviour.
- For a certificate with no node at index `count`, every changed branch is inert. The accepted set for old certificates is therefore unchanged.

**Adversarial tampering.** These were run through the core checker on the nested-until certificate: 60 nodes, admission index 1, 36 admission nodes, witness `24 -[26,28,31]-> Quiescent(30)`. Every tampered certificate was rejected:

- the admission instance stripped, with or without a fake `Quiescent` witness at each claim initial;
- the admission initials dropped;
- each of the 80 admission edges removed in turn;
- every truncated witness prefix retargeted to `Quiescent` or `Terminal`, a `Terminal` target under the dense source, and the witness path started at a claim initial;
- a claim node relabelled into the admission instance and reached by a fabricated edge;
- an unreachable extra vertex, and the same vertex added as a fake initial;
- each claim node relabelled to the admission index, and the admission nodes moved to index `count + 1`;
- each `divergent`/`positioned`/`tail` flag of each admission node flipped;
- the certificate transplanted to the violated worlds `nested(4)` and `nested(5)`;
- a copy of an old certificate's instance-0 graph injected as a fake admission instance (InitialMissing).

**Schema and versioning.** The PR adds no certificate member and changes no serialization. The admission instance reuses `ProductMark.instance`, and `CertificateMember::ALL` is untouched.

**QSL-2714.** `RequestClocks::model_selections` derives one selection per declaration, keyed by source and span, through the same `derived_configuration` that `selections()` uses for model-bound declarations at compile. Several clock selections therefore resolve per declaration, and no shared or wrong-selection binding was found. Replay and protocol emits use the same selections as the check and certificate requests.

**QSL-2719.** `read_identities` mirrors `ModelSystem::new_over_read_identities`: it reads the selections whose identity a universe population covers, or all of them when none is covered. A subject that reads two selections has no anchor and settles `unsupported` (FR-368-AC-22). A single read selection derives its own item, and timing follows only the selections the subject reads.

**Gates.**

- Capped clippy (`--cap-lints warn`) on fsl-analyze and fsl-replay reports nothing on the diff hunks.
- Inherited reds are identical on origin/main 438a9f3b79: `the_certificate_members_are_the_schema_fifty_one_names` and three `zone_live_certificate` tests fail with `NonZenoMismatch` at the first Accepted assertion.
- With that mismatch neutralised locally, the tamper and malformed assertions of those three tests pass on both main and PR head, so the red hides no regression.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
