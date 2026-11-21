// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-861-AC-4 (TC-2442): `fsl-eval` takes the partial-value, completeness,
//! activation and repair authority rules from `filament-observation` and
//! holds no second implementation of them.

use fsl_eval::completeness::{revise_complete_through, CompleteThroughRefusal};
use fsl_eval::temporal::{CoveredList, ObservedPosition, ProgressCompleteness};
use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn sources(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("a readable source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
fn fsl_eval_calls_the_observation_owner_for_its_four_authority_rule_sets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("the manifest");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("\n[").next())
        .expect("a [dependencies] table");
    assert!(
        dependencies.contains("filament-observation"),
        "fsl-eval names filament-observation in [dependencies]"
    );
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    let text: String = files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("a source file"))
        .collect();
    let grouped: Vec<&str> = text
        .lines()
        .filter(|line| {
            line.trim_start()
                .starts_with("use filament_observation::authority::{")
        })
        .collect();
    for module in ["partial", "completeness", "activation"] {
        let named = text.contains(&format!("filament_observation::authority::{module}"))
            || grouped.iter().any(|line| {
                line.split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|word| word == module)
            });
        assert!(named, "fsl-eval calls the owner's `{module}` module");
    }
}

/// Every shipped source file of `fsl-eval`, concatenated.
fn eval_source() -> String {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("a source file"))
        .collect()
}

/// FR-944's quarter-tick endpoint of `value`, open or closed, at the lower
/// or upper end.
fn quarter(value: i128, open: bool, lower: bool) -> i128 {
    let scaled = value.checked_mul(4).expect("an i128 endpoint");
    match (open, lower) {
        (false, _) => scaled,
        (true, true) => scaled + 1,
        (true, false) => scaled - 1,
    }
}

/// Assert that `fsl-eval` maps its covers onto the owner's quarter-tick
/// intervals rather than comparing them itself.
fn assert_quarter_tick_owner_mapping(text: &str) {
    assert!(
        text.contains("EventTimeInterval"),
        "fsl-eval builds owner EventTimeIntervals for its covers"
    );
    assert!(
        text.contains("\"/4\""),
        "the owner interval's unit identity carries the `/4` suffix"
    );
}

#[test]
#[trace("TC-2624", "FR-944-AC-7")]
fn quarter_tick_scaling_is_exact_at_the_domain_edge() {
    use fsl_eval::temporal::{ClockCover, Containment};
    let max = u64::MAX;
    let wide = i128::from(max);
    let counter = |k: u64| {
        let k = i128::from(k);
        (
            quarter(k - wide, false, true),
            quarter(k + 1 + wide, false, false) - 1,
        )
    };
    assert_eq!(counter(max), (0, 147_573_952_589_676_412_923));
    assert_eq!(
        counter(0),
        (-73_786_976_294_838_206_460, 73_786_976_294_838_206_463)
    );
    assert_eq!(
        quarter(i128::from(i64::MAX), false, true),
        36_893_488_147_419_103_228
    );
    assert_eq!(
        quarter(i128::from(i64::MIN), false, true),
        -36_893_488_147_419_103_232
    );
    // FR-944's owner reading of `within_bound` and `definitely_before` on
    // the quarter-tick intervals, compared with `ClockCover`'s answers.
    let within = |bound: i64, request: (i128, i128), ack: (i128, i128)| {
        let shift = quarter(i128::from(bound), false, true);
        let (lo, hi) = (request.0 + shift, request.1 + shift);
        let after = ack.1 > lo;
        let before_or_equal = ack.0 <= hi;
        match (after, before_or_equal) {
            (false, _) => Containment::True,
            (true, false) => Containment::False,
            (true, true) => Containment::Indeterminate,
        }
    };
    let cover = ClockCover::Counter { uncertainty: max };
    for bound in [i64::MIN, -1, 0, 1, i64::MAX] {
        for (request, ack) in [(0, max), (max, 0), (0, 0), (max, max)] {
            assert_eq!(
                cover.within_bound(bound, request, ack),
                within(bound, counter(request), counter(ack)),
                "bound {bound}, request {request}, ack {ack}"
            );
        }
    }
    for (a, b) in [(0, max), (max, 0), (0, 0)] {
        assert_eq!(
            cover.definitely_before(a, b),
            counter(a).1 < counter(b).0,
            "{a} before {b}"
        );
    }
    assert_quarter_tick_owner_mapping(&eval_source());
}

fn at(identity: &str, stamp: u64) -> ObservedPosition {
    ObservedPosition {
        identity: identity.to_owned(),
        stamp,
        fingerprint: None,
    }
}

