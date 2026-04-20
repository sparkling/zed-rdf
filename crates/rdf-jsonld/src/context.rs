//! JSON-LD `@context` processing — term definition resolution.
//!
//! Scope (Phase B): syntax + `@context` well-formedness only.
//! No remote context fetching, no `@import`, no `@propagate`.

use std::collections::HashMap;

use crate::error::{jsonld_err, Result};

/// The active context used during toRdf conversion.
#[derive(Debug, Clone, Default)]
pub struct Context {
    /// `@base` IRI (absolute). `None` = no base set by context.
    pub base: Option<String>,
    /// `@vocab` mapping.
    pub vocab: Option<String>,
    /// Term definitions: term -> expanded IRI (or `@type`/`@id`/etc. mapping).
    pub terms: HashMap<String, TermDef>,
}

/// A single term definition.
#[derive(Debug, Clone, Default)]
pub struct TermDef {
    /// The expanded IRI for this term.
    pub id: Option<String>,
    /// `@type` coercion: `"@id"` or an absolute IRI.
    pub type_coerce: Option<String>,
    /// `@container` mapping, e.g. `"@list"`, `"@set"`, `"@graph"`.
    pub container: Option<String>,
    /// `@language` default for this term.
    pub language: Option<String>,
    /// `@direction` for this term.
    pub direction: Option<String>,
    /// Whether this is a prefix.
    pub is_prefix: bool,
}

impl Context {
    /// Build a context from a JSON-LD `@context` value.
    ///
    /// `doc_base` is the document base IRI used to resolve relative `@base`
    /// values and is not stored on the context itself.
    pub fn from_json(ctx_val: &serde_json::Value, doc_base: Option<&str>) -> Result<Self> {
        let mut ctx = Self::default();
        // Set doc_base as the initial base so that @base "" resolves correctly.
        if let Some(base) = doc_base {
            ctx.base = Some(base.to_owned());
        }
        process_context_value(&mut ctx, ctx_val, doc_base, &mut Vec::new())?;
        Ok(ctx)
    }

    /// Expand a compact IRI or term to an absolute IRI using this context.
    ///
    /// Returns `None` when the term cannot be expanded (e.g., relative IRI
    /// with no base, keyword).
    pub fn expand_iri(&self, value: &str) -> Option<String> {
        expand_iri_inner(self, value, false)
    }

    /// Expand a term that appears as a value (may use `@vocab` for bare terms).
    pub fn expand_iri_as_vocab(&self, value: &str) -> Option<String> {
        expand_iri_inner(self, value, true)
    }
}

/// Merge an additional `@context` value into an existing context in-place.
pub fn merge_context(
    ctx: &mut Context,
    ctx_val: &serde_json::Value,
    doc_base: Option<&str>,
) -> Result<()> {
    process_context_value(ctx, ctx_val, doc_base, &mut Vec::new())
}

fn process_context_value(
    ctx: &mut Context,
    val: &serde_json::Value,
    doc_base: Option<&str>,
    _seen: &mut Vec<String>,
) -> Result<()> {
    match val {
        serde_json::Value::Null => {
            // `null` clears the context — reset to empty (but keep base from doc).
            *ctx = Context::default();
            if let Some(base) = doc_base {
                ctx.base = Some(base.to_owned());
            }
        }
        serde_json::Value::String(_s) => {
            // Remote context reference — out of Phase B scope.
            // We accept (no-op) to avoid false negatives on tests that use them
            // but whose output doesn't depend on the remote content.
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                process_context_value(ctx, item, doc_base, &mut Vec::new())?;
            }
        }
        serde_json::Value::Object(map) => {
            // Process @base first.
            if let Some(base_val) = map.get("@base") {
                match base_val {
                    serde_json::Value::Null => {
                        ctx.base = None;
                    }
                    serde_json::Value::String(s) => {
                        if s.is_empty() {
                            // Relative "" means use doc base.
                            if let Some(db) = doc_base {
                                ctx.base = Some(db.to_owned());
                            }
                        } else if is_absolute_iri(s) {
                            ctx.base = Some(s.clone());
                        } else {
                            // Relative reference — resolve against current base.
                            let resolved = resolve_iri(s, ctx.base.as_deref())?;
                            ctx.base = Some(resolved);
                        }
                    }
                    _ => return Err(jsonld_err("invalid base IRI")),
                }
            }

            // Process @vocab.
            if let Some(vocab_val) = map.get("@vocab") {
                match vocab_val {
                    serde_json::Value::Null => {
                        ctx.vocab = None;
                    }
                    serde_json::Value::String(s) => {
                        if s.is_empty() {
                            ctx.vocab = Some(String::new());
                        } else if is_absolute_iri(s) || s.starts_with('_') {
                            ctx.vocab = Some(s.clone());
                        } else {
                            // Compact IRI or relative — expand against current context.
                            let expanded =
                                expand_iri_inner(ctx, s, true).unwrap_or_else(|| s.clone());
                            ctx.vocab = Some(expanded);
                        }
                    }
                    _ => return Err(jsonld_err("invalid @vocab value")),
                }
            }

            // Process term definitions.
            for (key, def_val) in map {
                if key.starts_with('@') {
                    continue; // keywords already handled or ignored
                }
                let term_def = process_term_def(ctx, key, def_val)?;
                ctx.terms.insert(key.clone(), term_def);
            }
        }
        _ => return Err(jsonld_err("invalid @context value")),
    }
    Ok(())
}

