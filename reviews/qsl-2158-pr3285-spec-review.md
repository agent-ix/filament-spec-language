---
id: SR-202611210022442313144108507
title: "QSL-2158 PR 3285 finite fraction mean-refutation base review"
type: SpecReview
analysis: base
scope: "spec/functional/FR-202-replay-a-probabilistic-witness.md and spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md, diff 3498af270b^..3498af270b only"
review_set: base
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-202", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1043", type: references}
---

## Summary

This review covers only the mean/fraction refutation amendment in pinned
pure-spec commit `3498af270b4efc3bb1f07f453676af379939ad5d` (parent
`4fefc2c427`). The working copies of both documents are byte-identical to the
pinned blobs (`git diff 3498af270b --` on the two paths is empty). The
amendment follows the architecture ruling on semantics. The accepted
certificate uses the same finite-horizon-measure base and the deciding
extremum. Refutation needs a strict wrong-side value. Equality does not
refute. No probability-mass semantics are used, no wire field, profile or
request flag is added, and FSpec FR-413 subsystem evidence is reused. The AC
arithmetic is correct. Five defects block acceptance: the new criteria are not
registered as criteria, the amendment contradicts FR-202's existing
subsystem-replay rule and checked-subject entry scope, typed outcomes are
missing for non-refuting evidence, and partial-product admission is ambiguous.

## Verdict

**FAIL**. FND-001 and FND-002 are high. FND-003, FND-004, FND-005 and FND-007
are medium, and FND-006 is low. This verdict covers only the normative spec
scope of the two FR edits. It makes no claim about the implementation, the
tests, or whether the PR can merge.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The new AC rows come after a blank line, which ends each AC table. FR-1043-AC-8/9 (FR-1043 lines 210-212) and FR-202-AC-27/28/29 (FR-202 lines 509-512) are therefore not table rows. `quire properties --json` on both documents extracts FR-1043-AC-1..7 and FR-202 up to AC-26 only. `pandoc -f gfm` renders a split row as a `<p>`. As a result the matrix cannot see these five criteria and trace tags to them cannot bind. Fix: delete the blank lines at FR-1043 line 210 and FR-202 line 509. | FR-1043-AC-8, FR-1043-AC-9, FR-202-AC-27, FR-202-AC-28, FR-202-AC-29 | wrong-requirement |
| FND-002 | high | The new mean rule (FR-202 lines 118-147, FR-1043 lines 174-197) checks an `exact` minimum/maximum or an FR-1042 bound against the complete minimum/maximum base a proof would use. Lines 146-147 say "existing replay ... rules continue to apply", and FR-1043 line 193 reuses "FR-202's ... replay acceptance rules". The existing FR-202 rule at lines 450-452 is unamended: "For subsystem evidence it SHALL rebuild the induced chain ... by applying the witness scheduler, and run FR-201's checker on it." Adverse case: FR-1043-AC-8's exact minimum `11/48` document (optimization minimum) rechecked on the `slow`-induced chain has no choices left to minimise. It either refuses objective mismatch or is checked under a different base, so AC-8/FR-202-AC-28 cannot pass while line 450 holds. Fix: amend line 450 to exempt mean certificates, or state that they are checked over their base product. | FR-202, FR-1043, FR-202-AC-28, FR-1043-AC-8 | wrong-requirement |
| FND-003 | medium | The amendment does not say which replay entry accepts mean subsystem evidence. The checked-subject entries "SHALL report unsupported for subsystem evidence" (FR-202 line 222), and FR-202-AC-11, AC-16 and AC-21 assert subsystem evidence is unsupported or gives no token on the native/bounded entries. FR-202-AC-27/28/29 and FR-1043-AC-8 name no entry. Adverse case: a native implementation that satisfies AC-27 violates line 222 and AC-16, and a wire-facade one hits FND-002. Fix: name the entry in the mean bullet and AC-27, and scope line 222 accordingly. | FR-202-AC-27, FR-202, FR-202-AC-11, FR-202-AC-16, FR-202-AC-21 | wrong-requirement |
| FND-004 | medium | The amendment gives no typed outcome for non-refuting mean evidence. It only says "does not refute" or "yields no accepted refutation" for four cases: a value or bound equal to `c`; a wrong-kind document for the role (a `lower` on the minimum for `>= c`, an `upper` on the maximum for `<= c`); failed certificate conditions; and a witness policy that disagrees with the document's policy. The surrounding text types analogous cases differently: FR-202 line ~471 gives a sum at the bound `inconclusive`/`ReplayParity`; FR-1042 line 164 gives wrong kind `Malformed` at `kind`; FR-1043 line 154 gives `ThresholdMissed`; FR-1042 line 188 gives `CertificateRejected{rule, at}`. Adverse case: AC-9's exact `5/12` at `>= 5/12` may settle `ReplayParity` in one implementation and `CertificateRejected{ThresholdMissed}` in another. AC-28's `lower` on the minimum may settle `Malformed` at `kind` or parity. Fix: state each outcome, and say which of the checker or replay issues it. | FR-1043-AC-9, FR-202-AC-28, FR-202-AC-29, FR-1043 | missing-requirement |
| FND-005 | medium | The amendment does not say whether a partially explored product can carry a mean refutation. FR-202 line 125 requires the checker to check the "complete product", but the unamended line 172 says evidence on a partial product settles the item when it is "a `Lower` certificate refuting a `<= θ` bound". FR-1042 lines 92-96 also admit a `lower` on a partial product. Adverse case: `max_states` stops EN-5 on FR-1043-AC-8's maximum with an explored-part `lower` `2/5 > 1/3`. Under line 172 it refutes `<= 1/3`; under line 125 it must be refused. Fix: state whether a mean `lower` found on a partial product refutes, and which least value counts for unnamed successors of a `fraction` base. FR-1042 line 93 lists none for `fraction`. | FR-202, FR-1043, FR-1043-AC-8 | wrong-requirement |
| FND-006 | low | The new text names the mean threshold `c` (FR-1043 line 174, FR-202 line 118), but FR-1043 elsewhere writes `mean of F op θ` (lines 34, 74). In FR-1043, `c` is also `measure.threshold`, which is null for payoff `fraction` (lines 100-104). Adverse case: an implementer who takes "the same complete objective" and `c` literally writes the mean threshold into `measure.threshold`, and that refuses objective mismatch. Fix: use `θ` for the mean threshold in both new sections. | FR-1043, FR-202 | wrong-requirement |
| FND-007 | medium | FR-1043-AC-9 and FR-202-AC-28 give no concrete document for the wrong-optimization adverse case ("a lower bound on the minimum or an upper bound on the maximum ... even when strictly past the threshold"). Adverse case: a test can offer a `lower` on the minimum of `1/2`, which is invalid because the minimum is `11/48`. It is rejected for its conditions and passes the AC without exercising the role check. Fix: name valid documents, e.g. a `lower` on the minimum of `1/5` (`<= 11/48`) at `>= 1/2` and an `upper` on the maximum of `1/2` (`>= 5/12`) at `<= 1/3`, both valid and on the wrong side, and neither refutes. | FR-1043-AC-9, FR-202-AC-28 | wrong-requirement |

