//! Hover handler — returns vocabulary term documentation for the RDF term
//! under the cursor.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use crate::Language;

/// Vocabulary term entry: (full IRI, label, description).
#[derive(Clone, Copy)]
struct VocabEntry {
    iri: &'static str,
    label: &'static str,
    description: &'static str,
}

/// Return hover documentation for the RDF vocab term under `pos` in `text`.
///
/// The function converts the LSP `Position` (line/character) to a byte offset,
/// extracts the token at that offset, and looks it up across the known vocab
/// modules.  Returns `None` when the cursor is not on a recognised term.
#[must_use]
pub fn handle_hover(text: &str, _lang: Language, pos: Position) -> Option<Hover> {
    let token = token_at(text, pos)?;
    let entry = lookup_vocab_term(&token)?;
    let markdown = format!(
        "**{}**\n\n`{}`\n\n{}",
        entry.label, entry.iri, entry.description
    );
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown,
        }),
        range: None,
    })
}

// ---------------------------------------------------------------------------
// Cursor-to-byte-offset helpers
// ---------------------------------------------------------------------------

/// Convert an LSP `Position` (0-based line + UTF-16 character) to a byte
/// offset in `text`.  Returns `None` when the position is out of range.
fn pos_to_byte_offset(text: &str, pos: Position) -> Option<usize> {
    let target_line = pos.line as usize;
    let target_char = pos.character as usize;
    let mut current_line = 0usize;
    let mut byte_offset = 0usize;

    for ch in text.chars() {
        if current_line == target_line {
            break;
        }
        byte_offset += ch.len_utf8();
        if ch == '\n' {
            current_line += 1;
        }
    }

    if current_line < target_line {
        // Position is beyond the end of text.
        return None;
    }

    // Now advance `target_char` UTF-16 code-units within the line.
    let mut col = 0usize;
    for ch in text[byte_offset..].chars() {
        if ch == '\n' {
            break;
        }
        if col >= target_char {
            break;
        }
        col += ch.len_utf16();
        byte_offset += ch.len_utf8();
    }

    Some(byte_offset)
}

