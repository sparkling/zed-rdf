//! RDF/XML shadow parser — derived from the W3C RDF/XML Syntax Specification
//! <https://www.w3.org/TR/rdf-syntax-grammar/>.
//!
//! This module drives a `quick-xml` SAX-style event stream and applies
//! the grammar productions defined in the spec to produce a set of RDF triples
//! in the canonical form required by `rdf_diff`.

#![allow(
    clippy::option_if_let_else,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::type_complexity,
)]

use std::collections::{BTreeMap, HashMap};

use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Reader;
use rdf_diff::{Diagnostics, Fact, FactProvenance, Facts, ParseOutcome};

// ── Well-known namespace URIs ─────────────────────────────────────────────────

const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

fn rdf(local: &str) -> String {
    format!("{RDF}{local}")
}

// ── Public entry-point ────────────────────────────────────────────────────────

/// Parse `input` bytes as RDF/XML and return a canonical fact set.
pub fn parse(input: &[u8]) -> Result<ParseOutcome, Diagnostics> {
    let mut ctx = ParseContext::new();
    match ctx.run(input) {
        Ok(()) => {
            let raw: Vec<(Fact, FactProvenance)> = ctx
                .triples
                .into_iter()
                .map(|t| {
                    (
                        t,
                        FactProvenance {
                            offset: None,
                            parser: "rdf-xml-shadow".into(),
                        },
                    )
                })
                .collect();
            Ok(ParseOutcome {
                facts: Facts::canonicalise(raw, BTreeMap::new()),
                warnings: Diagnostics {
                    messages: vec![],
                    fatal: false,
                },
            })
        }
        Err(msg) => Err(Diagnostics {
            messages: vec![msg],
            fatal: true,
        }),
    }
}

// ── Term representation ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Term {
    Iri(String),
    Blank(String),
}

impl Term {
    fn canonical(&self) -> String {
        match self {
            Self::Iri(iri) => format!("<{iri}>"),
            Self::Blank(label) => format!("_:{label}"),
        }
    }
}

// ── Stack frames ──────────────────────────────────────────────────────────────

/// Each stack frame represents a grammar production currently in progress.
#[derive(Debug)]
enum Frame {
    /// The `rdf:RDF` wrapper or document root.
    Root,
    /// A node element (rdf:Description or typed node).
    Node { subject: Term },
    /// A property element collecting text content as a plain/typed literal.
    PropertyLiteral {
        subject: Term,
        predicate: String,
        datatype: Option<String>,
        lang: Option<String>,
        reification_id: Option<String>,
        text: String,
    },
    /// A property element whose content is a child node element.
    PropertyNodeExpected {
        subject: Term,
        predicate: String,
        datatype: Option<String>,
        lang: Option<String>,
        reification_id: Option<String>,
        /// Depth counter: property element starts at 1.
        depth: usize,
        child_seen: bool,
    },
    /// A property element with `rdf:parseType="Resource"`.
    PropertyResource {
        /// The blank node emitted as object of the property.
        blank_node: Term,
        /// Depth: starts at 1 (the property element itself).
        depth: usize,
    },
    /// A property element with `rdf:parseType="Literal"` — raw XML.
    PropertyXmlLiteral {
        subject: Term,
        predicate: String,
        reification_id: Option<String>,
        /// Inner nesting depth (0 = we are inside the property element).
        depth: usize,
        raw: String,
    },
    /// A property element with `rdf:parseType="Collection"`.
    PropertyCollection {
        subject: Term,
        predicate: String,
        reification_id: Option<String>,
        nodes: Vec<Term>,
        /// Depth: starts at 1 (the property element itself).
        depth: usize,
    },
}

// ── XML context (namespaces + lang + base) ────────────────────────────────────

#[derive(Debug, Clone)]
struct XmlContext {
    namespaces: HashMap<String, String>,
    lang: Option<String>,
    base: Option<String>,
}

impl XmlContext {
    fn new() -> Self {
        let mut ns = HashMap::new();
        ns.insert("xml".into(), XML_NS.into());
        Self {
            namespaces: ns,
            lang: None,
            base: None,
        }
    }

    /// Expand a namespace-prefixed `QName` to a full IRI, or return `None`
    /// if the prefix is undeclared.
    fn resolve_qname(&self, qname: &str) -> Option<String> {
        if qname.is_empty() {
            return None;
        }
        if let Some(colon) = qname.find(':') {
            let prefix = &qname[..colon];
            let local = &qname[colon + 1..];
            self.namespaces
                .get(prefix)
                .map(|uri| format!("{uri}{local}"))
        } else {
            self.namespaces
                .get("")
                .map(|uri| format!("{uri}{qname}"))
        }
    }

