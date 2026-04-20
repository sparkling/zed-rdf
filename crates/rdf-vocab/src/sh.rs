//! SHACL (Shapes Constraint Language) vocabulary terms.
//!
//! Namespace: `http://www.w3.org/ns/shacl#`
//! Reference: <https://www.w3.org/TR/shacl/>

/// SHACL namespace IRI (trailing `#`).
pub const NS: &str = "http://www.w3.org/ns/shacl#";

// ── Core shape classes ────────────────────────────────────────────────────────

/// `sh:Shape` — A SHACL shape (node shape or property shape).
///
/// Label: "Shape"
///
/// Description: "A shape is a collection of constraints that may be targeted for validation."
pub const SHAPE: &str = "http://www.w3.org/ns/shacl#Shape";

/// `sh:NodeShape` — A shape applying constraints to nodes.
///
/// Label: "Node shape"
///
/// Description: "A node shape is a shape that specifies constraints on the focus node itself."
pub const NODE_SHAPE: &str = "http://www.w3.org/ns/shacl#NodeShape";

/// `sh:PropertyShape` — A shape applying constraints to values of a property.
///
/// Label: "Property shape"
///
/// Description: "A property shape is a shape that specifies constraints on the values of a property."
pub const PROPERTY_SHAPE: &str = "http://www.w3.org/ns/shacl#PropertyShape";

// ── Targets ───────────────────────────────────────────────────────────────────

/// `sh:targetClass` — Applies a shape to all instances of a class.
///
/// Label: "target class"
///
/// Description: "Links a shape to a class, indicating that all instances of the class are focus nodes."
pub const TARGET_CLASS: &str = "http://www.w3.org/ns/shacl#targetClass";

/// `sh:targetNode` — Applies a shape to a specific node.
///
/// Label: "target node"
///
/// Description: "Links a shape to individual nodes, indicating that the nodes are focus nodes."
pub const TARGET_NODE: &str = "http://www.w3.org/ns/shacl#targetNode";

/// `sh:targetObjectsOf` — Applies a shape to all objects of a property.
///
/// Label: "target objects of"
///
/// Description: "Links a shape to a property, indicating that all objects of triples with the property are focus nodes."
pub const TARGET_OBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetObjectsOf";

/// `sh:targetSubjectsOf` — Applies a shape to all subjects of a property.
///
/// Label: "target subjects of"
///
/// Description: "Links a shape to a property, indicating that all subjects of triples with the property are focus nodes."
pub const TARGET_SUBJECTS_OF: &str = "http://www.w3.org/ns/shacl#targetSubjectsOf";

// ── Path ──────────────────────────────────────────────────────────────────────

/// `sh:path` — The property path that a property shape constrains.
///
/// Label: "path"
///
/// Description: "Specifies the property path of a property shape."
pub const PATH: &str = "http://www.w3.org/ns/shacl#path";

/// `sh:alternativePath` — A SPARQL property path alternative.
///
/// Label: "alternative path"
///
/// Description: "The (single) value of this property must be a list of path elements, representing the elements of alternative paths."
pub const ALTERNATIVE_PATH: &str = "http://www.w3.org/ns/shacl#alternativePath";

/// `sh:inversePath` — A SPARQL inverse property path.
///
/// Label: "inverse path"
///
/// Description: "The value of this property must be exactly one path element, representing the inverse of the given path element."
pub const INVERSE_PATH: &str = "http://www.w3.org/ns/shacl#inversePath";

/// `sh:zeroOrMorePath` — A SPARQL zero-or-more (*) path.
///
/// Label: "zero or more path"
///
/// Description: "The value of this property must be exactly one path element, representing the zero or more traversal of the given path element."
pub const ZERO_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrMorePath";

/// `sh:oneOrMorePath` — A SPARQL one-or-more (+) path.
///
/// Label: "one or more path"
///
/// Description: "The value of this property must be exactly one path element, representing the one or more traversal of the given path element."
pub const ONE_OR_MORE_PATH: &str = "http://www.w3.org/ns/shacl#oneOrMorePath";