/// Extract the IRI or prefixed-name token that contains `pos`.
///
/// Strategy: find the byte offset, then expand left and right over token
/// characters.  Token characters are: ASCII alphanumeric, `-`, `_`, `.`,
/// `:`, `#`, `/`, `<`, `>`, `@`.
fn token_at(text: &str, pos: Position) -> Option<String> {
    let cursor = pos_to_byte_offset(text, pos)?;
    if cursor > text.len() {
        return None;
    }

    let bytes = text.as_bytes();

    // Walk left to find token start.
    let mut start = cursor;
    while start > 0 && is_token_byte(bytes[start - 1]) {
        start -= 1;
    }

    // Walk right to find token end.
    let mut end = cursor;
    while end < bytes.len() && is_token_byte(bytes[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    let raw = &text[start..end];
    // Strip surrounding angle brackets from full IRIs.
    let token = raw.trim_matches(|c| c == '<' || c == '>');
    if token.is_empty() {
        None
    } else {
        Some(token.to_owned())
    }
}

const fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || b == b'-'
        || b == b'_'
        || b == b'.'
        || b == b':'
        || b == b'#'
        || b == b'/'
        || b == b'<'
        || b == b'>'
        || b == b'@'
}

// ---------------------------------------------------------------------------
// Vocabulary lookup
// ---------------------------------------------------------------------------

/// All known vocab terms (label, IRI, description) pulled from the
/// `rdf_vocab` constants.
#[allow(clippy::too_many_lines)]
fn all_vocab_entries() -> Vec<VocabEntry> {
    use rdf_vocab::{dcat, dcterms, foaf, owl, prov, rdf, rdfs, schema, sh, skos, xsd};

    macro_rules! entry {
        ($label:literal, $iri:expr, $desc:literal) => {
            VocabEntry {
                iri: $iri,
                label: $label,
                description: $desc,
            }
        };
    }

    vec![
        // rdf
        entry!("type", rdf::TYPE, "Indicates the class to which the resource belongs."),
        entry!("Property", rdf::PROPERTY, "The class of RDF properties."),
        entry!("Statement", rdf::STATEMENT, "The class of RDF statements used in reification."),
        entry!("subject", rdf::SUBJECT, "The subject of the RDF statement being reified."),
        entry!("predicate", rdf::PREDICATE, "The predicate of the RDF statement being reified."),
        entry!("object", rdf::OBJECT, "The object of the RDF statement being reified."),
        entry!("Bag", rdf::BAG, "An unordered container; membership may have duplicates."),
        entry!("Seq", rdf::SEQ, "An ordered container whose members are listed by index."),
        entry!("Alt", rdf::ALT, "A container representing a set of alternatives."),
        entry!("List", rdf::LIST, "The class of RDF lists."),
        entry!("nil", rdf::NIL, "The empty list, with no items in it."),
        entry!("first", rdf::FIRST, "The first item in a list node."),
        entry!("rest", rdf::REST, "The rest of the list after the first item."),
        entry!("value", rdf::VALUE, "Identifies the principal value associated with a container."),
        entry!("langString", rdf::LANG_STRING, "The datatype of language-tagged string literals."),
        entry!("HTML", rdf::HTML, "The datatype of RDF literals whose content is HTML."),
        entry!("XMLLiteral", rdf::XML_LITERAL, "The datatype of XML literal values."),
        entry!("JSON", rdf::JSON, "The datatype of RDF literals whose content is a JSON string."),
        // rdfs
        entry!("Class", rdfs::CLASS, "The class of all RDF/RDFS classes."),
        entry!("subClassOf", rdfs::SUB_CLASS_OF, "Relates a class to one of its superclasses."),
        entry!("subPropertyOf", rdfs::SUB_PROPERTY_OF, "Relates a property to one of its superproperties."),
        entry!("domain", rdfs::DOMAIN, "Indicates the class of the subject of a property."),
        entry!("range", rdfs::RANGE, "Indicates the class of the object of a property."),
        entry!("label", rdfs::LABEL, "A human-readable label for the subject."),
        entry!("comment", rdfs::COMMENT, "A description of the subject resource."),
        entry!("Resource", rdfs::RESOURCE, "The class resource, everything."),
        entry!("Literal", rdfs::LITERAL, "The class of literal values."),
        entry!("Datatype", rdfs::DATATYPE, "The class of RDF datatypes."),
        entry!("seeAlso", rdfs::SEE_ALSO, "Indicates a resource that might provide additional information."),
        entry!("isDefinedBy", rdfs::IS_DEFINED_BY, "Indicates the resource defining the subject resource."),
        entry!("Container", rdfs::CONTAINER, "The class of RDF containers."),
        entry!("member", rdfs::MEMBER, "A member of the subject resource."),
        // owl
        entry!("OwlClass", owl::CLASS, "The class of OWL classes."),
        entry!("ObjectProperty", owl::OBJECT_PROPERTY, "The class of OWL object properties."),
        entry!("DatatypeProperty", owl::DATATYPE_PROPERTY, "The class of OWL datatype properties."),
        entry!("AnnotationProperty", owl::ANNOTATION_PROPERTY, "The class of OWL annotation properties."),
        entry!("equivalentClass", owl::EQUIVALENT_CLASS, "The property that determines that two given classes are equivalent."),
        entry!("equivalentProperty", owl::EQUIVALENT_PROPERTY, "The property that determines that two given properties are equivalent."),
        entry!("sameAs", owl::SAME_AS, "The property that determines that two given individuals are equal."),
        entry!("differentFrom", owl::DIFFERENT_FROM, "The property that determines that two given individuals are different."),
        entry!("inverseOf", owl::INVERSE_OF, "The property that determines that two given properties are inverse."),
        entry!("Thing", owl::THING, "The class of OWL individuals."),
        entry!("Nothing", owl::NOTHING, "This is the empty class."),
        // xsd
        entry!("string", xsd::STRING, "The string datatype."),
        entry!("boolean", xsd::BOOLEAN, "The boolean datatype."),
        entry!("integer", xsd::INTEGER, "The integer datatype."),
        entry!("decimal", xsd::DECIMAL, "The decimal datatype."),
        entry!("float", xsd::FLOAT, "The float datatype."),
        entry!("double", xsd::DOUBLE, "The double datatype."),
        entry!("dateTime", xsd::DATE_TIME, "The dateTime datatype."),
        entry!("date", xsd::DATE, "The date datatype."),
        entry!("anyURI", xsd::ANY_URI, "The anyURI datatype."),
        // skos
        entry!("Concept", skos::CONCEPT, "An idea or notion; a unit of thought."),
        entry!("ConceptScheme", skos::CONCEPT_SCHEME, "A set of concepts."),
        entry!("prefLabel", skos::PREF_LABEL, "The preferred lexical label for a resource."),
        entry!("altLabel", skos::ALT_LABEL, "An alternative lexical label for a resource."),
        entry!("definition", skos::DEFINITION, "A statement or formal explanation of the meaning of a concept."),
        entry!("broader", skos::BROADER, "Relates a concept to a concept that is more general in meaning."),
        entry!("narrower", skos::NARROWER, "Relates a concept to a concept that is more specific in meaning."),
        entry!("related", skos::RELATED, "Relates a concept to a concept with which there is an associative semantic relationship."),
        entry!("inScheme", skos::IN_SCHEME, "Relates a resource to a concept scheme in which it is included."),
        // sh (SHACL)
        entry!("NodeShape", sh::NODE_SHAPE, "A node shape is a shape in the shapes graph."),
        entry!("PropertyShape", sh::PROPERTY_SHAPE, "A property shape is used as value of sh:property."),
        entry!("property", sh::PROPERTY, "Links a shape to its property shapes."),
        entry!("path", sh::PATH, "Specifies the property path of a property shape."),
        entry!("minCount", sh::MIN_COUNT, "Specifies the minimum number of value nodes."),
        entry!("maxCount", sh::MAX_COUNT, "Specifies the maximum number of value nodes."),
        entry!("datatype", sh::DATATYPE, "Specifies the datatype of value nodes."),
        entry!("shClass", sh::CLASS, "Specifies the class of value nodes."),
        entry!("in", sh::IN, "Specifies the condition that each value node is a member of a provided SHACL list."),
        // dcterms
        entry!("title", dcterms::TITLE, "A name given to the resource."),
        entry!("description", dcterms::DESCRIPTION, "An account of the resource."),
        entry!("creator", dcterms::CREATOR, "An entity primarily responsible for making the resource."),
        entry!("dctSubject", dcterms::SUBJECT, "The topic of the resource."),
        entry!("dctDate", dcterms::DATE, "A point or period of time associated with an event in the lifecycle of the resource."),
        entry!("format", dcterms::FORMAT, "The file format, physical medium, or dimensions of the resource."),
        entry!("identifier", dcterms::IDENTIFIER, "An unambiguous reference to the resource within a given context."),
        entry!("dctLanguage", dcterms::LANGUAGE, "A language of the resource."),
        // dcat
        entry!("Dataset", dcat::DATASET, "A collection of data, published or curated by a single source."),
        entry!("Distribution", dcat::DISTRIBUTION, "A specific representation of a dataset."),
        entry!("Catalog", dcat::CATALOG, "A curated collection of metadata about resources."),
        entry!("downloadURL", dcat::DOWNLOAD_URL, "The URL of the downloadable file in a given format."),
        // foaf
        entry!("Person", foaf::PERSON, "A person."),
        entry!("foafName", foaf::NAME, "A name for some thing."),
        entry!("mbox", foaf::MBOX, "A personal mailbox."),
        entry!("homepage", foaf::HOMEPAGE, "A homepage for some thing."),
        entry!("knows", foaf::KNOWS, "A person known by this person."),
        entry!("Organization", foaf::ORGANIZATION, "An organization."),
        // schema
        entry!("schemaThing", schema::THING, "The most generic type; every entity is a Thing."),
        entry!("schemaPerson", schema::PERSON, "A person (alive, dead, undead, or fictional)."),
        entry!("schemaOrganization", schema::ORGANIZATION, "An organization such as a school, NGO, corporation, club, etc."),
        entry!("schemaName", schema::NAME, "The name of the item."),
        entry!("schemaDescription", schema::DESCRIPTION, "A description of the item."),
        // prov
        entry!("Entity", prov::ENTITY, "An entity is a physical, digital, conceptual, or other kind of thing."),
        entry!("Activity", prov::ACTIVITY, "An activity is something that occurs over a period of time."),
        entry!("Agent", prov::AGENT, "An agent is something that bears some form of responsibility for an activity."),
        entry!("wasGeneratedBy", prov::WAS_GENERATED_BY, "Generation is the completion of production of a new entity."),
        entry!("wasDerivedFrom", prov::WAS_DERIVED_FROM, "A derivation is a transformation of an entity into another."),
        entry!("wasAttributedTo", prov::WAS_ATTRIBUTED_TO, "Attribution is the ascribing of an entity to an agent."),
    ]
}

/// Look up a token against all known vocabulary entries.
///
/// Matches either the full IRI or the local name portion (after `#` or last
/// `/`), or the part after `:` in a prefixed name like `rdf:type`.
fn lookup_vocab_term(token: &str) -> Option<VocabEntry> {
    let entries = all_vocab_entries();
    // Try exact IRI match first.
    for entry in &entries {
        if entry.iri == token {
            return Some(*entry);
        }
    }

    // Derive local name from token (prefixed name `rdf:type` -> `type`; or
    // fragment IRI `...#type` -> `type`; or path IRI `.../type` -> `type`).
    let local = {
        let after_colon = token.rfind(':').map_or("", |i| &token[i + 1..]);
        let after_hash = token.rfind('#').map_or("", |i| &token[i + 1..]);
        let after_slash = token.rfind('/').map_or("", |i| &token[i + 1..]);
        // Pick the longest non-empty suffix to prefer the most specific match.
        [after_colon, after_hash, after_slash, token]
            .iter()
            .copied()
            .filter(|s| !s.is_empty())
            .max_by_key(|s| s.len())
            .unwrap_or(token)
    };

    for entry in entries {
        let entry_local = entry
            .iri
            .rfind('#')
            .or_else(|| entry.iri.rfind('/'))
            .map_or(entry.iri, |i| &entry.iri[i + 1..]);

        if entry_local == local {
            return Some(entry);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_on_rdf_type_iri() {
        let text = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>";
        let pos = Position { line: 0, character: 10 };
        let result = handle_hover(text, Language::Turtle, pos);
        assert!(result.is_some(), "expected hover for rdf:type IRI");
    }

    #[test]
    fn hover_on_unknown_token_returns_none() {
        // "SomethingCompletelyUnknown" is not a vocab term
        let text = "ex:SomethingCompletelyUnknown";
        let pos = Position { line: 0, character: 3 };
        let result = handle_hover(text, Language::Turtle, pos);
        assert!(result.is_none());
    }

    #[test]
    fn hover_past_end_of_text_returns_none() {
        let text = "hello";
        let pos = Position { line: 99, character: 0 };
        assert!(handle_hover(text, Language::Turtle, pos).is_none());
    }
}
