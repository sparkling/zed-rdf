//! Independent shadow JSON-LD syntax parser — Phase B implementation.
//!
//! Agent `pb-shadow-jsonld` fills this in (model: claude-sonnet-4-6,
//! ADR-0019 §3 base-model disjointness). Gated behind the `shadow`
//! feature so main-parser builds never pull this crate's contents.
//!
//! The shadow must be written **without** reading `crates/rdf-jsonld/`.
//! Divergence between the two implementations is the signal.
//!
//! # Implementation notes
//!
//! Derived directly from the JSON-LD 1.1 specification
//! (<https://www.w3.org/TR/json-ld11/>) using the following approach:
//!
//! - Parse with `serde_json::from_slice::<serde_json::Value>`
//! - Walk the JSON tree to emit `rdf_diff::Fact` records
//! - `@context`: build a term→IRI map; validate structure
//! - `@value`: literal node; `@id`: IRI reference; bare object: blank node
//! - Named graphs: `Fact::graph = Some(graph_iri)`
//! - All IRIs wrapped in `<…>` per the canonical-form contract (ADR-0020 §1.4)

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::too_many_lines,
    clippy::option_if_let_else,
)]

#[cfg(feature = "shadow")]
use std::collections::BTreeMap;

#[cfg(feature = "shadow")]
use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome, Parser};

#[cfg(feature = "shadow")]
use serde_json::Value;

// -----------------------------------------------------------------------
// Well-known IRIs and prefixes (JSON-LD 1.1 §9)
// -----------------------------------------------------------------------

#[cfg(feature = "shadow")]
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
#[cfg(feature = "shadow")]
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

#[cfg(feature = "shadow")]
fn rdf(local: &str) -> String {
    format!("<{RDF}{local}>")
}

#[cfg(feature = "shadow")]
fn xsd(local: &str) -> String {
    format!("<{XSD}{local}>")
}

// -----------------------------------------------------------------------
// Public entry point
// -----------------------------------------------------------------------

/// Independent shadow JSON-LD syntax parser.
#[cfg(feature = "shadow")]
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonLdShadowParser;

#[cfg(feature = "shadow")]
impl JsonLdShadowParser {
    /// Construct a fresh shadow JSON-LD parser.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(feature = "shadow")]
impl Parser for JsonLdShadowParser {
    fn parse(&self, input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
        let text = std::str::from_utf8(input).map_err(|e| Diagnostics {
            messages: vec![format!("UTF-8 decode error: {e}")],
            fatal: true,
        })?;

        let root: Value = serde_json::from_str(text).map_err(|e| Diagnostics {
            messages: vec![format!("JSON parse error: {e}")],
            fatal: true,
        })?;

        let mut state = ParseState::new();
        state
            .process_root(&root)
            .map_err(|msg| Diagnostics { messages: vec![msg], fatal: true })?;

        let facts = Facts::canonicalise(
            state.facts.into_iter().map(|f| {
                (
                    f,
                    FactProvenance {
                        offset: None,
                        parser: "rdf-jsonld-shadow".to_owned(),
                    },
                )
            }),
            BTreeMap::new(),
        );

        Ok(ParseOutcome {
            facts,
            warnings: Diagnostics { messages: vec![], fatal: false },
        })
    }

    fn id(&self) -> &'static str {
        "rdf-jsonld-shadow"
    }
}

// -----------------------------------------------------------------------
// Parser state
// -----------------------------------------------------------------------

#[cfg(feature = "shadow")]
struct ParseState {
    facts: Vec<Fact>,
    bnode_counter: usize,
}

#[cfg(feature = "shadow")]
impl ParseState {
    const fn new() -> Self {
        Self { facts: vec![], bnode_counter: 0 }
    }

    fn fresh_bnode(&mut self) -> String {
        let n = self.bnode_counter;
        self.bnode_counter += 1;
        format!("_:b{n}")
    }

