---
id: SR-2026111919143300000002
title: "spec-review of QSL-2178 FR-1040 clock-place references and presence in population forms"
type: SpecReview
analysis: integrity
scope: "agent-ix/filament-spec-language; PR #3077 at f3ec9edd; spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md, spec/test-cases/TC-3084-clock-place-references-and-presence-in-population-forms.md; context: FR-230 (Places, Place identity, Selection, Presence reads), FR-139, FR-391, FR-093, FSpec FR-829 (PR #753), FSpec FR-322, FSpec FR-415, FSpec FR-272"
review_set: subset
---

## Summary

Spec review of FSL PR #3077. Ticket: QSL-2178. FR-1040 states that S3 checks `ref(p)` and `present(p)`, including inside population forms and through object references, and that S4 lowers them to FSpec FR-829's place-reference application and FR-322's `presence_read`. It lists the typed refusals that remain and orders call lowering after the callee. TC-3084 verifies AC-1 to AC-7. The review checked FR-1040 against the QSL-2178 ruling, FSL FR-230, FSpec FR-829 as proposed in PR #753, the FR-272 cause catalog and the open FSL PRs.

## Examined scope

- Ruling items 1 to 5 (examined). Every remaining refusal in the ruling appears with its code and cause: clock read in a population body, `ref` under a Map value or of a non-place, a non-summable `sum` summand at S3, ordering over place ids, and a clock-reading call in a data position. The no-presence-obligation rule for `ref` and the FR-230 selection obligation on `ref(x.q[i])`, which `where` can discharge, match the ruling. "A refused callee is not re-reported" is AC-7.
- Codes and causes (examined). `invalid_timed_model`/`clock-read-outside-constraint`, `ill_typed`/`type-mismatch`, `ill_typed`/`operator-ineligible` and `undefined_expression`/`unproved-presence` are all in FSpec FR-272.
- Soundness (examined). The binder visits only held objects, so the owner is never absent. The Map refusal agrees with FR-230 "Place identity". A `late(t: Clock)` callee is a valid clock-reading declaration under FSpec FR-415, so AC-6's refusal falls at the call and not at the callee, which is consistent with "Calls lower after their callee". The sum summand is refused at S3, so S4 never sees one.
- Ids (examined). FR-1040 and TC-3084 are free on main, and no open FSL PR uses them. The `depends_on` FSpec FR-829 is unmerged, so FSpec PR #753 must land first.
- Each AC can fail, and the PR leaves no `unsupported`/`not-yet-implemented` text (examined).

## Verdict

Not mergeable. FND-001 is high. The owner encoding (`deref` of the binder or data read) inherits FSpec FR-829's unresolved owner rule (FSpec SR FND-001). It also contradicts FSL FR-230 "Presence reads", which FR-1040 does not amend. FND-002 is medium: AC-5 and AC-6 use free variables. FND-003 and FND-004 are low. Merge only after FSpec #753 settles the owner encoding and this FR follows it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-2, AC-3 and AC-4, and the "Paths through a reference" bullet, require the owner term to be `filament.op.model.deref` over the binder `c` or the data read `self.peer`. FSL FR-230 "Presence reads" (line 603) writes `presence_read` with the owner term `self`, or another object's term such as `self.peer`, directly, and FR-1040 does not amend it. The published FSpec vectors also pass the `Reference<O>`-typed `self`/`other` directly. FR-1040 does not say how `self`, `other` or a parameter owner is carried, so binders and `other` (both `Reference<Call>` parameters) would be encoded differently. Align with the encoding FSpec #753 settles, and amend FR-230 "Presence reads" in the same PR | spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:125 |
| FND-002 | medium | AC-5's `i` (in `count(c in M::calls where i < size(c.q) and ref(c.q[i]) = self.r)`) and AC-6's `k` (in `ref(c.m[k].x)`) are not bound anywhere. TC-3084's fixture declares neither, and the clause position (refinement row, precondition or invariant) is not given. Whether the negative refuses at `i` depends on where `i` comes from. Bind them, for example as an operation parameter `i: Int[0, 1]` in a precondition | spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:126 |
| FND-003 | low | "FR-146's presence obligation" and "under FR-146's rules" are unqualified. In this repo, FR-146 is "write per-step simulation records for a refinement". The presence and guard rules are FSpec FR-146 "Accepted proof forms". Write "FSpec FR-146" | spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:67 |
| FND-004 | low | "`present(p)` of a guarded place SHALL lower to FSpec FR-322's `presence_read` ... the slot terms" covers the indexed read `present(x.q[i])`. FSpec FR-322 "Presence" says "The indexed read of an element (`self.q[i]`) is not a presence form in this version". Limit the bullet to binder-selected slots, or note the FSpec gap | spec/functional/FR-1040-check-and-lower-clock-place-references-and-presence-in-population-forms.md:68 |