/// `sh:zeroOrOnePath` — A SPARQL zero-or-one (?) path.
///
/// Label: "zero or one path"
///
/// Description: "The value of this property must be exactly one path element, representing the zero or one traversal of the given path element."
pub const ZERO_OR_ONE_PATH: &str = "http://www.w3.org/ns/shacl#zeroOrOnePath";

// ── Constraint parameters ─────────────────────────────────────────────────────

/// `sh:property` — A property shape linked to a node shape.
///
/// Label: "property"
///
/// Description: "Links a shape to its property shapes."
pub const PROPERTY: &str = "http://www.w3.org/ns/shacl#property";

/// `sh:datatype` — The expected datatype of values.
///
/// Label: "datatype"
///
/// Description: "Specifies an RDF datatype that all value nodes must have."
pub const DATATYPE: &str = "http://www.w3.org/ns/shacl#datatype";

/// `sh:nodeKind` — The expected node kind (IRI, blank node, literal, etc.).
///
/// Label: "node kind"
///
/// Description: "Specifies the node kind (e.g. IRI, `BlankNode`, Literal) of all value nodes."
pub const NODE_KIND: &str = "http://www.w3.org/ns/shacl#nodeKind";

/// `sh:class` — The expected class of values.
///
/// Label: "class"
///
/// Description: "The type that all value nodes must have."
pub const CLASS: &str = "http://www.w3.org/ns/shacl#class";

/// `sh:minCount` — Minimum number of values for a property.
///
/// Label: "min count"
///
/// Description: "Specifies the minimum number of values in the set of value nodes."
pub const MIN_COUNT: &str = "http://www.w3.org/ns/shacl#minCount";

/// `sh:maxCount` — Maximum number of values for a property.
///
/// Label: "max count"
///
/// Description: "Specifies the maximum number of values in the set of value nodes."
pub const MAX_COUNT: &str = "http://www.w3.org/ns/shacl#maxCount";

/// `sh:minLength` — Minimum string length of values.
///
/// Label: "min length"
///
/// Description: "Specifies the minimum string length of each value node that satisfies the constraint."
pub const MIN_LENGTH: &str = "http://www.w3.org/ns/shacl#minLength";

/// `sh:maxLength` — Maximum string length of values.
///
/// Label: "max length"
///
/// Description: "Specifies the maximum string length of each value node that satisfies the constraint."
pub const MAX_LENGTH: &str = "http://www.w3.org/ns/shacl#maxLength";

/// `sh:minExclusive` — Minimum exclusive numeric or date value.
///
/// Label: "min exclusive"
///
/// Description: "Specifies the minimum exclusive value of each value node."
pub const MIN_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#minExclusive";

/// `sh:minInclusive` — Minimum inclusive numeric or date value.
///
/// Label: "min inclusive"
///
/// Description: "Specifies the minimum inclusive value of each value node."
pub const MIN_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#minInclusive";

/// `sh:maxExclusive` — Maximum exclusive numeric or date value.
///
/// Label: "max exclusive"
///
/// Description: "Specifies the maximum exclusive value of each value node."
pub const MAX_EXCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxExclusive";

/// `sh:maxInclusive` — Maximum inclusive numeric or date value.
///
/// Label: "max inclusive"
///
/// Description: "Specifies the maximum inclusive value of each value node."
pub const MAX_INCLUSIVE: &str = "http://www.w3.org/ns/shacl#maxInclusive";

/// `sh:pattern` — A regular expression pattern that values must match.
///
/// Label: "pattern"
///
/// Description: "Specifies a regular expression pattern that the string representation of each value node must match."
pub const PATTERN: &str = "http://www.w3.org/ns/shacl#pattern";

/// `sh:flags` — Regex flags for `sh:pattern`.
///
/// Label: "flags"
///
/// Description: "An optional string of flags for the regex pattern."
pub const FLAGS: &str = "http://www.w3.org/ns/shacl#flags";

/// `sh:languageIn` — Values must have one of these language tags.
///
/// Label: "language in"
///
/// Description: "Specifies the language tags that each value must have."
pub const LANGUAGE_IN: &str = "http://www.w3.org/ns/shacl#languageIn";

