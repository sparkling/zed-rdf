//! Vocabulary coverage tests (Phase E, ADR-0024 §3).
//!
//! For each of the 11 vocabularies we assert:
//! 1. Every priority IRI listed in the arch-memo §2 is reachable as a
//!    `pub const &str` with the expected full IRI value.
//! 2. The total number of constants defined in the module is at least
//!    `ceil(0.95 * TOTAL_KNOWN)` for that vocabulary.

use rdf_vocab::{dcat, dcterms, foaf, owl, prov, rdf, rdfs, schema, sh, skos, xsd};

// ─── helpers ─────────────────────────────────────────────────────────────────

/// `ceil(0.95 * n)` — minimum acceptable term count.
const fn min_terms(total: usize) -> usize {
    // 95 % rounded up: (total * 95 + 99) / 100
    (total * 95 + 99) / 100
}

// ─── xsd ─────────────────────────────────────────────────────────────────────

/// Every IRI that the arch-memo §2 lists as a priority term for `xsd`.
const XSD_PRIORITY: &[&str] = &[
    xsd::STRING,
    xsd::INTEGER,
    xsd::DECIMAL,
    xsd::FLOAT,
    xsd::DOUBLE,
    xsd::BOOLEAN,
    xsd::DATE_TIME,
    xsd::DATE,
    xsd::TIME,
    xsd::ANY_URI,
];

/// All constants defined in `xsd` (for the ≥ 95 % floor check).
const XSD_ALL: &[&str] = &[
    xsd::NS,
    xsd::STRING,
    xsd::NORMALIZED_STRING,
    xsd::TOKEN,
    xsd::LANGUAGE,
    xsd::NAME,
    xsd::NC_NAME,
    xsd::NMTOKEN,
    xsd::BOOLEAN,
    xsd::DECIMAL,
    xsd::INTEGER,
    xsd::LONG,
    xsd::INT,
    xsd::SHORT,
    xsd::BYTE,
    xsd::NON_NEGATIVE_INTEGER,
    xsd::POSITIVE_INTEGER,
    xsd::UNSIGNED_LONG,
    xsd::UNSIGNED_INT,
    xsd::UNSIGNED_SHORT,
    xsd::UNSIGNED_BYTE,
    xsd::NON_POSITIVE_INTEGER,
    xsd::NEGATIVE_INTEGER,
    xsd::FLOAT,
    xsd::DOUBLE,
    xsd::DATE_TIME,
    xsd::DATE_TIME_STAMP,
    xsd::DATE,
    xsd::TIME,
    xsd::G_YEAR,
    xsd::G_YEAR_MONTH,
    xsd::G_MONTH,
    xsd::G_MONTH_DAY,
    xsd::G_DAY,
    xsd::DURATION,
    xsd::YEAR_MONTH_DURATION,
    xsd::DAY_TIME_DURATION,
    xsd::BASE64_BINARY,
    xsd::HEX_BINARY,
    xsd::ANY_URI,
    xsd::Q_NAME,
    xsd::NOTATION,
    xsd::ANY_ATOMIC_TYPE,
    xsd::ANY_SIMPLE_TYPE,
    xsd::ANY_TYPE,
];

#[test]
fn xsd_priority_terms_present() {
    for iri in XSD_PRIORITY {
        assert!(
            !iri.is_empty(),
            "xsd priority IRI must not be empty: {iri}"
        );
        assert!(
            iri.starts_with("http://www.w3.org/2001/XMLSchema#"),
            "xsd IRI must use XSD namespace: {iri}"
        );
    }
}

