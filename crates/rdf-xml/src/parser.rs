//! Hand-rolled RDF/XML parser over `quick-xml` SAX events.
//!
//! Grammar: W3C RDF/XML Syntax Specification 2004-02-10.
//! ADR-0007: hand-roll over `quick-xml` events (streaming).

#![allow(
    clippy::too_many_lines,
    clippy::module_name_repetitions,
    clippy::option_if_let_else,
    clippy::similar_names,
)]

use std::collections::{BTreeMap, HashSet};

use quick_xml::{events::Event, Reader};
use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome};

// ── RDF namespace ───────────────────────────────────────────────────────────

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

fn rdf(s: &str) -> String {
    format!("{RDF}{s}")
}
fn iri(s: &str) -> String {
    format!("<{s}>")
}

// ── Error helper ─────────────────────────────────────────────────────────────

fn err(msg: impl Into<String>) -> Diagnostics {
    Diagnostics { messages: vec![msg.into()], fatal: true }
}

// ── IRI resolution (RFC 3986 simplified, sufficient for the test suite) ─────

pub fn resolve(base: &str, reference: &str) -> String {
    if reference.is_empty() {
        return base.to_owned();
    }
    // Already absolute?
    if let Some(colon) = reference.find(':') {
        let pre_slash = reference.find('/').unwrap_or(reference.len());
        if colon < pre_slash {
            return reference.to_owned();
        }
    }
    if reference.starts_with('#') {
        let b = base.find('#').map_or(base, |i| &base[..i]);
        return format!("{b}{reference}");
    }
    if reference.starts_with("//") {
        let scheme_end = base.find(':').map_or(0, |i| i + 1);
        return format!("{}{}", &base[..scheme_end], reference);
    }
    if reference.starts_with('/') {
        let auth = if let Some(sc) = base.find("://") {
            let after = sc + 3;
            base[after..].find('/').map_or(base.len(), |i| after + i)
        } else {
            0
        };
        return format!("{}{}", &base[..auth], reference);
    }
    // Relative path.
    let b = base.find('#').map_or(base, |i| &base[..i]);
    let dir = b.rfind('/').map_or("", |i| &b[..=i]);
    let mut parts: Vec<&str> = dir.split('/').collect();
    for seg in reference.split('/') {
        match seg {
            ".." => { parts.pop(); }
            "." => {}
            s => parts.push(s),
        }
    }
    parts.join("/")
}

// ── Namespace stack ──────────────────────────────────────────────────────────

#[derive(Default, Clone)]
struct NsFrame {
    bindings: Vec<(String, String)>, // (prefix, ns_uri)
    base: Option<String>,
    lang: Option<String>, // None = unchanged; Some("") = cleared; Some(tag) = set
}

struct NsStack {
    frames: Vec<NsFrame>,
    /// The current resolved base IRI.
    base: String,
    /// The document-level base IRI (fallback when all frames are popped).
    doc_base: String,
    lang: Option<String>,
}

impl NsStack {
    fn new(base: &str) -> Self {
        Self {
            frames: vec![NsFrame {
                bindings: vec![
                    ("rdf".into(), RDF.into()),
                    ("xml".into(), XML_NS.into()),
                ],
                base: None,
                lang: None,
            }],
            base: base.to_owned(),
            doc_base: base.to_owned(),
            lang: None,
        }
    }

    fn resolve_prefix(&self, prefix: &str) -> Option<&str> {
        for frame in self.frames.iter().rev() {
            if let Some((_, ns)) = frame.bindings.iter().find(|(p, _)| p == prefix) {
                return Some(ns.as_str());
            }
        }
        None
    }

    fn push(&mut self, frame: NsFrame) {
        if let Some(ref b) = frame.base {
            self.base = resolve(&self.base, b);
        }
        if let Some(ref l) = frame.lang {
            if l.is_empty() {
                self.lang = None;
            } else {
                self.lang = Some(l.clone());
            }
        }
        self.frames.push(frame);
    }

    fn pop(&mut self) {
        self.frames.pop();
        // Recompute base and lang from scratch, starting from doc_base.
        let mut base = self.doc_base.clone();
        let mut lang: Option<String> = None;
        for frame in &self.frames {
            if let Some(ref b) = frame.base {
                base = resolve(&base, b);
            }
            if let Some(ref l) = frame.lang {
                if l.is_empty() {
                    lang = None;
                } else {
                    lang = Some(l.clone());
                }
            }
        }
        self.base = base;
        self.lang = lang;
    }