    /// Resolve a possibly-relative IRI reference against the current base.
    fn resolve_iri(&self, reference: &str) -> String {
        if reference.is_empty() {
            return self
                .base
                .as_deref()
                .map(strip_fragment)
                .unwrap_or_default();
        }
        if looks_absolute(reference) {
            return reference.to_owned();
        }
        match &self.base {
            Some(base) => resolve_relative(base, reference),
            None => reference.to_owned(),
        }
    }
}

// ── IRI helpers ───────────────────────────────────────────────────────────────

fn strip_fragment(iri: &str) -> String {
    if let Some(pos) = iri.find('#') {
        iri[..pos].to_owned()
    } else {
        iri.to_owned()
    }
}

fn looks_absolute(iri: &str) -> bool {
    let Some(colon) = iri.find(':') else {
        return false;
    };
    if colon == 0 {
        return false;
    }
    let scheme = &iri[..colon];
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// RFC 3986 §5.2.2 relative-reference resolution (minimal implementation).
fn resolve_relative(base: &str, reference: &str) -> String {
    if let Some(frag) = reference.strip_prefix('#') {
        return format!("{}#{frag}", strip_fragment(base));
    }
    if reference.starts_with("//") {
        if let Some(colon) = base.find(':') {
            return format!("{}{}", &base[..=colon], reference);
        }
        return reference.to_owned();
    }
    if reference.starts_with('/') {
        // Take origin (scheme + authority) from base.
        if let Some(after_scheme) = base.find("://") {
            let authority_end = base[after_scheme + 3..]
                .find('/')
                .map_or(base.len(), |p| after_scheme + 3 + p);
            return format!("{}{}", &base[..authority_end], reference);
        }
        if let Some(colon) = base.find(':') {
            return format!("{}{}", &base[..=colon], reference);
        }
        return reference.to_owned();
    }
    // Relative path — merge with base path.
    let base_path_end = base.rfind('/').map_or(0, |p| p + 1);
    let merged = format!("{}{}", &base[..base_path_end], reference);
    remove_dot_segments(&merged)
}

fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    let mut remaining = path;
    while !remaining.is_empty() {
        if remaining.starts_with("../") {
            remaining = &remaining[3..];
        } else if remaining.starts_with("./") || remaining.starts_with("/./") {
            remaining = &remaining[2..];
        } else if remaining == "/." {
            remaining = "/";
        } else if remaining.starts_with("/../") {
            remaining = &remaining[3..];
            output.pop();
        } else if remaining == "/.." {
            remaining = "/";
            output.pop();
        } else if remaining == "." || remaining == ".." {
            remaining = "";
        } else {
            let seg_end = if let Some(stripped) = remaining.strip_prefix('/') {
                1 + stripped.find('/').unwrap_or(remaining.len() - 1)
            } else {
                remaining.find('/').unwrap_or(remaining.len())
            };
            output.push(&remaining[..seg_end]);
            remaining = &remaining[seg_end..];
        }
    }
    output.concat()
}

// ── Parse context ─────────────────────────────────────────────────────────────

