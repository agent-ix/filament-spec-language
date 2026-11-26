---
id: SR-96743
title: "spec-review/dependency of FR-1051 certificate checker trusted base"
type: SpecReview
analysis: dependency
scope: "agent-ix/filament-spec-language; spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md; PR4035 47b83d02c1717277360e3b7794ea6851d200e3d1"
review_set: subset
relationships:
  - target: ix://agent-ix/filament-spec-language/FR-1051
    type: references
---

## Summary

Ticket: QSL-3504. Independent review at head 47b83d02c1717277360e3b7794ea6851d200e3d1 (merge base 9842eb939a). Read-only; no cargo was run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Frontmatter uses specifies (ADR-028, ADR-026) and references (FR-284); the project rule allows spec links only as implements or depends_on | spec/functional/FR-1051-keep-each-certificate-checker-on-a-declared-trusted-base.md:14 |

### FND-001 detail

The relationship types break the project rule for requirement frontmatter links, and the approved design asked for depends_on to ADR-026 and ADR-028. The specifies edges were added after the first review pass (FND-008). Use depends_on for the two ADRs and FR-284 (or drop FR-284); the narrowing of CE-7 and CF-6 is already stated in the Description.

## Verdict

Changes requested: relationship types. All seven targets exist.

## Scope and evidence

- FR-1051: examined

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Checked at 2483104d1e |
