---
id: SR-20261121051945000000001
title: "Spec review — QSL-2158 completion spec batch 5: probability certificate profiles (FSL #3263, FSpec #779)"
type: SpecReview
analysis: base
scope: "agent-ix/filament-spec-language PR #3263 at 615896b58a7b14a8b05901a8cc0b9c6ed956e478 (FR-1041..1044, FR-201/202/203/741, TC-3150..3153, TC-636/641/2065/2066); agent-ix/filament-specification PR #779 at 308600171332f6d21af6d504771e37db5d9d97d4 (FR-411/412/413, proposals/backend-provider-v1/schema.json, TC-2110/2111)"
review_set: subset
evaluated_revision: "615896b58a7b14a8b05901a8cc0b9c6ed956e478"
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-201
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-202
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1041
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1042
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1043
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
---

## Summary

Ticket: Linear QSL-2158. One spec-review pass over two spec-only PRs, read with `gh pr diff` and `git show` against `origin/main` and the PR heads in the existing FSL and FSpec clones. Nothing was built.

The deciding question was whether origin/main already specifies the four claim classes of FSL FR-741-AC-2 that the code lane reported as having no certificate profile. The lane's later correction ("main already specifies all four; the gaps are implementation gaps") is only partly right.

Main specifies classes 1 to 3 only in FR-201's general `ProbabilityCertificate` form (FR-201:116-123, 561-564, 573-580, 583-607, checked by FSpec FR-412:109-121). That general form has no wire objective: the schema says "FR-412 owns the objective's members" (schema.json:1343), and FR-412:61 gives prose only. The checker binary on main accepts closed profiles only, and its four TC-2065 stubs are ignored. On top of that, main contradicts itself. FR-201:244-248 says dyadic lower/upper documents "remain unsupported … until their own certificate profile is defined", while FR-201-AC-2 and FR-203-AC-1 require an accepted §15.1 value-bounds proof. FR-741-AC-2 calls P95's certificate `exact`, while FR-201:583-588 requires a `Conditional` one for a workload quantile. FSpec FR-411 says "a lower or upper certificate", while FR-201:111-112 says both. FSpec FR-412's general Lower/Exact rule for a minimum (progress "under every action", FR-412:117-120) cannot accept a true fair minimum when an unfair loop exists, and FR-412 does not say how the fair end components fix values. Class 4 (undefined witness) is the one that is substantially specified on main.

Arithmetic checked: the main stub `a_finite_horizon_quantile_certificate_is_accepted` (tools/fsl-probability-check/tests/it/certificate_kinds.rs) states `quantile 0.5 … <= 3` over `link`. From each undelivered state, `send_a` delivers, stays or stops with 1/3 each, so Pr(M ≤ 3) = 1/3 + 1/9 + 1/27 = 13/27 < 1/2, and the claim is false. FR-1043-AC-6's level 0.4 gives 13/27 − 2/5 = 11/135, which is correct. Every other numeric AC in both PRs was recomputed and holds: FR-1041-AC-1..4, FR-1042-AC-2..5, FR-1043-AC-1..3/5, FR-201-AC-56, FR-202-AC-23, FR-412-AC-13..15.

Id collisions: none. FSL main's highest ids are FR-1040 and TC-3137, and no open FSL PR adds FR-104x or TC-315x/316x. FSpec main has no TC-2110 or TC-2111, and no open FSpec PR touches FR-411..413.

## Verdict

**FAIL — keep both PRs, fix round needed.** 2 high, 4 medium, 4 low.

Per class:

