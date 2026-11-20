// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710: compile an FSL model unit to its FR-154 domain-package document.

use fsl_cst::{CompleteCause, CompleteCode, CompleteDiagnostic, ParsedSource};
use fsl_foundation::{Phase, Source, SourceIdentity, Span};
use serde_json::{Value, json};

use crate::model::accounting::ModelNormalizationLimits;

/// A model unit compiled to the Semantic IR document FR-154 admits.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledModelUnit {
    identity: String,
    bytes: Vec<u8>,
}

/// Whether bytes begin with an FSL model-unit header and `domain` selection.
pub(super) fn looks_like_model_unit(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    if !text.trim_start().starts_with("language") {
        return false;
    }
    let Ok(parsed) = fsl_cst::parse(
        SourceIdentity::new("filament", "model-unit-probe"),
        "model-unit.fsl",
        bytes,
        fsl_cst::Limits::default(),
    ) else {
        return false;
    };
    let tokens = parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == fsl_cst::TokenClass::Token)
        .map(|token| token.spelling())
        .collect::<Vec<_>>();
    tokens.get(5).is_some_and(|token| *token == b"domain")
}

/// Compile one anonymous model-unit byte source using its domain as the
/// source identity label.
pub(super) fn compile_model_unit_bytes(
    bytes: &[u8],
) -> Result<CompiledModelUnit, super::ModelRefusal> {
    let probe = fsl_cst::parse(
        SourceIdentity::new("filament", "model-unit-probe"),
        "model-unit.fsl",
        bytes,
        fsl_cst::Limits::default(),
    )
    .map_err(|diagnostic| refusal(format!("{diagnostic}")))?;
    let tokens = probe
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == fsl_cst::TokenClass::Token)
        .map(|token| String::from_utf8_lossy(token.spelling()).into_owned())
        .collect::<Vec<_>>();
    let identity = tokens
        .get(6)
        .and_then(|value| serde_json::from_str::<String>(value).ok())
        .ok_or_else(|| refusal("model-unit domain identity is missing"))?;
    let source = fsl_cst::diagnostic::read_source(
        SourceIdentity::new("filament", identity),
        "model-unit.fsl",
        bytes,
        fsl_cst::Limits::default().source_bytes,
    )
    .map_err(|error| refusal(format!("{error}")))?;
    let parsed = fsl_cst::parse_source(source, fsl_cst::Limits::default())
        .map_err(|diagnostic| refusal(format!("{diagnostic}")))?;
    compile_model_unit(&parsed, ModelNormalizationLimits::default()).map_err(|diagnostics| {
        refusal(diagnostics.first().map_or_else(
            || "model-unit compilation failed".to_owned(),
            ToString::to_string,
        ))
    })
}

/// Compile already-labelled sources to package input, preserving each source
/// identity in node origins.
pub fn package_input_from_sources<'a>(
    sources: impl IntoIterator<Item = &'a Source>,
) -> Result<super::PackageInput, super::ModelRefusal> {
    let mut documents = Vec::new();
    for source in sources {
        let parsed = fsl_cst::parse_source(source.clone(), fsl_cst::Limits::default())
            .map_err(|diagnostic| refusal(format!("{diagnostic}")))?;
        let compiled = compile_model_unit(&parsed, ModelNormalizationLimits::default()).map_err(
            |diagnostics| {
                refusal(diagnostics.first().map_or_else(
                    || "model-unit compilation failed".to_owned(),
                    ToString::to_string,
                ))
            },
        )?;
        documents.push((compiled.identity().to_owned(), compiled.bytes().to_vec()));
    }
    let mut packages = super::PackageInput::new();
    for (identity, bytes) in documents {
        if packages.insert(identity.clone(), bytes).is_some() {
            return Err(super::ModelRefusal {
                code: fsl_foundation::Code::InvalidPackage,
                cause: super::ModelRefusalCause::ConflictingPackage {
                    identity: identity.clone(),
                },
                detail: format!("two supplied model units declare identity {identity:?}"),
            });
        }
    }
    Ok(packages)
}