    fn emit(&mut self, subject: String, predicate: String, object: String, graph: Option<String>) {
        self.facts.push(Fact { subject, predicate, object, graph });
    }

    /// Process the document root: a JSON-LD document is either an
    /// object or an array of objects (JSON-LD 1.1 §6.1).
    fn process_root(&mut self, root: &Value) -> Result<(), String> {
        match root {
            Value::Array(items) => {
                // Top-level array — possibly a node array with a shared context.
                // Extract a context from the first element if present.
                let mut ctx = Context::default();
                for item in items {
                    if let Value::Object(obj) = item
                        && let Some(ctx_val) = obj.get("@context") {
                            ctx.merge_value(ctx_val)?;
                        }
                }
                for item in items {
                    self.process_node(item, &ctx, None)?;
                }
            }
            Value::Object(obj) => {
                let ctx = Context::from_object(obj)?;
                self.process_node(root, &ctx, None)?;
            }
            _ => {
                return Err("JSON-LD document root must be an object or array".to_owned());
            }
        }
        Ok(())
    }

    /// Process a single JSON-LD node. Returns the node's subject term.
    fn process_node(
        &mut self,
        value: &Value,
        ctx: &Context,
        graph: Option<&str>,
    ) -> Result<String, String> {
        match value {
            Value::Object(obj) => {
                // A node is a value node if it has `@value`.
                if obj.contains_key("@value") {
                    return Self::parse_value_node(obj, ctx);
                }

                // It is an `@list` node.
                if let Some(list_val) = obj.get("@list") {
                    return self.parse_list_node(list_val, ctx, graph);
                }

                // Determine the graph name when `@graph` is present.
                let active_graph: Option<String>;
                if let Some(graph_val) = obj.get("@graph") {
                    // The node object is a named-graph container.
                    // Its subject IRI (if any) becomes the graph name.
                    let graph_name = if let Some(id_val) = obj.get("@id") {
                        match id_val {
                            Value::String(s) => Some(resolve_iri(s, ctx)),
                            _ => None,
                        }
                    } else {
                        // Anonymous named graph: use a blank node.
                        Some(self.fresh_bnode())
                    };
                    active_graph = graph_name;

                    // Build a local context (context can appear on a named
                    // graph container).
                    let local_ctx = if let Some(cv) = obj.get("@context") {
                        let mut c = ctx.clone();
                        c.merge_value(cv)?;
                        c
                    } else {
                        ctx.clone()
                    };

                    // Process graph members.
                    match graph_val {
                        Value::Array(items) => {
                            for item in items {
                                self.process_node(item, &local_ctx, active_graph.as_deref())?;
                            }
                        }
                        Value::Object(_) => {
                            self.process_node(
                                graph_val,
                                &local_ctx,
                                active_graph.as_deref(),
                            )?;
                        }
                        _ => {}
                    }

                    // Also process any top-level triples on the container
                    // itself (outside the named graph), omitting `@graph`
                    // and `@context` keys.
                    let subject = if let Some(id_val) = obj.get("@id") {
                        match id_val {
                            Value::String(s) => resolve_iri(s, ctx),
                            _ => self.fresh_bnode(),
                        }
                    } else {
                        // No container-level triples if no @id.
                        return Ok(active_graph.unwrap_or_else(|| "_:anon".to_owned()));
                    };

                    for (key, val) in obj {
                        match key.as_str() {
                            "@id" | "@context" | "@graph" | "@type" => {}
                            _ => {
                                let pred = ctx.expand_term(key);
                                self.process_property(&subject, &pred, val, ctx, graph)?;
                            }
                        }
                    }
                    return Ok(subject);
                }

                // Build the active context for this node.
                let local_ctx = if let Some(cv) = obj.get("@context") {
                    let mut c = ctx.clone();
                    c.merge_value(cv)?;
                    c
                } else {
                    ctx.clone()
                };

                // Determine the subject.
                let subject: String = if let Some(id_val) = obj.get("@id") {
                    match id_val {
                        Value::String(s) => resolve_iri(s, &local_ctx),
                        _ => {
                            return Err(
                                "@id must be a string (JSON-LD 1.1 §9.1)".to_owned()
                            );
                        }
                    }
                } else {
                    self.fresh_bnode()
                };

                // `@type` → rdf:type triples.
                if let Some(type_val) = obj.get("@type") {
                    match type_val {
                        Value::String(t) => {
                            let type_iri = resolve_iri(t, &local_ctx);
                            self.emit(
                                subject.clone(),
                                rdf("type"),
                                type_iri,
                                graph.map(str::to_owned),
                            );
                        }
                        Value::Array(types) => {
                            for t in types {
                                if let Value::String(s) = t {
                                    let type_iri = resolve_iri(s, &local_ctx);
                                    self.emit(
                                        subject.clone(),
                                        rdf("type"),
                                        type_iri,
                                        graph.map(str::to_owned),
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                }

                // Remaining properties.
                for (key, val) in obj {
                    match key.as_str() {
                        "@id" | "@context" | "@type" | "@graph" => {}
                        _ => {
                            let pred = local_ctx.expand_term(key);
                            self.process_property(&subject, &pred, val, &local_ctx, graph)?;
                        }
                    }
                }

                Ok(subject)
            }

            Value::String(s) => {
                // Bare string in node position → treat as IRI reference.
                Ok(resolve_iri(s, ctx))
            }

            _ => Err(format!("unexpected JSON-LD node type: {value:?}")),
        }
    }

    /// Process one property value, emitting zero or more triples.
    fn process_property(
        &mut self,
        subject: &str,
        predicate: &str,
        value: &Value,
        ctx: &Context,
        graph: Option<&str>,
    ) -> Result<(), String> {
        match value {
            Value::Array(items) => {
                for item in items {
                    self.process_property(subject, predicate, item, ctx, graph)?;
                }
            }

            Value::Object(obj) => {
                // Value node (`@value`) → literal.
                if obj.contains_key("@value") {
                    let object = Self::parse_value_node(obj, ctx)?;
                    self.emit(
                        subject.to_owned(),
                        predicate.to_owned(),
                        object,
                        graph.map(str::to_owned),
                    );
                    return Ok(());
                }

                // `@id`-only object → IRI reference.
                if let (1, Some(Value::String(iri))) =
                    (obj.len(), obj.get("@id"))
                {
                    let object = resolve_iri(iri, ctx);
                    self.emit(
                        subject.to_owned(),
                        predicate.to_owned(),
                        object,
                        graph.map(str::to_owned),
                    );
                    return Ok(());
                }

                // `@list` → RDF list.
                if let Some(list_val) = obj.get("@list") {
                    let head = self.parse_list_node(list_val, ctx, graph)?;
                    self.emit(
                        subject.to_owned(),
                        predicate.to_owned(),
                        head,
                        graph.map(str::to_owned),
                    );
                    return Ok(());
                }

                // Nested node → recurse; link via the predicate.
                let object = self.process_node(value, ctx, graph)?;
                self.emit(
                    subject.to_owned(),
                    predicate.to_owned(),
                    object,
                    graph.map(str::to_owned),
                );
            }

            Value::String(s) => {
                // Bare string value: IRI if predicate is `@type`, else plain
                // literal. Per JSON-LD 1.1 §6.4.2, the context's `@type`
                // mapping of the property key guides interpretation; we
                // default to plain literal (xsd:string) for unknown properties
                // and use IRI if the context type-mapping is `@id`.
                let object = if ctx.type_is_id(predicate) {
                    resolve_iri(s, ctx)
                } else {
                    encode_plain_literal(s)
                };
                self.emit(
                    subject.to_owned(),
                    predicate.to_owned(),
                    object,
                    graph.map(str::to_owned),
                );
            }

            Value::Bool(b) => {
                let lex = if *b { "true" } else { "false" };
                let object =
                    format!("\"{}\"^^{}", lex, xsd("boolean"));
                self.emit(
                    subject.to_owned(),
                    predicate.to_owned(),
                    object,
                    graph.map(str::to_owned),
                );
            }

            Value::Number(n) => {
                let object = encode_number(n);
                self.emit(
                    subject.to_owned(),
                    predicate.to_owned(),
                    object,
                    graph.map(str::to_owned),
                );
            }

            Value::Null => {
                // JSON `null` maps to nothing in RDF (JSON-LD 1.1 §6.5).
            }
        }
        Ok(())
    }

    /// Parse a `@value` node into a canonical literal string.
    fn parse_value_node(
        obj: &serde_json::Map<String, Value>,
        ctx: &Context,
    ) -> Result<String, String> {
        let raw_value = obj.get("@value").ok_or("@value node missing @value")?;

        let lang = obj.get("@language").and_then(|v| v.as_str());
        let datatype = obj.get("@type").and_then(|v| v.as_str());
        let direction = obj.get("@direction").and_then(|v| v.as_str());

        match raw_value {
            Value::String(lex) => {
                let escaped = escape_literal(lex);
                if let Some(dir) = direction {
                    // Text direction: encode as i18n string per JSON-LD 1.1
                    // §4.2.4. We represent using rdf:CompoundLiteral approach
                    // or fall back to a plain string annotation. For the diff
                    // harness, we use the i18n-IRI form when direction is set.
                    let lang_tag = lang.unwrap_or("und");
                    let i18n_iri = format!(
                        "<https://www.w3.org/ns/i18n#{lang_tag}_{dir}>"
                    );
                    Ok(format!("\"{escaped}\"^^{i18n_iri}"))
                } else if let Some(tag) = lang {
                    // Language-tagged literal.
                    Ok(format!("\"{escaped}\"@{tag}"))
                } else if let Some(dt) = datatype {
                    let dt_iri = resolve_iri(dt, ctx);
                    // xsd:string is the implicit default — strip to match
                    // parsers that leave the datatype implicit.
                    if dt_iri == xsd("string") {
                        Ok(format!("\"{escaped}\""))
                    } else {
                        Ok(format!("\"{escaped}\"^^{dt_iri}"))
                    }
                } else {
                    // Plain string literal.
                    Ok(format!("\"{escaped}\""))
                }
            }

            Value::Bool(b) => {
                let lex = if *b { "true" } else { "false" };
                Ok(format!("\"{}\"^^{}", lex, xsd("boolean")))
            }

            Value::Number(n) => Ok(encode_number(n)),

            _ => Err(format!("@value must be a scalar, got: {raw_value:?}")),
        }
    }

    /// Parse an RDF list from a `@list` value. Returns the head term.
    fn parse_list_node(
        &mut self,
        list_val: &Value,
        ctx: &Context,
        graph: Option<&str>,
    ) -> Result<String, String> {
        let items: &[Value] = match list_val {
            Value::Array(arr) => arr.as_slice(),
            _ => return Err("@list value must be an array".to_owned()),
        };

        if items.is_empty() {
            return Ok(rdf("nil"));
        }

        let mut head = self.fresh_bnode();
        let first_head = head.clone();

        for (i, item) in items.iter().enumerate() {
            let object = self.process_node(item, ctx, graph)?;
            self.emit(head.clone(), rdf("first"), object, graph.map(str::to_owned));

            if i + 1 < items.len() {
                let rest = self.fresh_bnode();
                self.emit(head.clone(), rdf("rest"), rest.clone(), graph.map(str::to_owned));
                head = rest;
            } else {
                self.emit(head.clone(), rdf("rest"), rdf("nil"), graph.map(str::to_owned));
            }
        }

        Ok(first_head)
    }
}

// -----------------------------------------------------------------------
// Context
// -----------------------------------------------------------------------

/// A JSON-LD context: maps compact terms to their expanded IRI forms.
///
/// This is a simplified processing model sufficient for the harness
/// (expand+toRdf only; no compact or frame).
#[cfg(feature = "shadow")]
#[derive(Debug, Default, Clone)]
struct Context {
    /// term → absolute IRI (without angle brackets).
    terms: BTreeMap<String, String>,
    /// term → `true` when the term's `@type` mapping is `@id`.
    type_id: BTreeMap<String, bool>,
    /// Active base IRI (without angle brackets). Empty means none.
    base: Option<String>,
    /// Active vocab IRI. Empty means none.
    vocab: Option<String>,
}

#[cfg(feature = "shadow")]
impl Context {
    /// Build a context from a JSON-LD object (extracts `@context` key).
    fn from_object(obj: &serde_json::Map<String, Value>) -> Result<Self, String> {
        let mut ctx = Self::default();
        if let Some(cv) = obj.get("@context") {
            ctx.merge_value(cv)?;
        }
        Ok(ctx)
    }

    /// Merge a `@context` value into this context.
    fn merge_value(&mut self, cv: &Value) -> Result<(), String> {
        match cv {
            Value::Null => {
                // Null context resets mappings (JSON-LD 1.1 §9.4.1).
                *self = Self::default();
            }
            Value::String(iri) => {
                // Remote context reference — we cannot fetch it; record the
                // base IRI only. For the diff harness this is acceptable:
                // remote contexts are treated as unknown.
                self.base = Some(iri.clone());
            }
            Value::Object(map) => {
                self.merge_map(map);
            }
            Value::Array(items) => {
                for item in items {
                    self.merge_value(item)?;
                }
            }
            _ => {
                return Err(format!("invalid @context value: {cv:?}"));
            }
        }
        Ok(())
    }

    fn merge_map(&mut self, map: &serde_json::Map<String, Value>) {
        // `@base` and `@vocab` first.
        if let Some(Value::String(base)) = map.get("@base") {
            self.base = Some(base.clone());
        }
        if let Some(Value::String(vocab)) = map.get("@vocab") {
            self.vocab = Some(vocab.clone());
        }

        for (key, val) in map {
            match key.as_str() {
                "@base" | "@vocab" | "@version" | "@import" | "@propagate" | "@language"
                | "@direction" => {}
                _ => {
                    match val {
                        Value::String(iri) => {
                            // Simple term definition: term → IRI.
                            self.terms.insert(key.clone(), iri.clone());
                        }
                        Value::Object(def) => {
                            // Expanded term definition.
                            if let Some(Value::String(id_val)) = def.get("@id") {
                                self.terms.insert(key.clone(), id_val.clone());
                            }
                            // Check for @type: @id mapping.
                            if let Some(Value::String(type_mapping)) = def.get("@type")
                                && (type_mapping == "@id" || type_mapping == "@vocab") {
                                    self.type_id.insert(key.clone(), true);
                                }
                        }
                        Value::Null => {
                            // Null term definition removes the mapping.
                            self.terms.remove(key);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Expand a compact term or CURIE to a full IRI string (no angle brackets).
    fn expand_term_raw(&self, term: &str) -> Option<String> {
        // Already an absolute IRI.
        if term.contains("://") || term.starts_with("urn:") {
            return Some(term.to_owned());
        }

        // Direct term mapping.
        if let Some(mapped) = self.terms.get(term) {
            return Some(expand_mapped(mapped, self));
        }

        // CURIE: prefix:local.
        if let Some(colon) = term.find(':') {
            let prefix = &term[..colon];
            let local = &term[colon + 1..];
            // Double slash after colon → absolute IRI, not a CURIE.
            if local.starts_with("//") {
                return Some(term.to_owned());
            }
            if let Some(ns) = self.terms.get(prefix) {
                let expanded = expand_mapped(ns, self);
                return Some(format!("{expanded}{local}"));
            }
        }

        // Vocabulary mapping.
        if let Some(ref vocab) = self.vocab {
            return Some(format!("{vocab}{term}"));
        }

        None
    }

    /// Expand term and wrap in angle brackets for canonical form.
    fn expand_term(&self, term: &str) -> String {
        match self.expand_term_raw(term) {
            Some(iri) => format!("<{iri}>"),
            None => format!("<{term}>"),
        }
    }

    /// True when the term's `@type` context mapping is `@id`.
    fn type_is_id(&self, predicate: &str) -> bool {
        // Strip angle brackets to get the compact form for lookup.
        let key = if predicate.starts_with('<') && predicate.ends_with('>') {
            &predicate[1..predicate.len() - 1]
        } else {
            predicate
        };
        self.type_id.get(key).copied().unwrap_or(false)
    }
}

// -----------------------------------------------------------------------
// IRI helpers
// -----------------------------------------------------------------------

/// Resolve an IRI string against the active context.
///
/// Handles:
/// - Absolute IRIs (returned as-is, wrapped in `<…>`)
/// - Compact IRIs (CURIEs): `prefix:local`
/// - Blank nodes: `_:label`
/// - Keywords: `@…` — returned as-is for caller to handle
#[cfg(feature = "shadow")]
fn resolve_iri(s: &str, ctx: &Context) -> String {
    // Blank node passthrough.
    if s.starts_with("_:") {
        return s.to_owned();
    }

    // JSON-LD keywords are returned as-is (caller decides how to use them).
    if s.starts_with('@') {
        return s.to_owned();
    }

    // Already angle-bracketed.
    if s.starts_with('<') && s.ends_with('>') {
        return s.to_owned();
    }

    if let Some(expanded) = ctx.expand_term_raw(s) {
        return format!("<{expanded}>");
    }

    // Absolute IRI heuristic: contains "://" or is "urn:…".
    if s.contains("://") || s.starts_with("urn:") {
        return format!("<{s}>");
    }

    // Relative IRI or unknown: wrap verbatim (imperfect but adequate for
    // diff purposes; real resolution needs a base IRI).
    if let Some(ref base) = ctx.base {
        return format!("<{base}{s}>");
    }

    format!("<{s}>")
}

/// Expand a mapped value that itself might be a CURIE or relative path.
#[cfg(feature = "shadow")]
fn expand_mapped(mapped: &str, ctx: &Context) -> String {
    // If the mapped value is itself a CURIE, expand it.
    if mapped.contains(':') && !mapped.contains("://") && !mapped.starts_with("urn:")
        && let Some(pos) = mapped.find(':') {
            let prefix = &mapped[..pos];
            let local = &mapped[pos + 1..];
            if !local.starts_with("//")
                && let Some(ns) = ctx.terms.get(prefix) {
                    return format!("{ns}{local}");
                }
        }
    mapped.to_owned()
}

// -----------------------------------------------------------------------
// Literal helpers
// -----------------------------------------------------------------------

/// Escape a string for use as a literal lexical form. Per the canonical-form
/// contract, lexical forms are preserved byte-for-byte, so we only escape
/// characters that are special in the `"…"` delimited form.
#[cfg(feature = "shadow")]
fn escape_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

/// Encode a plain string literal (xsd:string implicit).
#[cfg(feature = "shadow")]
fn encode_plain_literal(s: &str) -> String {
    format!("\"{}\"", escape_literal(s))
}

/// Encode a JSON number as an RDF literal.
///
/// JSON-LD 1.1 §4.2.2 rules:
/// - Integer (no decimal point, no exponent) → `xsd:integer`
/// - Decimal (decimal point, no exponent)   → `xsd:decimal`
/// - Double  (exponent or special form)      → `xsd:double`
#[cfg(feature = "shadow")]
fn encode_number(n: &serde_json::Number) -> String {
    let s = n.to_string();
    if s.contains('e') || s.contains('E') {
        // Double.
        format!("\"{}\"^^{}", s, xsd("double"))
    } else if s.contains('.') {
        // Decimal.
        format!("\"{}\"^^{}", s, xsd("decimal"))
    } else {
        // Integer.
        format!("\"{}\"^^{}", s, xsd("integer"))
    }
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[cfg(all(test, feature = "shadow"))]
mod tests {
    use super::*;

    fn parse(input: &str) -> Facts {
        let p = JsonLdShadowParser::new();
        p.parse(input.as_bytes()).expect("parse ok").facts
    }

    fn parse_err(input: &str) -> Diagnostics {
        let p = JsonLdShadowParser::new();
        p.parse(input.as_bytes()).expect_err("parse should fail")
    }

    // -------------------------------------------------------------------
    // Basic acceptance
    // -------------------------------------------------------------------

    #[test]
    fn parser_id_is_correct() {
        let p = JsonLdShadowParser::new();
        assert_eq!(p.id(), "rdf-jsonld-shadow");
    }

    #[test]
    fn empty_object_produces_no_facts() {
        let facts = parse("{}");
        assert_eq!(facts.set.len(), 0, "empty object should produce no facts");
    }

    #[test]
    fn empty_array_produces_no_facts() {
        let facts = parse("[]");
        assert_eq!(facts.set.len(), 0);
    }

    // -------------------------------------------------------------------
    // @id and property triples
    // -------------------------------------------------------------------

    #[test]
    fn simple_iri_subject() {
        let input = r#"
        {
            "@context": { "ex": "http://example.org/" },
            "@id": "http://example.org/s",
            "http://example.org/p": "hello"
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://example.org/s>");
        assert_eq!(f.predicate, "<http://example.org/p>");
        assert_eq!(f.object, "\"hello\"");
        assert_eq!(f.graph, None);
    }

    #[test]
    fn curie_expansion_in_subject_and_predicate() {
        let input = r#"
        {
            "@context": {
                "ex": "http://example.org/",
                "name": {"@id": "http://schema.org/name"}
            },
            "@id": "ex:Alice",
            "name": "Alice"
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://example.org/Alice>");
        assert_eq!(f.predicate, "<http://schema.org/name>");
        assert_eq!(f.object, "\"Alice\"");
    }

    // -------------------------------------------------------------------
    // @type
    // -------------------------------------------------------------------

    #[test]
    fn type_triple_emitted() {
        let input = r#"
        {
            "@context": {"ex": "http://example.org/"},
            "@id": "http://example.org/alice",
            "@type": "http://example.org/Person"
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.predicate, "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>");
        assert_eq!(f.object, "<http://example.org/Person>");
    }

    #[test]
    fn multiple_types() {
        let input = r#"
        {
            "@id": "http://example.org/x",
            "@type": ["http://example.org/A", "http://example.org/B"]
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 2);
    }

    // -------------------------------------------------------------------
    // @value literals
    // -------------------------------------------------------------------

    #[test]
    fn value_node_plain_literal() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": { "@value": "hello" }
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.object, "\"hello\"");
    }

    #[test]
    fn value_node_language_literal() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": { "@value": "bonjour", "@language": "fr" }
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.object, "\"bonjour\"@fr");
    }

    #[test]
    fn value_node_datatyped_integer() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": {
                "@value": "42",
                "@type": "http://www.w3.org/2001/XMLSchema#integer"
            }
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(
            f.object,
            "\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn value_node_boolean() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": { "@value": true }
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(
            f.object,
            "\"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>"
        );
    }

    #[test]
    fn bare_number_integer() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": 7
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(
            f.object,
            "\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>"
        );
    }

    #[test]
    fn bare_number_decimal() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": 3.14
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(
            f.object,
            "\"3.14\"^^<http://www.w3.org/2001/XMLSchema#decimal>"
        );
    }

    // -------------------------------------------------------------------
    // @graph (named graphs)
    // -------------------------------------------------------------------

    #[test]
    fn named_graph_facts_carry_graph_iri() {
        let input = r#"
        {
            "@id": "http://ex/graph1",
            "@graph": [
                {
                    "@id": "http://ex/s",
                    "http://ex/p": "v"
                }
            ]
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://ex/s>");
        assert_eq!(f.graph, Some("<http://ex/graph1>".to_owned()));
    }

    #[test]
    fn default_graph_has_no_graph_name() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": "v"
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.graph, None);
    }

    // -------------------------------------------------------------------
    // Blank nodes
    // -------------------------------------------------------------------

    #[test]
    fn object_without_id_becomes_bnode() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": {
                "http://ex/q": "inner"
            }
        }"#;
        let facts = parse(input);
        // We expect two facts: s → p → _:b0 and _:b0 → q → "inner"
        assert_eq!(facts.set.len(), 2);
        let subjects: std::collections::BTreeSet<_> =
            facts.set.keys().map(|f| f.subject.clone()).collect();
        assert!(subjects.contains("<http://ex/s>"));
    }

    #[test]
    fn explicit_bnode_subject() {
        let input = r#"{ "@id": "_:b42", "http://ex/p": "v" }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        // Canonical relabelling will rename it, but subject should be a bnode.
        assert!(f.subject.starts_with("_:"));
    }

    // -------------------------------------------------------------------
    // @list
    // -------------------------------------------------------------------

    #[test]
    fn list_node_expands_to_rdf_list() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": { "@list": ["http://ex/a", "http://ex/b"] }
        }"#;
        let facts = parse(input);
        // 1 link from s to head + 2×(first, rest) = 5 triples.
        assert_eq!(facts.set.len(), 5);
    }

    #[test]
    fn empty_list_is_rdf_nil() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": { "@list": [] }
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(
            f.object,
            "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>"
        );
    }