    fn split_name(raw: &str) -> (Option<&str>, &str) {
        if let Some(c) = raw.find(':') {
            (Some(&raw[..c]), &raw[c + 1..])
        } else {
            (None, raw)
        }
    }

    /// Expand a full element name (with default namespace support).
    fn expand_element(&self, raw: &str) -> Option<(String, String)> {
        let (prefix, local) = Self::split_name(raw);
        let ns = match prefix {
            Some(p) => self.resolve_prefix(p)?.to_owned(),
            None => self.resolve_prefix("")?.to_owned(),
        };
        Some((ns, local.to_owned()))
    }

    /// Expand an attribute name. Attributes without prefix have NO namespace.
    fn expand_attr(&self, raw: &str) -> (String, String) {
        if raw == "xmlns" {
            return ("__xmlns__".into(), String::new());
        }
        if let Some(rest) = raw.strip_prefix("xmlns:") {
            return ("__xmlns__".into(), rest.to_owned());
        }
        let (prefix, local) = Self::split_name(raw);
        match prefix {
            Some("xml") => (XML_NS.into(), local.to_owned()),
            Some(p) => {
                let ns = self.resolve_prefix(p).unwrap_or("").to_owned();
                (ns, local.to_owned())
            }
            None => (String::new(), raw.to_owned()),
        }
    }
}

// ── Parsed attribute bag ─────────────────────────────────────────────────────

struct AttrBag {
    attrs: Vec<(String, String, String)>, // (ns, local, value)
}

impl AttrBag {
    fn get(&self, ns: &str, local: &str) -> Option<&str> {
        self.attrs.iter().find(|(n, l, _)| n == ns && l == local).map(|t| t.2.as_str())
    }
    fn get_rdf(&self, local: &str) -> Option<&str> {
        self.get(RDF, local)
    }
}

// ── Parser state ─────────────────────────────────────────────────────────────

struct State {
    counter: usize,
    facts: Vec<(Fact, FactProvenance)>,
    prefixes: BTreeMap<String, String>,
    seen_ids: HashSet<String>,
}

impl State {
    fn new() -> Self {
        Self {
            counter: 0,
            facts: Vec::new(),
            prefixes: BTreeMap::new(),
            seen_ids: HashSet::new(),
        }
    }

    fn bnode(&mut self) -> String {
        let n = self.counter;
        self.counter += 1;
        format!("_:b{n}")
    }

    fn emit(&mut self, s: &str, p: &str, o: &str) {
        self.facts.push((
            Fact { subject: s.into(), predicate: p.into(), object: o.into(), graph: None },
            FactProvenance { offset: None, parser: "rdf-xml".into() },
        ));
    }

    fn reify(&mut self, id_iri: &str, s: &str, p: &str, o: &str) {
        let stmt = iri(id_iri);
        self.emit(&stmt, &iri(&rdf("type")), &iri(&rdf("Statement")));
        self.emit(&stmt, &iri(&rdf("subject")), s);
        self.emit(&stmt, &iri(&rdf("predicate")), p);
        self.emit(&stmt, &iri(&rdf("object")), o);
    }
}

// ── Event-parsing helpers ─────────────────────────────────────────────────────

/// Collect xmlns decls + xml:base + xml:lang from element attributes; return
/// an `NsFrame` and the parsed attribute bag (without xmlns attrs).
fn collect_frame(
    bs: &quick_xml::events::BytesStart,
    ns: &NsStack,
) -> Result<(NsFrame, AttrBag), Diagnostics> {
    let mut frame = NsFrame::default();
    let mut attrs = Vec::new();

    for attr in bs.attributes() {
        let attr = attr.map_err(|e| err(format!("XML attr error: {e}")))?;
        let raw_key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|_| err("non-UTF8 attribute key"))?;
        let value = attr
            .unescape_value()
            .map_err(|e| err(format!("attr value error: {e}")))?
            .into_owned();

        if raw_key == "xmlns" {
            frame.bindings.push((String::new(), value.clone()));
            continue;
        }
        if let Some(rest) = raw_key.strip_prefix("xmlns:") {
            frame.bindings.push((rest.to_owned(), value.clone()));
            continue;
        }
        let (a_ns, a_local) = ns.expand_attr(raw_key);
        if a_ns == XML_NS && a_local == "base" {
            frame.base = Some(value.clone());
        }
        if a_ns == XML_NS && a_local == "lang" {
            frame.lang = Some(value.clone());
        }
        attrs.push((a_ns, a_local, value));
    }

    Ok((frame, AttrBag { attrs }))
}

