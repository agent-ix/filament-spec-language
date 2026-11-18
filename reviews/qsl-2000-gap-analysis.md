---
id: SR-2026111612172400000041
title: "Gap analysis of QSL-2000 slice A probabilistic node emission (filament-spec-language PR 2921)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@5f99c3c4a051689566803d8792a83db84905de5f; git diff origin/main...HEAD (PR 2921); FSL FR-740 AC-1..4 (TC-2062), FR-185-AC-7; FSpec FR-640 AC-1..5 (TC-1563) and FR-322 'Probabilistic-model nodes' / 'Structural node identity' at agent-ix/filament-specification origin/main 7e10eddb"
review_set: subset
date: 2026-11-18
---

## Summary

Ticket: QSL-2000 (slice A). Manual AC-to-test check over the diff's tagged tests and
code (quoin matrix not run against the stale local FSpec checkout; FSpec texts read
from origin/main). Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FSpec FR-640-AC-3 "changing one weight, one support value ... changes that node's identity" has no test; the body-derived identity (Reading A) is the PR's mechanism for it and is unasserted, and the weight-order half is not exercised (see SR-...40 FND-001) | fsl-package/src/emit/probabilistic.rs:1-12,295-300,355-360; fsl-package/src/emit/tests/probabilistic.rs:821-847 |
| FND-002 | low | FR-640 says the node path is `[node_tag, semantic_form, operand]` with "identity being its complete body", but neither FR-640 nor FR-322 defines how a body maps to operands; FR-322 lists an operand as a literal's text, a member name or an operand node, not a producer-composed `value*weight,...` string. Reading A is admissible (the reader does not compare a derived path with its body) but is an unspecified producer convention; an FSpec clarification is needed so another producer derives the same id | spec FR-640 Properties; FR-322 Structural node identity; fsl-package/src/emit/probabilistic.rs:291-300 |
| FND-003 | low | FR-640 says a support `value` is "a literal of the parameter's declared type"; the emitter types an `Int[1, 12]` (`Millis`) value at the built-in `integer` scalar node and a rational at `rational`, while lowering roots the declared type node it never references. IR admits it; the spec wording is ambiguous between type-node identity and value membership | fsl-package/src/emit/probabilistic.rs:268-275; fsl-semantics/src/check/lowering/probabilistic.rs:291-302 |

## Scope and evidence

Examined (clean unless listed above):
- FR-740-AC-1: partially met. Integer, quantity, rational-weight and lineage-workload
  cases emit, admit and rebuild equal to S3 (fsl-package round-trip and read-back tests,
  fsl-eval `the_package_route_rebuilds_the_probabilistic_model_of_the_emitted_package`).
  Full `Service` blocked by the reward (QSL-2440) and the Boolean/enum IR join (IR ticket);
  both refuse with a locus. Trace inflation noted in SR-...40 FND-002.
- FR-740-AC-2 / FSpec FR-640-AC-5: deferred, test ignored QSL-2440.
- FR-740-AC-3: weight 0 emitted and admitted (covered); delay half deferred (QSL-2439,
  digital_clocks.rs:848); reorder half untested (FND-001).
- FR-740-AC-4 / FSpec FR-640-AC-5 claim half: deferred QSL-2441.
- FSpec FR-640-AC-1: random-parameter and workload halves covered with exact
  probabilities (`1/4`, `3/4`) and declaration region; reward half deferred QSL-2440.
- FSpec FR-640-AC-2: deferred QSL-2439.
- FSpec FR-640-AC-4: misnamed-operation workload mutation now runs (un-ignored); other
  mutations are IR reader checks (not in this repo's diff).
- FR-185-AC-7: deferred (reward); see SR-...40 FND-002.
- Read-back equality: records equal S3's on operation key, declaring id/key, name,
  position, value type, support (value, weight, probability) and workload weights;
  the two documented differences (model alias, declared index) are read by no
  package-route consumer (ModelSystem / FR-187 lookups go by operation text and
  (declaring, name, position)).

## Verdict

Code conforms to the parts of FR-740/FR-640 it implements. Coverage gap FND-001 should be
closed in this PR (a reordered multi-operation workload and a changed-weight identity
assertion); FND-002/FND-003 are spec clarifications for FSpec, not blockers.