fn refusal(detail: impl Into<String>) -> super::ModelRefusal {
    super::ModelRefusal {
        code: fsl_foundation::Code::InvalidModelBinding,
        cause: super::ModelRefusalCause::MalformedDeclaration,
        detail: detail.into(),
    }
}

impl CompiledModelUnit {
    /// The domain identity declared by the unit.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// The RFC 8785 bytes of its Semantic IR 2.0.0 document.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Clone, Debug)]
struct Token {
    text: String,
    span: Span,
}

/// Compile an admitted model unit. Source labels and declaration spans are
/// retained in each document node's `origin`.
pub fn compile_model_unit(
    parsed: &ParsedSource,
    limits: ModelNormalizationLimits,
) -> Result<CompiledModelUnit, Vec<CompleteDiagnostic>> {
    if !parsed.is_admissible() {
        return Err(parsed.diagnostics().to_vec());
    }
    let tokens = parsed
        .cst()
        .tokens()
        .iter()
        .filter(|token| token.class() == fsl_cst::TokenClass::Token)
        .map(|token| Token {
            text: String::from_utf8_lossy(token.spelling()).into_owned(),
            span: token.span(),
        })
        .collect::<Vec<_>>();
    let source = parsed.source();
    let mut cursor = 0;
    let Some(identity) = read_identity(&tokens, &mut cursor) else {
        return Err(vec![diagnostic(
            parsed,
            CompleteCode::InvalidSyntax,
            CompleteCause::UnexpectedToken,
            tokens
                .get(cursor)
                .map_or(Span { start: 0, end: 0 }, |t| t.span),
            "expected a model unit's domain declaration",
        )]);
    };
    if identity == "filament/native"
        || identity == "filament/unit"
        || identity.starts_with("filament/native/")
        || identity.starts_with("filament/unit/")
    {
        let span = tokens
            .get(cursor.saturating_sub(2))
            .map_or(Span { start: 0, end: 0 }, |token| token.span);
        return Err(vec![diagnostic(
            parsed,
            CompleteCode::InvalidModelBinding,
            CompleteCause::MalformedDeclaration,
            span,
            "the domain identity is reserved by FR-154",
        )]);
    }

    let source_identity = format!(
        "ix://{}/{}",
        source.identity().authority,
        source.identity().identity
    );
    let mut types = Vec::new();
    let mut populations = Vec::new();
    let mut object_names = std::collections::BTreeSet::new();
    let mut population_names = std::collections::BTreeSet::new();
    let mut references = Vec::new();
    let mut index = cursor;
    while index < tokens.len() {
        let declaration_start = index;
        if tokens[index].text == "abstract" {
            index += 1;
        }
        match tokens.get(index).map(|token| token.text.as_str()) {
            Some("object") => {
                let Some((object, next)) = read_object(&tokens, index, &identity, parsed) else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid object declaration",
                    )]);
                };
                if !object_names.insert(object.name.clone()) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        object.name_span,
                        format!("object {} is declared more than once", object.name),
                    )]);
                }
                references.extend(object.relationship_targets.iter().cloned());
                types.push(object.wire);
                index = next;
            }
            Some("population") | Some("open") | Some("symmetric") => {
                let Some((population, next)) = read_population(&tokens, index, &identity, parsed)
                else {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::InvalidSyntax,
                        CompleteCause::UnexpectedToken,
                        tokens[declaration_start].span,
                        "invalid population declaration",
                    )]);
                };
                if !population_names.insert(
                    population["displayName"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                ) {
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::AmbiguousDeclaration,
                        CompleteCause::AmbiguousName,
                        tokens[declaration_start].span,
                        "population is declared more than once",
                    )]);
                }
                populations.push(population);
                index = next;
            }
            Some(_) => {
                let token = &tokens[index];
                return Err(vec![diagnostic(
                    parsed,
                    CompleteCode::UnsupportedConstruct,
                    CompleteCause::DeclarationForm,
                    token.span,
                    format!(
                        "model declaration `{}` is not supported by intake",
                        token.text
                    ),
                )]);
            }
            None => break,
        }
    }
    for (target, span) in references {
        if !object_names.contains(&target) {
            return Err(vec![diagnostic(
                parsed,
                CompleteCode::MissingDeclaration,
                CompleteCause::MissingDeclaration,
                span,
                format!("relationship target {target:?} names no object declaration"),
            )]);
        }
    }
    for population in &populations {
        if let Some(members) = population["members"].as_array() {
            for member in members {
                let Some(member_identity) = member.as_str() else {
                    continue;
                };
                let Some(target) = member_identity.strip_prefix(&format!("ix://{identity}/"))
                else {
                    continue;
                };
                if !object_names.contains(target) {
                    let span = tokens
                        .get(cursor.saturating_sub(1))
                        .map_or(Span { start: 0, end: 0 }, |token| token.span);
                    return Err(vec![diagnostic(
                        parsed,
                        CompleteCode::MissingDeclaration,
                        CompleteCause::MissingDeclaration,
                        span,
                        format!("population member {target:?} names no object declaration"),
                    )]);
                }
            }
        }
    }
    let _ = limits;
    let mut meanings = Vec::new();
    if !types.is_empty() {
        meanings.push(("object_type", "filament.meaning.model.object-type/v1"));
    }
    if !populations.is_empty() {
        meanings.push(("population", "filament.meaning.model.population/v1"));
    }
    let constructs = meanings
        .into_iter()
        .map(|(name, meaning)| {
            json!({
                "kind": {"module": identity, "name": name},
                "moduleVersion": "1.0.0",
                "construct": {"identity": "none", "shape": "record", "members": {}, "meaning": meaning},
            })
        })
        .collect::<Vec<_>>();
    let document = json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1"},
        "source": {"identity": source_identity, "version": "1.0.0", "dialect": "spec-bundle"},
        "package": {"identity": identity, "version": "1.0.0", "mappingVersions": [], "profileVersions": []},
        "occurrences": [], "extensions": [], "constructs": constructs,
        "types": types, "populations": populations,
    });
    let encoded = serde_json::to_vec(&document).map_err(|error| {
        vec![diagnostic(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            Span { start: 0, end: 0 },
            format!("model-unit document serialization failed: {error}"),
        )]
    })?;
    let tree = quire_canonical::read(&encoded, limits.intake.input_bytes).map_err(|error| {
        vec![diagnostic(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            Span { start: 0, end: 0 },
            format!("model-unit document could not be read for canonicalization: {error}"),
        )]
    })?;
    let bytes = quire_canonical::to_vec(
        &tree,
        quire_canonical::Limits::new(limits.intake.input_bytes),
    )
    .map_err(|error| {
        vec![diagnostic(
            parsed,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            Span { start: 0, end: 0 },
            format!("model-unit document could not be canonicalized: {error}"),
        )]
    })?;
    Ok(CompiledModelUnit { identity, bytes })
}