fn element_name(bs: &quick_xml::events::BytesStart) -> Result<String, Diagnostics> {
    std::str::from_utf8(bs.name().as_ref())
        .map(std::borrow::ToOwned::to_owned)
        .map_err(|_| err("non-UTF8 element name"))
}


// ── Validate rdf:ID (must be an XML Name) ────────────────────────────────────

/// Validate that `s` is an XML `NCName` (non-colonized name).
/// Per XML Namespaces §4: `NCName` ::= (Letter | '_') `NCNameChar`*
/// `NCNameChar` ::= Letter | Digit | '.' | '-' | '_' | `CombiningChar` | Extender
/// Colon is explicitly forbidden.
///
/// We use the XML 1.0 fifth edition `NameStartChar` / `NameChar` productions
/// (minus ':') as a practical approximation.
fn valid_xml_name(s: &str) -> bool {
    /// `NameStartChar` (no colon): underscore, ASCII letter, or specific Unicode ranges
    /// that are valid name-start characters per XML 1.0 5e spec §2.3.
    const fn nc_name_start(c: char) -> bool {
        matches!(c,
            'A'..='Z' | 'a'..='z' | '_'
            | '\u{00C0}'..='\u{00D6}'
            | '\u{00D8}'..='\u{00F6}'
            | '\u{00F8}'..='\u{02FF}'
            | '\u{0370}'..='\u{037D}'
            | '\u{037F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}'
            | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}'
            | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFFD}'
            | '\u{10000}'..='\u{EFFFF}'
        )
    }
    /// `NameChar` (no colon): start chars plus digits, hyphen, period, middle dot,
    /// combining chars, extenders per XML 1.0 5e spec §2.3.
    const fn nc_name_char(c: char) -> bool {
        nc_name_start(c)
            || matches!(c,
                '0'..='9' | '-' | '.'
                | '\u{00B7}'
                | '\u{0300}'..='\u{036F}'
                | '\u{203F}'..='\u{2040}'
            )
    }
    let mut chars = s.chars();
    match chars.next() {
        None => false,
        Some(c) if !nc_name_start(c) => false,
        _ => chars.all(nc_name_char),
    }
}

// ── Forbidden element names ───────────────────────────────────────────────────

fn forbidden_node_elt(ns: &str, local: &str) -> bool {
    ns == RDF && matches!(local,
        "RDF"|"ID"|"about"|"bagID"|"parseType"|"resource"|"nodeID"|
        "li"|"aboutEach"|"aboutEachPrefix"
    )
}

fn forbidden_prop_elt(ns: &str, local: &str) -> bool {
    ns == RDF
        && matches!(
            local,
            "RDF" | "Description" | "aboutEach" | "aboutEachPrefix"
            | "ID" | "about" | "bagID" | "parseType" | "resource" | "nodeID"
        )
}

fn forbidden_prop_attr(ns: &str, local: &str) -> bool {
    ns == RDF && matches!(local, "li"|"aboutEach"|"aboutEachPrefix")
}

// ── Core parser ───────────────────────────────────────────────────────────────

pub fn parse(input: &[u8], doc_base: &str) -> Result<ParseOutcome, Diagnostics> {
    let text = std::str::from_utf8(input).map_err(|_| err("input is not valid UTF-8"))?;
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);
    // Expand <foo/> to <foo></foo> so all elements come through as Start+End.
    reader.config_mut().expand_empty_elements = true;

    let mut state = State::new();
    let mut ns = NsStack::new(doc_base);

    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text error: {e}")))?;
                if !s.trim().is_empty() {
                    return Err(err("unexpected text before root element"));
                }
            }
            Event::Start(ref bs) | Event::Empty(ref bs) => {
                let raw = element_name(bs)?;
                let (frame, attrs) = collect_frame(bs, &ns)?;
                ns.push(frame);
                let base = ns.base.clone();

                let (elt_ns, elt_local) = ns.expand_element(&raw)
                    .ok_or_else(|| err(format!("unresolved element prefix in '{raw}'")))?;

                if elt_ns == RDF && elt_local == "RDF" {
                    // rdf:RDF wrapper — parse its children as node elements.
                    parse_node_elts(&mut reader, &mut state, &mut ns, doc_base)?;
                } else {
                    // Bare node element at document root.
                    parse_node_elt(
                        &mut reader, &mut state, &mut ns,
                        &elt_ns, &elt_local, &attrs, doc_base, &base,
                    )?;
                }
                ns.pop();
                // After root, consume remaining events to EOF.
                drain_to_eof(&mut reader)?;
                break;
            }
            Event::Eof => break,
            _ => {}
        }
    }

    let prefixes = std::mem::take(&mut state.prefixes);
    let facts = std::mem::take(&mut state.facts);
    Ok(ParseOutcome {
        facts: Facts::canonicalise(facts, prefixes),
        warnings: Diagnostics { messages: vec![], fatal: false },
    })
}