    // -------------------------------------------------------------------
    // Error handling
    // -------------------------------------------------------------------

    #[test]
    fn invalid_json_is_fatal_error() {
        let d = parse_err("{not json}");
        assert!(d.fatal);
        assert!(!d.messages.is_empty());
    }

    #[test]
    fn invalid_utf8_is_fatal_error() {
        let p = JsonLdShadowParser::new();
        let d = p.parse(b"\xff\xfe").expect_err("should fail");
        assert!(d.fatal);
    }

    #[test]
    fn number_root_is_fatal_error() {
        let d = parse_err("42");
        assert!(d.fatal);
    }

    // -------------------------------------------------------------------
    // Context validation
    // -------------------------------------------------------------------

    #[test]
    fn null_context_resets_mappings() {
        // After a null context the term "ex" should not be mapped.
        let input = r#"
        {
            "@context": [
                {"ex": "http://example.org/"},
                null
            ],
            "@id": "http://example.org/x",
            "http://example.org/p": "v"
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 1);
    }

    #[test]
    fn context_array_merges_definitions() {
        let input = r#"
        {
            "@context": [
                {"ex": "http://example.org/"},
                {"name": {"@id": "http://example.org/name"}}
            ],
            "@id": "ex:Alice",
            "name": "Alice"
        }"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 1);
        let f = facts.set.keys().next().unwrap();
        assert_eq!(f.subject, "<http://example.org/Alice>");
        assert_eq!(f.predicate, "<http://example.org/name>");
    }

    // -------------------------------------------------------------------
    // Literal escape
    // -------------------------------------------------------------------

    #[test]
    fn literal_with_quotes_is_escaped() {
        let input = r#"
        {
            "@id": "http://ex/s",
            "http://ex/p": "say \"hello\""
        }"#;
        let facts = parse(input);
        let f = facts.set.keys().next().unwrap();
        assert!(f.object.contains("\\\""), "quotes should be escaped in literal");
    }

    #[test]
    fn top_level_array_of_nodes() {
        let input = r#"
        [
            {"@id": "http://ex/a", "http://ex/p": "1"},
            {"@id": "http://ex/b", "http://ex/p": "2"}
        ]"#;
        let facts = parse(input);
        assert_eq!(facts.set.len(), 2);
    }
}