fn read_identity(tokens: &[Token], cursor: &mut usize) -> Option<String> {
    if tokens.get(*cursor)?.text != "language" {
        return None;
    }
    *cursor += 1;
    *cursor += 1;
    *cursor += 1;
    *cursor += 1;
    if tokens.get(*cursor)?.text != ";" {
        return None;
    }
    *cursor += 1;
    if tokens.get(*cursor)?.text != "domain" {
        return None;
    }
    *cursor += 1;
    let identity = serde_json::from_str::<String>(&tokens.get(*cursor)?.text).ok()?;
    *cursor += 1;
    if tokens.get(*cursor)?.text != ";" {
        return None;
    }
    *cursor += 1;
    Some(identity)
}

struct ObjectRead {
    name: String,
    name_span: Span,
    wire: Value,
    relationship_targets: Vec<(String, Span)>,
}

fn read_object(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(ObjectRead, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name_span = tokens.get(index)?.span;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    let mut identity_fields = Vec::new();
    if tokens
        .get(index)
        .is_some_and(|token| token.text == "identity")
    {
        index += 1;
        if tokens.get(index)?.text != "(" {
            return None;
        }
        index += 1;
        while tokens.get(index)?.text != ")" {
            if tokens.get(index)?.text != "," {
                identity_fields.push(tokens[index].text.clone());
            }
            index += 1;
        }
        index += 1;
    }
    while tokens.get(index)?.text != "{" {
        index += 1;
    }
    index += 1;
    let mut fields = Vec::new();
    let mut operations = Vec::new();
    let mut relationships = Vec::new();
    let mut relationship_targets = Vec::new();
    while tokens.get(index)?.text != "}" {
        let member_start = tokens.get(index)?.span.start;
        match tokens.get(index)?.text.as_str() {
            "relationship" => {
                let start = index;
                index += 1;
                let relationship_name = tokens.get(index)?.text.clone();
                index += 1;
                if tokens.get(index)?.text != ":" {
                    return None;
                }
                index += 1;
                let target = tokens.get(index)?.text.clone();
                let target_span = tokens.get(index)?.span;
                index += 1;
                let (lower, upper, ordered, unique, next) = read_multiplicity(tokens, index)?;
                index = next;
                if tokens
                    .get(index)
                    .is_some_and(|token| token.text == "inverse")
                {
                    index += 2;
                    let (_, _, _, _, next) = read_multiplicity(tokens, index)?;
                    index = next;
                }
                if tokens.get(index)?.text != ";" {
                    return None;
                }
                let end = tokens.get(index)?.span.end;
                index += 1;
                relationship_targets.push((target.clone(), target_span));
                relationships.push(json!({
                    "identity": format!("ix://{package}/relationship/{name}-{relationship_name}-{target}"),
                    "sourceEnd": {
                        "type": format!("ix://{package}/{name}"), "role": "source",
                        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                    },
                    "targetEnd": {
                        "type": format!("ix://{package}/{target}"), "role": relationship_name,
                        "multiplicity": {"lower": lower, "upper": upper, "ordered": ordered, "unique": unique},
                    },
                    "category": "structural", "composite": false,
                    "direction": "source-to-target",
                    "origin": source_origin(parsed, Span { start: tokens[start].span.start, end }),
                }));
            }
            "operation" => {
                let (operation, next) = read_operation(tokens, index, &name, package, parsed)?;
                operations.push(operation);
                index = next;
            }
            "invariant" => index = skip_statement(tokens, index)?,
            _ => {
                let field_name = tokens.get(index)?.text.clone();
                index += 1;
                if tokens.get(index)?.text != ":" {
                    return None;
                }
                index += 1;
                let mut type_name = tokens.get(index)?.text.clone();
                index += 1;
                let mut optional = false;
                let inline_type_bounds =
                    matches!(type_name.as_str(), "Int" | "Rational" | "Decimal" | "Text")
                        || (matches!(type_name.as_str(), "Float32" | "Float64")
                            && tokens.get(index + 1).is_some_and(|token| {
                                matches!(
                                    token.text.as_str(),
                                    "exact"
                                        | "toward-zero"
                                        | "toward-positive"
                                        | "toward-negative"
                                        | "nearest-even"
                                        | "nearest-away"
                                )
                            }));
                if inline_type_bounds && tokens.get(index).is_some_and(|token| token.text == "[") {
                    let mut depth = 0usize;
                    while let Some(token) = tokens.get(index) {
                        match token.text.as_str() {
                            "[" => depth += 1,
                            "]" => {
                                depth -= 1;
                                index += 1;
                                if depth == 0 {
                                    break;
                                }
                                continue;
                            }
                            _ => {}
                        }
                        index += 1;
                    }
                }
                if type_name == "Int" {
                    type_name = "Integer".to_owned();
                }
                if tokens.get(index).is_some_and(|token| token.text == "?") {
                    optional = true;
                    index += 1;
                }
                let mut lower = 1u64;
                let mut upper = Some(1u64);
                let mut ordered = false;
                let mut unique = true;
                if tokens.get(index).is_some_and(|token| token.text == "[") {
                    let (field_lower, field_upper, field_ordered, field_unique, next) =
                        read_multiplicity(tokens, index)?;
                    lower = field_lower;
                    upper = field_upper;
                    ordered = field_ordered;
                    unique = field_unique;
                    index = next;
                }
                if tokens.get(index)?.text != ";" {
                    return None;
                }
                index += 1;
                let field_identity = format!("ix://{package}/{name}/{field_name}");
                fields.push(json!({
                    "identity": field_identity,
                    "name": field_name,
                    "typeRef": type_ref(&type_name, package),
                    "presence": if optional {"optional"} else {"required"},
                    "nullable": false,
                    "defaultKind": "none",
                    "multiplicity": {"lower": lower, "upper": upper, "ordered": ordered, "unique": unique},
                    "origin": source_origin(parsed, Span { start: member_start, end: tokens[index - 1].span.end }),
                }));
            }
        }
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let object_identity = format!("ix://{package}/{name}");
    let wire = json!({
        "identity": object_identity,
        "displayName": object_identity,
        "kind": {"module": package, "name": "object_type"},
        "roles": [],
        "origin": source_origin(parsed, Span { start, end }),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
        "supertypes": [], "fields": fields, "operations": operations, "relationships": relationships,
        "identityFields": identity_fields.iter().map(|field| format!("ix://{package}/{name}/{field}")).collect::<Vec<_>>(),
    });
    Some((
        ObjectRead {
            name,
            name_span,
            wire,
            relationship_targets,
        },
        index,
    ))
}

fn read_multiplicity(
    tokens: &[Token],
    mut index: usize,
) -> Option<(u64, Option<u64>, bool, bool, usize)> {
    if tokens.get(index)?.text != "[" {
        return None;
    }
    index += 1;
    let lower = tokens.get(index)?.text.parse().ok()?;
    index += 1;
    if tokens.get(index)?.text != "," {
        return None;
    }
    index += 1;
    let upper = if tokens.get(index)?.text == "*" {
        None
    } else {
        Some(tokens.get(index)?.text.parse().ok()?)
    };
    index += 1;
    if tokens.get(index)?.text != "]" {
        return None;
    }
    index += 1;
    let ordered = tokens
        .get(index)
        .is_some_and(|token| token.text == "ordered");
    if ordered {
        index += 1;
    }
    let explicit_unique = tokens
        .get(index)
        .is_some_and(|token| token.text == "unique");
    if explicit_unique {
        index += 1;
    }
    let unique = true;
    Some((lower, upper, ordered, unique, index))
}

fn read_operation(
    tokens: &[Token],
    mut index: usize,
    owner: &str,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, usize)> {
    let start = tokens.get(index)?.span.start;
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    while tokens.get(index)?.text != ")" {
        index += 1;
    }
    index += 1;
    let returns = if tokens.get(index).is_some_and(|token| token.text == ":") {
        index += 1;
        let type_name = tokens.get(index)?.text.clone();
        index += 1;
        if tokens.get(index).is_some_and(|token| token.text == "[") {
            while tokens.get(index)?.text != "]" {
                index += 1;
            }
            index += 1;
        }
        Some(json!({
            "typeRef": type_ref(&type_name, package),
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        }))
    } else {
        None
    };
    let mut modifies_receiver = Vec::new();
    let mut creates = Vec::new();
    let mut deletes = Vec::new();
    while tokens.get(index)?.text != ";" {
        match tokens.get(index)?.text.as_str() {
            "modifies" => {
                index += 1;
                while tokens.get(index)?.text != ";"
                    && tokens.get(index)?.text != "creates"
                    && tokens.get(index)?.text != "deletes"
                {
                    if tokens[index].text == "self"
                        && tokens.get(index + 1).is_some_and(|token| token.text == ".")
                    {
                        if let Some(field) = tokens.get(index + 2) {
                            modifies_receiver
                                .push(format!("ix://{package}/{owner}/{}", field.text));
                        }
                        index += 3;
                    } else {
                        index += 1;
                    }
                }
            }
            "creates" => {
                index += 1;
                if let Some(target) = tokens.get(index) {
                    creates.push(format!("ix://{package}/{}", target.text));
                    index += 1;
                }
            }
            "deletes" => {
                index += 1;
                if let Some(target) = tokens.get(index) {
                    deletes.push(format!("ix://{package}/{}", target.text));
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let mut operation = json!({
        "identity": format!("ix://{package}/{owner}/{name}"),
        "name": name, "params": [], "pre": [], "post": [],
        "origin": source_origin(parsed, Span { start, end }),
        "frame": {"modifies": [], "modifies_receiver": modifies_receiver, "creates": creates, "deletes": deletes},
    });
    if let Some(returns) = returns {
        operation["returns"] = returns;
    }
    Some((operation, index))
}

fn read_population(
    tokens: &[Token],
    mut index: usize,
    package: &str,
    parsed: &ParsedSource,
) -> Option<(Value, usize)> {
    let start = tokens.get(index)?.span.start;
    let open = tokens.get(index)?.text == "open";
    let symmetric = tokens[index..]
        .iter()
        .take_while(|token| token.text != "population")
        .any(|token| token.text == "symmetric");
    while tokens.get(index)?.text != "population" {
        index += 1;
    }
    index += 1;
    let name = tokens.get(index)?.text.clone();
    index += 1;
    if tokens.get(index)?.text != ":" {
        return None;
    }
    index += 1;
    let first_type = tokens.get(index)?.text.clone();
    let mut members = vec![first_type];
    index += 1;
    while tokens.get(index).is_some_and(|token| token.text == ",") {
        index += 1;
        members.push(tokens.get(index)?.text.clone());
        index += 1;
    }
    if tokens.get(index)?.text != ";" {
        return None;
    }
    let end = tokens.get(index)?.span.end;
    index += 1;
    let identity = format!("ix://{package}/{name}");
    Some((
        json!({
            "identity": identity, "displayName": identity,
            "kind": {"module": package, "name": "population"},
            "members": members.iter().map(|member| format!("ix://{package}/{member}")).collect::<Vec<_>>(),
        "extent": if open { "open" } else { "closed" },
        "symmetric": symmetric,
        "origin": source_origin(parsed, Span { start, end }),
        }),
        index,
    ))
}

fn skip_statement(tokens: &[Token], mut index: usize) -> Option<usize> {
    while tokens.get(index)?.text != ";" {
        index += 1;
    }
    Some(index + 1)
}

fn type_ref(type_name: &str, package: &str) -> String {
    let native = match type_name {
        "Boolean" => "Boolean",
        "Text" => "Text",
        "Integer" => "Integer",
        "Rational" => "Rational",
        "Float32" => "Float32",
        "Float64" => "Float64",
        _ => return format!("ix://{package}/{type_name}"),
    };
    format!("ix://filament/native/{native}")
}

fn source_origin(parsed: &ParsedSource, span: Span) -> Value {
    let source = parsed.source();
    let position = source.locate(Span {
        end: span.end,
        ..span
    });
    let (line, column) = position.map_or((1, 1), |position| {
        (position.start.line, position.start.column)
    });
    json!({"source": {
        "sourceIdentity": format!("ix://{}/{}", source.identity().authority, source.identity().identity),
        "path": source.path(), "startLine": line, "startColumn": column,
    }})
}

fn diagnostic(
    parsed: &ParsedSource,
    code: CompleteCode,
    cause: CompleteCause,
    span: Span,
    message: impl Into<String>,
) -> CompleteDiagnostic {
    *fsl_cst::diagnostic::error(
        parsed.source(),
        code,
        cause,
        Phase::Parse,
        span.start,
        span.end,
        message,
    )
}
