//! JSON-LD to RDF conversion algorithm.
//!
//! Implements the W3C JSON-LD 1.1 "Deserialize JSON-LD to RDF" algorithm,
//! restricted to the Phase B scope: no remote context fetching, no expand/
//! compact/normalize semantics beyond what toRdf requires.

use std::collections::BTreeMap;

use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome};
use serde_json::Value;

use crate::context::{is_absolute_iri, resolve_iri, Context};
use crate::error::{jsonld_err, Result};

/// RDF IRIs / well-known constants.
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn rdf(local: &str) -> String {
    format!("{RDF}{local}")
}
fn xsd(local: &str) -> String {
    format!("{XSD}{local}")
}

/// State threaded through conversion.
struct State<'a> {
    parser_id: &'a str,
    bnode_counter: usize,
    facts: Vec<(Fact, FactProvenance)>,
}

impl<'a> State<'a> {
    const fn new(parser_id: &'a str) -> Self {
        Self {
            parser_id,
            bnode_counter: 0,
            facts: Vec::new(),
        }
    }

    fn fresh_bnode(&mut self) -> String {
        let n = self.bnode_counter;
        self.bnode_counter += 1;
        format!("_:b{n}")
    }

    fn emit(&mut self, subject: String, predicate: String, object: String, graph: Option<String>) {
        let fact = Fact {
            subject,
            predicate,
            object,
            graph,
        };
        let prov = FactProvenance {
            offset: None,
            parser: self.parser_id.to_owned(),
        };
        self.facts.push((fact, prov));
    }
}

/// Entry point: convert a parsed JSON-LD document to RDF quads.
pub fn convert(
    doc: &Value,
    doc_base: Option<&str>,
    parser_id: &str,
) -> std::result::Result<ParseOutcome, Diagnostics> {
    let mut state = State::new(parser_id);
    convert_inner(doc, doc_base, &mut state).map_err(Diagnostics::from)?;

    let facts = Facts::canonicalise(state.facts, BTreeMap::new());
    Ok(ParseOutcome {
        facts,
        warnings: Diagnostics {
            messages: Vec::new(),
            fatal: false,
        },
    })
}

fn convert_inner(
    doc: &Value,
    doc_base: Option<&str>,
    state: &mut State<'_>,
) -> Result<()> {
    // Build the active context from a top-level @context if present.
    let (ctx, nodes) = extract_context_and_nodes(doc, doc_base)?;
    convert_nodes(&nodes, &ctx, None, doc_base, state)
}

/// Extract `@context` and return a list of node objects to process.
fn extract_context_and_nodes<'v>(
    doc: &'v Value,
    doc_base: Option<&str>,
) -> Result<(Context, Vec<&'v Value>)> {
    match doc {
        Value::Object(map) => {
            let ctx = if let Some(ctx_val) = map.get("@context") {
                Context::from_json(ctx_val, doc_base)?
            } else {
                let mut c = Context::default();
                if let Some(base) = doc_base {
                    c.base = Some(base.to_owned());
                }
                c
            };

            // Top-level @graph without @id → default graph array shorthand.
            // If the object also has @id or other properties, treat it as a
            // regular node (convert_node will handle @graph as named graph).
            if let Some(graph_val) = map.get("@graph") {
                let has_id = map.contains_key("@id");
                let has_type = map.contains_key("@type");
                let has_props = map.keys().any(|k| {
                    !matches!(k.as_str(), "@context" | "@graph" | "@id" | "@type")
                });
                if !has_id && !has_type && !has_props {
                    // Pure default-graph collection.
                    let nodes = collect_array_or_single(graph_val);
                    return Ok((ctx, nodes));
                }
            }

            Ok((ctx, vec![doc]))
        }
        Value::Array(arr) => {
            let mut c = Context::default();
            if let Some(base) = doc_base {
                c.base = Some(base.to_owned());
            }
            Ok((c, arr.iter().collect()))
        }
        _ => Err(jsonld_err("JSON-LD document must be object or array")),
    }
}

