//! SKOS (Simple Knowledge Organization System) vocabulary terms.
//!
//! Namespace: `http://www.w3.org/2004/02/skos/core#`
//! Reference: <https://www.w3.org/TR/skos-reference/>

/// SKOS namespace IRI (trailing `#`).
pub const NS: &str = "http://www.w3.org/2004/02/skos/core#";

// ── Classes ───────────────────────────────────────────────────────────────────

/// `skos:Concept` — An idea or notion in a KOS.
///
/// Label: "Concept"
///
/// Description: "An idea or notion; a unit of thought."
pub const CONCEPT: &str = "http://www.w3.org/2004/02/skos/core#Concept";

/// `skos:ConceptScheme` — A knowledge organisation system.
///
/// Label: "`ConceptScheme`"
///
/// Description: "A set of concepts, optionally including statements about semantic relationships between those concepts."
pub const CONCEPT_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#ConceptScheme";

/// `skos:Collection` — A meaningful collection of concepts.
///
/// Label: "Collection"
///
/// Description: "A meaningful collection of concepts."
pub const COLLECTION: &str = "http://www.w3.org/2004/02/skos/core#Collection";

/// `skos:OrderedCollection` — An ordered collection of concepts.
///
/// Label: "`OrderedCollection`"
///
/// Description: "An ordered collection of concepts, where both the grouping and the ordering are meaningful."
pub const ORDERED_COLLECTION: &str =
    "http://www.w3.org/2004/02/skos/core#OrderedCollection";

// ── Labelling properties ──────────────────────────────────────────────────────

/// `skos:prefLabel` — The preferred lexical label for a concept.
///
/// Label: "preferred label"
///
/// Description: "The preferred lexical label for a resource, in a given language."
pub const PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";

/// `skos:altLabel` — An alternative lexical label for a concept.
///
/// Label: "alternative label"
///
/// Description: "An alternative lexical label for a resource."
pub const ALT_LABEL: &str = "http://www.w3.org/2004/02/skos/core#altLabel";

/// `skos:hiddenLabel` — A label useful for text searches but not intended for display.
///
/// Label: "hidden label"
///
/// Description: "A lexical label for a resource that should be hidden when generating visual displays of the resource, but should still be accessible to free text search operations."
pub const HIDDEN_LABEL: &str = "http://www.w3.org/2004/02/skos/core#hiddenLabel";

// ── Documentation properties ──────────────────────────────────────────────────

/// `skos:note` — A general note about a concept.
///
/// Label: "note"
///
/// Description: "A general note, for any purpose."
pub const NOTE: &str = "http://www.w3.org/2004/02/skos/core#note";

/// `skos:definition` — A formal definition of a concept.
///
/// Label: "definition"
///
/// Description: "A statement or formal explanation of the meaning of a concept."
pub const DEFINITION: &str = "http://www.w3.org/2004/02/skos/core#definition";

/// `skos:example` — An example of the concept's use.
///
/// Label: "example"
///
/// Description: "An example of the use of a concept."
pub const EXAMPLE: &str = "http://www.w3.org/2004/02/skos/core#example";

/// `skos:historyNote` — A note about the past state of a concept.
///
/// Label: "history note"
///
/// Description: "A note about the past state/use/meaning of a concept."
pub const HISTORY_NOTE: &str = "http://www.w3.org/2004/02/skos/core#historyNote";

/// `skos:editorialNote` — An editorial note about a concept.
///
/// Label: "editorial note"
///
/// Description: "A note for an editor, translator or maintainer of the vocabulary."
pub const EDITORIAL_NOTE: &str = "http://www.w3.org/2004/02/skos/core#editorialNote";

/// `skos:changeNote` — A note about a modification to a concept.
///
/// Label: "change note"
///
/// Description: "A note about a modification to a concept."
pub const CHANGE_NOTE: &str = "http://www.w3.org/2004/02/skos/core#changeNote";

/// `skos:scopeNote` — A note clarifying the scope of a concept.
///
/// Label: "scope note"
///
/// Description: "A note that helps to clarify the meaning and/or the use of a concept."
pub const SCOPE_NOTE: &str = "http://www.w3.org/2004/02/skos/core#scopeNote";

// ── Semantic relations ────────────────────────────────────────────────────────

/// `skos:semanticRelation` — A concept related to another in the same scheme.
///
/// Label: "is in semantic relation with"
///
/// Description: "Links a concept to a concept related by meaning."
pub const SEMANTIC_RELATION: &str =
    "http://www.w3.org/2004/02/skos/core#semanticRelation";

/// `skos:broader` — A broader concept in a hierarchy.
///
/// Label: "has broader"
///
/// Description: "Relates a concept to a concept that is more general in meaning."
pub const BROADER: &str = "http://www.w3.org/2004/02/skos/core#broader";

/// `skos:narrower` — A narrower concept in a hierarchy.
///
/// Label: "has narrower"
///
/// Description: "Relates a concept to a concept that is more specific in meaning."
pub const NARROWER: &str = "http://www.w3.org/2004/02/skos/core#narrower";

/// `skos:related` — A concept associated with this concept.
///
/// Label: "has related"
///
/// Description: "Relates a concept to a concept with which there is an associative semantic relationship."
pub const RELATED: &str = "http://www.w3.org/2004/02/skos/core#related";

