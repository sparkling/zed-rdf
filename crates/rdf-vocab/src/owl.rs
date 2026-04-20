//! OWL (Web Ontology Language) vocabulary terms.
//!
//! Namespace: `http://www.w3.org/2002/07/owl#`
//! Reference: <https://www.w3.org/TR/owl2-syntax/>

/// OWL namespace IRI (trailing `#`).
pub const NS: &str = "http://www.w3.org/2002/07/owl#";

// ── Class vocabulary ──────────────────────────────────────────────────────────

/// `owl:Class` — The class of OWL classes.
///
/// Label: "Class"
///
/// Description: "The class of OWL classes."
pub const CLASS: &str = "http://www.w3.org/2002/07/owl#Class";

/// `owl:Thing` — The most general OWL class (everything is a thing).
///
/// Label: "Thing"
///
/// Description: "The class of OWL individuals."
pub const THING: &str = "http://www.w3.org/2002/07/owl#Thing";

/// `owl:Nothing` — The empty OWL class (no individual is a member).
///
/// Label: "Nothing"
///
/// Description: "This is the empty class."
pub const NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";

/// `owl:equivalentClass` — Two classes that have the same extension.
///
/// Label: "equivalentClass"
///
/// Description: "The property that determines that two given classes are equivalent, and that is used to specify datatype definitions."
pub const EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";

/// `owl:disjointWith` — Two classes with no common instances.
///
/// Label: "disjointWith"
///
/// Description: "The property that determines that two given classes are disjoint."
pub const DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";

/// `owl:complementOf` — The complement of a class.
///
/// Label: "complementOf"
///
/// Description: "The property that determines that a given class is the complement of another class."
pub const COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";

/// `owl:unionOf` — The union of a list of classes.
///
/// Label: "unionOf"
///
/// Description: "The property that determines the collection of classes or data ranges that build a union."
pub const UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";

/// `owl:intersectionOf` — The intersection of a list of classes.
///
/// Label: "intersectionOf"
///
/// Description: "The property that determines the collection of classes or data ranges that build an intersection."
pub const INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";

/// `owl:oneOf` — A class defined by enumeration of its individuals.
///
/// Label: "oneOf"
///
/// Description: "The property that determines the collection of individuals or data values that build an enumeration."
pub const ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";

/// `owl:AllDisjointClasses` — A class listing a set of mutually disjoint classes.
///
/// Label: "`AllDisjointClasses`"
///
/// Description: "The class of collections of pairwise disjoint classes."
pub const ALL_DISJOINT_CLASSES: &str =
    "http://www.w3.org/2002/07/owl#AllDisjointClasses";

// ── Property vocabulary ───────────────────────────────────────────────────────

/// `owl:ObjectProperty` — The class of OWL object properties.
///
/// Label: "`ObjectProperty`"
///
/// Description: "The class of object properties."
pub const OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";

/// `owl:DatatypeProperty` — The class of OWL datatype properties.
///
/// Label: "`DatatypeProperty`"
///
/// Description: "The class of data properties."
pub const DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";

/// `owl:AnnotationProperty` — The class of OWL annotation properties.
///
/// Label: "`AnnotationProperty`"
///
/// Description: "The class of annotation properties."
pub const ANNOTATION_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#AnnotationProperty";

/// `owl:FunctionalProperty` — A property with at most one value per subject.
///
/// Label: "`FunctionalProperty`"
///
/// Description: "The class of functional properties."
pub const FUNCTIONAL_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#FunctionalProperty";

/// `owl:InverseFunctionalProperty` — A property with at most one subject per value.
///
/// Label: "`InverseFunctionalProperty`"
///
/// Description: "The class of inverse-functional properties."
pub const INVERSE_FUNCTIONAL_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";

/// `owl:SymmetricProperty` — A property that is its own inverse.
///
/// Label: "`SymmetricProperty`"
///
/// Description: "The class of symmetric properties."
pub const SYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";