fn collect_array_or_single(val: &Value) -> Vec<&Value> {
    match val {
        Value::Array(arr) => arr.iter().collect(),
        _ => vec![val],
    }
}

/// Convert an array/list of node objects in the given graph context.
fn convert_nodes(
    nodes: &[&Value],
    ctx: &Context,
    graph: Option<&str>,
    doc_base: Option<&str>,
    state: &mut State<'_>,
) -> Result<()> {
    for node in nodes {
        convert_node(node, ctx, graph, doc_base, state)?;
    }
    Ok(())
}

/// Convert a single node object.
fn convert_node(
    node: &Value,
    parent_ctx: &Context,
    graph: Option<&str>,
    doc_base: Option<&str>,
    state: &mut State<'_>,
) -> Result<String> {
    let Value::Object(map) = node else {
        return Err(jsonld_err("expected node object"));
    };

    // Merge local @context if present.
    let local_ctx;
    let ctx: &Context = if let Some(ctx_val) = map.get("@context") {
        let mut c = parent_ctx.clone();
        crate::context::merge_context(&mut c, ctx_val, doc_base)?;
        local_ctx = c;
        &local_ctx
    } else {
        parent_ctx
    };

    // Determine subject.
    let subject = match map.get("@id") {
        Some(Value::String(s)) => {
            if s.starts_with("_:") {
                s.clone()
            } else if s.is_empty() {
                // @id "" → document base.
                ctx.base
                    .clone()
                    .or_else(|| doc_base.map(String::from))
                    .unwrap_or_else(|| state.fresh_bnode())
            } else if is_absolute_iri(s) {
                s.clone()
            } else {
                // Relative IRI — resolve against base.
                resolve_iri(s, ctx.base.as_deref().or(doc_base))
                    .unwrap_or_else(|_| state.fresh_bnode())
            }
        }
        None => state.fresh_bnode(),
        _ => return Err(jsonld_err("@id must be a string")),
    };

    // Handle @type.
    if let Some(type_val) = map.get("@type") {
        let types = collect_string_array(type_val)?;
        for t in types {
            let expanded = expand_type(ctx, t, doc_base)?;
            state.emit(
                subject.clone(),
                rdf("type"),
                expanded,
                graph.map(String::from),
            );
        }
    }

    // Handle @graph (named graph).
    if let Some(graph_val) = map.get("@graph") {
        // The subject is the graph name.
        let graph_name = subject.clone();
        let inner_nodes = collect_array_or_single(graph_val);
        convert_nodes(&inner_nodes, ctx, Some(&graph_name), doc_base, state)?;
        // The node itself (with its @type etc.) gets emitted in the parent graph.
    }

    // Process remaining properties.
    for (key, val) in map {
        match key.as_str() {
            "@id" | "@context" | "@type" | "@graph" => continue,
            "@reverse" => {
                // @reverse is a map of predicate → [node objects].
                if let Value::Object(rev_map) = val {
                    for (rev_pred_str, rev_val) in rev_map {
                        let pred = expand_predicate(ctx, rev_pred_str)?;
                        let objs = collect_array_or_single(rev_val);
                        for obj_node in objs {
                            let obj_id =
                                convert_node(obj_node, ctx, graph, doc_base, state)?;
                            // Reversed: obj_id is subject, subject is object.
                            state.emit(
                                obj_id,
                                pred.clone(),
                                subject.clone(),
                                graph.map(String::from),
                            );
                        }
                    }
                }
                continue;
            }
            k if k.starts_with('@') => continue, // other keywords
            _ => {}
        }

        // Expand predicate.
        let Ok(pred) = expand_predicate(ctx, key) else {
            continue; // skip unexpandable predicates
        };

        // Get the term definition for coercion hints.
        let term_def = ctx.terms.get(key.as_str());

        // Convert the value(s).
        convert_property_value(
            val,
            &subject,
            &pred,
            term_def,
            ctx,
            graph,
            doc_base,
            state,
        )?;
    }

    Ok(subject)
}

