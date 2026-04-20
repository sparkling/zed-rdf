//! RDFS (RDF Schema) vocabulary terms.
//!
//! Namespace: `http://www.w3.org/2000/01/rdf-schema#`
//! Reference: <https://www.w3.org/TR/rdf-schema/>

/// RDFS namespace IRI (trailing `#`).
pub const NS: &str = "http://www.w3.org/2000/01/rdf-schema#";

/// `rdfs:Class` — The class of all RDFS classes.
///
/// Label: "Class"
///
/// Description: "The class of resources that are RDFS classes."
pub const CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";

/// `rdfs:Resource` — The class of all resources.
///
/// Label: "Resource"
///
/// Description: "The class resource, everything."
pub const RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";

/// `rdfs:Literal` — The class of literal values.
///
/// Label: "Literal"
///
/// Description: "The class of literal values such as strings and integers."
pub const LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";

/// `rdfs:Datatype` — The class of RDF datatypes.
///
/// Label: "Datatype"
///
/// Description: "The class of RDF datatypes."
pub const DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";

/// `rdfs:Container` — The class of RDF containers.
///
/// Label: "Container"
///
/// Description: "The class of RDF containers."
pub const CONTAINER: &str = "http://www.w3.org/2000/01/rdf-schema#Container";

/// `rdfs:ContainerMembershipProperty` — Properties used to state container membership.
///
/// Label: "`ContainerMembershipProperty`"
///
/// Description: "The class of container membership properties, rdf:_1, rdf:_2, ... all of which are sub-properties of rdfs:member."
pub const CONTAINER_MEMBERSHIP_PROPERTY: &str =
    "http://www.w3.org/2000/01/rdf-schema#ContainerMembershipProperty";

/// `rdfs:subClassOf` — Relates a class to a more general class.
///
/// Label: "subClassOf"
///
/// Description: "Indicates that all the instances of one class are instances of another."
pub const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";

/// `rdfs:subPropertyOf` — Relates a property to a more general property.
///
/// Label: "subPropertyOf"
///
/// Description: "Indicates that one property is a sub-property of another."
pub const SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";

/// `rdfs:domain` — The domain of an RDF property.
///
/// Label: "domain"
///
/// Description: "Indicates the class that subjects of triples using this property must belong to."
pub const DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";

/// `rdfs:range` — The range of an RDF property.
///
/// Label: "range"
///
/// Description: "Indicates the class or datatype that values of this property must belong to."
pub const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";

/// `rdfs:label` — A human-readable label for a resource.
///
/// Label: "label"
///
/// Description: "A human-readable name for the subject."
pub const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";

/// `rdfs:comment` — A human-readable description of a resource.
///
/// Label: "comment"
///
/// Description: "A description of the subject resource."
pub const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";

/// `rdfs:seeAlso` — References a further description of the resource.
///
/// Label: "seeAlso"
///
/// Description: "Indicates a resource that might provide additional information about the subject resource."
pub const SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";

/// `rdfs:isDefinedBy` — The definition source of a resource.
///
/// Label: "isDefinedBy"
///
/// Description: "Indicates a resource defining the subject resource. This property may be used to indicate an RDF vocabulary in which a resource is described."
pub const IS_DEFINED_BY: &str = "http://www.w3.org/2000/01/rdf-schema#isDefinedBy";

/// `rdfs:member` — A member of a container.
///
/// Label: "member"
///
/// Description: "A member of the subject resource."
pub const MEMBER: &str = "http://www.w3.org/2000/01/rdf-schema#member";
