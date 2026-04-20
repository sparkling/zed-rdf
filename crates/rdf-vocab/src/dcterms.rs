//! Dublin Core Terms (`DCTerms`) vocabulary terms.
//!
//! Namespace: `http://purl.org/dc/terms/`
//! Reference: <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/>

/// `DCTerms` namespace IRI (trailing `/`).
pub const NS: &str = "http://purl.org/dc/terms/";

// ── Properties ────────────────────────────────────────────────────────────────

/// `dcterms:title` — A name given to the resource.
///
/// Label: "Title"
///
/// Description: "A name given to the resource."
pub const TITLE: &str = "http://purl.org/dc/terms/title";

/// `dcterms:description` — An account of the resource.
///
/// Label: "Description"
///
/// Description: "An account of the resource. May include abstract, table of contents, graphical representation, or a free-text account."
pub const DESCRIPTION: &str = "http://purl.org/dc/terms/description";

/// `dcterms:subject` — The topic of the resource.
///
/// Label: "Subject"
///
/// Description: "The topic of the resource."
pub const SUBJECT: &str = "http://purl.org/dc/terms/subject";

/// `dcterms:creator` — An entity responsible for making the resource.
///
/// Label: "Creator"
///
/// Description: "An entity responsible for making the resource."
pub const CREATOR: &str = "http://purl.org/dc/terms/creator";

/// `dcterms:contributor` — An entity responsible for making contributions to the resource.
///
/// Label: "Contributor"
///
/// Description: "An entity responsible for making contributions to the resource."
pub const CONTRIBUTOR: &str = "http://purl.org/dc/terms/contributor";

/// `dcterms:publisher` — An entity responsible for making the resource available.
///
/// Label: "Publisher"
///
/// Description: "An entity responsible for making the resource available."
pub const PUBLISHER: &str = "http://purl.org/dc/terms/publisher";

/// `dcterms:date` — A point or period of time associated with an event in the resource lifecycle.
///
/// Label: "Date"
///
/// Description: "A point or period of time associated with an event in the lifecycle of the resource."
pub const DATE: &str = "http://purl.org/dc/terms/date";

/// `dcterms:type` — The nature or genre of the resource.
///
/// Label: "Type"
///
/// Description: "The nature or genre of the resource."
pub const TYPE: &str = "http://purl.org/dc/terms/type";

/// `dcterms:format` — The file format, physical medium, or dimensions of the resource.
///
/// Label: "Format"
///
/// Description: "The file format, physical medium, or dimensions of the resource."
pub const FORMAT: &str = "http://purl.org/dc/terms/format";

/// `dcterms:identifier` — An unambiguous reference to the resource within a given context.
///
/// Label: "Identifier"
///
/// Description: "An unambiguous reference to the resource within a given context."
pub const IDENTIFIER: &str = "http://purl.org/dc/terms/identifier";

/// `dcterms:language` — A language of the resource.
///
/// Label: "Language"
///
/// Description: "A language of the resource."
pub const LANGUAGE: &str = "http://purl.org/dc/terms/language";

/// `dcterms:relation` — A related resource.
///
/// Label: "Relation"
///
/// Description: "A related resource."
pub const RELATION: &str = "http://purl.org/dc/terms/relation";

/// `dcterms:coverage` — The spatial or temporal topic of the resource.
///
/// Label: "Coverage"
///
/// Description: "The spatial or temporal topic of the resource, the spatial applicability of the resource, or the jurisdiction under which the resource is relevant."
pub const COVERAGE: &str = "http://purl.org/dc/terms/coverage";

/// `dcterms:rights` — Information about rights held in and over the resource.
///
/// Label: "Rights"
///
/// Description: "Information about rights held in and over the resource."
pub const RIGHTS: &str = "http://purl.org/dc/terms/rights";

/// `dcterms:source` — A related resource from which the described resource is derived.
///
/// Label: "Source"
///
/// Description: "A related resource from which the described resource is derived."
pub const SOURCE: &str = "http://purl.org/dc/terms/source";

/// `dcterms:abstract` — A summary of the resource.
///
/// Label: "Abstract"
///
/// Description: "A summary of the resource."
pub const ABSTRACT: &str = "http://purl.org/dc/terms/abstract";