/// Convert a property value (may be array, value object, node object, scalar).
#[allow(clippy::too_many_arguments)]
fn convert_property_value(
    val: &Value,
    subject: &str,
    pred: &str,
    term_def: Option<&crate::context::TermDef>,
    ctx: &Context,
    graph: Option<&str>,
    doc_base: Option<&str>,
    state: &mut State<'_>,
) -> Result<()> {
    // Check @container: @list on the term definition.
    let container = term_def.and_then(|td| td.container.as_deref());

    match val {
        Value::Array(arr) => {
            if container == Some("@list") {
                // Treat the array as an rdf:List.
                let obj = convert_list(arr, ctx, graph, doc_base, state)?;
                state.emit(
                    subject.to_owned(),
                    pred.to_owned(),
                    obj,
                    graph.map(String::from),
                );
            } else {
                for item in arr {
                    convert_property_value(
                        item, subject, pred, term_def, ctx, graph, doc_base, state,
                    )?;
                }
            }
        }
        Value::Object(map) if map.contains_key("@list") => {
            let list_items = collect_array_or_single(&map["@list"]);
            let items_vec: Vec<Value> = list_items.iter().map(|v| (*v).clone()).collect();
            let obj = convert_list(&items_vec, ctx, graph, doc_base, state)?;
            state.emit(
                subject.to_owned(),
                pred.to_owned(),
                obj,
                graph.map(String::from),
            );
        }
        Value::Object(map) if map.contains_key("@value") => {
            let obj = convert_value_object(map, term_def, ctx)?;
            state.emit(
                subject.to_owned(),
                pred.to_owned(),
                obj,
                graph.map(String::from),
            );
        }
        Value::Object(_) => {
            let obj = convert_node(val, ctx, graph, doc_base, state)?;
            state.emit(
                subject.to_owned(),
                pred.to_owned(),
                obj,
                graph.map(String::from),
            );
        }
        scalar => {
            let obj = convert_scalar(scalar, term_def, ctx, doc_base)?;
            state.emit(
                subject.to_owned(),
                pred.to_owned(),
                obj,
                graph.map(String::from),
            );
        }
    }
    Ok(())
}

/// Convert a `@value` object to a canonical literal string.
fn convert_value_object(
    map: &serde_json::Map<String, Value>,
    term_def: Option<&crate::context::TermDef>,
    ctx: &Context,
) -> Result<String> {
    let value = &map["@value"];

    // @language
    if let Some(Value::String(lang)) = map.get("@language") {
        let lex = value_to_lex(value)?;
        return Ok(format!("\"{}\"@{}", escape_lex(&lex), lang));
    }

    // @type
    if let Some(Value::String(ty)) = map.get("@type") {
        if ty == "@none" || ty == "@json" {
            let lex = value_to_lex(value)?;
            return Ok(format!("\"{}\"", escape_lex(&lex)));
        }
        let dt = if is_absolute_iri(ty) {
            ty.clone()
        } else {
            ctx.expand_iri(ty).unwrap_or_else(|| ty.clone())
        };
        let lex = value_to_lex(value)?;
        return Ok(format!("\"{}\"^^<{}>", escape_lex(&lex), dt));
    }

    // Term-level coercion.
    if let Some(td) = term_def
        && let Some(ref tc) = td.type_coerce {
            match tc.as_str() {
                "@id" => {
                    // Treat the value as an IRI.
                    if let Value::String(s) = value {
                        return Ok(format!("<{s}>"));
                    }
                }
                "@vocab" => {
                    if let Value::String(s) = value {
                        let expanded =
                            ctx.expand_iri_as_vocab(s).unwrap_or_else(|| s.clone());
                        return Ok(format!("<{expanded}>"));
                    }
                }
                dt => {
                    let lex = value_to_lex(value)?;
                    return Ok(format!("\"{}\"^^<{}>", escape_lex(&lex), dt));
                }
            }
        }

    // Default: plain literal from the JSON value.
    let lex = value_to_lex(value)?;
    Ok(format!("\"{}\"", escape_lex(&lex)))
}

