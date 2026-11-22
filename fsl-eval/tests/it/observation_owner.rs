// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-861-AC-4 (TC-2442): `fsl-eval` takes the partial-value, completeness,
//! activation and repair authority rules from `filament-observation` and
//! holds no second implementation of them.

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

/// Names of items defined (fn, struct, enum, trait, type) in `source`,
/// and of fns whose signature takes two `ProgressCompleteness` values, which
/// is a comparison of two completeness assertions.
fn defined_items(source: &str) -> (Vec<String>, Vec<String>) {
    use syn::visit::Visit;
    #[derive(Default)]
    struct Items {
        defined: Vec<String>,
        comparing: Vec<String>,
    }
    /// Whether `pat` names a `ProgressCompleteness` variant.
    fn completeness_variant(pat: &syn::Pat) -> bool {
        let path = match pat {
            syn::Pat::Path(node) => &node.path,
            syn::Pat::Struct(node) => &node.path,
            syn::Pat::TupleStruct(node) => &node.path,
            syn::Pat::Reference(node) => return completeness_variant(&node.pat),
            _ => return false,
        };
        path.segments.len() >= 2
            && path.segments[path.segments.len() - 2].ident == "ProgressCompleteness"
    }
    /// Whether `expr` is a field or identifier named `completeness`.
    fn names_completeness(expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Field(node) => {
                matches!(&node.member, syn::Member::Named(name) if name == "completeness")
            }
            syn::Expr::Path(node) => node.path.is_ident("completeness"),
            syn::Expr::Paren(node) => names_completeness(&node.expr),
            syn::Expr::Reference(node) => names_completeness(&node.expr),
            _ => false,
        }
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
        fn visit_pat_tuple(&mut self, node: &'ast syn::PatTuple) {
            if node
                .elems
                .iter()
                .filter(|pat| completeness_variant(pat))
                .count()
                >= 2
            {
                self.comparing
                    .push("a tuple pattern over two completeness variants".to_owned());
            }
            syn::visit::visit_pat_tuple(self, node);
        }
        fn visit_pat_slice(&mut self, node: &'ast syn::PatSlice) {
            if node
                .elems
                .iter()
                .filter(|pat| completeness_variant(pat))
                .count()
                >= 2
            {
                self.comparing
                    .push("a slice pattern over two completeness variants".to_owned());
            }
            syn::visit::visit_pat_slice(self, node);
        }
        fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
            use syn::BinOp::{Eq, Ge, Gt, Le, Lt, Ne};
            if matches!(node.op, Eq(_) | Ne(_) | Lt(_) | Le(_) | Gt(_) | Ge(_))
                && names_completeness(&node.left)
                && names_completeness(&node.right)
            {
                self.comparing
                    .push("a comparison of two completeness values".to_owned());
            }
            syn::visit::visit_expr_binary(self, node);
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

/// Name fragments no item of shipped source may carry: an affected-region
/// plan is the owner's `repair::plan`, which FSL does not call or copy.
const AFFECTED_REGION_FRAGMENTS: &[&str] = &["affected_region", "AffectedRegion"];

/// Findings of the scan over `files`: `(file, finding)` for each forbidden
/// definition, each function with two `ProgressCompleteness` parameters, each
/// use of the owner's `repair` module and each item named for an
/// affected-region plan.
fn second_copies(files: &[(String, String)]) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (name, source) in files {
        let (defined, comparing) = defined_items(source);
        for item in defined {
            if FORBIDDEN_ITEMS.contains(&item.as_str())
                || AFFECTED_REGION_FRAGMENTS
                    .iter()
                    .any(|fragment| item.contains(fragment))
            {
                found.push((name.clone(), item));
            }
        }
        for item in comparing {
            found.push((
                name.clone(),
                format!("{item} compares two completeness assertions"),
            ));
        }
        if source.contains("filament_observation::authority::repair") {
            found.push((
                name.clone(),
                "a use of filament_observation::authority::repair".to_owned(),
            ));
        }
    }
    found
}