/// FR-944's "Owner requirement: completeness revision" cases, run on the
/// one local function FR-861 names. This pins the rule text case by case; it
/// does not scan the source for a second comparison, so it carries no AC tag.
#[test]
#[trace("TC-2442")]
fn the_complete_through_rule_accepts_and_refuses_as_fr_944_states() {
    let held = [at("a", 0), at("b", 50), at("c", 100)];
    let covered: CoveredList = held.iter().cloned().collect();
    let retained = ProgressCompleteness::Complete { through: 100 };
    let revise = |asserted, positions: &[ObservedPosition]| {
        revise_complete_through(retained, asserted, &covered, positions)
    };
    for through in [100, 140] {
        assert_eq!(
            revise(ProgressCompleteness::Complete { through }, &held),
            Ok(()),
            "Complete through {through} after Complete through 100"
        );
    }
    assert_eq!(
        revise(ProgressCompleteness::Complete { through: 60 }, &held),
        Err(CompleteThroughRefusal::Retreat),
        "Complete through 60"
    );
    assert_eq!(
        revise(ProgressCompleteness::Incomplete, &held),
        Err(CompleteThroughRefusal::Retreat),
        "Incomplete"
    );
    let same = ProgressCompleteness::Complete { through: 100 };
    for (stamp, expected) in [(80, true), (100, true), (101, false)] {
        let uncovered = at("d", stamp);
        assert_eq!(
            revise(same, std::slice::from_ref(&uncovered)),
            if expected {
                Err(CompleteThroughRefusal::UncoveredPosition(Box::new(
                    uncovered,
                )))
            } else {
                Ok(())
            },
            "an uncovered position stamped {stamp} against a boundary of 100"
        );
    }
    assert_eq!(
        revise_complete_through(
            ProgressCompleteness::Incomplete,
            ProgressCompleteness::Incomplete,
            &covered,
            &held
        ),
        Ok(()),
        "a retained Incomplete refuses nothing"
    );
}

fn bundle_global_lines(source: &str) -> Vec<usize> {
    use syn::visit::Visit;
    struct BundleType(bool);
    impl<'ast> Visit<'ast> for BundleType {
        fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
            self.0 |= node.path.segments.iter().any(|segment| {
                segment.ident == "StaticProducerBundle" || segment.ident == "AdmittedStaticBundle"
            });
            syn::visit::visit_type_path(self, node);
        }
    }
    struct Globals(Vec<usize>);
    impl<'ast> Visit<'ast> for Globals {
        fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
            if node.attrs.iter().any(|attribute| matches!(&attribute.meta,
                syn::Meta::List(list) if list.path.is_ident("cfg") && list.tokens.to_string() == "test")) {
                return;
            }
            syn::visit::visit_item_mod(self, node);
        }
        fn visit_item_static(&mut self, node: &'ast syn::ItemStatic) {
            if node.attrs.iter().any(|attribute| matches!(&attribute.meta,
                syn::Meta::List(list) if list.path.is_ident("cfg") && list.tokens.to_string() == "test")) {
                return;
            }
            let mut bundle = BundleType(false);
            bundle.visit_type(&node.ty);
            if bundle.0 {
                self.0.push(node.ident.span().start().line);
            }
        }
        fn visit_field(&mut self, node: &'ast syn::Field) {
            let mut bundle = BundleType(false);
            bundle.visit_type(&node.ty);
            struct Lock(bool);
            impl<'ast> Visit<'ast> for Lock {
                fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
                    self.0 |= node
                        .path
                        .segments
                        .iter()
                        .any(|segment| segment.ident == "Mutex" || segment.ident == "RwLock");
                    syn::visit::visit_type_path(self, node);
                }
            }
            let mut lock = Lock(false);
            lock.visit_type(&node.ty);
            if bundle.0 && lock.0 {
                use syn::spanned::Spanned;
                self.0.push(node.span().start().line);
            }
        }
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            fn bundle(tokens: proc_macro2::TokenStream) -> bool {
                tokens.into_iter().any(|token| match token {
                    proc_macro2::TokenTree::Ident(ident) => {
                        ident == "StaticProducerBundle" || ident == "AdmittedStaticBundle"
                    }
                    proc_macro2::TokenTree::Group(group) => bundle(group.stream()),
                    proc_macro2::TokenTree::Punct(_) | proc_macro2::TokenTree::Literal(_) => false,
                })
            }
            if (node.path.is_ident("lazy_static") || node.path.is_ident("thread_local"))
                && bundle(node.tokens.clone())
            {
                use syn::spanned::Spanned;
                self.0.push(node.span().start().line);
            }
        }
    }
    let parsed = syn::parse_file(source).expect("Rust source");
    let mut globals = Globals(Vec::new());
    globals.visit_file(&parsed);
    globals.0
}

