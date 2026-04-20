//! Rename and code-action handlers — Phase G.

use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, Position, TextEdit, Url, WorkspaceEdit,
};

use crate::Language;

// ---------------------------------------------------------------------------
// Rename
// ---------------------------------------------------------------------------

/// Compute the workspace edits required to rename the symbol under `pos` in
/// `text` (URI `uri`) from its current name to `new_name`.
///
/// Returns `None` when the cursor is not on a renameable symbol.
///
/// Language-specific rules:
/// - **Turtle/TriG**: prefix labels are renameable. All occurrences of
///   `<label>:` in the document are replaced with `<new_name>:`.
/// - **SPARQL**: variable names (`?<name>` / `$<name>`) are renameable.
///   All occurrences in the document are replaced.
/// - Other languages: not supported (returns `None`).
#[must_use]
pub fn handle_rename(
    text: &str,
    lang: Language,
    uri: Url,
    pos: Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    match lang {
        Language::Turtle | Language::TriG => rename_turtle_prefix(text, uri, pos, new_name),
        Language::Sparql => rename_sparql_variable(text, uri, pos, new_name),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Turtle prefix rename
// ---------------------------------------------------------------------------

fn rename_turtle_prefix(
    text: &str,
    uri: Url,
    pos: Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    let token = prefix_label_at(text, pos)?;
    let edits = collect_prefix_renames(text, &token, new_name);
    if edits.is_empty() {
        return None;
    }
    let mut changes = HashMap::new();
    changes.insert(uri, edits);
    Some(WorkspaceEdit { changes: Some(changes), ..Default::default() })
}

/// Extract the prefix label under the cursor (e.g. `"ex"` from `"ex:Thing"`).
///
/// Returns `None` when the cursor is inside an IRI literal (`<...>`), inside a
/// string literal, or when no `:` immediately follows the label token.
fn prefix_label_at(text: &str, pos: Position) -> Option<String> {
    let line = text.lines().nth(pos.line as usize)?;
    let ch = pos.character as usize;
    let bytes = line.as_bytes();
    if ch >= bytes.len() { return None; }

    // Walk left to find start of label and check we are not inside an IRI.
    let mut start = ch;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_' || bytes[start - 1] == b'-') {
        start -= 1;
    }
    // If the character immediately before the label is '<', we are inside an IRI — not a prefix.
    if start > 0 && bytes[start - 1] == b'<' { return None; }

    // Walk right to find ':'
    let mut end = start;
    while end < bytes.len() && bytes[end] != b':' && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-') {
        end += 1;
    }
    if end >= bytes.len() || bytes[end] != b':' { return None; }

    let label = &line[start..end];
    if label.is_empty() { return None; }
    Some(label.to_string())
}

/// Collect `TextEdit`s replacing all occurrences of `<old>:` with `<new>:`.
fn collect_prefix_renames(text: &str, old: &str, new_name: &str) -> Vec<TextEdit> {
    let old_pattern = format!("{old}:");
    let new_pattern = format!("{new_name}:");
    let mut edits = Vec::new();

    for (line_no, line) in text.lines().enumerate() {
        let mut search_from = 0usize;
        while let Some(idx) = line[search_from..].find(&old_pattern) {
            let abs = search_from + idx;
            edits.push(TextEdit {
                range: lsp_types::Range {
                    start: Position { line: u32::try_from(line_no).unwrap_or(u32::MAX), character: u32::try_from(abs).unwrap_or(u32::MAX) },
                    end: Position { line: u32::try_from(line_no).unwrap_or(u32::MAX), character: u32::try_from(abs + old_pattern.len()).unwrap_or(u32::MAX) },
                },
                new_text: new_pattern.clone(),
            });
            search_from = abs + old_pattern.len();
        }
    }
    edits
}

// ---------------------------------------------------------------------------
// SPARQL variable rename
// ---------------------------------------------------------------------------

fn rename_sparql_variable(
    text: &str,
    uri: Url,
    pos: Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    let var_name = sparql_var_at(text, pos)?;
    let edits = collect_var_renames(text, &var_name, new_name);
    if edits.is_empty() { return None; }
    let mut changes = HashMap::new();
    changes.insert(uri, edits);
    Some(WorkspaceEdit { changes: Some(changes), ..Default::default() })
}

fn sparql_var_at(text: &str, pos: Position) -> Option<String> {
    let line = text.lines().nth(pos.line as usize)?;
    let ch = pos.character as usize;
    let bytes = line.as_bytes();
    if ch >= bytes.len() { return None; }

    // Cursor may be on the sigil (? or $) or on the name itself.
    let name_start = if bytes[ch] == b'?' || bytes[ch] == b'$' {
        ch + 1
    } else {
        // Walk left to find name start.
        let mut start = ch;
        while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
            start -= 1;
        }
        if start == 0 || (bytes[start - 1] != b'?' && bytes[start - 1] != b'$') {
            return None;
        }
        start
    };

    let mut end = name_start;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if end <= name_start { return None; }
    Some(line[name_start..end].to_string())
}