## Coverage

Examined units: FR-202 Description (lines 27-35), Emission mean bullets
(lines 118-147), FR-202-AC-27/28/29; FR-1043 relationships, Forms preamble
(lines 66-67), Checking/results (lines 148-170), Mean-refutation role
(lines 172-197), FR-1043-AC-5/8/9. Context only: FR-202 lines 172-174, 218-227
and 436-470; FR-1042 lines 84-100, 112-123 and 155-195; FR-202-AC-11/16/21.

- **Arithmetic.** `Retry` over `[0, 2]` has 3 positions with delivery
  probabilities 0, 1/2 and 3/4, so the mean is `(5/4)/3 = 5/12`. With `fast`
  `1/2` and `slow` `1/4`, the sum `2p1 + p2 − p1·p2` is minimised by
  `slow, slow` at `11/16`, giving `11/48`, and maximised by `fast, fast` at
  `5/4`, giving `5/12`. Mixed policies fall in between. `upper` `1/4` is at
  least `11/48` and below `1/2`, so it refutes. `lower` `2/5` is at most
  `5/12` and above `1/3`, so it refutes. The AC-9 equality cases (`5/12`,
  `upper` `1/2`, `lower` `2/5`) are valid bounds at equality. The FR-1042
  policy table (lines 120-121) requires a policy for an `upper` on a minimum
  and a `lower` on a maximum, which matches AC-8's `slow`/`fast`.
- **Architecture ruling.** Same base and profile: yes. Deciding extremum:
  minimum for `>= c`, maximum for `<= c`. Strict sign: yes. Equality does not
  refute: yes. The proof `ThresholdMissed` does not reject the refutation
  role (FR-1043 lines 189-190, FR-202 lines 138-140). A path set or
  probability-mass comparison is excluded. No new wire field, profile or
  request flag. Cancellation and caller limits are inherited (FR-1043 line
  197, FR-202 lines 146-147, FR-202-AC-29).
- **Contradiction check.** The FR-1043 Forms table is scoped to proofs by
  the new preamble, and line 154 is narrowed to the proof threshold, so
  neither contradicts the new role. The FR-1042 claim-side rule (line 157)
  is overridden explicitly, and the overriding text does not conflict with
  it. The remaining conflicts are FND-002, FND-003 and FND-005.
