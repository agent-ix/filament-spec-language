<!-- reviewer repo=agent-ix/filament-spec-language visibility=public quoin=0.36.10 module=spec-artifacts-process@v0.29.2 id=SR-967175026302197768900474181941935084814 method=spec-review/base lang=spec pr=filament-spec-language#3689 model=gpt-6.2-sol run=A1937C16-4F7B-4F1F-9071-91D5585B6802 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| - | - | - | Clean review; FR-1042-AC-17 is within FR-201's supported source objective mapping |

+++ [reviewer data]

```yaml
clean: true
scope:
  - {id: FR-1042-AC-17, path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md, role: examined, excerpt: "Over AC-3's `W`, with no fairness set, for `probability <= rational(3,5) [eventually holds(delivered)]` under every scheduler, the maximum-probability `upper` document with `s = 3/5` and no policy is accepted as the claim-side document with objective Maximum. Offering the minimum-probability `upper` document with `s = 1/4` and policy `wait` for this claim rejects objective mismatch, as the source Every AtMost claim maps to Maximum; the same minimum objective is not an admitted Every AtMost source mode (FR-201, closed cyclic-MDP eventual profile)."}
  - {id: TC-3212, path: spec/test-cases/TC-3212-build-value-bounds-capabilities-in-conformance-order.md, role: examined, excerpt: "For AC-17's Every AtMost claim, check the Maximum `upper` as the claim-side document, reject the Minimum `upper` as objective mismatch, then check the Maximum `lower` counterpart with `wait` and `go` policy hints."}
  - {id: FR-201, path: spec/functional/FR-201-check-a-probability-certificate.md, role: context_only, excerpt: "Source `Every` AtLeast selects minimum; `Every` AtMost selects maximum. This profile adds no minimum AtMost or maximum AtLeast source mode."}
findings: []
bindings: []
```

+++