fn collect_var_renames(text: &str, old: &str, new_name: &str) -> Vec<TextEdit> {
    let mut edits = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            if (bytes[i] == b'?' || bytes[i] == b'$') && i + 1 < bytes.len() {
                let start = i;
                i += 1;
                let name_start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
                let name = &line[name_start..i];
                if name == old {
                    edits.push(TextEdit {
                        range: lsp_types::Range {
                            start: Position { line: u32::try_from(line_no).unwrap_or(u32::MAX), character: u32::try_from(start).unwrap_or(u32::MAX) },
                            end: Position { line: u32::try_from(line_no).unwrap_or(u32::MAX), character: u32::try_from(i).unwrap_or(u32::MAX) },
                        },
                        new_text: format!("{}{new_name}", line.as_bytes()[start] as char),
                    });
                }
            } else {
                i += 1;
            }
        }
    }
    edits
}

// ---------------------------------------------------------------------------
// Code actions
// ---------------------------------------------------------------------------

/// Compute code actions available at `pos` in `text`.
///
/// Phase G implements three actions for Turtle/TriG documents:
/// - **Extract prefix**: when the cursor is on a full IRI `<http://...>` that
///   has no prefix declaration, offer to add one.
/// - **Add missing prefix**: when a `curie:` is used but undeclared, offer to
///   add a `@prefix` declaration.  Uses the prefix.cc registry heuristic.
/// - **Sort prefixes**: sort all `@prefix` / `PREFIX` declarations
///   alphabetically.
#[must_use]
pub fn handle_code_actions(
    text: &str,
    lang: Language,
    _uri: &Url,
    pos: Position,
) -> Vec<CodeAction> {
    match lang {
        Language::Turtle | Language::TriG => turtle_code_actions(text, pos),
        Language::Sparql => sparql_code_actions(text, pos),
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Turtle code actions
// ---------------------------------------------------------------------------

fn turtle_code_actions(text: &str, pos: Position) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    // Sort prefixes action — always offer if any @prefix exists
    if text.contains("@prefix") || text.contains("PREFIX") {
        actions.push(sort_prefixes_action(text));
    }

    // Add-missing-prefix: detect cursored curie with no matching @prefix
    if let Some(action) = add_missing_prefix_action(text, pos) {
        actions.push(action);
    }

    // Extract-prefix: cursor on bare <IRI> that already has a common prefix
    if let Some(action) = extract_prefix_action(text, pos) {
        actions.push(action);
    }

    actions
}

fn sort_prefixes_action(text: &str) -> CodeAction {
    let sorted_text = sort_prefix_declarations(text);
    let edit = whole_doc_edit(text, &sorted_text);
    CodeAction {
        title: "Sort prefix declarations".to_string(),
        kind: Some(CodeActionKind::SOURCE),
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::new()), // populated by caller with URI
            ..Default::default()
        }),
        diagnostics: None,
        command: None,
        is_preferred: None,
        disabled: None,
        data: Some(serde_json::json!({ "sorted": sorted_text, "edit": edit })),
    }
}

fn add_missing_prefix_action(text: &str, pos: Position) -> Option<CodeAction> {
    let line = text.lines().nth(pos.line as usize)?;
    let ch = pos.character as usize;
    let bytes = line.as_bytes();
    if ch >= bytes.len() { return None; }

    // Find curie at cursor
    let mut start = ch;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_' || bytes[start - 1] == b'-') { start -= 1; }
    let mut end = ch;
    while end < bytes.len() && bytes[end] != b':' && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') { end += 1; }

    if end >= bytes.len() || bytes[end] != b':' { return None; }
    let prefix_label = &line[start..end];
    if prefix_label.is_empty() { return None; }

    // Check it's not already declared
    let decl_pattern = format!("@prefix {prefix_label}:");
    let decl_pattern2 = format!("PREFIX {prefix_label}:");
    if text.contains(&decl_pattern) || text.contains(&decl_pattern2) { return None; }

    // Known well-known prefixes
    let iri = well_known_prefix_iri(prefix_label)?;
    let new_decl = format!("@prefix {prefix_label}: <{iri}> .\n");

    Some(CodeAction {
        title: format!("Add missing prefix: {prefix_label}"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::new()),
            ..Default::default()
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: Some(serde_json::json!({ "insert_prefix": new_decl })),
    })
}

fn extract_prefix_action(text: &str, pos: Position) -> Option<CodeAction> {
    let line = text.lines().nth(pos.line as usize)?;
    let ch = pos.character as usize;
    let bytes = line.as_bytes();
    if ch >= bytes.len() || bytes[ch] != b'<' { return None; }

    // Find the IRI
    let start = ch + 1;
    let end = line[start..].find('>')? + start;
    let iri = &line[start..end];

    // Find a known namespace prefix for it
    let (label, ns) = KNOWN_NAMESPACES.iter().find(|(_, ns)| iri.starts_with(ns))?;

    let local = &iri[ns.len()..];
    if local.is_empty() { return None; }

    let replacement = format!("{label}:{local}");
    let decl = format!("@prefix {label}: <{ns}> .\n");

    Some(CodeAction {
        title: format!("Extract prefix: {label}"),
        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(HashMap::new()),
            ..Default::default()
        }),
        command: None,
        is_preferred: None,
        disabled: None,
        data: Some(serde_json::json!({ "decl": decl, "replacement": replacement, "original_iri": format!("<{}>", iri) })),
    })
}