- **FSpec carrier.** The author's citations were verified read-only at
  filament-specification `903947958abe7ff32865ace12d5f29ed0ec24074`:
  - schema.json `ProbabilisticCounterexample.evidence` `subsystem` carries
    `certificates[1..2]` of `ProbabilityCertificate`, and `scheduler` is
    optional;
  - FR-413's subsystem paragraph allows expected-reward refutations "with
    their own certificate kinds";
  - FR-412's finite-horizon-measure bullet says "the completed fraction for
    a mean";
  - FR-412's side rule says "`upper` for ... a refutation of `>= θ`;
    `exact` ... for either".

  These support the carrier claim. FSpec does not need to change.
- **Hash/pin antipattern.** None. "identity" means the structural item
  identity, not a digest or a pin.
- **Structure.** `quire validate --scope .` on the two paths exited 0 with no
  output. `git diff --check 3498af270b^ 3498af270b` on the two paths exited
  0. The validator does not detect FND-001.

## Assurance Context

No AssuranceProfile document and no `review_policy` exist in this repository.
A search for `type: AssuranceProfile` and `review_policy` found only prose in
older reviews under `reviews/`. No enforced spec-review run applies and no
acceptance gate is claimed. Tools: quoin 0.36.8, quire 0.52.0
(engine 0.62.2), module `spec-artifacts-process@v0.36.0`. No cargo, tests,
matrix or full validation sweep ran, by dispatch.

## Dispositions

### Round 1

Rechecked against pure-spec commit `beb797a543184e85a1f724296b76dc00fd930951`
(diff `3498af270b..beb797a543` on the two paths). The working documents are
byte-identical to that commit. `quire properties --json` on both documents now
extracts FR-1043-AC-1..9 and FR-202-AC-27..29, each exactly once.
`quire validate --scope .` on the two paths exits 0 with no output, and
`git diff --check` exits 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: The blank lines are gone, so both AC tables are contiguous. quire extracts FR-1043-AC-8/9 and FR-202-AC-27/28/29. |
| FND-002 | fixed | Round 1: FR-202 Replay rules: "For subsystem evidence other than a mean certificate it SHALL rebuild the induced chain ... For a mean certificate it SHALL check the original finite-horizon-measure base product, preserving its workload, minimum or maximum optimization ... applying the witness scheduler SHALL NOT replace that base with an induced-chain objective." |
| FND-003 | fixed | Round 1: FR-202 adds the "Checked-subject mean-certificate replay entry". Native replay routes mean subsystem evidence to it, and the wire facade dispatches to the same checks. Line 222 is narrowed to "The checked-subject path-set subsets", and AC-11, AC-16 and AC-21 are narrowed to the path-set entry and subset. AC-27 and FR-1043-AC-8 name the entry. |
| FND-004 | fixed | Round 1: Outcomes are now typed, and each case has exactly one outcome. A valid document failing the strict comparison, including equality, gives `ThresholdMissed` at the initial key. A wrong kind for the role gives `Malformed` at `kind`, "even if its number lies on the wrong side". Failed conditions keep FR-201's rule and locus. Each of these settles `inconclusive`, `CertificateRejected{rule, at}`. Policy disagreement or a witness that does not attain the exact extremum gives `ReplayParity`, and identity or base mismatch keeps its refusal. The text is the same in FR-1043 and FR-202, and AC-9, AC-28 and AC-29 match it. |
| FND-005 | fixed | Round 1: The text drops "complete product" for "product coverage". `exact` and `upper` use the full stopped product. A `lower` may name a subset, with each unnamed `fraction` successor counted as 0, and an accepted partial `lower > θ` refutes `<= θ` without a counterpart. This is consistent with FR-202 line 172 and FR-1042 lines 90-97. |
| FND-006 | fixed | Round 1: The new mean text uses `θ` throughout, and FR-1043 adds that `θ` "SHALL NOT be written into `measure.threshold`, which remains null for payoff `fraction`". The remaining `c` occurrences are quantile `M op c`, which is correct. |
| FND-007 | fixed | Round 1: AC-28 and FR-1043-AC-9 name a valid `lower` on the minimum of `1/5` (at most `11/48`) at `>= 1/2`, and a valid `upper` on the maximum of `1/2` (at least `5/12`) at `<= 1/3`. Each rejects `Malformed` at `kind`. |

Round-1 checks: the arithmetic is unchanged and correct. The partial `lower`
`2/5 > 1/3` on the maximum `5/12` can be met by a subset that leaves out only
zero-payoff decided states, so AC-9 can be satisfied. The new entry's contract
covers the package, model, claim binding, typed certificate, limits, `&Cancel`,
a final poll and an opaque outcome. It adds no wire field, profile, request flag
or hash or pin. FSpec is unchanged and is not required. No new findings.

**Round-1 verdict: PASS.** The normative spec scope of the FR-202 and FR-1043
mean/fraction refutation amendment is accepted on BASE. This verdict makes no
claim about the implementation, the tests, or whether the PR can merge.