/// The shipped source of every FSL crate of the workspace: each `src`
/// directory of the root crate and of the `fsl-*` crates.
fn workspace_sources() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root");
    let mut directories = vec![root.join("src")];
    for entry in std::fs::read_dir(root).expect("the workspace root is readable") {
        let path = entry.expect("a directory entry").path();
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("fsl-"));
        if named && path.join("src").is_dir() {
            directories.push(path.join("src"));
        }
    }
    let mut paths = Vec::new();
    for directory in directories {
        sources(&directory, &mut paths);
    }
    paths
        .iter()
        .map(|path| {
            (
                path.display().to_string(),
                std::fs::read_to_string(path).expect("a source file"),
            )
        })
        .collect()
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
fn no_fsl_crate_defines_a_second_copy_of_the_four_owners_rule_sets() {
    // The scan finds what it forbids, and names the file and item.
    let planted = [(
        "planted.rs".to_owned(),
        "fn scope_disposition() {} struct OrderPossibilities; \
         fn revise_again(a: ProgressCompleteness, b: ProgressCompleteness) {} \
         fn affected_region_of() {} \
         use filament_observation::authority::repair;"
            .to_owned(),
    )];
    let found = second_copies(&planted);
    assert_eq!(found.len(), 5, "{found:?}");
    assert!(found.iter().all(|(file, _)| file == "planted.rs"));
    // Both forms of a comparison of two values are found, in any function.
    for (source, finding) in [
        (
            "fn judge(a: Entry, b: Entry) { match (a.completeness, b.completeness) { \
             (ProgressCompleteness::Complete { through: x }, \
             ProgressCompleteness::Complete { through: y }) if y < x => {} _ => {} } }",
            "a tuple pattern over two completeness variants",
        ),
        (
            "fn judge(a: Entry, b: Entry) -> bool { a.completeness != b.completeness }",
            "a comparison of two completeness values",
        ),
    ] {
        let planted = [("planted_form.rs".to_owned(), source.to_owned())];
        let found = second_copies(&planted);
        assert_eq!(found.len(), 1, "{source}: {found:?}");
        assert!(found[0].1.contains(finding), "{found:?}");
    }
    // One parameter of the type is no comparison.
    let single = [(
        "single.rs".to_owned(),
        "fn reading(a: ProgressCompleteness) {}".to_owned(),
    )];
    assert!(second_copies(&single).is_empty());
    let found = second_copies(&workspace_sources());
    assert!(
        found.is_empty(),
        "second copies of an owner rule: {found:?}"
    );
}

#[test]
#[trace("TC-2442", "FR-861-AC-4")]
fn the_progress_ledger_calls_the_owners_completeness_revision_check() {
    let ledger = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/temporal/ledger.rs"),
    )
    .expect("the progress ledger source");
    assert!(
        ledger.contains("completeness::check_revision"),
        "the progress ledger calls `completeness::check_revision`"
    );
}

/// One case on which the evaluator's reported disposition is compared with
/// the one FR-524 states.
#[derive(Debug, Eq, PartialEq)]
struct Disagreement {
    case: String,
    evaluator: String,
    stated: String,
}

/// Equal answers pass; any other pair is reported with the case and both
/// answers, never passed on as either one.
fn agree(case: &str, evaluator: &str, stated: &str) -> Result<(), Disagreement> {
    if evaluator == stated {
        return Ok(());
    }
    Err(Disagreement {
        case: case.to_owned(),
        evaluator: evaluator.to_owned(),
        stated: stated.to_owned(),
    })
}