/// `sh:uniqueLang` — Each language tag used in values must be unique.
///
/// Label: "unique lang"
///
/// Description: "Specifies whether or not each value node may use the same language tag as another value node."
pub const UNIQUE_LANG: &str = "http://www.w3.org/ns/shacl#uniqueLang";

/// `sh:in` — Values must be from a given list.
///
/// Label: "in"
///
/// Description: "Specifies a list of allowed values so that each value node must be among the members of the given list."
pub const IN: &str = "http://www.w3.org/ns/shacl#in";

/// `sh:hasValue` — The set of values must contain a specific value.
///
/// Label: "has value"
///
/// Description: "Specifies a value that must be among the value nodes."
pub const HAS_VALUE: &str = "http://www.w3.org/ns/shacl#hasValue";

/// `sh:node` — Values must conform to a referenced shape.
///
/// Label: "node"
///
/// Description: "Specifies the node shape that all value nodes must conform to."
pub const NODE: &str = "http://www.w3.org/ns/shacl#node";

/// `sh:not` — Values must not conform to a shape.
///
/// Label: "not"
///
/// Description: "Specifies a shape that the value nodes must not conform to."
pub const NOT: &str = "http://www.w3.org/ns/shacl#not";

/// `sh:and` — Values must conform to all shapes in a list.
///
/// Label: "and"
///
/// Description: "Specifies a list of shapes so that each value node must conform to all shapes."
pub const AND: &str = "http://www.w3.org/ns/shacl#and";

/// `sh:or` — Values must conform to at least one shape in a list.
///
/// Label: "or"
///
/// Description: "Specifies a list of shapes so that each value node must conform to at least one of the shapes."
pub const OR: &str = "http://www.w3.org/ns/shacl#or";

/// `sh:xone` — Values must conform to exactly one shape in a list.
///
/// Label: "exactly one"
///
/// Description: "Specifies a list of shapes so that each value node must conform to exactly one of the shapes."
pub const XONE: &str = "http://www.w3.org/ns/shacl#xone";

/// `sh:qualifiedValueShape` — The shape used in a qualified value constraint.
///
/// Label: "qualified value shape"
///
/// Description: "The shape that a specified number of values must conform to."
pub const QUALIFIED_VALUE_SHAPE: &str =
    "http://www.w3.org/ns/shacl#qualifiedValueShape";

/// `sh:qualifiedMinCount` — Minimum number of values matching a qualified shape.
///
/// Label: "qualified min count"
///
/// Description: "The minimum number of value nodes that must conform to the specified qualified value shape."
pub const QUALIFIED_MIN_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMinCount";

/// `sh:qualifiedMaxCount` — Maximum number of values matching a qualified shape.
///
/// Label: "qualified max count"
///
/// Description: "The maximum number of value nodes that may conform to the specified qualified value shape."
pub const QUALIFIED_MAX_COUNT: &str = "http://www.w3.org/ns/shacl#qualifiedMaxCount";

/// `sh:disjoint` — Values must be disjoint from values of another property.
///
/// Label: "disjoint"
///
/// Description: "Specifies a property so that the set of values must be disjoint with the value nodes of that property on the focus node."
pub const DISJOINT: &str = "http://www.w3.org/ns/shacl#disjoint";

/// `sh:equals` — Values must equal values of another property.
///
/// Label: "equals"
///
/// Description: "Specifies a property so that the set of values must be equal to the value nodes of that property on the focus node."
pub const EQUALS: &str = "http://www.w3.org/ns/shacl#equals";

/// `sh:lessThan` — Values must be less than values of another property.
///
/// Label: "less than"
///
/// Description: "Specifies a property so that each value node must be less than all the values of the given property."
pub const LESS_THAN: &str = "http://www.w3.org/ns/shacl#lessThan";

/// `sh:lessThanOrEquals` — Values must be less than or equal to values of another property.
///
/// Label: "less than or equals"
///
/// Description: "Specifies a property so that each value node must be less than or equal to all the values of the given property."
pub const LESS_THAN_OR_EQUALS: &str = "http://www.w3.org/ns/shacl#lessThanOrEquals";