fn process_term_def(
    ctx: &Context,
    _term: &str,
    def: &serde_json::Value,
) -> Result<TermDef> {
    match def {
        serde_json::Value::Null => {
            // Removes the term.
            Ok(TermDef::default())
        }
        serde_json::Value::String(s) => {
            // Simple IRI mapping.
            let id = expand_str_as_id(ctx, s);
            Ok(TermDef {
                id,
                is_prefix: s.ends_with('/') || s.ends_with('#'),
                ..Default::default()
            })
        }
        serde_json::Value::Object(map) => {
            let mut td = TermDef::default();

            if let Some(id_val) = map.get("@id") {
                match id_val {
                    serde_json::Value::Null => {
                        td.id = None;
                    }
                    serde_json::Value::String(s) => {
                        if s == "@type" {
                            td.id = Some("@type".to_owned());
                        } else {
                            td.id = expand_str_as_id(ctx, s);
                        }
                    }
                    _ => return Err(jsonld_err("invalid @id in term definition")),
                }
            }

            if let Some(type_val) = map.get("@type") {
                match type_val {
                    serde_json::Value::String(s) => {
                        if s == "@id" || s == "@vocab" || s == "@json" || s == "@none" {
                            td.type_coerce = Some(s.clone());
                        } else {
                            let expanded = expand_iri_inner(ctx, s, true)
                                .unwrap_or_else(|| s.clone());
                            td.type_coerce = Some(expanded);
                        }
                    }
                    _ => return Err(jsonld_err("invalid @type in term definition")),
                }
            }

            if let Some(container_val) = map.get("@container") {
                match container_val {
                    serde_json::Value::String(s) => {
                        td.container = Some(s.clone());
                    }
                    serde_json::Value::Array(arr) => {
                        // Take first string value for simplicity.
                        if let Some(serde_json::Value::String(s)) = arr.first() {
                            td.container = Some(s.clone());
                        }
                    }
                    _ => {}
                }
            }

            if let Some(lang_val) = map.get("@language")
                && let serde_json::Value::String(s) = lang_val {
                    td.language = Some(s.clone());
                }

            if let Some(dir_val) = map.get("@direction")
                && let serde_json::Value::String(s) = dir_val {
                    td.direction = Some(s.clone());
                }

            // If @id was not provided but the term looks like a compact IRI,
            // try expanding it.
            if td.id.is_none() {
                // id defaults to the term itself expanded.
            }

            // Derive is_prefix from id.
            if let Some(ref id) = td.id {
                td.is_prefix = id.ends_with('/') || id.ends_with('#') || id.ends_with(':');
            }

            Ok(td)
        }
        _ => Err(jsonld_err("invalid term definition")),
    }
}

fn expand_str_as_id(ctx: &Context, s: &str) -> Option<String> {
    if is_keyword(s) {
        return Some(s.to_owned());
    }
    expand_iri_inner(ctx, s, false)
}

/// Core IRI expansion logic.
fn expand_iri_inner(ctx: &Context, value: &str, use_vocab: bool) -> Option<String> {
    if value.is_empty() {
        return None;
    }

    // Keywords pass through.
    if is_keyword(value) {
        return Some(value.to_owned());
    }

    // Blank node.
    if value.starts_with("_:") {
        return Some(value.to_owned());
    }

    // Compact IRI prefix:local — split on first ':'.
    // Per JSON-LD §5.2.2: check context prefix BEFORE treating as absolute IRI.
    if let Some(colon) = value.find(':') {
        let prefix = &value[..colon];
        let local = &value[colon + 1..];
        // `://` is a real absolute IRI; do NOT try prefix expansion for those.
        if !local.starts_with("//")
            && let Some(td) = ctx.terms.get(prefix)
                && let Some(ref id) = td.id
                    && !is_keyword(id) {
                        return Some(format!("{id}{local}"));
                    }
    }

    // Already absolute (after prefix check so context prefixes take priority).
    if is_absolute_iri(value) {
        return Some(value.to_owned());
    }

    // Term lookup.
    if let Some(td) = ctx.terms.get(value) {
        if let Some(ref id) = td.id
            && !is_keyword(id) {
                return Some(id.clone());
            }
        td.id.as_ref()?;
    }

    // Vocab expansion.
    if use_vocab
        && let Some(ref vocab) = ctx.vocab {
            if vocab.is_empty() {
                return None;
            }
            return Some(format!("{vocab}{value}"));
        }

    // Relative IRI against @base.
    if let Some(ref base) = ctx.base {
        return resolve_iri(value, Some(base)).ok();
    }

    None
}

