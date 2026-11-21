---
id: SR-202611211854275
title: "dependency analysis of FSL PR3373 QSL-2739 provider contract alignment"
type: SpecReview
analysis: dependency
scope: "agent-ix/filament-spec-language; PR3373 780a3d7b890510be4a4e8c83e0b585df46d1c42d; FR-1044, FR-530 relationships and Dependencies sections; FSpec #792 5423fcf482"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1044
    type: references
  - target: ix://agent-ix/filament-spec-language/FR-530
    type: references
---

## Summary

Ticket: QSL-2739. This review checks the declared edges and cited authorities against what the changed text actually depends on.

FR-1044 already carries `depends_on` edges to FSpec FR-348, FR-491, FR-492, FR-331, FR-290 and FR-489, and to FSL FR-530, FR-620, FR-621, FR-507, FR-281, FR-285 and FR-635. Upstream #792 is merged (5423fcf482), so no edge points at unmerged authority.

Enablement and feature work stay separate:
- The code PR depends on this spec PR.
- AC-10 package parity is owned outside this PR, by the package-consumer API lane, and stays open.
- The text driver is external.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-1044 cites "FSpec FR-290 'Replay context'" for the forged-witness rejection (line 331), but FSpec FR-290 (protocol claim kind) has no such section. The section is FSL FR-290 "Settle plugin results as typed terminal records", lines 332–376, and FR-1044 has no `depends_on` edge to FSL FR-290. Fix: cite FSL FR-290 and add the edge, or drop the citation if failure-domain FND-002 changes the cause. | spec/functional/FR-1044-request-a-component-composition-item-through-analyze.md:331; spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:332 |
| FND-002 | low | FR-530's new Dependencies bullet names FSpec FR-331 and FSL FR-1044, but the frontmatter `relationships` carry neither. Fix: add `depends_on` FSpec FR-331, and `references` FSL FR-1044 (FR-1044 already `depends_on` FR-530, so keep the edge one-directional to avoid a cycle). | spec/functional/FR-530-carry-and-check-liveness-and-fairness-premises-in-native-composition.md:5-31,186-188 |

## Verdict

**Minor changes.** Both findings are low. There is no blocking dependency: every upstream authority the PR applies is merged, and no new ticket is needed.

## Scope and evidence

Spec-only. The upstream objects were read with `git show 5423fcf482:<path>` in the read-only checkout /home/peter/dev/filament-specification.

- FR-1044 frontmatter and Dependencies: examined.
- FR-530 frontmatter and Dependencies: examined.
- FSpec FR-290 (grep for "Replay context": no hit) and FSL FR-290 Replay context: context_only.

## Dispositions

Round 1 (final disposition), fix commits 8692503282 and 5a4b61b1925aa3365c1773fe3fb8945784f99af5 over 780a3d7b89, same base deccdc2a50; verified against the committed text at 5a4b61b192, not the author's report. quire validate exit 0 on FR-1044, FR-530, FR-507, TC-1330.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The citation is now FSL FR-290 'Replay context', used only for the assignment path. FR-1044 gains depends_on FSL FR-290, and the Dependencies bullet names it. |
| FND-002 | fixed | FR-530 frontmatter adds depends_on FSpec FR-331 and references FSL FR-1044. That is one-directional, so there is no cycle. |