/// `sh:closed` — No properties other than those declared in the shape are allowed.
///
/// Label: "closed"
///
/// Description: "If set to true, no other properties may be present besides those declared via sh:property."
pub const CLOSED: &str = "http://www.w3.org/ns/shacl#closed";

/// `sh:ignoredProperties` — Properties to ignore when checking closed shapes.
///
/// Label: "ignored properties"
///
/// Description: "An optional list of properties that are also permitted in closed shapes."
pub const IGNORED_PROPERTIES: &str = "http://www.w3.org/ns/shacl#ignoredProperties";

// ── Results vocabulary ────────────────────────────────────────────────────────

/// `sh:ValidationResult` — A result produced during validation.
///
/// Label: "Validation result"
///
/// Description: "The class of validation results, used to report conformance or non-conformance."
pub const VALIDATION_RESULT: &str = "http://www.w3.org/ns/shacl#ValidationResult";

/// `sh:ValidationReport` — A validation report produced by a validator.
///
/// Label: "Validation report"
///
/// Description: "The class of SHACL validation reports."
pub const VALIDATION_REPORT: &str = "http://www.w3.org/ns/shacl#ValidationReport";

/// `sh:Violation` — A validation result that indicates a violation.
///
/// Label: "Violation"
///
/// Description: "The severity for a constraint violation."
pub const VIOLATION: &str = "http://www.w3.org/ns/shacl#Violation";

/// `sh:Warning` — A validation result that indicates a warning.
///
/// Label: "Warning"
///
/// Description: "The severity for a validation warning."
pub const WARNING: &str = "http://www.w3.org/ns/shacl#Warning";

/// `sh:Info` — A validation result that indicates an informational message.
///
/// Label: "Info"
///
/// Description: "The severity for a validation info message."
pub const INFO: &str = "http://www.w3.org/ns/shacl#Info";

/// `sh:conforms` — Whether a data graph conforms to a shapes graph.
///
/// Label: "conforms"
///
/// Description: "True if the validation did not produce any violation results, and false otherwise."
pub const CONFORMS: &str = "http://www.w3.org/ns/shacl#conforms";

/// `sh:result` — A result produced by validation.
///
/// Label: "result"
///
/// Description: "The validation results contained in a validation report."
pub const RESULT: &str = "http://www.w3.org/ns/shacl#result";

/// `sh:resultSeverity` — The severity of a result.
///
/// Label: "result severity"
///
/// Description: "The severity of the result, e.g. sh:Violation."
pub const RESULT_SEVERITY: &str = "http://www.w3.org/ns/shacl#resultSeverity";

/// `sh:sourceConstraintComponent` — The constraint component that produced the result.
///
/// Label: "source constraint component"
///
/// Description: "The constraint component that caused the result."
pub const SOURCE_CONSTRAINT_COMPONENT: &str =
    "http://www.w3.org/ns/shacl#sourceConstraintComponent";

/// `sh:sourceShape` — The shape that produced the result.
///
/// Label: "source shape"
///
/// Description: "The shape that is the source of the result."
pub const SOURCE_SHAPE: &str = "http://www.w3.org/ns/shacl#sourceShape";

/// `sh:focusNode` — The focus node that was validated.
///
/// Label: "focus node"
///
/// Description: "The focus node that was validated when the result was produced."
pub const FOCUS_NODE: &str = "http://www.w3.org/ns/shacl#focusNode";

/// `sh:value` — The value that caused the constraint violation.
///
/// Label: "value"
///
/// Description: "An RDF node that has caused the result."
pub const VALUE: &str = "http://www.w3.org/ns/shacl#value";

/// `sh:resultPath` — The path that was validated.
///
/// Label: "result path"
///
/// Description: "The path of a validation result."
pub const RESULT_PATH: &str = "http://www.w3.org/ns/shacl#resultPath";