/// Convert a scalar JSON value, possibly with term-level type coercion.
fn convert_scalar(
    val: &Value,
    term_def: Option<&crate::context::TermDef>,
    ctx: &Context,
    doc_base: Option<&str>,
) -> Result<String> {
    // Term-level type coercion.
    if let Some(td) = term_def {
        if let Some(ref tc) = td.type_coerce {
            match tc.as_str() {
                "@id" => {
                    if let Value::String(s) = val {
                        let resolved = if is_absolute_iri(s) || s.starts_with("_:") {
                            s.clone()
                        } else {
                            resolve_iri(s, ctx.base.as_deref().or(doc_base))
                                .unwrap_or_else(|_| s.clone())
                        };
                        return Ok(format!("<{resolved}>"));
                    }
                }
                "@vocab" => {
                    if let Value::String(s) = val {
                        let expanded = ctx
                            .expand_iri_as_vocab(s)
                            .or_else(|| {
                                if is_absolute_iri(s) {
                                    Some(s.clone())
                                } else {
                                    None
                                }
                            })
                            .unwrap_or_else(|| s.clone());
                        return Ok(format!("<{expanded}>"));
                    }
                }
                dt if !dt.starts_with('@') => {
                    let lex = value_to_lex(val)?;
                    return Ok(format!("\"{}\"^^<{}>", escape_lex(&lex), dt));
                }
                _ => {}
            }
        }

        // Term language coercion.
        if let Some(ref lang) = td.language
            && let Value::String(s) = val {
                return Ok(format!("\"{}\"@{}", escape_lex(s), lang));
            }
    }

    // JSON number / boolean → typed literals.
    match val {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                return Ok(format!("\"{}\"^^<{}>", i, xsd("integer")));
            }
            if let Some(f) = n.as_f64() {
                // JSON-LD spec §8.6: use xsd:double in E-notation.
                return Ok(format!(
                    "\"{}\"^^<{}>",
                    format_double(f),
                    xsd("double")
                ));
            }
            Err(jsonld_err("cannot serialize JSON number"))
        }
        Value::Bool(b) => Ok(format!(
            "\"{}\"^^<{}>",
            if *b { "true" } else { "false" },
            xsd("boolean")
        )),
        Value::String(s) => Ok(format!("\"{}\"", escape_lex(s))),
        _ => Err(jsonld_err("cannot convert JSON null to RDF literal")),
    }
}

/// Convert an `@list` value to an rdf:List chain. Returns the head node.
fn convert_list(
    items: &[Value],
    ctx: &Context,
    graph: Option<&str>,
    doc_base: Option<&str>,
    state: &mut State<'_>,
) -> Result<String> {
    if items.is_empty() {
        return Ok(format!("<{}>", rdf("nil")));
    }

    let head = state.fresh_bnode();
    let mut current = head.clone();

    for (i, item) in items.iter().enumerate() {
        // first / object value.
        let first_obj = convert_list_item(item, ctx, graph, doc_base, state)?;
        state.emit(
            current.clone(),
            rdf("first"),
            first_obj,
            graph.map(String::from),
        );

        let rest = if i + 1 < items.len() {
            
            state.fresh_bnode()
        } else {
            format!("<{}>", rdf("nil"))
        };
        state.emit(
            current.clone(),
            rdf("rest"),
            rest.clone(),
            graph.map(String::from),
        );
        current = rest;
    }

    Ok(head)
}

