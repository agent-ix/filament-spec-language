---
id: SR-967175026302197768900474181941935084838
title: "code-review of QSL-3226 verification-plan fixture (FSL PR #3844)"
type: SpecReview
analysis: code-review
scope: "agent-ix/filament-spec-language@208c7f909f03c431874cc8d503b1aba3c98383fc; tests/it/request_surface_s1_16.rs"
review_set: subset
---
# SR-967175026302197768900474181941935084838: code-review of QSL-3226 verification-plan fixture (FSL PR #3844)

## Summary

Ticket: QSL-3226. PR agent-ix/filament-spec-language#3844 at head
208c7f909f03c431874cc8d503b1aba3c98383fc. The diff against `origin/main...HEAD` touches one file,
`tests/it/request_surface_s1_16.rs`. The rust-review lane is folded into this file. No cargo
command was run by the reviewer. The evidence is the coder's per-test logs at this head (each of
the six `RELEASE_PLAN` tests 1/1 passed, peak RSS 61-127 MB) plus reading the code.

The change does four things:

- It renames the second plan's reserved step `only` to `sole`. The pre-fix log shows the refusal
  `invalid_syntax` / "`only` is a reserved word" at stage `source`, so the original red is
  explained and fixed.
- It runs the single-plan case from `bump_directory(&one, &["c1"], true)`. That is one object,
  with the model's `versionNumber` bound narrowed from 1000 to 3 (asserted at line 2479), and the
  snapshot written as `s.json`. This bounds FR-120's per-expansion successor set and removes the
  25 GB blowup. QSL-3262 tracks the product-side `max_candidates` defect separately.
- It renames the claim `Always` to `Ready` in the shared `RELEASE_PLAN`. No spec text and no test
  assertion names that claim, so none of the six sharing tests depends on the name.
- It drops the `FSpec-FR-824-AC-2` trace.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-1038-AC-2 refusal loop cannot fail for a wrong pointer, a wrong code, or a stray outcome. `refusal_names(&value, pointer)` is a substring test of the whole document for `/verification_plan/plan`, but the AC requires exactly `/request/verification_plan/plan`. Neither `invalid_runtime_input` nor `missing_declaration` is asserted. "No `verification_plan` outcome" is checked only as the absence of `dispositions`. So a regression that refuses at `/verification_plan/plan`, or codes `Nope` as `invalid_runtime_input`, or writes an emit-only outcome object, still passes. The code was there before this PR, but the PR's job is to retain this AC's assertions. Fix it in this PR with `refusal_path(&value) == "/request/verification_plan/plan"`, `value["code"]` per case, and `value.get("verification_plan").is_none()`. `fsl-command/src/command/plan.rs:65-71` builds exactly that path, so the stronger assertions hold. | tests/it/request_surface_s1_16.rs:744-751, tests/it/request_surface_s1_16.rs:419-424 |
| FND-002 | low | The test keeps `#[trace("FSpec-TC-2039", "FSpec-FR-824-AC-3")]`, but it exercises none of FSpec FR-824-AC-3's clauses: `seed` `"7"`, `emit`/`execute` false, `engine`, `seed` on `model_check`, `document`, and `Sized`. The selection cases it does run are FR-824-AC-2's, and the PR correctly untagged those per e92. The remaining FSpec tag is a wrong binding. Drop it along with AC-2 and leave `FR-1038-AC-2`. FR-824-AC-3 keeps real coverage from the reader test at line 588 and from `fsl-command/src/command/plan/request.rs`. | tests/it/request_surface_s1_16.rs:680-688 |
| FND-003 | low | The four new single-plan assertions have no `"{value}"` message, unlike every other assertion in the file. If `safe` is not `refuted` (for example `unsupported` because the initial was not admitted), the failure shows only `left`/`right` and not the refusal or the disposition that explains it. | tests/it/request_surface_s1_16.rs:700-712 |

## Verdict

Changes requested, for one medium finding (FND-001) and two low findings. Each is a small edit
in the same test.

What holds:

- Every success-path assertion FR-1038-AC-2 needs is present and can fail for the intended
  reason. `plan` == `Release` and the outcome `format` show the only plan ran. `safe` == `refuted`
  is not made vacuous by the bounded domain. `Never` is `false` at every state, so the result does
  not depend on the domain. It is only `refuted` if the step actually executed from the admitted
  `s.json` initial, since a no-initial run settles `unsupported`, as line 559 shows. That makes it
  a real check that the snapshot was consumed.
- `inputs.max_trace_length` == 8 is asserted.
- The two-plan `missing-member`, `Nope` and `Never` `missing-name` cases each reach `select` in
  `plan.rs:303-365` now that `sole` parses. The test's cause and exit-20 checks can fail.
- `seed` `"7"` is covered exactly by the reader test at line 593, which is tagged FR-1038-AC-2.
- Dropping `FSpec-FR-824-AC-2` is right. Its `{"max_trace_length": 8}` no-initial run and its
  `ambiguous-name` clause are not exercised here.
- The other five `RELEASE_PLAN` tests check step names (`safe`, `tested`, `timely`), node counts
  and edges, never the claim name, and each passed at this head in the coder's logs.
- `bump_directory`'s narrowing assert (line 2479) guards against a silent fixture drift back to
  1000.