struct ParseContext {
    triples: Vec<Fact>,
    bnode_counter: usize,
    named_bnodes: HashMap<String, String>,
    stack: Vec<Frame>,
    ctx_stack: Vec<XmlContext>,
    /// `rdf:li` counter per node-element nesting level.
    li_counters: Vec<usize>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            triples: Vec::new(),
            bnode_counter: 0,
            named_bnodes: HashMap::new(),
            stack: Vec::new(),
            ctx_stack: vec![XmlContext::new()],
            li_counters: Vec::new(),
        }
    }

    fn fresh_blank(&mut self) -> Term {
        let n = self.bnode_counter;
        self.bnode_counter += 1;
        Term::Blank(format!("b{n}"))
    }

    fn named_blank(&mut self, id: &str) -> Term {
        if let Some(label) = self.named_bnodes.get(id) {
            return Term::Blank(label.clone());
        }
        let label = format!("n{}", self.named_bnodes.len());
        self.named_bnodes.insert(id.to_owned(), label.clone());
        Term::Blank(label)
    }

    fn ctx(&self) -> &XmlContext {
        self.ctx_stack.last().expect("context stack never empty")
    }

    fn push_xml_context(&mut self, extra_ns: HashMap<String, String>, lang: Option<String>, base: Option<String>) {
        let mut new_ctx = self.ctx().clone();
        for (prefix, uri) in extra_ns {
            new_ctx.namespaces.insert(prefix, uri);
        }
        if let Some(l) = lang {
            new_ctx.lang = Some(l);
        }
        if let Some(b) = base {
            let resolved = new_ctx.resolve_iri(&b);
            new_ctx.base = Some(resolved);
        }
        self.ctx_stack.push(new_ctx);
    }

    fn pop_xml_context(&mut self) {
        if self.ctx_stack.len() > 1 {
            self.ctx_stack.pop();
        }
    }

    // ── Triple emission ────────────────────────────────────────────────────

    fn emit_resource(&mut self, s: &Term, p: &str, o: &Term) {
        self.triples.push(Fact {
            subject: s.canonical(),
            predicate: format!("<{p}>"),
            object: o.canonical(),
            graph: None,
        });
    }

    fn emit_literal(&mut self, s: &Term, p: &str, lex: &str, dt: Option<&str>, lang: Option<&str>) {
        let object = make_literal_canonical(lex, dt, lang);
        self.triples.push(Fact {
            subject: s.canonical(),
            predicate: format!("<{p}>"),
            object,
            graph: None,
        });
    }

    fn emit_reification_resource(&mut self, reif_iri: &str, s: &Term, p: &str, o: &Term) {
        let r = Term::Iri(reif_iri.to_owned());
        self.emit_resource(&r, &rdf("type"), &Term::Iri(rdf("Statement")));
        self.emit_resource(&r, &rdf("subject"), s);
        self.emit_resource(&r, &rdf("predicate"), &Term::Iri(p.to_owned()));
        self.emit_resource(&r, &rdf("object"), o);
    }

    fn emit_reification_literal(
        &mut self,
        reif_iri: &str,
        s: &Term,
        p: &str,
        lex: &str,
        dt: Option<&str>,
        lang: Option<&str>,
    ) {
        let r = Term::Iri(reif_iri.to_owned());
        self.emit_resource(&r, &rdf("type"), &Term::Iri(rdf("Statement")));
        self.emit_resource(&r, &rdf("subject"), s);
        self.emit_resource(&r, &rdf("predicate"), &Term::Iri(p.to_owned()));
        self.emit_literal(&r, &rdf("object"), lex, dt, lang);
    }

    // ── Main event loop ────────────────────────────────────────────────────

    fn run(&mut self, input: &[u8]) -> Result<(), String> {
        let mut reader = Reader::from_reader(input);
        reader.config_mut().trim_text(false);

        loop {
            let event = reader
                .read_event()
                .map_err(|e| format!("XML parse error: {e}"))?;
            match event {
                Event::Start(ref e) => {
                    let (ns_map, lang, base) = extract_xml_meta(e)?;
                    self.push_xml_context(ns_map, lang, base);
                    self.on_start(e, false)?;
                }
                Event::Empty(ref e) => {
                    let (ns_map, lang, base) = extract_xml_meta(e)?;
                    self.push_xml_context(ns_map, lang, base);
                    self.on_start(e, true)?;
                    self.pop_xml_context();
                }
                Event::End(ref e) => {
                    self.on_end(e)?;
                    self.pop_xml_context();
                }
                Event::Text(ref e) => self.on_text(e)?,
                Event::CData(cd) => {
                    let text = cd
                        .escape()
                        .map_err(|e| format!("CData error: {e}"))?;
                    let s = std::str::from_utf8(text.as_ref())
                        .map_err(|e| format!("CData UTF-8 error: {e}"))?;
                    let t = BytesText::from_escaped(s);
                    self.on_text(&t)?;
                }
                Event::Eof => break,
                _ => {}
            }
        }
        Ok(())
    }

    fn on_start(&mut self, e: &BytesStart<'_>, is_empty: bool) -> Result<(), String> {
        let qname = std::str::from_utf8(e.name().as_ref())
            .map_err(|_| "Non-UTF-8 element name".to_owned())?
            .to_owned();

        let resolved = self.ctx().resolve_qname(&qname);

        // Determine the grammar context from the top of the stack.
        match self.stack.last() {
            // ── Root / top-level ──────────────────────────────────────────
            None | Some(Frame::Root) => {
                if resolved.as_deref() == Some(&rdf("RDF")) {
                    if !is_empty {
                        self.stack.push(Frame::Root);
                    }
                } else {
                    // Treat as a bare node element at the document root.
                    self.start_node_element(e, is_empty, resolved.as_deref())?;
                }
            }

            // ── Node element — expects property children ──────────────────
            // ── parseType="Resource" — expects property children ──────────
            Some(Frame::Node { .. } | Frame::PropertyResource { .. }) => {
                self.start_property_element(e, is_empty, resolved.as_deref())?;
            }

            // ── PropertyNodeExpected — first child decides ─────────────────
            Some(Frame::PropertyNodeExpected { child_seen, .. }) => {
                if !child_seen {
                    // A child element → treat as node element, link back.
                    self.start_child_node_in_property(e, is_empty, resolved.as_deref())?;
                }
                // More children are invalid but we silently skip.
            }

            // ── parseType="Collection" — expects node children ─────────────
            Some(Frame::PropertyCollection { .. }) => {
                self.start_collection_node(e, is_empty, resolved.as_deref())?;
            }

            // ── parseType="Literal" — raw XML accumulation ────────────────
            Some(Frame::PropertyXmlLiteral { .. }) => {
                self.on_xml_literal_start(e, is_empty, &qname)?;
            }

            // ── PropertyLiteral — element inside literal? Treat as XML. ───
            Some(Frame::PropertyLiteral { .. }) => {
                // Mixed content (text + element) — treat as text for simplicity.
            }
        }
        Ok(())
    }

    fn on_end(&mut self, _e: &BytesEnd<'_>) -> Result<(), String> {
        match self.stack.last() {
            None => return Ok(()),
            Some(Frame::Root) => {
                self.stack.pop();
                return Ok(());
            }
            Some(Frame::PropertyXmlLiteral { .. }) => {
                return self.finalise_xml_literal();
            }
            _ => {}
        }

        match self.stack.last_mut() {
            Some(Frame::PropertyLiteral { .. }) => {
                return self.finalise_property_literal();
            }
            Some(Frame::PropertyResource { depth, .. }) => {
                *depth -= 1;
                if *depth == 0 {
                    self.stack.pop();
                    self.li_counters.pop();
                }
                return Ok(());
            }
            Some(Frame::PropertyNodeExpected { depth, .. }) => {
                *depth -= 1;
                if *depth == 0 {
                    return self.finalise_property_node_expected();
                }
                return Ok(());
            }
            Some(Frame::PropertyCollection { depth, .. }) => {
                *depth -= 1;
                if *depth == 0 {
                    return self.finalise_collection();
                }
                return Ok(());
            }
            Some(Frame::Node { .. }) => {
                self.stack.pop();
                self.li_counters.pop();
                return Ok(());
            }
            _ => {}
        }
        Ok(())
    }

    #[allow(clippy::collapsible_match)]
    fn on_text(&mut self, e: &BytesText<'_>) -> Result<(), String> {
        let text = e
            .unescape()
            .map_err(|er| format!("text unescape error: {er}"))?;

        match self.stack.last_mut() {
            Some(Frame::PropertyLiteral { text: buf, .. }) => {
                buf.push_str(&text);
            }
            Some(Frame::PropertyXmlLiteral { raw, .. }) => {
                raw.push_str(&text);
            }
            Some(Frame::PropertyNodeExpected {
                child_seen,
                subject,
                predicate,
                datatype,
                lang,
                reification_id,
                ..
            }) => {
                if !*child_seen && !text.trim().is_empty() {
                    // Non-whitespace text → convert to literal frame.
                    let new_frame = Frame::PropertyLiteral {
                        subject: subject.clone(),
                        predicate: predicate.clone(),
                        datatype: datatype.clone(),
                        lang: lang.clone(),
                        reification_id: reification_id.clone(),
                        text: text.into_owned(),
                    };
                    *self.stack.last_mut().unwrap() = new_frame;
                }
            }
            _ => {}
        }
        Ok(())
    }

    // ── Node element handling ─────────────────────────────────────────────

    fn start_node_element(
        &mut self,
        e: &BytesStart<'_>,
        is_empty: bool,
        resolved_name: Option<&str>,
    ) -> Result<(), String> {
        let ctx = self.ctx().clone();
        let subject = extract_subject_term(e, &ctx, &mut self.bnode_counter, &mut self.named_bnodes)?;

        // Emit rdf:type for typed nodes.
        if let Some(type_iri) = resolved_name
            && type_iri != rdf("Description") {
            self.emit_resource(
                &subject,
                &rdf("type"),
                &Term::Iri(type_iri.to_owned()),
            );
        }

        // Process property attributes on the node element.
        self.process_node_property_attrs(e, &subject, &ctx)?;

        if !is_empty {
            self.li_counters.push(1);
            self.stack.push(Frame::Node { subject });
        }
        Ok(())
    }

    /// Property attributes that appear directly on a node element (abbreviated form).
    fn process_node_property_attrs(
        &mut self,
        e: &BytesStart<'_>,
        subject: &Term,
        ctx: &XmlContext,
    ) -> Result<(), String> {
        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|er| format!("Attribute error: {er}"))?;
            let key = std::str::from_utf8(attr.key.as_ref())
                .map_err(|_| "Non-UTF-8 attr key".to_owned())?;
            let val = attr
                .unescape_value()
                .map_err(|er| format!("Attr value unescape: {er}"))?;

            if key.starts_with("xmlns") || key.starts_with("xml:") {
                continue;
            }
            let Some(pred) = ctx.resolve_qname(key) else {
                continue;
            };

            // Skip node-identification attributes.
            if pred == rdf("about")
                || pred == rdf("nodeID")
                || pred == rdf("ID")
            {
                continue;
            }

            // rdf:type as an attribute.
            if pred == rdf("type") {
                let type_iri = ctx.resolve_iri(&val);
                self.emit_resource(subject, &rdf("type"), &Term::Iri(type_iri));
                continue;
            }

            // Skip deprecated / unsupported rdf: attrs.
            if pred == rdf("bagID")
                || pred == rdf("aboutEach")
                || pred == rdf("aboutEachPrefix")
            {
                continue;
            }

            // Anything else is a property attribute → plain literal.
            let lang = ctx.lang.as_deref();
            self.emit_literal(subject, &pred, &val, None, lang);
        }
        Ok(())
    }

    // ── Property element handling ─────────────────────────────────────────

    fn start_property_element(
        &mut self,
        e: &BytesStart<'_>,
        is_empty: bool,
        resolved_name: Option<&str>,
    ) -> Result<(), String> {
        let ctx = self.ctx().clone();

        let subject = match self.stack.last() {
            Some(Frame::Node { subject }) => subject.clone(),
            Some(Frame::PropertyResource { blank_node, .. }) => blank_node.clone(),
            _ => return Err("property element outside node context".into()),
        };

        let pred_raw = resolved_name
            .ok_or_else(|| "property element has unresolvable name".to_owned())?;

        // rdf:li shorthand.
        let pred_iri = if pred_raw == rdf("li") {
            let counter = self
                .li_counters
                .last_mut()
                .expect("li counter should exist");
            let n = *counter;
            *counter += 1;
            rdf(&format!("_{n}"))
        } else {
            pred_raw.to_owned()
        };

        // Extract property-element-level attributes.
        let mut rdf_resource: Option<String> = None;
        let mut rdf_node_id: Option<String> = None;
        let mut rdf_datatype: Option<String> = None;
        let mut rdf_parse_type: Option<String> = None;
        let mut rdf_id: Option<String> = None;
        // Remaining property attributes (for object blank-node shorthand).
        let mut extra_attrs: Vec<(String, String)> = Vec::new();

        for attr_result in e.attributes() {
            let attr = attr_result.map_err(|er| format!("Attribute error: {er}"))?;
            let key = std::str::from_utf8(attr.key.as_ref())
                .map_err(|_| "Non-UTF-8 attr key".to_owned())?;
            let val = attr
                .unescape_value()
                .map_err(|er| format!("Attr value unescape: {er}"))?;

            if key.starts_with("xmlns") || key.starts_with("xml:") {
                continue;
            }
            let Some(rk) = ctx.resolve_qname(key) else {
                continue;
            };

            if rk == rdf("resource") {
                rdf_resource = Some(ctx.resolve_iri(&val));
            } else if rk == rdf("nodeID") {
                rdf_node_id = Some(val.into_owned());
            } else if rk == rdf("datatype") {
                rdf_datatype = Some(ctx.resolve_iri(&val));
            } else if rk == rdf("parseType") {
                rdf_parse_type = Some(val.into_owned());
            } else if rk == rdf("ID") {
                rdf_id = Some(build_reification_iri(&ctx, &val));
            } else if rk == rdf("type") {
                // rdf:type on property element is itself a predicate attribute.
                extra_attrs.push((rk, val.into_owned()));
            } else if !rk.starts_with(RDF) {
                extra_attrs.push((rk, val.into_owned()));
            }
        }

        // ── Dispatch on parseType ────────────────────────────────────────

        if let Some(pt) = rdf_parse_type.as_deref() {
            return self.start_parse_type_property(
                &subject,
                &pred_iri,
                rdf_id.as_deref(),
                is_empty,
                pt,
            );
        }

        // ── rdf:resource → IRI object ────────────────────────────────────

        if let Some(resource) = rdf_resource {
            let obj = Term::Iri(resource);
            self.emit_resource(&subject, &pred_iri, &obj);
            if let Some(ref reif) = rdf_id {
                self.emit_reification_resource(reif, &subject, &pred_iri, &obj);
            }
            // Even if !is_empty, rdf:resource dominates; content is ignored.
            // We don't push a frame; the end tag will be handled by depth.
            if !is_empty {
                // Push a "drain" frame to absorb the closing tag.
                self.stack.push(Frame::PropertyNodeExpected {
                    subject: subject.clone(),
                    predicate: pred_iri,
                    datatype: None,
                    lang: None,
                    reification_id: None,
                    depth: 1,
                    child_seen: true, // mark done so finalise does nothing
                });
            }
            return Ok(());
        }

        // ── rdf:nodeID → blank-node object ───────────────────────────────

        if let Some(node_id) = rdf_node_id {
            let obj = self.named_blank(&node_id);
            self.emit_resource(&subject, &pred_iri, &obj);
            if let Some(ref reif) = rdf_id {
                self.emit_reification_resource(reif, &subject, &pred_iri, &obj);
            }
            if !is_empty {
                self.stack.push(Frame::PropertyNodeExpected {
                    subject,
                    predicate: pred_iri,
                    datatype: None,
                    lang: None,
                    reification_id: None,
                    depth: 1,
                    child_seen: true,
                });
            }
            return Ok(());
        }

        // ── Extra property attrs → blank-node object ──────────────────────

        if !extra_attrs.is_empty() && is_empty {
            let bn = self.fresh_blank();
            self.emit_resource(&subject, &pred_iri, &bn);
            if let Some(ref reif) = rdf_id {
                self.emit_reification_resource(reif, &subject, &pred_iri, &bn);
            }
            let lang = ctx.lang.as_deref();
            for (pa_pred, pa_val) in &extra_attrs {
                self.emit_literal(&bn, pa_pred, pa_val, None, lang);
            }
            return Ok(());
        }

        // ── Empty element → empty-string literal ─────────────────────────

        if is_empty {
            let lang = ctx.lang.as_deref();
            let dt = rdf_datatype.as_deref();
            self.emit_literal(&subject, &pred_iri, "", dt, lang);
            if let Some(ref reif) = rdf_id {
                self.emit_reification_literal(reif, &subject, &pred_iri, "", dt, lang);
            }
            return Ok(());
        }

        // ── Non-empty → collect text or node child ─────────────────────────

        self.stack.push(Frame::PropertyNodeExpected {
            subject,
            predicate: pred_iri,
            datatype: rdf_datatype,
            lang: ctx.lang.clone(),
            reification_id: rdf_id,
            depth: 1,
            child_seen: false,
        });
        Ok(())
    }

    fn start_parse_type_property(
        &mut self,
        subject: &Term,
        pred_iri: &str,
        rdf_id: Option<&str>,
        is_empty: bool,
        parse_type: &str,
    ) -> Result<(), String> {
        match parse_type {
            "Resource" => {
                let bn = self.fresh_blank();
                self.emit_resource(subject, pred_iri, &bn);
                if let Some(reif) = rdf_id {
                    self.emit_reification_resource(reif, subject, pred_iri, &bn);
                }
                if !is_empty {
                    self.li_counters.push(1);
                    self.stack.push(Frame::PropertyResource {
                        blank_node: bn,
                        depth: 1,
                    });
                }
            }
            "Literal" => {
                if is_empty {
                    let xml_dt = rdf("XMLLiteral");
                    self.emit_literal(subject, pred_iri, "", Some(&xml_dt), None);
                    if let Some(reif) = rdf_id {
                        self.emit_reification_literal(
                            reif, subject, pred_iri, "", Some(&xml_dt), None,
                        );
                    }
                } else {
                    self.stack.push(Frame::PropertyXmlLiteral {
                        subject: subject.clone(),
                        predicate: pred_iri.to_owned(),
                        reification_id: rdf_id.map(str::to_owned),
                        depth: 0,
                        raw: String::new(),
                    });
                }
            }
            "Collection" => {
                if is_empty {
                    self.emit_resource(subject, pred_iri, &Term::Iri(rdf("nil")));
                    if let Some(reif) = rdf_id {
                        self.emit_reification_resource(
                            reif,
                            subject,
                            pred_iri,
                            &Term::Iri(rdf("nil")),
                        );
                    }
                } else {
                    self.li_counters.push(1);
                    self.stack.push(Frame::PropertyCollection {
                        subject: subject.clone(),
                        predicate: pred_iri.to_owned(),
                        reification_id: rdf_id.map(str::to_owned),
                        nodes: Vec::new(),
                        depth: 1,
                    });
                }
            }
            other => {
                return Err(format!("Unknown rdf:parseType value: {other}"));
            }
        }
        Ok(())
    }

    // ── PropertyNodeExpected: child node element ───────────────────────────

    fn start_child_node_in_property(
        &mut self,
        e: &BytesStart<'_>,
        is_empty: bool,
        resolved_name: Option<&str>,
    ) -> Result<(), String> {
        // Extract subject/predicate from the top frame before we push a new one.
        let (parent_subject, predicate, reif_id) = match self.stack.last_mut() {
            Some(Frame::PropertyNodeExpected {
                subject,
                predicate,
                reification_id,
                child_seen,
                depth,
                ..
            }) => {
                *child_seen = true;
                *depth += 1; // account for the child node element
                (
                    subject.clone(),
                    predicate.clone(),
                    reification_id.clone(),
                )
            }
            _ => return Err("start_child_node_in_property: unexpected frame".into()),
        };

        let ctx = self.ctx().clone();
        let child_subject =
            extract_subject_term(e, &ctx, &mut self.bnode_counter, &mut self.named_bnodes)?;

        // Emit the parent → child triple.
        self.emit_resource(&parent_subject, &predicate, &child_subject);
        if let Some(ref reif) = reif_id {
            self.emit_reification_resource(reif, &parent_subject, &predicate, &child_subject);
        }

        // Typed node: emit rdf:type.
        if let Some(type_iri) = resolved_name
            && type_iri != rdf("Description") {
            self.emit_resource(
                &child_subject,
                &rdf("type"),
                &Term::Iri(type_iri.to_owned()),
            );
        }

        self.process_node_property_attrs(e, &child_subject, &ctx)?;

        if !is_empty {
            self.li_counters.push(1);
            self.stack.push(Frame::Node {
                subject: child_subject,
            });
        }
        Ok(())
    }

    // ── Collection node handling ─────────────────────────────────────────

    fn start_collection_node(
        &mut self,
        e: &BytesStart<'_>,
        is_empty: bool,
        resolved_name: Option<&str>,
    ) -> Result<(), String> {
        let ctx = self.ctx().clone();
        let node_subject =
            extract_subject_term(e, &ctx, &mut self.bnode_counter, &mut self.named_bnodes)?;

        if let Some(type_iri) = resolved_name
            && type_iri != rdf("Description") {
            self.emit_resource(
                &node_subject,
                &rdf("type"),
                &Term::Iri(type_iri.to_owned()),
            );
        }
        self.process_node_property_attrs(e, &node_subject, &ctx)?;

        if let Some(Frame::PropertyCollection { nodes, depth, .. }) = self.stack.last_mut() {
            nodes.push(node_subject.clone());
            *depth += 1;
        }

        if !is_empty {
            self.li_counters.push(1);
            self.stack.push(Frame::Node {
                subject: node_subject,
            });
        }
        Ok(())
    }

    // ── Finalisation helpers ──────────────────────────────────────────────

    fn finalise_property_literal(&mut self) -> Result<(), String> {
        let frame = self.stack.pop().expect("frame exists");
        let Frame::PropertyLiteral {
            subject,
            predicate,
            datatype,
            lang,
            reification_id,
            text,
        } = frame
        else {
            return Err("expected PropertyLiteral frame".into());
        };
        let dt = datatype.as_deref();
        let lg = lang.as_deref();
        self.emit_literal(&subject, &predicate, &text, dt, lg);
        if let Some(ref reif) = reification_id {
            self.emit_reification_literal(reif, &subject, &predicate, &text, dt, lg);
        }
        Ok(())
    }

    fn finalise_property_node_expected(&mut self) -> Result<(), String> {
        let frame = self.stack.pop().expect("frame exists");
        let Frame::PropertyNodeExpected {
            subject,
            predicate,
            datatype,
            lang,
            reification_id,
            child_seen,
            ..
        } = frame
        else {
            return Err("expected PropertyNodeExpected frame".into());
        };

        if !child_seen {
            // No child element → empty literal.
            let dt = datatype.as_deref();
            let lg = lang.as_deref();
            self.emit_literal(&subject, &predicate, "", dt, lg);
            if let Some(ref reif) = reification_id {
                self.emit_reification_literal(reif, &subject, &predicate, "", dt, lg);
            }
        }
        // If child_seen, the triple was emitted when we saw the child.
        Ok(())
    }

    fn finalise_collection(&mut self) -> Result<(), String> {
        let frame = self.stack.pop().expect("frame exists");
        let Frame::PropertyCollection {
            subject,
            predicate,
            reification_id,
            nodes,
            ..
        } = frame
        else {
            return Err("expected PropertyCollection frame".into());
        };

        if nodes.is_empty() {
            let nil = Term::Iri(rdf("nil"));
            self.emit_resource(&subject, &predicate, &nil);
            if let Some(ref reif) = reification_id {
                self.emit_reification_resource(reif, &subject, &predicate, &nil);
            }
            return Ok(());
        }

        let list_nodes: Vec<Term> = (0..nodes.len())
            .map(|_| self.fresh_blank())
            .collect();

        self.emit_resource(&subject, &predicate, &list_nodes[0]);
        if let Some(ref reif) = reification_id {
            self.emit_reification_resource(reif, &subject, &predicate, &list_nodes[0]);
        }

        for (i, (list_node, item)) in list_nodes.iter().zip(nodes.iter()).enumerate() {
            self.emit_resource(list_node, &rdf("first"), item);
            let rest = if i + 1 < list_nodes.len() {
                list_nodes[i + 1].clone()
            } else {
                Term::Iri(rdf("nil"))
            };
            self.emit_resource(list_node, &rdf("rest"), &rest);
        }
        Ok(())
    }

    fn finalise_xml_literal(&mut self) -> Result<(), String> {
        let frame = self.stack.pop().expect("frame exists");
        let Frame::PropertyXmlLiteral {
            subject,
            predicate,
            reification_id,
            raw,
            ..
        } = frame
        else {
            return Err("expected PropertyXmlLiteral frame".into());
        };
        let xml_dt = rdf("XMLLiteral");
        self.emit_literal(&subject, &predicate, &raw, Some(&xml_dt), None);
        if let Some(ref reif) = reification_id {
            self.emit_reification_literal(reif, &subject, &predicate, &raw, Some(&xml_dt), None);
        }
        Ok(())
    }

    fn on_xml_literal_start(
        &mut self,
        e: &BytesStart<'_>,
        is_empty: bool,
        qname: &str,
    ) -> Result<(), String> {
        if let Some(Frame::PropertyXmlLiteral { raw, depth, .. }) = self.stack.last_mut() {
            raw.push('<');
            raw.push_str(qname);
            for attr_result in e.attributes() {
                let attr = attr_result.map_err(|er| format!("Attr error: {er}"))?;
                let k = std::str::from_utf8(attr.key.as_ref())
                    .map_err(|_| "Non-UTF-8 attr key".to_owned())?;
                let v = attr
                    .unescape_value()
                    .map_err(|er| format!("Attr value unescape: {er}"))?;
                raw.push(' ');
                raw.push_str(k);
                raw.push_str("=\"");
                raw.push_str(&v.replace('"', "&quot;"));
                raw.push('"');
            }
            if is_empty {
                raw.push_str("/>");
            } else {
                raw.push('>');
                *depth += 1;
            }
        }
        Ok(())
    }
}