/// `dcterms:accessRights` — Information about who access the resource or indication of security status.
///
/// Label: "Access Rights"
///
/// Description: "Information about who can access the resource or an indication of its security status."
pub const ACCESS_RIGHTS: &str = "http://purl.org/dc/terms/accessRights";

/// `dcterms:accrualMethod` — The method by which items are added to a collection.
///
/// Label: "Accrual Method"
///
/// Description: "The method by which items are added to a collection."
pub const ACCRUAL_METHOD: &str = "http://purl.org/dc/terms/accrualMethod";

/// `dcterms:accrualPeriodicity` — The frequency with which items are added to a collection.
///
/// Label: "Accrual Periodicity"
///
/// Description: "The frequency with which items are added to a collection."
pub const ACCRUAL_PERIODICITY: &str = "http://purl.org/dc/terms/accrualPeriodicity";

/// `dcterms:accrualPolicy` — The policy governing the addition of items to a collection.
///
/// Label: "Accrual Policy"
///
/// Description: "The policy governing the addition of items to a collection."
pub const ACCRUAL_POLICY: &str = "http://purl.org/dc/terms/accrualPolicy";

/// `dcterms:alternative` — An alternative name for the resource.
///
/// Label: "Alternative Title"
///
/// Description: "An alternative name for the resource."
pub const ALTERNATIVE: &str = "http://purl.org/dc/terms/alternative";

/// `dcterms:audience` — A class of agents for whom the resource is intended or useful.
///
/// Label: "Audience"
///
/// Description: "A class of agents for whom the resource is intended or useful."
pub const AUDIENCE: &str = "http://purl.org/dc/terms/audience";

/// `dcterms:available` — Date that the resource became or will become available.
///
/// Label: "Date Available"
///
/// Description: "Date (often a range) that the resource became or will become available."
pub const AVAILABLE: &str = "http://purl.org/dc/terms/available";

/// `dcterms:bibliographicCitation` — A bibliographic reference for the resource.
///
/// Label: "Bibliographic Citation"
///
/// Description: "A bibliographic reference for the resource."
pub const BIBLIOGRAPHIC_CITATION: &str =
    "http://purl.org/dc/terms/bibliographicCitation";

/// `dcterms:conformsTo` — An established standard to which the described resource conforms.
///
/// Label: "Conforms To"
///
/// Description: "An established standard to which the described resource conforms."
pub const CONFORMS_TO: &str = "http://purl.org/dc/terms/conformsTo";

/// `dcterms:created` — Date of creation of the resource.
///
/// Label: "Date Created"
///
/// Description: "Date of creation of the resource."
pub const CREATED: &str = "http://purl.org/dc/terms/created";

/// `dcterms:dateAccepted` — Date of acceptance of the resource.
///
/// Label: "Date Accepted"
///
/// Description: "Date of acceptance of the resource (e.g. of thesis by university department, of article by journal, etc.)."
pub const DATE_ACCEPTED: &str = "http://purl.org/dc/terms/dateAccepted";

/// `dcterms:dateCopyrighted` — Date of copyright of the resource.
///
/// Label: "Date Copyrighted"
///
/// Description: "Date of copyright of the resource."
pub const DATE_COPYRIGHTED: &str = "http://purl.org/dc/terms/dateCopyrighted";

/// `dcterms:dateSubmitted` — Date of submission of the resource.
///
/// Label: "Date Submitted"
///
/// Description: "Date of submission of the resource (e.g. thesis, articles, etc.)."
pub const DATE_SUBMITTED: &str = "http://purl.org/dc/terms/dateSubmitted";

/// `dcterms:educationLevel` — A class of agents, defined in terms of progression through an educational or training context.
///
/// Label: "Audience Education Level"
///
/// Description: "A class of agents, defined in terms of progression through an educational or training context, for which the described resource is intended."
pub const EDUCATION_LEVEL: &str = "http://purl.org/dc/terms/educationLevel";

/// `dcterms:extent` — The size or duration of the resource.
///
/// Label: "Extent"
///
/// Description: "The size or duration of the resource."
pub const EXTENT: &str = "http://purl.org/dc/terms/extent";