/// `skos:broaderTransitive` — Transitive closure of broader.
///
/// Label: "has broader transitive"
///
/// Description: "The transitive closure of the skos:broader property."
pub const BROADER_TRANSITIVE: &str =
    "http://www.w3.org/2004/02/skos/core#broaderTransitive";

/// `skos:narrowerTransitive` — Transitive closure of narrower.
///
/// Label: "has narrower transitive"
///
/// Description: "The transitive closure of the skos:narrower property."
pub const NARROWER_TRANSITIVE: &str =
    "http://www.w3.org/2004/02/skos/core#narrowerTransitive";

// ── Mapping properties ────────────────────────────────────────────────────────

/// `skos:mappingRelation` — A mapping relation between concepts in different schemes.
///
/// Label: "is in mapping relation with"
///
/// Description: "Relates two concepts coming, by convention, from different schemes, and that have comparable meanings."
pub const MAPPING_RELATION: &str =
    "http://www.w3.org/2004/02/skos/core#mappingRelation";

/// `skos:broadMatch` — A concept from another scheme that is broader.
///
/// Label: "has broader match"
///
/// Description: "Used to state a mapping link between two conceptual resources in which the first is more specific in meaning than the second."
pub const BROAD_MATCH: &str = "http://www.w3.org/2004/02/skos/core#broadMatch";

/// `skos:narrowMatch` — A concept from another scheme that is narrower.
///
/// Label: "has narrower match"
///
/// Description: "Used to state a mapping link between two conceptual resources in which the first is more specific in meaning than the second."
pub const NARROW_MATCH: &str = "http://www.w3.org/2004/02/skos/core#narrowMatch";

/// `skos:exactMatch` — A closely related concept in another scheme with the same meaning.
///
/// Label: "has exact match"
///
/// Description: "Used to link two concepts, indicating a high degree of confidence that the concepts can be used interchangeably across a wide range of information retrieval applications."
pub const EXACT_MATCH: &str = "http://www.w3.org/2004/02/skos/core#exactMatch";

/// `skos:closeMatch` — A closely related concept in another scheme.
///
/// Label: "has close match"
///
/// Description: "Used to link two concepts that are sufficiently similar that they can be used interchangeably in some information retrieval applications."
pub const CLOSE_MATCH: &str = "http://www.w3.org/2004/02/skos/core#closeMatch";

/// `skos:relatedMatch` — An associatively related concept in another scheme.
///
/// Label: "has related match"
///
/// Description: "Used to state an associative mapping link between two conceptual resources in different concept schemes."
pub const RELATED_MATCH: &str = "http://www.w3.org/2004/02/skos/core#relatedMatch";

// ── Scheme membership ─────────────────────────────────────────────────────────

/// `skos:inScheme` — A concept scheme this concept belongs to.
///
/// Label: "is in scheme"
///
/// Description: "Relates a resource (for example a concept) to a concept scheme in which it is included."
pub const IN_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#inScheme";

/// `skos:hasTopConcept` — The top concept of a scheme.
///
/// Label: "has top concept"
///
/// Description: "Relates, by convention, a concept scheme to a concept which is topmost in the broader/narrower concept hierarchies for that scheme."
pub const HAS_TOP_CONCEPT: &str = "http://www.w3.org/2004/02/skos/core#hasTopConcept";

/// `skos:topConceptOf` — The scheme for which this concept is a top concept.
///
/// Label: "is top concept in scheme"
///
/// Description: "Relates a concept to the concept scheme that it is a top level concept of."
pub const TOP_CONCEPT_OF: &str = "http://www.w3.org/2004/02/skos/core#topConceptOf";

// ── Collection membership ─────────────────────────────────────────────────────

/// `skos:member` — A member of a SKOS collection.
///
/// Label: "has member"
///
/// Description: "Relates a collection to one of its members."
pub const MEMBER: &str = "http://www.w3.org/2004/02/skos/core#member";

/// `skos:memberList` — An ordered list of members in an ordered collection.
///
/// Label: "has member list"
///
/// Description: "Relates an ordered collection to the RDF list containing its members."
pub const MEMBER_LIST: &str = "http://www.w3.org/2004/02/skos/core#memberList";

// ── Notation ──────────────────────────────────────────────────────────────────

/// `skos:notation` — A notation or code for a concept within a specific scheme.
///
/// Label: "notation"
///
/// Description: "A notation, also known as classification code, is a string of characters such as '300', 'Law' or 'Ozone Layer Depletion' uniquely identifying a concept within the scope of a given concept scheme."
pub const NOTATION: &str = "http://www.w3.org/2004/02/skos/core#notation";

/// `skos:subject` — A subject of a resource.
///
/// Label: "subject"
///
/// Description: "A subject of a resource. Deprecated in SKOS; use skos:related for associative relations."
pub const SUBJECT: &str = "http://www.w3.org/2004/02/skos/core#subject";

/// `skos:prefSymbol` — A preferred symbol for a concept.
///
/// Label: "preferred symbol"
///
/// Description: "The preferred symbol used to label a concept."
pub const PREF_SYMBOL: &str = "http://www.w3.org/2004/02/skos/core#prefSymbol";