fn drain_to_eof(reader: &mut Reader<&[u8]>) -> Result<(), Diagnostics> {
    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::Eof => break,
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text error: {e}")))?;
                if !s.trim().is_empty() {
                    return Err(err("trailing text after root element"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// ── Node element list ─────────────────────────────────────────────────────────

fn parse_node_elts(
    reader: &mut Reader<&[u8]>,
    state: &mut State,
    ns: &mut NsStack,
    doc_base: &str,
) -> Result<(), Diagnostics> {
    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::Start(ref bs) | Event::Empty(ref bs) => {
                let raw = element_name(bs)?;
                let (frame, attrs) = collect_frame(bs, ns)?;
                ns.push(frame);
                let base = ns.base.clone();

                let (elt_ns, elt_local) = ns.expand_element(&raw)
                    .ok_or_else(|| err(format!("unresolved element prefix in '{raw}'")))?;

                parse_node_elt(reader, state, ns, &elt_ns, &elt_local, &attrs, doc_base, &base)?;
                ns.pop();
            }
            Event::End(_) | Event::Eof => break,
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text error: {e}")))?;
                if !s.trim().is_empty() {
                    return Err(err("unexpected text in node-element context"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// ── Node element ─────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn parse_node_elt(
    reader: &mut Reader<&[u8]>,
    state: &mut State,
    ns: &mut NsStack,
    elt_ns: &str,
    elt_local: &str,
    attrs: &AttrBag,
    doc_base: &str,
    base: &str,
) -> Result<String, Diagnostics> {
    if forbidden_node_elt(elt_ns, elt_local) {
        return Err(err(format!(
            "RDFXML-NODE-001: forbidden rdf:{elt_local} as node element"
        )));
    }

    // Check for mutually exclusive subject attributes.
    let has_about = attrs.get_rdf("about").is_some();
    let has_id = attrs.get_rdf("ID").is_some();
    let has_nid = attrs.get_rdf("nodeID").is_some();
    if has_nid && has_id {
        return Err(err("RDFXML-NID-003: rdf:nodeID and rdf:ID are mutually exclusive"));
    }
    if has_nid && has_about {
        return Err(err("RDFXML-NID-002: rdf:nodeID and rdf:about are mutually exclusive"));
    }

    // rdf:bagID is forbidden (deprecated in RDF 1.1).
    if attrs.get_rdf("bagID").is_some() {
        return Err(err("RDFXML-BAGID-001: rdf:bagID is deprecated and not allowed"));
    }

    // rdf:aboutEach / rdf:aboutEachPrefix are forbidden.
    if attrs.get_rdf("aboutEach").is_some() || attrs.get_rdf("aboutEachPrefix").is_some() {
        return Err(err("RDFXML-ABOUTEACH-001: rdf:aboutEach and rdf:aboutEachPrefix are removed"));
    }

    // Determine the subject term.
    let subject: String;
    if let Some(about) = attrs.get_rdf("about") {
        subject = iri(&resolve(base, about));
    } else if let Some(id) = attrs.get_rdf("ID") {
        if !valid_xml_name(id) {
            return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} is not a valid XML Name")));
        }
        let id_iri = make_id_iri(base, id);
        if !state.seen_ids.insert(id_iri.clone()) {
            return Err(err(format!("RDFXML-ID-002: duplicate rdf:ID {id:?}")));
        }
        subject = iri(&id_iri);
    } else if let Some(nid) = attrs.get_rdf("nodeID") {
        if !valid_xml_name(nid) {
            return Err(err(format!("RDFXML-NID-001: rdf:nodeID {nid:?} is not a valid XML Name")));
        }
        // Cannot have rdf:nodeID together with rdf:about or rdf:ID.
        if attrs.get_rdf("about").is_some() {
            return Err(err("RDFXML-NID-002: rdf:nodeID and rdf:about are mutually exclusive"));
        }
        subject = format!("_:{nid}");
    } else {
        subject = state.bnode();
    }

    // Typed node: emit rdf:type unless it's rdf:Description.
    if !(elt_ns == RDF && elt_local == "Description") {
        let type_iri = format!("{elt_ns}{elt_local}");
        state.emit(&subject, &iri(&rdf("type")), &iri(&type_iri));
    }

    // Property attributes on the node element.
    let lang = ns.lang.clone();
    for (a_ns, a_local, a_val) in &attrs.attrs {
        if a_ns.is_empty() || a_ns == "__xmlns__" {
            continue;
        }
        if a_ns == XML_NS {
            continue;
        }
        if a_ns == RDF && matches!(a_local.as_str(), "about"|"ID"|"nodeID"|"type") {
            // rdf:type as attribute is handled specially.
            if a_ns == RDF && a_local == "type" {
                state.emit(&subject, &iri(&rdf("type")), &iri(a_val));
            }
            continue;
        }
        if forbidden_prop_attr(a_ns, a_local) {
            return Err(err(format!("RDFXML-PATTR-001: forbidden rdf:{a_local} as property attribute")));
        }
        if a_ns == RDF && matches!(a_local.as_str(), "RDF"|"Description"|"datatype"|"parseType"|"resource"|"ID"|"nodeID"|"about"|"bagID"|"aboutEach"|"aboutEachPrefix") {
            // Structural attributes handled elsewhere.
            continue;
        }
        let pred = iri(&format!("{a_ns}{a_local}"));
        let obj = literal_with_lang(a_val, lang.as_deref());
        state.emit(&subject, &pred, &obj);
    }

    // Property elements as children.
    parse_prop_elts(reader, state, ns, &subject, doc_base)?;

    Ok(subject)
}

// ── Property element list ─────────────────────────────────────────────────────

fn parse_prop_elts(
    reader: &mut Reader<&[u8]>,
    state: &mut State,
    ns: &mut NsStack,
    subject: &str,
    doc_base: &str,
) -> Result<(), Diagnostics> {
    let mut li_counter: usize = 0;
    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::Start(ref bs) | Event::Empty(ref bs) => {
                let raw = element_name(bs)?;
                let (frame, attrs) = collect_frame(bs, ns)?;
                ns.push(frame);
                let inner_base = ns.base.clone();

                let (elt_ns, elt_local) = ns.expand_element(&raw)
                    .ok_or_else(|| err(format!("unresolved prefix in '{raw}'")))?;

                if forbidden_prop_elt(&elt_ns, &elt_local) {
                    ns.pop();
                    return Err(err(format!(
                        "RDFXML-PELT-001: forbidden rdf:{elt_local} as property element"
                    )));
                }

                // rdf:li → rdf:_N
                let (pred_ns, pred_local) = if elt_ns == RDF && elt_local == "li" {
                    li_counter += 1;
                    (RDF.to_owned(), format!("_{li_counter}"))
                } else {
                    (elt_ns.clone(), elt_local.clone())
                };
                let pred_iri = format!("{pred_ns}{pred_local}");

                parse_prop_elt(
                    reader, state, ns,
                    subject, &elt_ns, &elt_local,
                    &pred_iri, &attrs, doc_base, &inner_base,
                )?;
                ns.pop();
            }
            Event::End(_) | Event::Eof => break,
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text error: {e}")))?;
                if !s.trim().is_empty() {
                    return Err(err("unexpected text in property-element context"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

// ── Property element ─────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn parse_prop_elt(
    reader: &mut Reader<&[u8]>,
    state: &mut State,
    ns: &mut NsStack,
    subject: &str,
    _elt_ns: &str,
    _elt_local: &str,
    pred_iri: &str,
    attrs: &AttrBag,
    doc_base: &str,
    base: &str,
) -> Result<(), Diagnostics> {
    let predicate = iri(pred_iri);
    let lang = ns.lang.clone();

    // rdf:bagID is forbidden on property elements (deprecated in RDF 1.1).
    if attrs.get_rdf("bagID").is_some() {
        return Err(err("RDFXML-BAGID-002: rdf:bagID is deprecated and not allowed on property elements"));
    }

    // Determine parse type.
    let parse_type = attrs.get_rdf("parseType").map(std::borrow::ToOwned::to_owned);

    // Validate conflicting attributes.
    let has_res = attrs.get_rdf("resource").is_some();
    let has_nid_attr = attrs.get_rdf("nodeID").is_some();
    if has_res && has_nid_attr {
        return Err(err("RDFXML-PROP-001: rdf:resource and rdf:nodeID are mutually exclusive"));
    }
    if has_res && parse_type.as_deref() == Some("Literal") {
        return Err(err("RDFXML-PROP-002: rdf:parseType=\"Literal\" and rdf:resource are mutually exclusive"));
    }

    if let Some(ref pt) = parse_type {
        match pt.as_str() {
            "Resource" => {
                // Create a fresh blank node as object, then parse children as property elts.
                let obj_node: String = if let Some(nid) = attrs.get_rdf("nodeID") {
                    format!("_:{nid}")
                } else {
                    state.bnode()
                };
                let rdf_id = attrs.get_rdf("ID");
                state.emit(subject, &predicate, &obj_node);
                if let Some(id) = rdf_id {
                    if !valid_xml_name(id) {
                        return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} is not a valid XML Name")));
                    }
                    let id_iri = make_id_iri(base, id);
                    if !state.seen_ids.insert(id_iri.clone()) {
                        return Err(err(format!("RDFXML-ID-002: duplicate rdf:ID {id:?}")));
                    }
                    state.reify(&id_iri, subject, &predicate, &obj_node);
                }
                parse_prop_elts(reader, state, ns, &obj_node, doc_base)?;
                return Ok(());
            }
            "Literal" => {
                // Collect raw XML as a string; ignore sub-elements.
                let xml_lit = collect_xml_literal(reader)?;
                let obj = format!("{xml_lit:?}^^{}", iri(&rdf("XMLLiteral")));
                let rdf_id = attrs.get_rdf("ID");
                state.emit(subject, &predicate, &obj);
                if let Some(id) = rdf_id {
                    if !valid_xml_name(id) {
                        return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} is not valid")));
                    }
                    let id_iri = make_id_iri(base, id);
                    state.reify(&id_iri, subject, &predicate, &obj);
                }
                return Ok(());
            }
            "Collection" => {
                return parse_collection_prop(
                    reader, state, ns, subject, &predicate, attrs, doc_base, base,
                );
            }
            other => {
                // Unknown parseType — treat as Literal per spec §7.2.18.
                let xml_lit = collect_xml_literal(reader)?;
                let obj = format!("{xml_lit:?}^^{}", iri(&rdf("XMLLiteral")));
                state.emit(subject, &predicate, &obj);
                let _ = other;
                return Ok(());
            }
        }
    }

    // No parseType — check for rdf:resource or rdf:nodeID first.
    if let Some(res) = attrs.get_rdf("resource") {
        let obj_iri = resolve(base, res);
        let obj = iri(&obj_iri);
        let rdf_id = attrs.get_rdf("ID");
        // Additional property attrs on the resource object.
        let lang2 = ns.lang.clone();
        emit_prop_attrs_on_resource(state, &obj_iri, &obj, attrs, lang2.as_deref());
        state.emit(subject, &predicate, &obj);
        if let Some(id) = rdf_id {
            if !valid_xml_name(id) {
                return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} not valid XML Name")));
            }
            let id_iri = make_id_iri(base, id);
            if !state.seen_ids.insert(id_iri.clone()) {
                return Err(err(format!("RDFXML-ID-002: duplicate rdf:ID {id:?}")));
            }
            state.reify(&id_iri, subject, &predicate, &obj);
        }
        // Must be empty (no text, no child elements) — consume the End event
        // (expand_empty_elements ensures this is always an End).
        consume_end(reader)?;
        return Ok(());
    }

    if let Some(nid) = attrs.get_rdf("nodeID") {
        if !valid_xml_name(nid) {
            return Err(err(format!("RDFXML-NID-001: rdf:nodeID {nid:?} is not a valid XML Name")));
        }
        let obj = format!("_:{nid}");
        let rdf_id = attrs.get_rdf("ID");
        state.emit(subject, &predicate, &obj);
        if let Some(id) = rdf_id {
            if !valid_xml_name(id) {
                return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} not valid XML Name")));
            }
            let id_iri = make_id_iri(base, id);
            state.reify(&id_iri, subject, &predicate, &obj);
        }
        consume_end(reader)?;
        return Ok(());
    }

    // Peek at children: either text/typed-literal OR nested node element.
    parse_prop_elt_body(reader, state, ns, subject, &predicate, attrs, doc_base, base, lang.as_deref())
}

/// Emit property attributes that belong to an IRI/bNode resource object.
fn emit_prop_attrs_on_resource(
    state: &mut State,
    obj_iri: &str,
    obj_term: &str,
    attrs: &AttrBag,
    lang: Option<&str>,
) {
    let _ = obj_term; // unused — we need the IRI form
    for (a_ns, a_local, a_val) in &attrs.attrs {
        if a_ns.is_empty() || a_ns == "__xmlns__" || a_ns == XML_NS {
            continue;
        }
        if a_ns == RDF && matches!(a_local.as_str(), "resource"|"ID"|"nodeID"|"parseType"|"datatype"|"type"|"RDF"|"Description"|"bagID"|"li"|"aboutEach"|"aboutEachPrefix") {
            if a_ns == RDF && a_local == "type" {
                state.emit(&iri(obj_iri), &iri(&rdf("type")), &iri(a_val));
            }
            continue;
        }
        let pred = iri(&format!("{a_ns}{a_local}"));
        let obj = literal_with_lang(a_val, lang);
        state.emit(&iri(obj_iri), &pred, &obj);
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_prop_elt_body(
    reader: &mut Reader<&[u8]>,
    state: &mut State,
    ns: &mut NsStack,
    subject: &str,
    predicate: &str,
    attrs: &AttrBag,
    doc_base: &str,
    base: &str,
    lang: Option<&str>,
) -> Result<(), Diagnostics> {
    let rdf_id = attrs.get_rdf("ID");
    let rdf_dt = attrs.get_rdf("datatype");

    // Also check for property attributes (when body turns out to be empty string).
    // We collect text and look for child elements.
    let mut text_buf = String::new();
    let mut has_child_element = false;
    let mut child_subject: Option<String> = None;

    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text decode: {e}")))?;
                text_buf.push_str(&s);
            }
            Event::Start(ref bs) | Event::Empty(ref bs) => {
                has_child_element = true;
                let raw = element_name(bs)?;
                let (frame, child_attrs) = collect_frame(bs, ns)?;
                ns.push(frame);
                let inner_base = ns.base.clone();
                let (cn, cl) = ns.expand_element(&raw)
                    .ok_or_else(|| err(format!("unresolved prefix in '{raw}'")))?;
                let subj = parse_node_elt(reader, state, ns, &cn, &cl, &child_attrs, doc_base, &inner_base)?;
                ns.pop();
                child_subject = Some(subj);
            }
            Event::End(_) | Event::Eof => break,
            _ => {}
        }
    }

    let obj: String = if has_child_element {
        // Object is the child node element's subject.
        child_subject.unwrap_or_else(|| state.bnode())
    } else {
        // Plain or typed literal.
        if let Some(dt) = rdf_dt {
            // datatype attribute — ignore lang per spec.
            let dt_iri = resolve(base, dt);
            format!("{:?}^^{}", text_buf, iri(&dt_iri))
        } else {
            // Check for additional prop attrs on the subject being described.
            // (For empty property elements with no rdf:resource — the text is the literal value.)
            if has_prop_attrs(attrs) {
                // Value attrs — complex case: create a bnode, emit prop attrs on it, text literal ignored.
                let bnode = state.bnode();
                let lang2 = ns.lang.clone();
                emit_prop_attrs_on_resource(state, &bnode, &bnode, attrs, lang2.as_deref());
                state.emit(subject, predicate, &bnode);
                if let Some(id) = rdf_id {
                    if !valid_xml_name(id) {
                        return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} not valid")));
                    }
                    let id_iri = make_id_iri(base, id);
                    if !state.seen_ids.insert(id_iri.clone()) {
                        return Err(err("RDFXML-ID-002: duplicate rdf:ID".to_string()));
                    }
                    state.reify(&id_iri, subject, predicate, &bnode);
                }
                return Ok(());
            }
            literal_with_lang(&text_buf, lang)
        }
    };
    state.emit(subject, predicate, &obj);

    if let Some(id) = rdf_id {
        if !valid_xml_name(id) {
            return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} not valid")));
        }
        let id_iri = make_id_iri(base, id);
        if !state.seen_ids.insert(id_iri.clone()) {
            return Err(err("RDFXML-ID-002: duplicate rdf:ID".to_string()));
        }
        state.reify(&id_iri, subject, predicate, &obj);
    }

    Ok(())
}

fn has_prop_attrs(attrs: &AttrBag) -> bool {
    attrs.attrs.iter().any(|(a_ns, a_local, _)| {
        if a_ns.is_empty() || a_ns == "__xmlns__" || a_ns == XML_NS {
            return false;
        }
        if a_ns == RDF && matches!(a_local.as_str(), "about"|"ID"|"nodeID"|"parseType"|"datatype"|"resource"|"RDF"|"Description"|"bagID"|"li"|"aboutEach"|"aboutEachPrefix") {
            return false;
        }
        true
    })
}

// ── Collection (rdf:parseType="Collection") ───────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn parse_collection_prop(
    reader: &mut Reader<&[u8]>,
    state: &mut State,
    ns: &mut NsStack,
    subject: &str,
    predicate: &str,
    attrs: &AttrBag,
    doc_base: &str,
    base: &str,
) -> Result<(), Diagnostics> {
    // Collect all child node elements.
    let mut items: Vec<String> = Vec::new();
    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::Start(ref bs) | Event::Empty(ref bs) => {
                let raw = element_name(bs)?;
                let (frame, child_attrs) = collect_frame(bs, ns)?;
                ns.push(frame);
                let inner_base = ns.base.clone();
                let (cn, cl) = ns.expand_element(&raw)
                    .ok_or_else(|| err(format!("unresolved prefix in '{raw}'")))?;
                let item_subj = parse_node_elt(reader, state, ns, &cn, &cl, &child_attrs, doc_base, &inner_base)?;
                ns.pop();
                items.push(item_subj);
            }
            Event::End(_) | Event::Eof => break,
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text error: {e}")))?;
                if !s.trim().is_empty() {
                    return Err(err("unexpected text in collection"));
                }
            }
            _ => {}
        }
    }

    let nil = iri(&rdf("nil"));
    let first_iri = iri(&rdf("first"));
    let rest_iri = iri(&rdf("rest"));

    if items.is_empty() {
        state.emit(subject, predicate, &nil);
    } else {
        // Build linked list.
        let nodes: Vec<String> = (0..items.len()).map(|_| state.bnode()).collect();
        state.emit(subject, predicate, &nodes[0]);
        for (i, item) in items.iter().enumerate() {
            state.emit(&nodes[i], &first_iri, item);
            let rest = if i + 1 < items.len() { nodes[i + 1].clone() } else { nil.clone() };
            state.emit(&nodes[i], &rest_iri, &rest);
        }
    }

    // Handle rdf:ID on collection property.
    if let Some(id) = attrs.get_rdf("ID") {
        if !valid_xml_name(id) {
            return Err(err(format!("RDFXML-ID-001: rdf:ID {id:?} not valid")));
        }
        let id_iri = make_id_iri(base, id);
        // Actually reify the triple that was emitted: subject predicate nodes[0].
        // We need to go back. Since we already emitted, state.facts.last() should be the rest triple.
        // A cleaner approach: find the first node from facts.
        // For now, extract from state.facts.
        if let Some(head) = find_collection_head(&state.facts, subject, predicate) {
            if !state.seen_ids.insert(id_iri.clone()) {
                return Err(err("RDFXML-ID-002: duplicate rdf:ID".to_string()));
            }
            state.reify(&id_iri, subject, predicate, &head);
        }
    }

    Ok(())
}