#[test]
#[trace("TC-3036", "FR-944-AC-15")]
fn observation_producers_are_caller_owned_without_global_storage() {
    assert_eq!(
        bundle_global_lines("static B: OnceLock<AdmittedStaticBundle> = OnceLock::new();"),
        vec![1]
    );
    assert_eq!(
        bundle_global_lines("struct Host { bundle: Mutex<StaticProducerBundle> }"),
        vec![1]
    );
    assert!(bundle_global_lines(
        "#[cfg(test)] mod tests { static B: OnceLock<AdmittedStaticBundle> = OnceLock::new(); }"
    )
    .is_empty());
    assert!(bundle_global_lines(
        "const NAME: &str = \"static B: OnceLock<AdmittedStaticBundle>\";"
    )
    .is_empty());
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let findings: Vec<_> = files
        .iter()
        .flat_map(|path| {
            let text = std::fs::read_to_string(path).expect("source");
            bundle_global_lines(&text)
                .into_iter()
                .map(|line| format!("{}:{line}", path.display()))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(findings.is_empty(), "producer globals: {findings:?}");
}

#[test]
#[trace("TC-2624", "FR-944-AC-14")]
fn evidence_calls_consume_optional_activation_views_for_scopes_and_instances() {
    use crate::temporal_activation as fixture;
    use fsl_eval::temporal::evidence::{ActivationState, NonValueKind, TriggerEvidence, Truth};
    use fsl_eval::temporal::{ProgressClosure, Valuation};
    let env = fixture::env(fixture::GUARDED, "eventually[0,2] holds(p)");
    let atom = env.atom("p");
    // This low-level checked-facts fixture supplies the same opaque package
    // selection bytes as the owner unit fixtures; source/reader parity is separate.
    let mut subject = env.subject(&[]);
    subject.canonical_package_bytes = Some(b"{}");
    subject.time_unit_scale = fsl_eval::temporal::evidence::ClockScale::PerSecond(
        filament_exact::Rational::new(1_i64.into(), 1000_i64.into()).expect("ms"),
    );
    let mut request = fixture::request(&env, fixture::at_t0(&[(atom, Valuation::True)]));
    request.trigger = Some(fixture::id("t1"));
    let mut delivery = fixture::delivery("t1", "r1", "payload", &[]);
    delivery.guard = Some(false);
    request.deliveries.push(delivery);
    let inactive = fixture::call(&env, &subject, &request).expect("owner inactive");
    assert_eq!(inactive.activation, ActivationState::Inactive);
    assert!(inactive.instances.is_empty());
    assert_eq!(
        inactive.non_value.expect("inactive detail").kind,
        NonValueKind::Inactive
    );
    for (closure, evidence, kind) in [
        (
            ProgressClosure::Open,
            TriggerEvidence::Admitted,
            "trigger-scope-open",
        ),
        (
            ProgressClosure::Closed,
            TriggerEvidence::Missing,
            "trigger-evidence-missing",
        ),
        (
            ProgressClosure::Closed,
            TriggerEvidence::Refused,
            "trigger-evidence-refused",
        ),
    ] {
        let mut unknown = request.clone();
        unknown.trigger_scope = closure;
        unknown.trigger_evidence = evidence;
        let result = fixture::call(&env, &subject, &unknown).expect("owner unknown");
        assert_eq!(result.activation, ActivationState::Unknown);
        assert_eq!(result.non_value.expect("unknown detail").code, kind);
    }
    let env = fixture::env(fixture::EACH, fixture::ONE_CAPTURE);
    let atom = env.atom("p");
    let mut subject = env.subject(&[]);
    subject.canonical_package_bytes = Some(b"{}");
    subject.time_unit_scale = fsl_eval::temporal::evidence::ClockScale::PerSecond(
        filament_exact::Rational::new(1_i64.into(), 1000_i64.into()).expect("ms"),
    );
    let mut request = fixture::request(&env, fixture::at_t0(&[(atom, Valuation::True)]));
    request.trigger = Some(fixture::id("t1"));
    request.deliveries = vec![
        fixture::delivery("t1", "r1", "one", &["5"]),
        fixture::delivery("t2", "r2", "two", &["5"]),
    ];
    let result = fixture::call(&env, &subject, &request)
        .expect("two instance-specific activation histories");
    assert_eq!(result.activation, ActivationState::Active);
    assert_eq!(result.truth, Some(Truth::True));
    assert_eq!(result.instances.len(), 2);
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
#[ignore = "QSL-2145: nothing in fsl-eval calls the owner's `repair::plan` yet; the retained results hold no owner bundle views, regions or result bytes (FR-944 \"Repair\")"]
fn fsl_eval_calls_the_owner_repair_module() {
    assert!(
        eval_source().contains("filament_observation::authority::repair"),
        "fsl-eval calls the owner's `repair` module"
    );
}

/// The items FR-861-AC-4 forbids `fsl-eval` to define: the local rules the
/// four owners replace. A definition of any of these names is a second copy.
const FORBIDDEN_ITEMS: &[&str] = &[
    // partial: the local interval algebra (`EvidenceLetters` reads through the owner).
    "PossibilitySet",
    "OrderPossibilities",
    "EventTimeInterval",
    "relation_to",
    // activation: the local scope disposition rule.
    "scope_disposition",
    "no_instance",
    "ActivationView",
    "ActivationSelection",
    // completeness: a second revision rule or view.
    "CompletenessView",
    "revise_completeness",
    "revise",
    // repair: affected-region planning.
    "plan_repair",
    "RepairPlan",
    "FactRegion",
    "PriorResult",
    "DependencyEdge",
    "affected_region",
];

/// The one QSL-2327 item FR-861 permits while the ledger cannot call the
/// owner's `completeness::revise`.
const TRACKED_EXCEPTION: &str = "revise_complete_through";

/// Names of items defined (fn, struct, enum, trait, type, const) in `source`,
/// and of fns whose signature takes two `ProgressCompleteness` values, which
/// is a comparison of two completeness assertions.
fn defined_items(source: &str) -> (Vec<String>, Vec<String>) {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Items {
        defined: Vec<String>,
        comparing: Vec<String>,
    }
    fn completeness_params(signature: &syn::Signature) -> usize {
        signature
            .inputs
            .iter()
            .filter(|input| match input {
                syn::FnArg::Typed(typed) => matches!(&*typed.ty, syn::Type::Path(path)
                    if path.path.segments.last().is_some_and(|s| s.ident == "ProgressCompleteness")),
                syn::FnArg::Receiver(_) => false,
            })
            .count()
    }
    impl<'ast> Visit<'ast> for Items {
        fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
            self.defined.push(node.sig.ident.to_string());
            if completeness_params(&node.sig) >= 2 {
                self.comparing.push(node.sig.ident.to_string());
            }
            syn::visit::visit_item_fn(self, node);
        }
        fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
            self.defined.push(node.sig.ident.to_string());
            if completeness_params(&node.sig) >= 2 {
                self.comparing.push(node.sig.ident.to_string());
            }
            syn::visit::visit_impl_item_fn(self, node);
        }
        fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
            self.defined.push(node.ident.to_string());
            syn::visit::visit_item_struct(self, node);
        }
        fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
            self.defined.push(node.ident.to_string());
            syn::visit::visit_item_enum(self, node);
        }
        fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
            self.defined.push(node.ident.to_string());
            syn::visit::visit_item_trait(self, node);
        }
        fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
            self.defined.push(node.ident.to_string());
            syn::visit::visit_item_type(self, node);
        }
    }
    let parsed = syn::parse_file(source).expect("Rust source");
    let mut items = Items::default();
    items.visit_file(&parsed);
    (items.defined, items.comparing)
}