// ── Literal canonical form helper ─────────────────────────────────────────────

fn make_literal_canonical(lex: &str, dt: Option<&str>, lang: Option<&str>) -> String {
    // Escape the lexical form for embedding in the canonical literal string.
    let escaped = escape_literal_lex(lex);
    if let Some(tag) = lang {
        format!("\"{escaped}\"@{tag}")
    } else if let Some(datatype) = dt {
        if datatype == "http://www.w3.org/2001/XMLSchema#string" {
            format!("\"{escaped}\"")
        } else {
            format!("\"{escaped}\"^^<{datatype}>")
        }
    } else {
        format!("\"{escaped}\"")
    }
}

/// Escape a lexical form so it is safe inside double quotes in a canonical literal.
/// Only `"` and `\` need escaping.
fn escape_literal_lex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            other => out.push(other),
        }
    }
    out
}

// ── XML attribute extraction helpers ─────────────────────────────────────────

/// Extract namespace declarations, `xml:lang`, and `xml:base` from an element.
fn extract_xml_meta(
    e: &BytesStart<'_>,
) -> Result<(HashMap<String, String>, Option<String>, Option<String>), String> {
    let mut ns_map = HashMap::new();
    let mut lang: Option<String> = None;
    let mut base: Option<String> = None;

    for attr_result in e.attributes() {
        let attr = attr_result.map_err(|er| format!("Attribute error: {er}"))?;
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|_| "Non-UTF-8 attr key".to_owned())?;
        let val = attr
            .unescape_value()
            .map_err(|er| format!("Attr value unescape: {er}"))?;

        if key == "xmlns" {
            ns_map.insert(String::new(), val.into_owned());
        } else if let Some(prefix) = key.strip_prefix("xmlns:") {
            ns_map.insert(prefix.to_owned(), val.into_owned());
        } else if key == "xml:lang" {
            lang = Some(val.into_owned());
        } else if key == "xml:base" {
            base = Some(val.into_owned());
        }
    }
    Ok((ns_map, lang, base))
}