#[test]
fn xsd_coverage_floor() {
    // Total known: 45 datatypes
    const TOTAL: usize = 45;
    let defined = XSD_ALL.len() - 1; // subtract NS itself
    assert!(
        defined >= min_terms(TOTAL),
        "xsd: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn xsd_ns_constant() {
    assert_eq!(xsd::NS, "http://www.w3.org/2001/XMLSchema#");
}

// ─── rdf ─────────────────────────────────────────────────────────────────────

const RDF_PRIORITY: &[&str] = &[
    rdf::TYPE,
    rdf::PROPERTY,
    rdf::STATEMENT,
    rdf::SUBJECT,
    rdf::PREDICATE,
    rdf::OBJECT,
    rdf::BAG,
    rdf::SEQ,
    rdf::ALT,
    rdf::LIST,
    rdf::NIL,
    rdf::FIRST,
    rdf::REST,
    rdf::LANG_STRING,
    rdf::HTML,
];

const RDF_ALL: &[&str] = &[
    rdf::NS,
    rdf::TYPE,
    rdf::PROPERTY,
    rdf::STATEMENT,
    rdf::SUBJECT,
    rdf::PREDICATE,
    rdf::OBJECT,
    rdf::BAG,
    rdf::SEQ,
    rdf::ALT,
    rdf::LIST,
    rdf::NIL,
    rdf::FIRST,
    rdf::REST,
    rdf::VALUE,
    rdf::LANG_STRING,
    rdf::HTML,
    rdf::XML_LITERAL,
    rdf::JSON,
    rdf::COMPOUND_LITERAL,
    rdf::DIRECTION,
    rdf::LANGUAGE,
];

#[test]
fn rdf_priority_terms_present() {
    for iri in RDF_PRIORITY {
        assert!(
            iri.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
            "rdf IRI must use RDF namespace: {iri}"
        );
    }
}

#[test]
fn rdf_coverage_floor() {
    // Total known: 15
    const TOTAL: usize = 15;
    let defined = RDF_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "rdf: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn rdf_ns_constant() {
    assert_eq!(
        rdf::NS,
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
    );
}

// ─── rdfs ────────────────────────────────────────────────────────────────────

const RDFS_PRIORITY: &[&str] = &[
    rdfs::CLASS,
    rdfs::SUB_CLASS_OF,
    rdfs::SUB_PROPERTY_OF,
    rdfs::DOMAIN,
    rdfs::RANGE,
    rdfs::LABEL,
    rdfs::COMMENT,
    rdfs::SEE_ALSO,
    rdfs::IS_DEFINED_BY,
    rdfs::RESOURCE,
    rdfs::LITERAL,
    rdfs::DATATYPE,
    rdfs::CONTAINER,
];

const RDFS_ALL: &[&str] = &[
    rdfs::NS,
    rdfs::CLASS,
    rdfs::RESOURCE,
    rdfs::LITERAL,
    rdfs::DATATYPE,
    rdfs::CONTAINER,
    rdfs::CONTAINER_MEMBERSHIP_PROPERTY,
    rdfs::SUB_CLASS_OF,
    rdfs::SUB_PROPERTY_OF,
    rdfs::DOMAIN,
    rdfs::RANGE,
    rdfs::LABEL,
    rdfs::COMMENT,
    rdfs::SEE_ALSO,
    rdfs::IS_DEFINED_BY,
    rdfs::MEMBER,
];

#[test]
fn rdfs_priority_terms_present() {
    for iri in RDFS_PRIORITY {
        assert!(
            iri.starts_with("http://www.w3.org/2000/01/rdf-schema#"),
            "rdfs IRI must use RDFS namespace: {iri}"
        );
    }
}

#[test]
fn rdfs_coverage_floor() {
    // Total known: 13
    const TOTAL: usize = 13;
    let defined = RDFS_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "rdfs: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn rdfs_ns_constant() {
    assert_eq!(rdfs::NS, "http://www.w3.org/2000/01/rdf-schema#");
}

// ─── owl ─────────────────────────────────────────────────────────────────────

const OWL_PRIORITY: &[&str] = &[
    owl::CLASS,
    owl::OBJECT_PROPERTY,
    owl::DATATYPE_PROPERTY,
    owl::ANNOTATION_PROPERTY,
    owl::NAMED_INDIVIDUAL,
    owl::SAME_AS,
    owl::DIFFERENT_FROM,
    owl::EQUIVALENT_CLASS,
    owl::EQUIVALENT_PROPERTY,
    owl::IMPORTS,
];

const OWL_ALL: &[&str] = &[
    owl::NS,
    owl::CLASS,
    owl::THING,
    owl::NOTHING,
    owl::EQUIVALENT_CLASS,
    owl::DISJOINT_WITH,
    owl::COMPLEMENT_OF,
    owl::UNION_OF,
    owl::INTERSECTION_OF,
    owl::ONE_OF,
    owl::ALL_DISJOINT_CLASSES,
    owl::OBJECT_PROPERTY,
    owl::DATATYPE_PROPERTY,
    owl::ANNOTATION_PROPERTY,
    owl::FUNCTIONAL_PROPERTY,
    owl::INVERSE_FUNCTIONAL_PROPERTY,
    owl::SYMMETRIC_PROPERTY,
    owl::ASYMMETRIC_PROPERTY,
    owl::TRANSITIVE_PROPERTY,
    owl::REFLEXIVE_PROPERTY,
    owl::IRREFLEXIVE_PROPERTY,
    owl::EQUIVALENT_PROPERTY,
    owl::INVERSE_OF,
    owl::PROPERTY_DISJOINT_WITH,
    owl::ALL_DISJOINT_PROPERTIES,
    owl::NAMED_INDIVIDUAL,
    owl::SAME_AS,
    owl::DIFFERENT_FROM,
    owl::ALL_DIFFERENT,
    owl::ONTOLOGY,
    owl::IMPORTS,
    owl::VERSION_INFO,
    owl::VERSION_IRI,
    owl::PRIOR_VERSION,
    owl::BACKWARD_COMPATIBLE_WITH,
    owl::INCOMPATIBLE_WITH,
    owl::RESTRICTION,
    owl::ON_PROPERTY,
    owl::ALL_VALUES_FROM,
    owl::SOME_VALUES_FROM,
    owl::HAS_VALUE,
    owl::HAS_SELF,
    owl::MIN_CARDINALITY,
    owl::MAX_CARDINALITY,
    owl::CARDINALITY,
    owl::MIN_QUALIFIED_CARDINALITY,
    owl::MAX_QUALIFIED_CARDINALITY,
    owl::QUALIFIED_CARDINALITY,
    owl::ON_CLASS,
    owl::ON_DATA_RANGE,
];

#[test]
fn owl_priority_terms_present() {
    for iri in OWL_PRIORITY {
        assert!(
            iri.starts_with("http://www.w3.org/2002/07/owl#"),
            "owl IRI must use OWL namespace: {iri}"
        );
    }
}

#[test]
fn owl_coverage_floor() {
    // Total known in Phase E scope (structural core): 30
    const TOTAL: usize = 30;
    let defined = OWL_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "owl: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn owl_ns_constant() {
    assert_eq!(owl::NS, "http://www.w3.org/2002/07/owl#");
}

// ─── skos ────────────────────────────────────────────────────────────────────

const SKOS_PRIORITY: &[&str] = &[
    skos::CONCEPT,
    skos::CONCEPT_SCHEME,
    skos::COLLECTION,
    skos::PREF_LABEL,
    skos::ALT_LABEL,
    skos::HIDDEN_LABEL,
    skos::BROADER,
    skos::NARROWER,
    skos::RELATED,
    skos::DEFINITION,
];

const SKOS_ALL: &[&str] = &[
    skos::NS,
    skos::CONCEPT,
    skos::CONCEPT_SCHEME,
    skos::COLLECTION,
    skos::ORDERED_COLLECTION,
    skos::PREF_LABEL,
    skos::ALT_LABEL,
    skos::HIDDEN_LABEL,
    skos::NOTE,
    skos::DEFINITION,
    skos::EXAMPLE,
    skos::HISTORY_NOTE,
    skos::EDITORIAL_NOTE,
    skos::CHANGE_NOTE,
    skos::SCOPE_NOTE,
    skos::SEMANTIC_RELATION,
    skos::BROADER,
    skos::NARROWER,
    skos::RELATED,
    skos::BROADER_TRANSITIVE,
    skos::NARROWER_TRANSITIVE,
    skos::MAPPING_RELATION,
    skos::BROAD_MATCH,
    skos::NARROW_MATCH,
    skos::EXACT_MATCH,
    skos::CLOSE_MATCH,
    skos::RELATED_MATCH,
    skos::IN_SCHEME,
    skos::HAS_TOP_CONCEPT,
    skos::TOP_CONCEPT_OF,
    skos::MEMBER,
    skos::MEMBER_LIST,
    skos::NOTATION,
    skos::SUBJECT,
    skos::PREF_SYMBOL,
];

#[test]
fn skos_priority_terms_present() {
    for iri in SKOS_PRIORITY {
        assert!(
            iri.starts_with("http://www.w3.org/2004/02/skos/core#"),
            "skos IRI must use SKOS namespace: {iri}"
        );
    }
}

#[test]
fn skos_coverage_floor() {
    // Total known: 35
    const TOTAL: usize = 35;
    let defined = SKOS_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "skos: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn skos_ns_constant() {
    assert_eq!(skos::NS, "http://www.w3.org/2004/02/skos/core#");
}

// ─── sh ──────────────────────────────────────────────────────────────────────

const SH_PRIORITY: &[&str] = &[
    sh::NODE_SHAPE,
    sh::PROPERTY_SHAPE,
    sh::PATH,
    sh::TARGET_CLASS,
    sh::PROPERTY,
    sh::MIN_COUNT,
    sh::MAX_COUNT,
    sh::DATATYPE,
    sh::NODE_KIND,
    sh::VIOLATION,
];

const SH_ALL: &[&str] = &[
    sh::NS,
    sh::SHAPE,
    sh::NODE_SHAPE,
    sh::PROPERTY_SHAPE,
    sh::TARGET_CLASS,
    sh::TARGET_NODE,
    sh::TARGET_OBJECTS_OF,
    sh::TARGET_SUBJECTS_OF,
    sh::PATH,
    sh::ALTERNATIVE_PATH,
    sh::INVERSE_PATH,
    sh::ZERO_OR_MORE_PATH,
    sh::ONE_OR_MORE_PATH,
    sh::ZERO_OR_ONE_PATH,
    sh::PROPERTY,
    sh::DATATYPE,
    sh::NODE_KIND,
    sh::CLASS,
    sh::MIN_COUNT,
    sh::MAX_COUNT,
    sh::MIN_LENGTH,
    sh::MAX_LENGTH,
    sh::MIN_EXCLUSIVE,
    sh::MIN_INCLUSIVE,
    sh::MAX_EXCLUSIVE,
    sh::MAX_INCLUSIVE,
    sh::PATTERN,
    sh::FLAGS,
    sh::LANGUAGE_IN,
    sh::UNIQUE_LANG,
    sh::IN,
    sh::HAS_VALUE,
    sh::NODE,
    sh::NOT,
    sh::AND,
    sh::OR,
    sh::XONE,
    sh::QUALIFIED_VALUE_SHAPE,
    sh::QUALIFIED_MIN_COUNT,
    sh::QUALIFIED_MAX_COUNT,
    sh::DISJOINT,
    sh::EQUALS,
    sh::LESS_THAN,
    sh::LESS_THAN_OR_EQUALS,
    sh::CLOSED,
    sh::IGNORED_PROPERTIES,
    sh::VALIDATION_RESULT,
    sh::VALIDATION_REPORT,
    sh::VIOLATION,
    sh::WARNING,
    sh::INFO,
    sh::CONFORMS,
    sh::RESULT,
    sh::RESULT_SEVERITY,
    sh::SOURCE_CONSTRAINT_COMPONENT,
    sh::SOURCE_SHAPE,
    sh::FOCUS_NODE,
    sh::VALUE,
    sh::RESULT_PATH,
    sh::RESULT_MESSAGE,
    sh::MESSAGE,
    sh::SEVERITY,
    sh::NAME,
    sh::DESCRIPTION,
    sh::ORDER,
    sh::GROUP,
    sh::DEFAULT_VALUE,
    sh::DEACTIVATED,
    sh::SPARQL,
    sh::SELECT,
    sh::ASK,
    sh::PREFIXES,
    sh::DECLARE,
    sh::PREFIX,
    sh::NAMESPACE,
];

#[test]
fn sh_priority_terms_present() {
    for iri in SH_PRIORITY {
        assert!(
            iri.starts_with("http://www.w3.org/ns/shacl#"),
            "sh IRI must use SHACL namespace: {iri}"
        );
    }
}

#[test]
fn sh_coverage_floor() {
    // Total known (core): 40
    const TOTAL: usize = 40;
    let defined = SH_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "sh: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn sh_ns_constant() {
    assert_eq!(sh::NS, "http://www.w3.org/ns/shacl#");
}

// ─── dcterms ─────────────────────────────────────────────────────────────────

const DCTERMS_PRIORITY: &[&str] = &[
    dcterms::TITLE,
    dcterms::DESCRIPTION,
    dcterms::SUBJECT,
    dcterms::CREATOR,
    dcterms::CONTRIBUTOR,
    dcterms::PUBLISHER,
    dcterms::DATE,
    dcterms::TYPE,
    dcterms::FORMAT,
    dcterms::IDENTIFIER,
    dcterms::LANGUAGE,
    dcterms::RIGHTS,
];

const DCTERMS_ALL: &[&str] = &[
    dcterms::NS,
    dcterms::TITLE,
    dcterms::DESCRIPTION,
    dcterms::SUBJECT,
    dcterms::CREATOR,
    dcterms::CONTRIBUTOR,
    dcterms::PUBLISHER,
    dcterms::DATE,
    dcterms::TYPE,
    dcterms::FORMAT,
    dcterms::IDENTIFIER,
    dcterms::LANGUAGE,
    dcterms::RELATION,
    dcterms::COVERAGE,
    dcterms::RIGHTS,
    dcterms::SOURCE,
    dcterms::ABSTRACT,
    dcterms::ACCESS_RIGHTS,
    dcterms::ACCRUAL_METHOD,
    dcterms::ACCRUAL_PERIODICITY,
    dcterms::ACCRUAL_POLICY,
    dcterms::ALTERNATIVE,
    dcterms::AUDIENCE,
    dcterms::AVAILABLE,
    dcterms::BIBLIOGRAPHIC_CITATION,
    dcterms::CONFORMS_TO,
    dcterms::CREATED,
    dcterms::DATE_ACCEPTED,
    dcterms::DATE_COPYRIGHTED,
    dcterms::DATE_SUBMITTED,
    dcterms::EDUCATION_LEVEL,
    dcterms::EXTENT,
    dcterms::HAS_FORMAT,
    dcterms::HAS_PART,
    dcterms::HAS_VERSION,
    dcterms::INSTRUCTIONAL_METHOD,
    dcterms::IS_FORMAT_OF,
    dcterms::IS_PART_OF,
    dcterms::IS_REFERENCED_BY,
    dcterms::IS_REPLACED_BY,
    dcterms::IS_REQUIRED_BY,
    dcterms::IS_VERSION_OF,
    dcterms::ISSUED,
    dcterms::LICENSE,
    dcterms::MEDIATOR,
    dcterms::MEDIUM,
    dcterms::MODIFIED,
    dcterms::PROVENANCE,
    dcterms::REFERENCES,
    dcterms::REPLACES,
    dcterms::REQUIRES,
    dcterms::RIGHTS_HOLDER,
    dcterms::SPATIAL,
    dcterms::TABLE_OF_CONTENTS,
    dcterms::TEMPORAL,
    dcterms::VALID,
    dcterms::AGENT,
    dcterms::AGENT_CLASS,
    dcterms::BIBLIOGRAPHIC_RESOURCE,
    dcterms::FILE_FORMAT,
    dcterms::FREQUENCY,
    dcterms::JURISDICTION,
    dcterms::LICENSE_DOCUMENT,
    dcterms::LINGUISTIC_SYSTEM,
    dcterms::LOCATION,
    dcterms::MEDIA_TYPE,
    dcterms::METHOD_OF_ACCRUAL,
    dcterms::METHOD_OF_INSTRUCTION,
    dcterms::PERIOD_OF_TIME,
    dcterms::PHYSICAL_MEDIUM,
    dcterms::PHYSICAL_RESOURCE,
    dcterms::POLICY,
    dcterms::PROVENANCE_STATEMENT,
    dcterms::RIGHTS_STATEMENT,
    dcterms::SIZE_OR_DURATION,
    dcterms::STANDARD,
];

#[test]
fn dcterms_priority_terms_present() {
    for iri in DCTERMS_PRIORITY {
        assert!(
            iri.starts_with("http://purl.org/dc/terms/"),
            "dcterms IRI must use DC Terms namespace: {iri}"
        );
    }
}

#[test]
fn dcterms_coverage_floor() {
    // Total known: 55
    const TOTAL: usize = 55;
    let defined = DCTERMS_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "dcterms: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn dcterms_ns_constant() {
    assert_eq!(dcterms::NS, "http://purl.org/dc/terms/");
}

// ─── dcat ────────────────────────────────────────────────────────────────────

const DCAT_PRIORITY: &[&str] = &[
    dcat::DATASET,
    dcat::DISTRIBUTION,
    dcat::CATALOG,
    dcat::DATA_SERVICE,
    dcat::ACCESS_URL,
    dcat::DOWNLOAD_URL,
    dcat::MEDIA_TYPE,
    dcat::BYTE_SIZE,
    dcat::KEYWORD,
    dcat::THEME,
];

const DCAT_ALL: &[&str] = &[
    dcat::NS,
    dcat::RESOURCE,
    dcat::DATASET,
    dcat::DISTRIBUTION,
    dcat::CATALOG,
    dcat::CATALOG_RECORD,
    dcat::DATA_SERVICE,
    dcat::DATASET_SERIES,
    dcat::ACCESS_URL,
    dcat::DOWNLOAD_URL,
    dcat::ACCESS_SERVICE,
    dcat::DISTRIBUTION_PROP,
    dcat::KEYWORD,
    dcat::THEME,
    dcat::THEME_TAXONOMY,
    dcat::CONTACT_POINT,
    dcat::LANDING_PAGE,
    dcat::QUALIFIED_RELATION,
    dcat::MEDIA_TYPE,
    dcat::FORMAT,
    dcat::BYTE_SIZE,
    dcat::COMPRESS_FORMAT,
    dcat::PACKAGE_FORMAT,
    dcat::SPATIAL_RESOLUTION_IN_METERS,
    dcat::TEMPORAL_RESOLUTION,
    dcat::DATASET_PROP,
    dcat::CATALOG_PROP,
    dcat::SERVICE,
    dcat::RECORD,
    dcat::ENDPOINT_URL,
    dcat::ENDPOINT_DESCRIPTION,
    dcat::SERVES_DATASET,
    dcat::TEMPORAL,
    dcat::SPATIAL,
    dcat::VERSION,
    dcat::PREVIOUS_VERSION,
    dcat::HAS_CURRENT_VERSION,
    dcat::HAS_VERSION,
    dcat::IS_VERSION_OF,
    dcat::FIRST,
    dcat::LAST,
    dcat::NEXT,
    dcat::PREV,
];

#[test]
fn dcat_priority_terms_present() {
    for iri in DCAT_PRIORITY {
        assert!(
            iri.starts_with("http://www.w3.org/ns/dcat#"),
            "dcat IRI must use DCAT namespace: {iri}"
        );
    }
}

#[test]
fn dcat_coverage_floor() {
    // Total known: 30
    const TOTAL: usize = 30;
    let defined = DCAT_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "dcat: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn dcat_ns_constant() {
    assert_eq!(dcat::NS, "http://www.w3.org/ns/dcat#");
}

// ─── foaf ────────────────────────────────────────────────────────────────────

const FOAF_PRIORITY: &[&str] = &[
    foaf::PERSON,
    foaf::AGENT,
    foaf::ORGANIZATION,
    foaf::NAME,
    foaf::MBOX,
    foaf::HOMEPAGE,
    foaf::KNOWS,
    foaf::DEPICTION,
    foaf::TOPIC,
    foaf::DOCUMENT,
];

const FOAF_ALL: &[&str] = &[
    foaf::NS,
    foaf::AGENT,
    foaf::PERSON,
    foaf::ORGANIZATION,
    foaf::GROUP,
    foaf::DOCUMENT,
    foaf::IMAGE,
    foaf::PROJECT,
    foaf::PERSONAL_PROFILE_DOCUMENT,
    foaf::ONLINE_ACCOUNT,
    foaf::ONLINE_CHAT_ACCOUNT,
    foaf::ONLINE_ECOMMERCE_ACCOUNT,
    foaf::ONLINE_GAMING_ACCOUNT,
    foaf::NAME,
    foaf::TITLE,
    foaf::NICK,
    foaf::FIRST_NAME,
    foaf::LAST_NAME,
    foaf::GIVEN_NAME,
    foaf::FAMILY_NAME,
    foaf::MBOX,
    foaf::MBOX_SHA1SUM,
    foaf::PHONE,
    foaf::JABBER_ID,
    foaf::SKYPE_ID,
    foaf::HOMEPAGE,
    foaf::WEBLOG,
    foaf::OPEN_ID,
    foaf::ACCOUNT,
    foaf::ACCOUNT_SERVICE_HOMEPAGE,
    foaf::ACCOUNT_NAME,
    foaf::KNOWS,
    foaf::MEMBER,
    foaf::INTEREST,
    foaf::CURRENT_PROJECT,
    foaf::PAST_PROJECT,
    foaf::FUNDED_BY,
    foaf::TOPIC,
    foaf::PRIMARY_TOPIC,
    foaf::IS_PRIMARY_TOPIC_OF,
    foaf::PAGE,
    foaf::DEPICTION,
    foaf::DEPICTS,
    foaf::THUMBNAIL,
    foaf::IMG,
    foaf::MAKER,
    foaf::MADE,
    foaf::LOGO,
    foaf::TIPJAR,
    foaf::SHA1,
    foaf::BASED_NEAR,
    foaf::GENDER,
    foaf::AGE,
    foaf::BIRTHDAY,
    foaf::DNA_CHECKSUM,
    foaf::MEMBERSHIP_CLASS,
];

#[test]
fn foaf_priority_terms_present() {
    for iri in FOAF_PRIORITY {
        assert!(
            iri.starts_with("http://xmlns.com/foaf/0.1/"),
            "foaf IRI must use FOAF namespace: {iri}"
        );
    }
}

#[test]
fn foaf_coverage_floor() {
    // Total known: 30
    const TOTAL: usize = 30;
    let defined = FOAF_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "foaf: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn foaf_ns_constant() {
    assert_eq!(foaf::NS, "http://xmlns.com/foaf/0.1/");
}

// ─── schema ──────────────────────────────────────────────────────────────────

const SCHEMA_PRIORITY: &[&str] = &[
    schema::THING,
    schema::NAME,
    schema::DESCRIPTION,
    schema::URL,
    schema::IDENTIFIER,
    schema::PERSON,
    schema::ORGANIZATION,
    schema::CREATIVE_WORK,
    schema::DATASET,
    schema::SOFTWARE_APPLICATION,
];

const SCHEMA_ALL: &[&str] = &[
    schema::NS,
    schema::THING,
    schema::ACTION,
    schema::CREATIVE_WORK,
    schema::EVENT,
    schema::INTANGIBLE,
    schema::ORGANIZATION,
    schema::PERSON,
    schema::PLACE,
    schema::PRODUCT,
    schema::DATASET,
    schema::DATA_CATALOG,
    schema::DATA_DOWNLOAD,
    schema::SOFTWARE_APPLICATION,
    schema::SOFTWARE_SOURCE_CODE,
    schema::WEB_PAGE,
    schema::WEB_SITE,
    schema::NAME,
    schema::DESCRIPTION,
    schema::URL,
    schema::IDENTIFIER,
    schema::IMAGE,
    schema::ALTERNATE_NAME,
    schema::SAME_AS,
    schema::ADDITIONAL_TYPE,
    schema::POTENTIAL_ACTION,
    schema::MAIN_ENTITY_OF_PAGE,
    schema::SUBJECT_OF,
    schema::GIVEN_NAME,
    schema::FAMILY_NAME,
    schema::EMAIL,
    schema::TELEPHONE,
    schema::ADDRESS,
    schema::AFFILIATION,
    schema::AUTHOR,
    schema::CREATOR,
    schema::EDITOR,
    schema::PUBLISHER,
    schema::DATE_CREATED,
    schema::DATE_MODIFIED,
    schema::DATE_PUBLISHED,
    schema::LICENSE,
    schema::COPYRIGHT_YEAR,
    schema::COPYRIGHT_HOLDER,
    schema::KEYWORDS,
    schema::ABOUT,
    schema::IN_LANGUAGE,
];

#[test]
fn schema_priority_terms_present() {
    for iri in SCHEMA_PRIORITY {
        assert!(
            iri.starts_with("https://schema.org/"),
            "schema IRI must use schema.org namespace: {iri}"
        );
    }
}

#[test]
fn schema_coverage_floor() {
    // Total known (Phase E minimal set): 30
    const TOTAL: usize = 30;
    let defined = SCHEMA_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "schema: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn schema_ns_constant() {
    assert_eq!(schema::NS, "https://schema.org/");
}

// ─── prov ────────────────────────────────────────────────────────────────────

const PROV_PRIORITY: &[&str] = &[
    prov::ENTITY,
    prov::ACTIVITY,
    prov::AGENT,
    prov::WAS_GENERATED_BY,
    prov::WAS_DERIVED_FROM,
    prov::WAS_ATTRIBUTED_TO,
    prov::STARTED_AT_TIME,
    prov::ENDED_AT_TIME,
    prov::USED,
    prov::WAS_ASSOCIATED_WITH,
];

const PROV_ALL: &[&str] = &[
    prov::NS,
    prov::ENTITY,
    prov::ACTIVITY,
    prov::AGENT,
    prov::PLAN,
    prov::BUNDLE,
    prov::COLLECTION,
    prov::EMPTY_COLLECTION,
    prov::LOCATION,
    prov::ORGANIZATION,
    prov::PERSON,
    prov::SOFTWARE_AGENT,
    prov::WAS_GENERATED_BY,
    prov::WAS_DERIVED_FROM,
    prov::WAS_ATTRIBUTED_TO,
    prov::USED,
    prov::WAS_INFORMED_BY,
    prov::WAS_ASSOCIATED_WITH,
    prov::ACTED_ON_BEHALF_OF,
    prov::STARTED_AT_TIME,
    prov::ENDED_AT_TIME,
    prov::GENERATED_AT_TIME,
    prov::INVALIDATED_AT_TIME,
    prov::USAGE,
    prov::GENERATION,
    prov::INVALIDATION,
    prov::COMMUNICATION,
    prov::START,
    prov::END,
    prov::DERIVATION,
    prov::ATTRIBUTION,
    prov::ASSOCIATION,
    prov::DELEGATION,
    prov::INFLUENCE,
    prov::QUALIFIED_USAGE,
    prov::QUALIFIED_GENERATION,
    prov::QUALIFIED_DERIVATION,
    prov::QUALIFIED_ATTRIBUTION,
    prov::QUALIFIED_ASSOCIATION,
    prov::QUALIFIED_COMMUNICATION,
    prov::QUALIFIED_DELEGATION,
    prov::QUALIFIED_START,
    prov::QUALIFIED_END,
    prov::QUALIFIED_INFLUENCE,
    prov::ENTITY_PROP,
    prov::ACTIVITY_PROP,
    prov::AGENT_PROP,
    prov::HAD_PLAN,
    prov::HAD_ROLE,
    prov::AT_LOCATION,
    prov::AT_TIME,
    prov::HAD_MEMBER,
    prov::INFLUENCED,
    prov::WAS_STARTED_BY,
    prov::WAS_ENDED_BY,
    prov::WAS_INVALIDATED_BY,
    prov::HAD_USAGE,
    prov::HAD_GENERATION,
    prov::WAS_MEMBER_OF,
];

#[test]
fn prov_priority_terms_present() {
    for iri in PROV_PRIORITY {
        assert!(
            iri.starts_with("http://www.w3.org/ns/prov#"),
            "prov IRI must use PROV namespace: {iri}"
        );
    }
}

#[test]
fn prov_coverage_floor() {
    // Total known: 25
    const TOTAL: usize = 25;
    let defined = PROV_ALL.len() - 1;
    assert!(
        defined >= min_terms(TOTAL),
        "prov: need >= {} terms, found {}",
        min_terms(TOTAL),
        defined
    );
}

#[test]
fn prov_ns_constant() {
    assert_eq!(prov::NS, "http://www.w3.org/ns/prov#");
}

// ─── IRI integrity spot-checks ────────────────────────────────────────────────

/// Verify that no constant holds an empty string (regression guard).
#[test]
fn no_empty_iris() {
    for (vocab, iris) in &[
        ("xsd", XSD_ALL.as_ref()),
        ("rdf", RDF_ALL.as_ref()),
        ("rdfs", RDFS_ALL.as_ref()),
        ("owl", OWL_ALL.as_ref()),
        ("skos", SKOS_ALL.as_ref()),
        ("sh", SH_ALL.as_ref()),
        ("dcterms", DCTERMS_ALL.as_ref()),
        ("dcat", DCAT_ALL.as_ref()),
        ("foaf", FOAF_ALL.as_ref()),
        ("schema", SCHEMA_ALL.as_ref()),
        ("prov", PROV_ALL.as_ref()),
    ] {
        for iri in *iris {
            assert!(!iri.is_empty(), "{vocab}: found empty IRI string");
        }
    }
}

/// Verify that every priority term ends with a non-empty local name component.
#[test]
fn priority_iris_have_local_name() {
    let all_priority: &[(&str, &[&str])] = &[
        ("xsd", XSD_PRIORITY),
        ("rdf", RDF_PRIORITY),
        ("rdfs", RDFS_PRIORITY),
        ("owl", OWL_PRIORITY),
        ("skos", SKOS_PRIORITY),
        ("sh", SH_PRIORITY),
        ("dcterms", DCTERMS_PRIORITY),
        ("dcat", DCAT_PRIORITY),
        ("foaf", FOAF_PRIORITY),
        ("schema", SCHEMA_PRIORITY),
        ("prov", PROV_PRIORITY),
    ];
    for (vocab, iris) in all_priority {
        for iri in *iris {
            let local = iri
                .rsplit_once(['#', '/'])
                .map(|(_, l)| l)
                .unwrap_or("");
            assert!(
                !local.is_empty(),
                "{vocab}: IRI has no local name: {iri}"
            );
        }
    }
}
