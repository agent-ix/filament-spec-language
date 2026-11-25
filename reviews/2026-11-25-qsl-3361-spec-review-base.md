---
id: SR-967175026302197768900474181941935084869
title: "QSL-3361 certificate package stack refusal spec review"
type: SpecReview
analysis: base
scope: "QSL-3361; FR-621-AC-23; TC-3154"
review_set: subset
---

## Summary

Reviewed the new refusal criterion and its linked integration test against the reported route: `HeldCertificate::resolve` to `check_probability_certificate_package`. The criterion names the caller-facing checker, literal 10,000-node/9,999-link default depth, 512 KiB stack, malformed index sequence and typed result.

## Verdict

PASS. The acceptance statement is observable and the test procedure exercises the package-checking path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found (placeholder) | - |