/// Extract the subject `Term` for a node element from its identifying attributes.
fn extract_subject_term(
    e: &BytesStart<'_>,
    ctx: &XmlContext,
    bnode_counter: &mut usize,
    named_bnodes: &mut HashMap<String, String>,
) -> Result<Term, String> {
    let mut about: Option<String> = None;
    let mut node_id: Option<String> = None;
    let mut id_attr: Option<String> = None;

    for attr_result in e.attributes() {
        let attr = attr_result.map_err(|er| format!("Attribute error: {er}"))?;
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|_| "Non-UTF-8 attr key".to_owned())?;
        let val = attr
            .unescape_value()
            .map_err(|er| format!("Attr value unescape: {er}"))?;

        if key.starts_with("xmlns") || key.starts_with("xml:") {
            continue;
        }
        let Some(rk) = ctx.resolve_qname(key) else {
            continue;
        };

        if rk == rdf("about") {
            about = Some(ctx.resolve_iri(&val));
        } else if rk == rdf("nodeID") {
            node_id = Some(val.into_owned());
        } else if rk == rdf("ID") {
            id_attr = Some(build_reification_iri(ctx, &val));
        }
    }

    let subject = if let Some(iri) = about {
        Term::Iri(iri)
    } else if let Some(nid) = node_id {
        if let Some(label) = named_bnodes.get(&nid) {
            Term::Blank(label.clone())
        } else {
            let label = format!("n{}", named_bnodes.len());
            named_bnodes.insert(nid, label.clone());
            Term::Blank(label)
        }
    } else if let Some(id) = id_attr {
        Term::Iri(id)
    } else {
        let n = *bnode_counter;
        *bnode_counter += 1;
        Term::Blank(format!("b{n}"))
    };

    Ok(subject)
}

/// Build the absolute IRI for an `rdf:ID` attribute value.
fn build_reification_iri(ctx: &XmlContext, id_val: &str) -> String {
    let base = ctx.base.as_deref().unwrap_or("");
    if base.is_empty() {
        format!("#{id_val}")
    } else {
        format!("{}#{id_val}", strip_fragment(base))
    }
}