/// FR-861-AC-3 (TC-2442): through `evaluate_temporal_evidence`, whose
/// dispositions come from the owner's `activation` module, FR-524-AC-27 to
/// AC-29's scopes report the activation state and non-value code FR-524
/// states; a disagreement is reported with the case and both answers.
#[test]
#[trace(
    "TC-2442",
    "FR-861-AC-3",
    "FR-524-AC-27",
    "FR-524-AC-28",
    "FR-524-AC-29"
)]
fn the_owner_activation_gives_the_dispositions_fr_524_states() {
    use crate::temporal_activation as fixture;
    use fsl_eval::temporal::evidence::TriggerEvidence;
    use fsl_eval::temporal::{ProgressClosure, ProgressCompleteness, Valuation};
    let (env, atom) = fixture::each_env();
    let mut subject = env.subject(&[]);
    subject.canonical_package_bytes = Some(b"{}");
    subject.time_unit_scale = fsl_eval::temporal::evidence::ClockScale::PerSecond(
        filament_exact::Rational::new(1_i64.into(), 1000_i64.into()).expect("ms"),
    );
    let reported = |closure, evidence, record: ProgressCompleteness| {
        let mut asked = fixture::request(&env, fixture::at_t0(&[(atom, Valuation::True)]));
        asked.trigger_scope = closure;
        asked.trigger_evidence = evidence;
        if let Some(held) = asked.completeness.as_mut() {
            held.state = record;
        }
        let result = fixture::call(&env, &subject, &asked).expect("a result");
        format!(
            "{:?} {}",
            result.activation,
            result.non_value.map_or("none", |non_value| non_value.code)
        )
    };
    let complete = ProgressCompleteness::Complete { through: 100 };
    let cases = [
        (
            "FR-524-AC-27: closed scope, admitted evidence, no instance",
            reported(ProgressClosure::Closed, TriggerEvidence::Admitted, complete),
            "Inactive no-trigger-admitted",
        ),
        (
            "FR-524-AC-27: open scope",
            reported(ProgressClosure::Open, TriggerEvidence::Admitted, complete),
            "Unknown trigger-scope-open",
        ),
        (
            "FR-524-AC-28: missing evidence, closed",
            reported(ProgressClosure::Closed, TriggerEvidence::Missing, complete),
            "Unknown trigger-evidence-missing",
        ),
        (
            "FR-524-AC-28: missing evidence, open",
            reported(ProgressClosure::Open, TriggerEvidence::Missing, complete),
            "Unknown trigger-evidence-missing",
        ),
        (
            "FR-524-AC-28: refused evidence, closed",
            reported(ProgressClosure::Closed, TriggerEvidence::Refused, complete),
            "Unknown trigger-evidence-refused",
        ),
        (
            "FR-524-AC-28: refused evidence, open",
            reported(ProgressClosure::Open, TriggerEvidence::Refused, complete),
            "Unknown trigger-evidence-refused",
        ),
        (
            "FR-524-AC-29: closed scope, incomplete trigger record",
            reported(
                ProgressClosure::Closed,
                TriggerEvidence::Admitted,
                ProgressCompleteness::Incomplete,
            ),
            "Unknown trigger-scope-open",
        ),
    ];
    let failures: Vec<_> = cases
        .iter()
        .filter_map(|(case, evaluator, stated)| agree(case, evaluator, stated).err())
        .collect();
    assert!(failures.is_empty(), "disagreements: {failures:?}");
    // A disagreement is reported naming the case and both answers.
    assert_eq!(
        agree(cases[0].0, "Unknown trigger-scope-open", cases[0].2),
        Err(Disagreement {
            case: cases[0].0.to_owned(),
            evaluator: "Unknown trigger-scope-open".to_owned(),
            stated: "Inactive no-trigger-admitted".to_owned(),
        })
    );
}

/// FR-944-AC-4 (TC-2624, TC-3036): a position whose content, stamp and
/// order key equal a covered position's, listed under a new FR-106 identity,
/// shares that position's owner record identity in the admitted history, but
/// coverage is decided by the FR-106 identity, so it is refused
/// `UncoveredMember`: FR-502's contradiction on `Completeness`.
#[test]
#[trace("TC-2624", "TC-3036", "FR-944-AC-4")]
fn a_position_equal_to_a_covered_one_under_a_new_identity_is_an_uncovered_member() {
    use crate::temporal_activation as fixture;
    use filament_eval::{CallAdmissionFailure, CallFailure};
    use fsl_eval::temporal::Valuation;
    let (env, atom) = fixture::each_env();
    let subject = env.subject(&[]);
    let values = [(atom, Valuation::True)];
    let mut asked = fixture::request(&env, fixture::at_t0(&values));
    asked.progress_ledger = Some(fsl_eval::temporal::ProgressLedger::new());
    let first = fixture::call(&env, &subject, &asked).expect("the first request");
    asked.progress_ledger = first.progress_ledger.clone();
    let mut twin = fixture::position(0, &values);
    twin.observation = fsl_eval::temporal::evidence::EvidenceRef::named("twin-of-obs0");
    asked.positions.push(twin);
    let refused = fixture::call(&env, &subject, &asked);
    let Err(CallFailure::Admission(failure)) = refused else {
        panic!("a refusal, got {refused:?}");
    };
    let CallAdmissionFailure::Refused(record) = *failure else {
        panic!("a refusal record");
    };
    let fields = &record.record.fields;
    assert_eq!(
        fields.get("cause").map(String::as_str),
        Some("progress-contradiction")
    );
    assert_eq!(
        fields.get("contradicted_dimension").map(String::as_str),
        Some("Completeness")
    );
}
