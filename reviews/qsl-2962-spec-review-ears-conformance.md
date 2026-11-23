---
id: SR-3611004
title: "EARS review of the compile request call_site text (filament-spec-language PR 3611)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/filament-spec-language@ce7b26509779e9bcdb14c50656ddf19938f4178a; spec/functional/FR-027-export-compiled-native-package.md (Outputs call_site sentences); spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md (Missing selections note); spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md (Dependencies line)"
review_set: subset
date: '2026-11-23'
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-027
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-286
    type: references
---

# EARS review of the compile request call_site text (PR 3611)

## Summary

Ticket: QSL-2962. The new normative sentences were checked by hand against
the EARS patterns. `quire validate` raised no grammar warning on a changed
line; its 7 warnings (FR-121:199, 293; FR-286:387; FR-865:261) are on
unchanged text. The FR-121 Dependencies line is non-normative.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-027's new sentences mix patterns, and their subjects are not the system. "The command SHALL, after the request is admitted, locate ..." is an event requirement without `When`. "a `CallSiteRefusal` SHALL refuse the compile" gives the refusal, not the command, as subject. "A `kind` other than ... refuses `invalid-request`" has no `shall`. Candidate: "When a `1-draft` request carrying `call_site` is admitted, the command shall locate that selection over the request's own checked package. If the selection is refused, then the command shall refuse the compile with the one diagnostic `OutcomeDiagnostic::from_call_site_refusal` builds, category `refusal`, exit 20. If `call_site` is not `{kind: "imported-function", library, function}` with exactly those members, then the command shall refuse `invalid-request` naming `call_site`." | spec/functional/FR-027-export-compiled-native-package.md:96 |
| FND-002 | low | The FR-286 note's normative sentence is passive, and its condition does not hold: "that diagnostic SHALL carry `selection` `extension-definition` ... and no producer of the pair SHALL write it without `selection` except the one below". The second clause repeats the section's opening "Such a diagnostic SHALL carry `selection`, except ...". The first is moot once AC-41 goes (SR-3611001 FND-003). Remove the SHALLs from the note and keep it as descriptive prose. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:591 |

## Verdict

Minor. Both are wording. They fall out of the SR-3611001 fixes, and the FR-027
candidate also carries the category, exit and own-package wording.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-027:100 now reads "When a `1-draft` request carrying `call_site` is admitted and its compile succeeds, the command shall locate ...". The refusals read "If the selection is refused, then the command shall refuse the compile ..." and "If `call_site` is not an object ..., then the command shall refuse `invalid-request` naming `call_site`." |
| FND-002 | fixed | The FR-286 note is now descriptive prose with no SHALL ("This is the current implementation gap, not a requirement"). |