fn find_collection_head(
    facts: &[(Fact, FactProvenance)],
    subject: &str,
    predicate: &str,
) -> Option<String> {
    facts.iter().rev().find(|(f, _)| f.subject == subject && f.predicate == predicate)
        .map(|(f, _)| f.object.clone())
}

// ── Collect XML literal (skip sub-elements) ───────────────────────────────────

fn collect_xml_literal(reader: &mut Reader<&[u8]>) -> Result<String, Diagnostics> {
    let mut depth = 0usize;
    let mut buf = String::new();
    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::Start(ref bs) => {
                depth += 1;
                let raw = std::str::from_utf8(bs.name().as_ref()).unwrap_or("").to_owned();
                buf.push('<');
                buf.push_str(&raw);
                buf.push('>');
            }
            Event::Empty(ref bs) => {
                let raw = std::str::from_utf8(bs.name().as_ref()).unwrap_or("").to_owned();
                buf.push('<');
                buf.push_str(&raw);
                buf.push_str("/>");
            }
            Event::End(_) => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text error: {e}")))?;
                buf.push_str(&s);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(buf)
}

// ── Consume End event ─────────────────────────────────────────────────────────

fn consume_end(reader: &mut Reader<&[u8]>) -> Result<(), Diagnostics> {
    loop {
        match reader.read_event().map_err(|e| err(format!("XML error: {e}")))? {
            Event::End(_) | Event::Eof => return Ok(()),
            Event::Text(t) => {
                let s = t.unescape().map_err(|e| err(format!("text error: {e}")))?;
                if !s.trim().is_empty() {
                    return Err(err("unexpected text in empty property element"));
                }
            }
            _ => {}
        }
    }
}

// ── Literal construction ──────────────────────────────────────────────────────

fn literal_with_lang(lex: &str, lang: Option<&str>) -> String {
    let escaped = escape_literal(lex);
    match lang {
        Some(l) if !l.is_empty() => format!("\"{escaped}\"@{l}"),
        _ => format!("\"{escaped}\""),
    }
}

fn escape_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out
}

// ── rdf:ID IRI construction ───────────────────────────────────────────────────

fn make_id_iri(doc_base: &str, id: &str) -> String {
    let base_no_frag = doc_base.find('#').map_or(doc_base, |i| &doc_base[..i]);
    format!("{base_no_frag}#{id}")
}