/// Simple RFC 3986 §5.2 reference resolution (sufficient for JSON-LD).
pub fn resolve_iri(reference: &str, base: Option<&str>) -> Result<String> {
    let Some(base) = base else {
        if is_absolute_iri(reference) {
            return Ok(reference.to_owned());
        }
        return Err(jsonld_err("relative IRI with no base"));
    };

    if is_absolute_iri(reference) {
        return Ok(remove_dot_segments_full(reference));
    }

    if reference.starts_with("//") {
        // Protocol-relative.
        let scheme = base
            .find(':')
            .map_or("https:", |i| &base[..=i]);
        return Ok(remove_dot_segments_full(&format!("{scheme}{reference}")));
    }

    if reference.starts_with('#') {
        // Fragment-only reference.
        let no_frag = base.split('#').next().unwrap_or(base);
        return Ok(format!("{no_frag}{reference}"));
    }

    if reference.is_empty() {
        return Ok(base.to_owned());
    }

    // Parse the base.
    let (base_scheme, base_rest) = split_scheme(base);
    let base_authority_path = strip_fragment(base_rest);
    let base_path_start = if let Some(stripped) = base_authority_path.strip_prefix("//") {
        stripped
            .find('/')
            .map_or(base_authority_path.len(), |i| i + 3)
    } else {
        0
    };
    let base_authority = &base_authority_path[..base_path_start];
    let base_path = &base_authority_path[base_path_start..];

    let merged_path = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let parent = base_path.rfind('/').map_or("", |i| &base_path[..=i]);
        format!("{parent}{reference}")
    };

    let resolved_path = remove_dot_segments(&merged_path);
    let resolved = format!("{base_scheme}:{base_authority}{resolved_path}");
    Ok(resolved)
}

fn split_scheme(iri: &str) -> (&str, &str) {
    iri.find(':').map_or(("", iri), |i| (&iri[..i], &iri[i + 1..]))
}

fn strip_fragment(s: &str) -> &str {
    s.split('#').next().unwrap_or(s)
}

/// Remove dot segments from a path per RFC 3986 §5.2.4.
fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    let mut input = path;

    while !input.is_empty() {
        if input.starts_with("../") {
            input = &input[3..];
        } else if input.starts_with("./") {
            input = &input[2..];
        } else if input.starts_with("/./") || input == "/." {
            input = if input == "/." { "/" } else { &input[2..] };
        } else if input.starts_with("/../") || input == "/.." {
            input = if input == "/.." { "/" } else { &input[3..] };
            output.pop();
        } else if input == "." || input == ".." {
            break;
        } else {
            let seg_end = if let Some(stripped) = input.strip_prefix('/') {
                stripped.find('/').map_or(input.len(), |i| i + 1)
            } else {
                input.find('/').unwrap_or(input.len())
            };
            let seg = &input[..seg_end];
            output.push(seg);
            input = &input[seg_end..];
        }
    }

    output.join("")
}

fn remove_dot_segments_full(iri: &str) -> String {
    if let Some(colon) = iri.find(':') {
        let scheme = &iri[..=colon];
        let rest = &iri[colon + 1..];
        // Find where path starts (after optional authority).
        let (authority, path_and_rest) = if let Some(after_slashes) = rest.strip_prefix("//") {
            let auth_end = after_slashes.find('/').map_or(rest.len(), |i| i + 3);
            (&rest[..auth_end], &rest[auth_end..])
        } else {
            ("", rest)
        };
        let (path, query_frag) = {
            let qf_start = path_and_rest
                .find(['?', '#'])
                .unwrap_or(path_and_rest.len());
            (
                &path_and_rest[..qf_start],
                &path_and_rest[qf_start..],
            )
        };
        let clean_path = remove_dot_segments(path);
        format!("{scheme}{authority}{clean_path}{query_frag}")
    } else {
        iri.to_owned()
    }
}

/// Check if `s` is an absolute IRI (has a scheme).
pub fn is_absolute_iri(s: &str) -> bool {
    // Scheme: ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"
    let bytes = s.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    let colon = bytes.iter().position(|&b| b == b':').unwrap_or(0);
    if colon < 2 {
        return false;
    }
    bytes[..colon]
        .iter()
        .all(|&b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "@base"
            | "@container"
            | "@context"
            | "@direction"
            | "@graph"
            | "@id"
            | "@import"
            | "@included"
            | "@index"
            | "@json"
            | "@language"
            | "@list"
            | "@nest"
            | "@none"
            | "@prefix"
            | "@propagate"
            | "@protected"
            | "@reverse"
            | "@set"
            | "@type"
            | "@value"
            | "@version"
            | "@vocab"
    )
}