fn convert_list_item(
    item: &Value,
    ctx: &Context,
    graph: Option<&str>,
    doc_base: Option<&str>,
    state: &mut State<'_>,
) -> Result<String> {
    match item {
        Value::Object(map) if map.contains_key("@value") => {
            convert_value_object(map, None, ctx)
        }
        Value::Object(_) => convert_node(item, ctx, graph, doc_base, state),
        _ => convert_scalar(item, None, ctx, doc_base),
    }
}

/// Expand a `@type` value to a canonical IRI.
fn expand_type(ctx: &Context, ty: &str, doc_base: Option<&str>) -> Result<String> {
    if ty.starts_with("_:") {
        return Ok(ty.to_owned());
    }
    // Try context expansion first (handles compact IRIs like foaf:Person).
    if let Some(expanded) = ctx.expand_iri_as_vocab(ty)
        && !expanded.starts_with('@') {
            return Ok(format!("<{expanded}>"));
        }
    // Try relative IRI resolution against base.
    if let Some(base) = ctx.base.as_deref().or(doc_base)
        && let Ok(resolved) = resolve_iri(ty, Some(base)) {
            return Ok(format!("<{resolved}>"));
        }
    Err(jsonld_err(format!("cannot expand @type value: {ty}")))
}

/// Expand a predicate key to an absolute IRI.
fn expand_predicate(ctx: &Context, key: &str) -> Result<String> {
    // Context expansion handles compact IRIs, terms, vocab, and absolute IRIs.
    if let Some(expanded) = ctx.expand_iri(key).or_else(|| ctx.expand_iri_as_vocab(key))
        && !expanded.starts_with('@') {
            return Ok(format!("<{expanded}>"));
        }
    Err(jsonld_err(format!("cannot expand predicate: {key}")))
}

/// Convert a JSON value to a string lexical form.
fn value_to_lex(val: &Value) -> Result<String> {
    match val {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_string())
            } else if let Some(f) = n.as_f64() {
                Ok(format_double(f))
            } else {
                Err(jsonld_err("cannot serialize number"))
            }
        }
        Value::Bool(b) => Ok(if *b { "true" } else { "false" }.to_owned()),
        _ => Err(jsonld_err("cannot convert to lexical form")),
    }
}

/// Format a double in JSON-LD's E-notation (`1.23E4`).
fn format_double(f: f64) -> String {
    // JSON-LD §8.6: use the XSD double lexical form, which is E-notation.
    // We use one decimal digit and then "E0" for whole multiples.
    if f.is_nan() || f.is_infinite() {
        return format!("{f}");
    }
    // Use Rust's default to string and convert to E-notation.
    // Simple approach: format with enough precision.
    let s = format!("{f:E}");
    // Rust uses e, JSON-LD expects E; the exponent has no leading plus.
    // Convert "1.23e4" to "1.23E4".
    let s = s.replace('e', "E");
    // Remove + in exponent if present.
    let s = s.replace("E+", "E");
    // Remove leading zeros in exponent.
    if let Some(e_pos) = s.find('E') {
        let (mantissa, exp_str) = s.split_at(e_pos);
        let exp = &exp_str[1..]; // skip 'E'
        let trimmed_exp = exp.trim_start_matches('-').trim_start_matches('0');
        let exp_val: i32 = trimmed_exp.parse().unwrap_or(0);
        let neg = exp.starts_with('-');
        let exp_out = if neg {
            format!("-{exp_val}")
        } else {
            exp_val.to_string()
        };
        return format!("{mantissa}E{exp_out}");
    }
    s
}

/// Escape a literal lexical form for canonical N-Quads representation.
/// Only `\` and `"` need escaping.
fn escape_lex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

fn collect_string_array(val: &Value) -> Result<Vec<&str>> {
    match val {
        Value::String(s) => Ok(vec![s.as_str()]),
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.as_str()),
                _ => Err(jsonld_err("@type array must contain strings")),
            })
            .collect(),
        _ => Err(jsonld_err("@type must be a string or array")),
    }
}