/// Findings of the scan over `files`: `(file, item)` for each forbidden
/// definition and each second comparison of two completeness assertions.
fn second_copies(files: &[(String, String)]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (name, source) in files {
        let (defined, comparing) = defined_items(source);
        for item in defined {
            if FORBIDDEN_ITEMS.contains(&item.as_str()) {
                found.push((name.clone(), item));
            }
        }
        for item in comparing {
            if item != TRACKED_EXCEPTION {
                found.push((
                    name.clone(),
                    format!("{item} compares two completeness assertions"),
                ));
            }
        }
    }
    found
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
fn fsl_eval_defines_no_second_copy_of_the_four_owners_rule_sets() {
    // The scan finds what it forbids.
    let planted = [(
        "planted.rs".to_owned(),
        "fn scope_disposition() {} struct OrderPossibilities; \
         fn revise_again(a: ProgressCompleteness, b: ProgressCompleteness) {}"
            .to_owned(),
    )];
    let found = second_copies(&planted);
    assert_eq!(found.len(), 3, "{found:?}");
    // The one tracked exception passes; a second comparison does not.
    let exception = [(
        "exception.rs".to_owned(),
        "pub fn revise_complete_through(a: ProgressCompleteness, b: ProgressCompleteness) {}"
            .to_owned(),
    )];
    assert!(second_copies(&exception).is_empty());
    // fsl-eval's own sources, shipped code and tests alike.
    let mut paths = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut paths,
    );
    let files: Vec<(String, String)> = paths
        .iter()
        .map(|path| {
            (
                path.display().to_string(),
                std::fs::read_to_string(path).expect("a source file"),
            )
        })
        .collect();
    let found = second_copies(&files);
    assert!(
        found.is_empty(),
        "second copies of an owner rule: {found:?}"
    );
    assert!(
        files
            .iter()
            .any(|(_, source)| defined_items(source).0.iter().any(|item| item == TRACKED_EXCEPTION)),
        "the tracked QSL-2327 exception is still the one comparison; delete it with this allowance when the ledger calls `completeness::revise`"
    );
}