/// `dcterms:hasFormat` — A pre-existing related resource that is substantially the same as the described resource, but in another format.
///
/// Label: "Has Format"
///
/// Description: "A pre-existing related resource that is substantially the same as the described resource, but in another format."
pub const HAS_FORMAT: &str = "http://purl.org/dc/terms/hasFormat";

/// `dcterms:hasPart` — A related resource that is included either physically or logically in the described resource.
///
/// Label: "Has Part"
///
/// Description: "A related resource that is included either physically or logically in the described resource."
pub const HAS_PART: &str = "http://purl.org/dc/terms/hasPart";

/// `dcterms:hasVersion` — A related resource that is a version, edition, or adaptation of the described resource.
///
/// Label: "Has Version"
///
/// Description: "A related resource that is a version, edition, or adaptation of the described resource."
pub const HAS_VERSION: &str = "http://purl.org/dc/terms/hasVersion";

/// `dcterms:instructionalMethod` — A process, used to engender knowledge, attitudes and skills, that the described resource is designed to support.
///
/// Label: "Instructional Method"
///
/// Description: "A process, used to engender knowledge, attitudes and skills, that the described resource is designed to support."
pub const INSTRUCTIONAL_METHOD: &str = "http://purl.org/dc/terms/instructionalMethod";

/// `dcterms:isFormatOf` — A pre-existing related resource that is substantially the same as the described resource, but in another format.
///
/// Label: "Is Format Of"
///
/// Description: "A pre-existing related resource that is substantially the same as the described resource, but in another format."
pub const IS_FORMAT_OF: &str = "http://purl.org/dc/terms/isFormatOf";

/// `dcterms:isPartOf` — A related resource in which the described resource is physically or logically included.
///
/// Label: "Is Part Of"
///
/// Description: "A related resource in which the described resource is physically or logically included."
pub const IS_PART_OF: &str = "http://purl.org/dc/terms/isPartOf";

/// `dcterms:isReferencedBy` — A related resource that references, cites, or otherwise points to the described resource.
///
/// Label: "Is Referenced By"
///
/// Description: "A related resource that references, cites, or otherwise points to the described resource."
pub const IS_REFERENCED_BY: &str = "http://purl.org/dc/terms/isReferencedBy";

/// `dcterms:isReplacedBy` — A related resource that supplants, displaces, or supersedes the described resource.
///
/// Label: "Is Replaced By"
///
/// Description: "A related resource that supplants, displaces, or supersedes the described resource."
pub const IS_REPLACED_BY: &str = "http://purl.org/dc/terms/isReplacedBy";

/// `dcterms:isRequiredBy` — A related resource that requires the described resource to support its function, delivery, or coherence.
///
/// Label: "Is Required By"
///
/// Description: "A related resource that requires the described resource to support its function, delivery, or coherence."
pub const IS_REQUIRED_BY: &str = "http://purl.org/dc/terms/isRequiredBy";

/// `dcterms:isVersionOf` — A related resource of which the described resource is a version, edition, or adaptation.
///
/// Label: "Is Version Of"
///
/// Description: "A related resource of which the described resource is a version, edition, or adaptation."
pub const IS_VERSION_OF: &str = "http://purl.org/dc/terms/isVersionOf";

/// `dcterms:issued` — Date of formal issuance of the resource.
///
/// Label: "Date Issued"
///
/// Description: "Date of formal issuance of the resource."
pub const ISSUED: &str = "http://purl.org/dc/terms/issued";

/// `dcterms:license` — A legal document giving official permission to do something with the resource.
///
/// Label: "License"
///
/// Description: "A legal document giving official permission to do something with the resource."
pub const LICENSE: &str = "http://purl.org/dc/terms/license";

/// `dcterms:mediator` — An entity that mediates access to the resource and for whose benefit the resource has been created.
///
/// Label: "Mediator"
///
/// Description: "An entity that mediates access to the resource and for whose benefit the resource has been created."
pub const MEDIATOR: &str = "http://purl.org/dc/terms/mediator";

/// `dcterms:medium` — The material or physical carrier of the resource.
///
/// Label: "Medium"
///
/// Description: "The material or physical carrier of the resource."
pub const MEDIUM: &str = "http://purl.org/dc/terms/medium";