/// `sh:resultMessage` — A human-readable message about the result.
///
/// Label: "result message"
///
/// Description: "Human-readable messages explaining the cause of the result."
pub const RESULT_MESSAGE: &str = "http://www.w3.org/ns/shacl#resultMessage";

// ── Miscellaneous ─────────────────────────────────────────────────────────────

/// `sh:message` — A message to include in validation results.
///
/// Label: "message"
///
/// Description: "A human-readable message (possibly with placeholders for variables) to be used as the result message when the constraint fails."
pub const MESSAGE: &str = "http://www.w3.org/ns/shacl#message";

/// `sh:severity` — The severity of a constraint violation.
///
/// Label: "severity"
///
/// Description: "Optionally indicates the severity of the constraint, e.g. sh:Warning."
pub const SEVERITY: &str = "http://www.w3.org/ns/shacl#severity";

/// `sh:name` — A human-readable name for a shape parameter.
///
/// Label: "name"
///
/// Description: "Human-readable labels for a property shape in the context of the surrounding shape."
pub const NAME: &str = "http://www.w3.org/ns/shacl#name";

/// `sh:description` — A human-readable description of a shape parameter.
///
/// Label: "description"
///
/// Description: "Human-readable descriptions of the property in the context of the surrounding shape."
pub const DESCRIPTION: &str = "http://www.w3.org/ns/shacl#description";

/// `sh:order` — A numeric rank for ordering properties.
///
/// Label: "order"
///
/// Description: "Specifies the relative order of this compared to its siblings."
pub const ORDER: &str = "http://www.w3.org/ns/shacl#order";

/// `sh:group` — A property group containing this property.
///
/// Label: "group"
///
/// Description: "The property group that this property belongs to."
pub const GROUP: &str = "http://www.w3.org/ns/shacl#group";

/// `sh:defaultValue` — The default value for a property when no value is present.
///
/// Label: "default value"
///
/// Description: "A default value for a property in the context of a property shape. This property may be used in user interface tools."
pub const DEFAULT_VALUE: &str = "http://www.w3.org/ns/shacl#defaultValue";

/// `sh:deactivated` — Whether a shape or constraint is deactivated.
///
/// Label: "deactivated"
///
/// Description: "If set to true, all nodes conform to this."
pub const DEACTIVATED: &str = "http://www.w3.org/ns/shacl#deactivated";

/// `sh:sparql` — A SPARQL-based constraint.
///
/// Label: "SPARQL constraint"
///
/// Description: "Links a shape with SPARQL constraints components."
pub const SPARQL: &str = "http://www.w3.org/ns/shacl#sparql";

/// `sh:select` — A SPARQL SELECT query for a SPARQL-based constraint.
///
/// Label: "select"
///
/// Description: "A SPARQL SELECT query that will return one row per validation result."
pub const SELECT: &str = "http://www.w3.org/ns/shacl#select";

/// `sh:ask` — A SPARQL ASK query for a SPARQL-based constraint.
///
/// Label: "ask"
///
/// Description: "A SPARQL ASK query that will return true if the constraint is satisfied."
pub const ASK: &str = "http://www.w3.org/ns/shacl#ask";

/// `sh:prefixes` — Namespace prefix declarations for SPARQL queries.
///
/// Label: "prefixes"
///
/// Description: "The prefixes that shall be applied before parsing the associated SPARQL query."
pub const PREFIXES: &str = "http://www.w3.org/ns/shacl#prefixes";

/// `sh:declare` — A prefix declaration.
///
/// Label: "declare"
///
/// Description: "Links a prefix declaration to its defining resource."
pub const DECLARE: &str = "http://www.w3.org/ns/shacl#declare";

/// `sh:prefix` — The prefix string in a prefix declaration.
///
/// Label: "prefix"
///
/// Description: "The prefix of a prefix declaration."
pub const PREFIX: &str = "http://www.w3.org/ns/shacl#prefix";

/// `sh:namespace` — The namespace IRI in a prefix declaration.
///
/// Label: "namespace"
///
/// Description: "The namespace associated with a prefix in a prefix declaration."
pub const NAMESPACE: &str = "http://www.w3.org/ns/shacl#namespace";