/// `owl:AsymmetricProperty` — A property that cannot be its own inverse.
///
/// Label: "`AsymmetricProperty`"
///
/// Description: "The class of asymmetric properties."
pub const ASYMMETRIC_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#AsymmetricProperty";

/// `owl:TransitiveProperty` — A property that is transitive.
///
/// Label: "`TransitiveProperty`"
///
/// Description: "The class of transitive properties."
pub const TRANSITIVE_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#TransitiveProperty";

/// `owl:ReflexiveProperty` — A property that every individual has to itself.
///
/// Label: "`ReflexiveProperty`"
///
/// Description: "The class of reflexive properties."
pub const REFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ReflexiveProperty";

/// `owl:IrreflexiveProperty` — A property that no individual has to itself.
///
/// Label: "`IrreflexiveProperty`"
///
/// Description: "The class of irreflexive properties."
pub const IRREFLEXIVE_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#IrreflexiveProperty";

/// `owl:equivalentProperty` — Two properties with the same extension.
///
/// Label: "equivalentProperty"
///
/// Description: "The property that determines that two given properties are equivalent."
pub const EQUIVALENT_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#equivalentProperty";

/// `owl:inverseOf` — The inverse of an object property.
///
/// Label: "inverseOf"
///
/// Description: "The property that determines that two given properties are inverse."
pub const INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";

/// `owl:propertyDisjointWith` — Two disjoint properties.
///
/// Label: "propertyDisjointWith"
///
/// Description: "The property that determines that two given properties are disjoint."
pub const PROPERTY_DISJOINT_WITH: &str =
    "http://www.w3.org/2002/07/owl#propertyDisjointWith";

/// `owl:AllDisjointProperties` — A class listing mutually disjoint properties.
///
/// Label: "`AllDisjointProperties`"
///
/// Description: "The class of collections of pairwise disjoint properties."
pub const ALL_DISJOINT_PROPERTIES: &str =
    "http://www.w3.org/2002/07/owl#AllDisjointProperties";

// ── Individual vocabulary ─────────────────────────────────────────────────────

/// `owl:NamedIndividual` — An explicitly named individual in OWL.
///
/// Label: "`NamedIndividual`"
///
/// Description: "The class of named individuals."
pub const NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";

/// `owl:sameAs` — Two URIs that denote the same individual.
///
/// Label: "sameAs"
///
/// Description: "The property that determines that two given individuals are equal."
pub const SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";

/// `owl:differentFrom` — Two URIs that denote different individuals.
///
/// Label: "differentFrom"
///
/// Description: "The property that determines that two given individuals are different."
pub const DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";

/// `owl:AllDifferent` — A class of collections of mutually different individuals.
///
/// Label: "`AllDifferent`"
///
/// Description: "The class of collections of pairwise different individuals."
pub const ALL_DIFFERENT: &str = "http://www.w3.org/2002/07/owl#AllDifferent";

// ── Ontology vocabulary ───────────────────────────────────────────────────────

/// `owl:Ontology` — An OWL ontology resource.
///
/// Label: "Ontology"
///
/// Description: "The class of ontologies."
pub const ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";

/// `owl:imports` — Includes an ontology by reference.
///
/// Label: "imports"
///
/// Description: "The property that is used for importing other ontologies into a given ontology."
pub const IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";

/// `owl:versionInfo` — Version information about an ontology.
///
/// Label: "versionInfo"
///
/// Description: "The annotation property that provides version information for an ontology or another OWL construct."
pub const VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";

/// `owl:versionIRI` — The IRI that identifies a particular version of an ontology.
///
/// Label: "versionIRI"
///
/// Description: "The property that identifies the version IRI of an ontology."
pub const VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";

/// `owl:priorVersion` — A prior version of an ontology.
///
/// Label: "priorVersion"
///
/// Description: "The annotation property that indicates the prior version of a given ontology."
pub const PRIOR_VERSION: &str = "http://www.w3.org/2002/07/owl#priorVersion";