// ---------------------------------------------------------------------------
// SPARQL code actions (Phase G: none — placeholder)
// ---------------------------------------------------------------------------

const fn sparql_code_actions(_text: &str, _pos: Position) -> Vec<CodeAction> {
    vec![]
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn sort_prefix_declarations(text: &str) -> String {
    let mut prefix_lines: Vec<&str> = Vec::new();
    let mut other_lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("@prefix") || t.starts_with("PREFIX") {
            prefix_lines.push(line);
        } else {
            other_lines.push(line);
        }
    }
    prefix_lines.sort_unstable();
    let mut out = prefix_lines.join("\n");
    if !other_lines.is_empty() {
        if !out.is_empty() { out.push('\n'); }
        out.push_str(&other_lines.join("\n"));
    }
    if text.ends_with('\n') { out.push('\n'); }
    out
}

fn whole_doc_edit(old: &str, new: &str) -> TextEdit {
    let line_count = u32::try_from(old.lines().count()).unwrap_or(u32::MAX);
    let last_col = old.lines().last().map_or(0, |l| u32::try_from(l.len()).unwrap_or(u32::MAX));
    TextEdit {
        range: lsp_types::Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: line_count, character: last_col },
        },
        new_text: new.to_string(),
    }
}

fn well_known_prefix_iri(prefix: &str) -> Option<&'static str> {
    KNOWN_NAMESPACES.iter().find(|(label, _)| *label == prefix).map(|(_, iri)| *iri)
}

const KNOWN_NAMESPACES: &[(&str, &str)] = &[
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("skos", "http://www.w3.org/2004/02/skos/core#"),
    ("sh", "http://www.w3.org/ns/shacl#"),
    ("dcterms", "http://purl.org/dc/terms/"),
    ("dc", "http://purl.org/dc/elements/1.1/"),
    ("foaf", "http://xmlns.com/foaf/0.1/"),
    ("schema", "https://schema.org/"),
    ("prov", "http://www.w3.org/ns/prov#"),
    ("dcat", "http://www.w3.org/ns/dcat#"),
    ("ex", "http://example.org/"),
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_uri() -> Url { Url::parse("file:///test.ttl").unwrap() }

    #[test]
    fn rename_turtle_prefix_replaces_all() {
        let text = "@prefix ex: <http://example.org/> .\nex:Foo a ex:Bar .\n";
        let uri = make_uri();
        let pos = Position { line: 1, character: 0 };
        let result = handle_rename(text, Language::Turtle, uri, pos, "example");
        assert!(result.is_some());
        let edits = result.unwrap().changes.unwrap();
        let edits = edits.values().next().unwrap();
        assert!(edits.iter().all(|e| e.new_text.starts_with("example:")));
    }

    #[test]
    fn rename_sparql_variable() {
        let text = "SELECT ?x WHERE { ?x ?p ?o }";
        let uri = Url::parse("file:///q.sparql").unwrap();
        let pos = Position { line: 0, character: 7 };
        let result = handle_rename(text, Language::Sparql, uri, pos, "subject");
        assert!(result.is_some());
    }

    #[test]
    fn rename_unknown_language_returns_none() {
        let result = handle_rename("", Language::RdfXml, make_uri(), Position { line: 0, character: 0 }, "foo");
        assert!(result.is_none());
    }

    #[test]
    fn code_actions_sort_prefixes_offered() {
        let text = "@prefix ex: <http://example.org/> .\n@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n";
        let uri = make_uri();
        let actions = handle_code_actions(text, Language::Turtle, &uri, Position { line: 0, character: 0 });
        assert!(actions.iter().any(|a| a.title.contains("Sort")));
    }

    #[test]
    fn code_actions_add_missing_prefix() {
        let text = "ex:Foo a rdfs:Class .\n";
        let uri = make_uri();
        let pos = Position { line: 0, character: 13 }; // on 'rdfs'
        let actions = handle_code_actions(text, Language::Turtle, &uri, pos);
        assert!(actions.iter().any(|a| a.title.contains("missing prefix")));
    }

    #[test]
    fn code_actions_extract_prefix() {
        let text = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> .\n";
        let uri = make_uri();
        let pos = Position { line: 0, character: 0 };
        let actions = handle_code_actions(text, Language::Turtle, &uri, pos);
        assert!(actions.iter().any(|a| a.title.contains("Extract prefix")));
    }

    #[test]
    fn sort_prefix_declarations_sorts_alphabetically() {
        let text = "@prefix z: <http://z.org/> .\n@prefix a: <http://a.org/> .\n";
        let sorted = sort_prefix_declarations(text);
        let lines: Vec<_> = sorted.lines().collect();
        assert!(lines[0].contains("a:"), "expected 'a' first");
        assert!(lines[1].contains("z:"), "expected 'z' second");
    }
}
