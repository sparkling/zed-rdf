//! RDF core vocabulary terms.
//!
//! Namespace: `http://www.w3.org/1999/02/22-rdf-syntax-ns#`
//! Reference: <https://www.w3.org/TR/rdf11-concepts/>

/// RDF namespace IRI (trailing `#`).
pub const NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// `rdf:type` — Relates a resource to its class.
///
/// Label: "type"
///
/// Description: "Indicates the class to which the resource belongs."
pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

/// `rdf:Property` — The class of RDF properties.
///
/// Label: "Property"
///
/// Description: "The class of RDF properties."
pub const PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";

/// `rdf:Statement` — The class of RDF statements (reified triples).
///
/// Label: "Statement"
///
/// Description: "The class of RDF statements used in reification."
pub const STATEMENT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement";

/// `rdf:subject` — The subject of a reified RDF statement.
///
/// Label: "subject"
///
/// Description: "The subject of the RDF statement being reified."
pub const SUBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject";

/// `rdf:predicate` — The predicate of a reified RDF statement.
///
/// Label: "predicate"
///
/// Description: "The predicate of the RDF statement being reified."
pub const PREDICATE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate";

/// `rdf:object` — The object of a reified RDF statement.
///
/// Label: "object"
///
/// Description: "The object of the RDF statement being reified."
pub const OBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#object";

/// `rdf:Bag` — An unordered container class.
///
/// Label: "Bag"
///
/// Description: "An unordered container; membership may have duplicates."
pub const BAG: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Bag";

/// `rdf:Seq` — An ordered container class.
///
/// Label: "Seq"
///
/// Description: "An ordered container whose members are listed by index."
pub const SEQ: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Seq";

/// `rdf:Alt` — A container class representing alternatives.
///
/// Label: "Alt"
///
/// Description: "A container representing a set of alternatives."
pub const ALT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Alt";

/// `rdf:List` — The class of RDF lists.
///
/// Label: "List"
///
/// Description: "The class of RDF lists."
pub const LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";

/// `rdf:nil` — The empty list.
///
/// Label: "nil"
///
/// Description: "The empty list, with no items in it. The terminator of RDF lists."
pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// `rdf:first` — The first item in an RDF list node.
///
/// Label: "first"
///
/// Description: "The first item in a list node."
pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";

/// `rdf:rest` — The remainder of an RDF list after the first item.
///
/// Label: "rest"
///
/// Description: "The rest of the list after the first item."
pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";

/// `rdf:value` — The primary value of a structured resource.
///
/// Label: "value"
///
/// Description: "Identifies the principal value associated with a container."
pub const VALUE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#value";

/// `rdf:langString` — The datatype of language-tagged string literals.
///
/// Label: "langString"
///
/// Description: "The datatype of language-tagged string literals."
pub const LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// `rdf:HTML` — The datatype of RDF literals whose content is HTML fragments.
///
/// Label: "HTML"
///
/// Description: "The datatype of RDF literals whose content is HTML."
pub const HTML: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML";

/// `rdf:XMLLiteral` — The datatype of XML literal values.
///
/// Label: "`XMLLiteral`"
///
/// Description: "The datatype of XML literal values; the lexical form is a serialized XML fragment."
pub const XML_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral";

/// `rdf:JSON` — The datatype of RDF literals whose content is JSON.
///
/// Label: "JSON"
///
/// Description: "The datatype of RDF literals whose content is a JSON string."
pub const JSON: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON";

/// `rdf:CompoundLiteral` — The class of compound literals.
///
/// Label: "`CompoundLiteral`"
///
/// Description: "A class of literals that have both a language tag and a direction."
pub const COMPOUND_LITERAL: &str =
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#CompoundLiteral";

/// `rdf:direction` — The base direction of a compound literal.
///
/// Label: "direction"
///
/// Description: "The base direction component of a compound literal."
pub const DIRECTION: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#direction";

/// `rdf:language` — The language tag component of a compound literal.
///
/// Label: "language"
///
/// Description: "The language tag component of a compound literal."
pub const LANGUAGE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#language";