/// `owl:backwardCompatibleWith` — A prior ontology compatible with this version.
///
/// Label: "backwardCompatibleWith"
///
/// Description: "The annotation property that indicates that a given ontology is backward compatible with another ontology."
pub const BACKWARD_COMPATIBLE_WITH: &str =
    "http://www.w3.org/2002/07/owl#backwardCompatibleWith";

/// `owl:incompatibleWith` — An ontology incompatible with this version.
///
/// Label: "incompatibleWith"
///
/// Description: "The annotation property that indicates that a given ontology is incompatible with another ontology."
pub const INCOMPATIBLE_WITH: &str = "http://www.w3.org/2002/07/owl#incompatibleWith";

// ── Restriction vocabulary ────────────────────────────────────────────────────

/// `owl:Restriction` — A class description formed as a property restriction.
///
/// Label: "Restriction"
///
/// Description: "The class of property restrictions."
pub const RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";

/// `owl:onProperty` — The property that a restriction applies to.
///
/// Label: "onProperty"
///
/// Description: "The property that determines the property that a property restriction refers to."
pub const ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";

/// `owl:allValuesFrom` — All values of the property must be from a class.
///
/// Label: "allValuesFrom"
///
/// Description: "The property that determines the class that a universal property restriction refers to."
pub const ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";

/// `owl:someValuesFrom` — At least one value of the property must be from a class.
///
/// Label: "someValuesFrom"
///
/// Description: "The property that determines the class that an existential property restriction refers to."
pub const SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";

/// `owl:hasValue` — The property has a specific value for this restriction.
///
/// Label: "hasValue"
///
/// Description: "The property that determines the individual that a has-value restriction refers to."
pub const HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";

/// `owl:hasSelf` — A self-restriction: the property relates an individual to itself.
///
/// Label: "hasSelf"
///
/// Description: "The property that determines the property that a self restriction refers to."
pub const HAS_SELF: &str = "http://www.w3.org/2002/07/owl#hasSelf";

/// `owl:minCardinality` — A minimum cardinality restriction.
///
/// Label: "minCardinality"
///
/// Description: "The property that determines the cardinality of a minimum cardinality restriction."
pub const MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";

/// `owl:maxCardinality` — A maximum cardinality restriction.
///
/// Label: "maxCardinality"
///
/// Description: "The property that determines the cardinality of a maximum cardinality restriction."
pub const MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";

/// `owl:cardinality` — An exact cardinality restriction.
///
/// Label: "cardinality"
///
/// Description: "The property that determines the cardinality of an exact cardinality restriction."
pub const CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";

/// `owl:minQualifiedCardinality` — A minimum qualified cardinality restriction.
///
/// Label: "minQualifiedCardinality"
///
/// Description: "The property that determines the cardinality of a minimum qualified cardinality restriction."
pub const MIN_QUALIFIED_CARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#minQualifiedCardinality";

/// `owl:maxQualifiedCardinality` — A maximum qualified cardinality restriction.
///
/// Label: "maxQualifiedCardinality"
///
/// Description: "The property that determines the cardinality of a maximum qualified cardinality restriction."
pub const MAX_QUALIFIED_CARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";

/// `owl:qualifiedCardinality` — An exact qualified cardinality restriction.
///
/// Label: "qualifiedCardinality"
///
/// Description: "The property that determines the cardinality of an exact qualified cardinality restriction."
pub const QUALIFIED_CARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#qualifiedCardinality";

/// `owl:onClass` — The class used in a qualified cardinality restriction.
///
/// Label: "onClass"
///
/// Description: "The property that determines the class that a qualified object cardinality restriction refers to."
pub const ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";

/// `owl:onDataRange` — The data range used in a qualified cardinality restriction.
///
/// Label: "onDataRange"
///
/// Description: "The property that determines the data range that a qualified data cardinality restriction refers to."
pub const ON_DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#onDataRange";
