---
id: SR-967175026302197768900474181941935084781
title: "integrity review of PR #3550 every-scheduler long-run proof profile"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-202-replay-a-probabilistic-witness.md, spec/test-cases/TC-3190-check-every-scheduler-long-run-proofs.md, spec/test-cases/TC-3191-refute-a-long-run-fraction-over-every-scheduler.md (git diff origin/main...900ddd14)"
review_set: subset
---
# integrity review of PR #3550

## Summary

Ticket: QSL-2924 (item 3). PR: filament-spec-language#3550 at 900ddd1477c116b0c5ad487671b073327f60e92d. Integrity (completeness, consistency, atomicity) of PR #3550's FR-201 MDP long-run profile, admission section, FR-201-AC-70..74, FR-202-AC-45, TC-3190/3191.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-201-AC-30 still says every-scheduler/MDP and unsupported kind/profile long-run claims remain unsupported, contradicting the new admission section and AC-70/AC-74 | spec/functional/FR-201-check-a-probability-certificate.md:1460 |
| FND-002 | medium | AC-73's 'adding a transient key' variant has no input: AC-71's Loop has no transient state | spec/functional/FR-201-check-a-probability-certificate.md:1490 |
| FND-003 | low | 'reuses the cyclic MDP eventual profile's rules' is not accurate for minimum: the progress rule here differs | spec/functional/FR-201-check-a-probability-certificate.md:1300-1321 |

## Detail

**FND-001** (high, soundness, confidence high): FR-201-AC-30 was not amended. It still says that 'every-scheduler/MDP ... and unsupported kind/profile remain explicitly unsupported'. Its bound test, fsl-analyze/tests/it/long_run_certificate.rs:398-411 (TC-1401, #[trace("TC-1401","FR-201-AC-30")]), asserts DispositionKind::Unsupported for exactly two claims: an every-scheduler `using inf` long_run_fraction claim and a `using ev` workload long_run_fraction claim. The new 'Long-run claim profile admission' section and FR-201-AC-70/71/74 now require that both are admitted and certified, and AC-74 says the claim 'SHALL NOT settle unsupported/not-yet-implemented'. So two ACs in the same FR contradict each other, and the existing traced test enforces the old one. Fix: rescope AC-30 to this DTMC profile. Say that unweighted every-scheduler claims use the MDP long-run profile (AC-70..74), that event-position claims are admitted as the admission section states, and that only the timed profile, weighted/fair every-scheduler claims and the other listed shapes stay outside. The implementation PR must then flip that test.

**FND-002** (medium, untestable-ac, confidence high): AC-73 lists 'adding a transient key rejects InequalityFails at the smallest differing key' as a variant over AC-71's Loop. Every Loop state is in a MEC: c1 and c2 are in {c1,c2}, and g and f are terminal-stutter MECs {g} and {f}. AC-70's Avail2 has no transient state either. So there is no transient key to add, and the variant cannot be built as written. Fix: add a transient state to a stated fixture (for example an initial not-P state s0 that steps to c1), or drop the clause. Adding s0 also changes AC-71's initial values, so name the fixture explicitly.

**FND-003** (low, ambiguous, confidence medium): The combination bullet opens with 'reuses the cyclic MDP eventual profile's rules with a stopping payoff in place of target value 1'. For minimum, that profile requires EVERY enabled action at a positive-valued state to have a lower-rank successor, plus Z zero-support actions (FR-201:481-488). This profile requires only SOME tight progressing action for both optimizations. The weaker rule is sound here, because every end component has a stop payoff rho_C, so the subharmonic inequalities bound every scheduler. But 'reuses ... rules' invites an implementer to carry the eventual minimum rule over. Under that rule AC-71's minimum would be rejected: `stay` at c1 has no lower-rank successor when every state has rank 0. Fix: reword to 'is modelled on' and keep the profile's own rules as the single statement.

## Verdict

I hand-verified the mathematics and every number. The per-MEC gain/bias optimality conditions (max: h >= T_a for all staying actions, with equality for one; min: <=) are a sound and complete certificate without a unichain assumption. A MEC restricted to its staying actions is communicating, so the optimal gain is constant on it for both min and max. Multichain policy iteration ends with h satisfying the single optimality equation, so a rational certificate exists. Summing the inequalities bounds every staying scheduler, and any tight memoryless selection has fraction exactly rho_C on each of its recurrent classes. The stop-payoff combination with SOME-tight-progress ranks is sound and complete for both min and max. The true V is constant on each MEC, every staying action is tight there, and a bottom tight-closed end component without S would contradict the tight exit. The MEC refinement, check order and inclusive threshold are correct. Avail2: min 1000/1001 with h(down) = -2000/1001, and repair_fast gives -1200/1001. Max 1800/1801 with h(down) = -2000/1801, and repair_slow gives -2800/1801. The wrong-gain rejections at down (-2000/1801 > -2800/1801 and -2000/1401 > -2400/1401) hold. Loop: rho 1/2 with biases (0, -1/2); min V = 1/2 with S = all states; max V = 3/4 with ranks 1 and 2. The bogus fixed points and their rank loci, the 3/4-component loci (c1 fails with h(c2) = -3/4, c2 fails with -1/4) and the omitted-MEC false minimum 3/4 are all correct. All thresholds check. The profile admission decision is correct and is in fact required: ADR-024 section 7.3's LongRun is written `using ev`, FR-199's use case decides it, FSpec FR-407 places no profile restriction on long-run and continues terminal windows on stutter, and ADR-018 section 1 admits ev and inf over models. The defects are the stale AC-30 contradiction, the unbuildable AC-73 variant and one misleading 'reuses'.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | AC-45's event-position case says 'the same scheduler and certificate', but the certificate's identity binds the profile; TC-3191 does not run the case | spec/functional/FR-202-replay-a-probabilistic-witness.md:676 |

**FND-004** (low, ambiguous, confidence medium): FR-202-AC-45 now says the `>= 9993/10000` claim written `using` event-position 'is refuted by the same scheduler and certificate'. FR-201's admission section says the selected profile 'remains part of the checked claim the certificate's identity binds'. Read literally, the infinite-trace certificate bytes would be refused with IdentityMismatch under the ev claim. The intended meaning is a certificate with the same conditions, bound to the ev claim. TC-3191's procedure and expected results also do not mention the event-position run. Fix: say 'the same scheduler and a certificate with the same component conditions bound to that claim', and add the ev run to TC-3191. Avail2 has no terminal state, so this case shows only that replay admits ev, not the difference between stutter and false extension. That is acceptable, because FR-201-AC-74 covers the difference on the proof side.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified at 11c18bc0 (round 1) |
| FND-002 | fixed | verified at 11c18bc0 (round 1) |
| FND-003 | fixed | verified at 11c18bc0 (round 1) |