/// `dcterms:modified` — Date on which the resource was changed.
///
/// Label: "Date Modified"
///
/// Description: "Date on which the resource was changed."
pub const MODIFIED: &str = "http://purl.org/dc/terms/modified";

/// `dcterms:provenance` — A statement of any changes in ownership and custody of the resource since its creation.
///
/// Label: "Provenance"
///
/// Description: "A statement of any changes in ownership and custody of the resource since its creation that are significant for its authenticity, integrity, and interpretation."
pub const PROVENANCE: &str = "http://purl.org/dc/terms/provenance";

/// `dcterms:references` — A related resource that is referenced, cited, or otherwise pointed to by the described resource.
///
/// Label: "References"
///
/// Description: "A related resource that is referenced, cited, or otherwise pointed to by the described resource."
pub const REFERENCES: &str = "http://purl.org/dc/terms/references";

/// `dcterms:replaces` — A related resource that is supplanted, displaced, or superseded by the described resource.
///
/// Label: "Replaces"
///
/// Description: "A related resource that is supplanted, displaced, or superseded by the described resource."
pub const REPLACES: &str = "http://purl.org/dc/terms/replaces";

/// `dcterms:requires` — A related resource that is required by the described resource to support its function, delivery, or coherence.
///
/// Label: "Requires"
///
/// Description: "A related resource that is required by the described resource to support its function, delivery, or coherence."
pub const REQUIRES: &str = "http://purl.org/dc/terms/requires";

/// `dcterms:rightsHolder` — A person or organization owning or managing rights over the resource.
///
/// Label: "Rights Holder"
///
/// Description: "A person or organization owning or managing rights over the resource."
pub const RIGHTS_HOLDER: &str = "http://purl.org/dc/terms/rightsHolder";

/// `dcterms:spatial` — Spatial characteristics of the resource.
///
/// Label: "Spatial Coverage"
///
/// Description: "Spatial characteristics of the resource."
pub const SPATIAL: &str = "http://purl.org/dc/terms/spatial";

/// `dcterms:tableOfContents` — A list of subunits of the resource.
///
/// Label: "Table Of Contents"
///
/// Description: "A list of subunits of the resource."
pub const TABLE_OF_CONTENTS: &str = "http://purl.org/dc/terms/tableOfContents";

/// `dcterms:temporal` — Temporal characteristics of the resource.
///
/// Label: "Temporal Coverage"
///
/// Description: "Temporal characteristics of the resource."
pub const TEMPORAL: &str = "http://purl.org/dc/terms/temporal";

/// `dcterms:valid` — Date (often a range) of validity of a resource.
///
/// Label: "Date Valid"
///
/// Description: "Date (often a range) of validity of a resource."
pub const VALID: &str = "http://purl.org/dc/terms/valid";

// ── Classes ───────────────────────────────────────────────────────────────────

/// `dcterms:Agent` — A resource that acts or has the power to act.
///
/// Label: "Agent"
///
/// Description: "A resource that acts or has the power to act."
pub const AGENT: &str = "http://purl.org/dc/terms/Agent";

/// `dcterms:AgentClass` — A group of agents.
///
/// Label: "Agent Class"
///
/// Description: "A group of agents."
pub const AGENT_CLASS: &str = "http://purl.org/dc/terms/AgentClass";

/// `dcterms:BibliographicResource` — A book, article, or other documentary resource.
///
/// Label: "Bibliographic Resource"
///
/// Description: "A book, article, or other documentary resource."
pub const BIBLIOGRAPHIC_RESOURCE: &str =
    "http://purl.org/dc/terms/BibliographicResource";

/// `dcterms:FileFormat` — A digital document format.
///
/// Label: "File Format"
///
/// Description: "A digital document format."
pub const FILE_FORMAT: &str = "http://purl.org/dc/terms/FileFormat";

/// `dcterms:Frequency` — A rate at which something recurs.
///
/// Label: "Frequency"
///
/// Description: "A rate at which something recurs."
pub const FREQUENCY: &str = "http://purl.org/dc/terms/Frequency";

/// `dcterms:Jurisdiction` — The extent or range of judicial, law enforcement, or other authority.
///
/// Label: "Jurisdiction"
///
/// Description: "The extent or range of judicial, law enforcement, or other authority."
pub const JURISDICTION: &str = "http://purl.org/dc/terms/Jurisdiction";

