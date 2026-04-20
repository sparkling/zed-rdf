//! RDF Language Server Protocol implementation (Phase F, ADR-0025).
//!
//! Supports: NT, NQ, Turtle, `TriG`, RDF/XML, JSON-LD, `TriX`, N3,
//! SPARQL, `ShEx`, Datalog.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod dispatch;
pub mod features;
pub mod semantic_tokens;
pub mod incremental;
pub mod rename;

/// RDF/SPARQL/`ShEx`/Datalog language variants supported by the LSP server.
///
/// Language is detected once per request from the document URI's file
/// extension. Unknown extensions fall through to `None`; handlers degrade
/// gracefully (return empty results, not panic).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Language {
    /// N-Triples (`.nt`)
    NTriples,
    /// N-Quads (`.nq`)
    NQuads,
    /// Turtle (`.ttl`, `.turtle`)
    Turtle,
    /// `TriG` (`.trig`)
    TriG,
    /// RDF/XML (`.rdf`, `.xml`)
    RdfXml,
    /// JSON-LD (`.jsonld`)
    JsonLd,
    /// `TriX` (`.trix`)
    TriX,
    /// Notation3 (`.n3`)
    N3,
    /// SPARQL (`.sparql`, `.rq`, `.ru`)
    Sparql,
    /// `ShEx` (`.shex`)
    ShEx,
    /// Datalog (`.dl`)
    Datalog,
}

impl Language {
    /// Detect the language from a document URI's file extension.
    ///
    /// Returns `None` for unknown extensions.
    #[must_use]
    pub fn from_uri(uri: &lsp_types::Url) -> Option<Self> {
        match uri.path().rsplit('.').next()? {
            "nt" => Some(Self::NTriples),
            "nq" => Some(Self::NQuads),
            "ttl" | "turtle" => Some(Self::Turtle),
            "trig" => Some(Self::TriG),
            "rdf" | "xml" => Some(Self::RdfXml),
            "jsonld" | "json-ld" => Some(Self::JsonLd),
            "trix" => Some(Self::TriX),
            "n3" => Some(Self::N3),
            "sparql" | "rq" | "ru" => Some(Self::Sparql),
            "shex" => Some(Self::ShEx),
            "dl" => Some(Self::Datalog),
            _ => None,
        }
    }
}

/// Start the LSP server, reading from stdin and writing to stdout.
///
/// Blocks until the client sends a `shutdown` request followed by an `exit`
/// notification. Any I/O or protocol errors are propagated to stderr and
/// terminate the process.
pub fn run_server() {
    if let Err(e) = dispatch::run_server() {
        eprintln!("rdf-lsp: fatal error: {e}");
        std::process::exit(1);
    }
}