- **Fair unbounded eventual: partly on main.** FR-201:116-123, 561-564, AC-3 (FairTerminates only, where U is empty) and FR-200:80-86 (engine) are on main. The certificate conditions with nonempty U are missing, and no wire profile exists: cyclic-mdp-eventual requires `fairness` empty (FR-201:403, 414). FSpec FR-412 "Fair minimum" and FSL FR-1041 fill a genuine gap. I checked both soundness arguments (w = 1 − v; a pre-fixed point of max dominates the lfp; tight progressing actions realise w; sup over fair schedulers = max). Lifting the empty-fairness restriction by pointing at FR-1041 is consistent with main.
- **Value bounds: partly on main.** FR-201:111-112, FR-198:146, FR-412:115-120 and AC-2 are on main. Main has no wire profile and no pairing rule, and it contradicts itself at FR-201:244-248. Removing that bullet is right. FR-1042 and the FR-412 pairing paragraph are genuine. FR-1042 also contradicts FR-198 on rewards (FND-002) and carries the partial-coverage soundness hole (FND-001).
- **Finite-horizon measure, quantile, P95: partly on main.** XF-2 (FR-201:573-580, 588-591) and Conditional Ratio (FR-201:583-607) are on main, but main has no measure-product objective members and no decided-state payoffs. FR-1043 and FR-412 "Finite-horizon measure" are genuine. Changing FR-741-AC-2 to `conditional`/`ratio` is right: it matches main FR-201:583-588, and main FR-741-AC-2 was the wrong text. Adding `always[0,h]` to the bounded subset is a genuine gap fill: §15.1 is a bounded `always`, and main certifies it only in the unserialisable general form. It changes no main bytes, and FR-201-AC-11 still holds.
- **Undefined witness: yes, mostly on main.** FR-202:58, 368-373, AC-5, FR-203:110 and AC-5, FR-196-AC-5/13, and FSpec FR-413 undefined path and replay are all on main. Only two parts are genuine: a checked-subject replay entry (main's checked-subject entries exclude it, and the wire facade is blocked, FR-202:337-340), and the wire `undefined` member that FR-202:58's native evidence already has. The rest of FR-1044 duplicates main (FND-007).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A `lower` document may name a subset of states, and an unnamed successor counts as 0. Over FR-1043's `shifted` base, values lie in [−level, 1−level], so counting a successor as 0 can overstate the bound, which is unsound. Retry fast/slow at level 1/2: a `lower` naming only the initial key with value 0 passes (Q_fast = 1/4, Q_slow = 1/8, bounded base, no progress rule), although min Pr(M ≤ 2) = 7/16. FSpec FR-412:120/135 has the same hole (latent on main at FR-201:588-591). Fix: over a base whose payoff can be negative, count an unnamed successor as the payoff minimum (−level), or require full coverage. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:86-91, 117-123; FSpec spec/objects/interfaces/FR-412-probability-certificate.md:120, 135 |
| FND-002 | high | FR-1042 says expected rewards have "no dyadic method" and settles them `Incomplete(LimitReached{MaxRationalBits…})` (AC-7). Main FR-198:117 decides expected rewards by interval iteration, FR-198:146 makes a dyadic interval yield a Lower and an Upper certificate, and FR-201:111-112 and FSpec FR-412 "Expected reward" certify them. Fix: add the three reward profiles as `base`s with FR-412's reward conditions, and keep the Incomplete rule for long-run only, which has no dyadic method in FR-199. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:69-75, 178 |
| FND-003 | medium | FSpec FR-411:233 and FR-412:230 now require a `lower` and an `upper` certificate for `value-bounds`, but the backend-provider result still has one `certificate`, required for `exact-value`/`value-bounds` (schema.json:1447, 1460). So the second document has no wire place, although FSL FR-741:110 puts it in `counterpart`. Fix: add the counterpart member to the provider result, required for `value-bounds` except a partial-product `lower`. | FSpec proposals/backend-provider-v1/schema.json:1447, 1460 |
| FND-004 | medium | "A missing counterpart … SHALL be rejected `Malformed` at `counterpart`". `counterpart` is neither an FR-412 certificate member nor an FR-331 `CertificateMember`, and FR-201's checker takes one certificate (FR-201:70-77), so no checker can return that result. Fix: state that EN-5/settlement (FR-203) checks the pairing and give the outcome a missing counterpart settles, or add a paired check entry. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:143-151 |
| FND-005 | medium | FR-1043 certifies a workload quantile whose values exceed `max_rational_bits` through the `shifted` payoff in FR-1042 value bounds. Unchanged FR-201:611-613 says a quantile under a workload SHALL carry a `Conditional` certificate. Fix: add that exception to FR-201's Conditional certificates section. | spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md:69-74; spec/functional/FR-201-check-a-probability-certificate.md:611-613 |
| FND-006 | medium | FR-201-AC-56 is appended after the blank line that splits the AC table (pr :1301, main :1275), so, like AC-47..55, it is not minted until FSL #3261 lands. Fix: rebase onto or merge after #3261. | spec/functional/FR-201-check-a-probability-certificate.md:1301, 1311 |
| FND-007 | low | FR-1044 restates main. Its Emission and Replay rules repeat FR-202:368-373 and FSpec FR-413's replay, and FR-1044-AC-1 repeats FR-203-AC-5 and FR-202-AC-5 (TC-641). Fix: keep only the checked-subject entry (:78-83), the undecided-control rule (:101-104) and the every-scheduler entry check (AC-3), preferably as a "Checked-subject undefined entry" subsection of FR-202, and drop AC-1. | spec/functional/FR-1044-replay-an-undefined-evaluation-witness.md:61-75, 105-119, 134 |
| FND-008 | low | FR-1044 settles `ReplayParity` for any repeated key, under a workload too, where no scheduler is involved. FSpec FR-413:155-157 forbids repeats only under every scheduler. Fix: restrict the rule to every scheduler. | spec/functional/FR-1044-replay-an-undefined-evaluation-witness.md:92-93 |
| FND-009 | low | FR-1041-AC-3's accepted document lists only `s` and `u`, but :83 requires exact coverage of the stopped graph, which includes `g` and `f`. Fix: add `g` 1 and `f` 0 at rank 0. | spec/functional/FR-1041-certify-unbounded-fair-eventual-verdicts.md:83, 165 |
| FND-010 | low | "Other bounded temporal forms are written in the general form above" replaces main's "remain unsupported by this profile". The general form has no wire objective, and unchanged FR-201-AC-11 says such forms return explicit unsupported. Fix: keep main's wording. | spec/functional/FR-201-check-a-probability-certificate.md:174-175 |

## Recommendation

- **FSL #3263: keep.** Keep FR-1041 (with FND-009), FR-1042 (with FND-001, FND-002 and FND-004), and FR-1043 together with the FR-201 Conditional exception (FND-005). Keep the FR-201 amendments: `always[0,h]` bounded subset, the rewrite of :264-271, the FR-1041 pointer at :428, and AC-56 (after #3261), but restore the :174-175 wording (FND-010). Keep the FR-202 bounded `always` extension with AC-16/AC-23, FR-203's row and ACs, the FR-741 AC-2/AC-3 changes and `counterpart`, TC-3150..3152 and the TC edits. Reduce FR-1044 and TC-3153 to the checked-subject entry (FND-007, FND-008).
- **FSpec #779: keep.** Keep FR-412 Fair minimum, Finite-horizon measure, the value-bounds pairing, the ranking/policy/absence rows and AC-13..15; FR-411's row; FR-413's `undefined-path` members, replay text and AC-11; the schema `undefined` member; and TC-2110/2111. Fix FND-003 in the schema and FND-001 in FR-412:120/135.

## New findings (disposition pass 1)

Fix heads: FSL #3263 `9fa94a75d3696ee520548c61dc74bf72a13177d0`, FSpec #779 `f29957a7aaeaa8623bbf8cca1c48766a29aec1b7`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | medium | The fix round admitted expected-reward bases, but "a `lower` proof on a partial product reports upper 1" was kept. For a reward base, 1 is not an upper bound: the value may exceed 1, or be `+∞`. So `ValueBounds{lower, 1}` states an uncertified, possibly false interval end. FSpec FR-331:666-669 and the schema's `counterpart` rule key the exception on `basis.upper` = 1, which inherits this. The same key also lets a full-product `lower` whose dyadic upper end happens to be exactly 1 omit `counterpart` on the wire, although FSL settles that case `failed`. Fix: for a reward base (and in general), report the partial-product upper as unbounded (+∞) or as the base's least upper bound (1 for a probability, `1 − q` for `shifted`), and key the schema exception on an explicit partial-product marker, not on `upper` = 1. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:169, 177-178; FSpec proposals/backend-provider-v1/schema.json (value-bounds `counterpart` rule); FSpec spec/objects/interfaces/FR-331-backend-provider-envelope.md:666-669 |
| FND-012 | low | FR-1042:166-174 assigns the pairing check to "EN-5's settlement (FR-203)". FR-203's settlement table is not amended: it has no row for a `ValueBounds` proof with a missing counterpart (`failed`, `runtime_invariant`/`established-invariant-broken`) or a rejected counterpart (`inconclusive`, `CertificateRejected`). The outcomes agree with FR-203's existing internal-fault and rejected-certificate rows, so nothing contradicts. Fix: add the two rows to FR-203's table next to :110. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:166-174; spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:110 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-1042:92-95 counts an unnamed successor as the base's least value (`−level` for `shifted`); FR-201:584-586, FSpec FR-412:120-123 and FR-411 partial-product match; FR-1042-AC-10 / FR-412-AC-16 reject the counterexample. |
| FND-002 | fixed | FR-1042:67-71 admits the three reward profiles as bases, with a reward row in the conditions table; the Incomplete rule is now long-run only; AC-7 is rewritten and AC-9 added. |
| FND-003 | fixed | FSpec schema adds result member `counterpart` and a value-bounds rule, and FR-331 documents it (see FND-011 for the exception key). |
| FND-004 | fixed | Pairing moved to settlement: missing counterpart `failed`/`runtime_invariant`, rejected counterpart `inconclusive` `CertificateRejected`; the checker checks one document, told the side. |
| FND-005 | fixed | FR-201:614-619 adds the workload-quantile overflow exception to the Conditional section. |
| FND-006 | fixed | FR-201-AC-56 now sits at :1306, before the table split at :1307, so it is minted without #3261. |
| FND-007 | fixed | FR-1044 deleted; FR-202 gains "Checked-subject undefined entry" (:353-391) and the wire-facade bullet points to it; ACs moved to FR-202-AC-24..26 (TC-3153); no FR-1044 references remain. |
| FND-008 | fixed | FR-202:372-377 limits the repeated-key ReplayParity to every scheduler and admits repeats under a workload; AC-26 tests both. |
| FND-009 | fixed | FR-1041-AC-3 and TC-3150 now list `g` 1 and `f` 0 at rank 0. |
| FND-010 | fixed | FR-201:172-173 restores "Other temporal forms remain unsupported by this profile." |
| FND-011 | fixed | Round 2: FR-1042:177-185 marks a partial-product `lower` `exploration: Partial` with `upper` set to the base's least upper bound (1, `1 − level`, 1 for `fraction`, `+∞` written `"infinity"` for rewards); the schema keys the counterpart exemption on `basis.exploration` (requiring `certificate.kind` `lower`, forbidding `counterpart`) and admits `"infinity"` only with the marker; FR-331, FR-411, FR-412, FR-203 (`upper: ExtRational`, `exploration`) and FR-741 agree; FR-331-AC-59, FR-1042-AC-11 and FR-203-AC-9 cover it; the existing vector (positive/21, lower 99/100, upper 1) now carries the marker and satisfies the rule. |
| FND-012 | fixed | Round 2: FR-203:113-114 adds the rejected-counterpart row (`inconclusive`, `unsettled`, `CertificateRejected`) and the missing-counterpart-without-marker row (`failed`, `runtime_invariant`/`established-invariant-broken`), matching the existing internal-fault and rejected-certificate rows. |

## New findings (disposition pass 2)

Fix heads: FSL #3263 `9424c06b16129a02c0fb1d0643eb9d97b78a3024`, FSpec #779 `0bb2dea90ffd248a9f8cd2ae1300d037c018a1e7`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | low | The `exploration` marker is not written into every place that lists the value-bounds basis. FSpec FR-331:352's proof-basis catalogue still reads `value-bounds{lower, upper, method, reductions}`, with no `exploration` and no `"infinity"` upper. The FSpec least-upper-bound lists (FR-331:656-661, FR-411:267-271, schema description) name probability, shifted and reward bases but not the `fraction`/mean payoff, which FR-1042 gives 1. FSL FR-203:111's `proved` `ValueBounds` row names only the two-document case, not the partial-product `lower` that AC-9 settles `proved`. Fix: add `exploration` and the `"infinity"` upper to FR-331:352, add the fraction payoff (1) to the FSpec lists, and add "or a partial-product `lower` (`exploration: Partial`)" to FR-203:111. | FSpec spec/objects/interfaces/FR-331-backend-provider-envelope.md:352, 656-661; FSpec spec/functional/analysis/FR-411-settle-probabilistic-claims-exactly.md:267-271; spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md:111 |