/// `dcterms:LicenseDocument` — A legal document giving official permission to do something with a resource.
///
/// Label: "License Document"
///
/// Description: "A legal document giving official permission to do something with a resource."
pub const LICENSE_DOCUMENT: &str = "http://purl.org/dc/terms/LicenseDocument";

/// `dcterms:LinguisticSystem` — A system of signs, symbols, sounds, gestures, or rules used in communication.
///
/// Label: "Linguistic System"
///
/// Description: "A system of signs, symbols, sounds, gestures, or rules used in communication."
pub const LINGUISTIC_SYSTEM: &str = "http://purl.org/dc/terms/LinguisticSystem";

/// `dcterms:Location` — A spatial region or named place.
///
/// Label: "Location"
///
/// Description: "A spatial region or named place."
pub const LOCATION: &str = "http://purl.org/dc/terms/Location";

/// `dcterms:MediaType` — A file format or physical medium.
///
/// Label: "Media Type"
///
/// Description: "A file format or physical medium."
pub const MEDIA_TYPE: &str = "http://purl.org/dc/terms/MediaType";

/// `dcterms:MethodOfAccrual` — A method by which resources are added to a collection.
///
/// Label: "Method of Accrual"
///
/// Description: "A method by which resources are added to a collection."
pub const METHOD_OF_ACCRUAL: &str = "http://purl.org/dc/terms/MethodOfAccrual";

/// `dcterms:MethodOfInstruction` — A process that is used to engender knowledge, attitudes, or skills.
///
/// Label: "Method of Instruction"
///
/// Description: "A process that is used to engender knowledge, attitudes, or skills."
pub const METHOD_OF_INSTRUCTION: &str = "http://purl.org/dc/terms/MethodOfInstruction";

/// `dcterms:PeriodOfTime` — An interval of time that is named or defined by its start and end dates.
///
/// Label: "Period of Time"
///
/// Description: "An interval of time that is named or defined by its start and end dates."
pub const PERIOD_OF_TIME: &str = "http://purl.org/dc/terms/PeriodOfTime";

/// `dcterms:PhysicalMedium` — A physical material or carrier.
///
/// Label: "Physical Medium"
///
/// Description: "A physical material or carrier."
pub const PHYSICAL_MEDIUM: &str = "http://purl.org/dc/terms/PhysicalMedium";

/// `dcterms:PhysicalResource` — A material thing.
///
/// Label: "Physical Resource"
///
/// Description: "A material thing."
pub const PHYSICAL_RESOURCE: &str = "http://purl.org/dc/terms/PhysicalResource";

/// `dcterms:Policy` — A plan or course of action by an authority.
///
/// Label: "Policy"
///
/// Description: "A plan or course of action by an authority, intended to influence and determine decisions, actions, and other matters."
pub const POLICY: &str = "http://purl.org/dc/terms/Policy";

/// `dcterms:ProvenanceStatement` — Any changes in ownership and custody of a resource.
///
/// Label: "Provenance Statement"
///
/// Description: "A statement of any changes in ownership and custody of a resource since its creation that are significant for its authenticity, integrity, and interpretation."
pub const PROVENANCE_STATEMENT: &str = "http://purl.org/dc/terms/ProvenanceStatement";

/// `dcterms:RightsStatement` — A statement about the intellectual property rights held in or over a resource.
///
/// Label: "Rights Statement"
///
/// Description: "A statement about the intellectual property rights (IPR) held in or over a resource, a legal document giving official permission to do something with a resource, or a statement about access rights."
pub const RIGHTS_STATEMENT: &str = "http://purl.org/dc/terms/RightsStatement";

/// `dcterms:SizeOrDuration` — A dimension or extent, or a time taken to play or execute.
///
/// Label: "Size or Duration"
///
/// Description: "A dimension or extent, or a time taken to play or execute."
pub const SIZE_OR_DURATION: &str = "http://purl.org/dc/terms/SizeOrDuration";

/// `dcterms:Standard` — A reference point against which other things can be evaluated or compared.
///
/// Label: "Standard"
///
/// Description: "A reference point against which other things can be evaluated or compared."
pub const STANDARD: &str = "http://purl.org/dc/terms/Standard";
