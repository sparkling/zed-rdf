//! DCAT (Data Catalog Vocabulary) terms.
//!
//! Namespace: `http://www.w3.org/ns/dcat#`
//! Reference: <https://www.w3.org/TR/vocab-dcat-3/>

/// DCAT namespace IRI (trailing `#`).
pub const NS: &str = "http://www.w3.org/ns/dcat#";

// ── Classes ───────────────────────────────────────────────────────────────────

/// `dcat:Resource` — Anything described by DCAT.
///
/// Label: "Catalogued Resource"
///
/// Description: "Resource published or curated by a single agent."
pub const RESOURCE: &str = "http://www.w3.org/ns/dcat#Resource";

/// `dcat:Dataset` — A collection of data, available for access or download.
///
/// Label: "Dataset"
///
/// Description: "A collection of data, published or curated by a single source, and available for access or download in one or more representations."
pub const DATASET: &str = "http://www.w3.org/ns/dcat#Dataset";

/// `dcat:Distribution` — A specific representation of a dataset.
///
/// Label: "Distribution"
///
/// Description: "A specific representation of a dataset. A dataset might be available in multiple serializations that may differ in various ways, including natural language, media-type or format, schematic organization, temporal and spatial resolution, level of detail or profiles."
pub const DISTRIBUTION: &str = "http://www.w3.org/ns/dcat#Distribution";

/// `dcat:Catalog` — A curated collection of metadata about resources.
///
/// Label: "Catalog"
///
/// Description: "A curated collection of metadata about resources (e.g., datasets and data services in the context of a data catalog)."
pub const CATALOG: &str = "http://www.w3.org/ns/dcat#Catalog";

/// `dcat:CatalogRecord` — A record in a data catalog.
///
/// Label: "Catalog Record"
///
/// Description: "A record in a data catalog, describing the registration of a single resource (e.g., a dataset or data service)."
pub const CATALOG_RECORD: &str = "http://www.w3.org/ns/dcat#CatalogRecord";

/// `dcat:DataService` — A collection of operations that provides access to one or more datasets.
///
/// Label: "Data service"
///
/// Description: "A collection of operations that provides access to one or more datasets or data processing functions."
pub const DATA_SERVICE: &str = "http://www.w3.org/ns/dcat#DataService";

/// `dcat:DatasetSeries` — A collection of datasets that are published separately but share some characteristics.
///
/// Label: "Dataset series"
///
/// Description: "A collection of datasets that are published separately, but share some characteristics that group them."
pub const DATASET_SERIES: &str = "http://www.w3.org/ns/dcat#DatasetSeries";

// ── Distribution access/download ──────────────────────────────────────────────

/// `dcat:accessURL` — A URL of the resource that gives access to a distribution.
///
/// Label: "access URL"
///
/// Description: "A URL of the resource that gives access to a distribution of the dataset. E.g., landing page, feed, SPARQL endpoint."
pub const ACCESS_URL: &str = "http://www.w3.org/ns/dcat#accessURL";

/// `dcat:downloadURL` — The URL of the downloadable file.
///
/// Label: "download URL"
///
/// Description: "The URL of the downloadable file in a given format. E.g., CSV file or RDF file."
pub const DOWNLOAD_URL: &str = "http://www.w3.org/ns/dcat#downloadURL";

/// `dcat:accessService` — A data service that gives access to the distribution.
///
/// Label: "data access service"
///
/// Description: "A data service that gives access to the distribution of the dataset."
pub const ACCESS_SERVICE: &str = "http://www.w3.org/ns/dcat#accessService";

// ── Dataset properties ────────────────────────────────────────────────────────

/// `dcat:distribution` — An available distribution of the dataset.
///
/// Label: "distribution"
///
/// Description: "An available distribution of the dataset."
pub const DISTRIBUTION_PROP: &str = "http://www.w3.org/ns/dcat#distribution";

/// `dcat:keyword` — A keyword or tag describing the dataset.
///
/// Label: "keyword"
///
/// Description: "A keyword or tag describing a resource."
pub const KEYWORD: &str = "http://www.w3.org/ns/dcat#keyword";

/// `dcat:theme` — The main category of the dataset.
///
/// Label: "theme/category"
///
/// Description: "A main category of the resource. A resource can have multiple themes."
pub const THEME: &str = "http://www.w3.org/ns/dcat#theme";

/// `dcat:themeTaxonomy` — A knowledge organization system used to classify the catalog's datasets.
///
/// Label: "theme taxonomy"
///
/// Description: "The knowledge organization system (KOS) used to classify catalog's datasets and services."
pub const THEME_TAXONOMY: &str = "http://www.w3.org/ns/dcat#themeTaxonomy";

/// `dcat:contactPoint` — Relevant contact information for the dataset.
///
/// Label: "contact point"
///
/// Description: "Relevant contact information for the catalogued resource. Use of vCard is recommended."
pub const CONTACT_POINT: &str = "http://www.w3.org/ns/dcat#contactPoint";

/// `dcat:landingPage` — A webpage providing access to the dataset.
///
/// Label: "landing page"
///
/// Description: "A Web page that can be navigated to in a Web browser to gain access to the catalog, a dataset, its distributions and/or additional information."
pub const LANDING_PAGE: &str = "http://www.w3.org/ns/dcat#landingPage";

/// `dcat:qualifiedRelation` — A description of a relationship with another resource.
///
/// Label: "qualified relation"
///
/// Description: "A description of a relationship with another resource."
pub const QUALIFIED_RELATION: &str = "http://www.w3.org/ns/dcat#qualifiedRelation";

// ── Distribution properties ───────────────────────────────────────────────────

/// `dcat:mediaType` — The media type of the distribution.
///
/// Label: "media type"
///
/// Description: "The media type of the distribution as defined by IANA."
pub const MEDIA_TYPE: &str = "http://www.w3.org/ns/dcat#mediaType";

/// `dcat:format` — The file format of the distribution.
///
/// Label: "format"
///
/// Description: "The file format of the distribution."
pub const FORMAT: &str = "http://www.w3.org/ns/dcat#format";

/// `dcat:byteSize` — The size of the distribution in bytes.
///
/// Label: "byte size"
///
/// Description: "The size of a distribution in bytes."
pub const BYTE_SIZE: &str = "http://www.w3.org/ns/dcat#byteSize";

/// `dcat:compressFormat` — The compression format of the distribution.
///
/// Label: "compression format"
///
/// Description: "The compression format of the distribution in which the data is contained in a compressed form, e.g., to reduce the size of the downloadable file."
pub const COMPRESS_FORMAT: &str = "http://www.w3.org/ns/dcat#compressFormat";

/// `dcat:packageFormat` — The package format of the distribution.
///
/// Label: "packaging format"
///
/// Description: "The package format of the distribution in which one or more data files are grouped together, e.g., to enable a set of related files to be downloaded together."
pub const PACKAGE_FORMAT: &str = "http://www.w3.org/ns/dcat#packageFormat";

/// `dcat:spatialResolutionInMeters` — The minimum spatial separation resolvable in the dataset.
///
/// Label: "spatial resolution (meters)"
///
/// Description: "The minimum spatial separation resolvable in a dataset, measured in meters."
pub const SPATIAL_RESOLUTION_IN_METERS: &str =
    "http://www.w3.org/ns/dcat#spatialResolutionInMeters";

/// `dcat:temporalResolution` — The minimum time period resolvable in the dataset.
///
/// Label: "temporal resolution"
///
/// Description: "The minimum time period resolvable in the dataset."
pub const TEMPORAL_RESOLUTION: &str = "http://www.w3.org/ns/dcat#temporalResolution";

// ── Catalog properties ────────────────────────────────────────────────────────

/// `dcat:dataset` — A dataset that is listed in the catalog.
///
/// Label: "dataset"
///
/// Description: "A dataset that is listed in the catalog."
pub const DATASET_PROP: &str = "http://www.w3.org/ns/dcat#dataset";

/// `dcat:catalog` — A catalog whose contents are of interest in the context of this catalog.
///
/// Label: "catalog"
///
/// Description: "A catalog whose contents are of interest in the context of this catalog."
pub const CATALOG_PROP: &str = "http://www.w3.org/ns/dcat#catalog";

/// `dcat:service` — A service that is offered via the catalog.
///
/// Label: "service"
///
/// Description: "A service that is offered via the catalog."
pub const SERVICE: &str = "http://www.w3.org/ns/dcat#service";

/// `dcat:record` — A catalog record that is part of the catalog.
///
/// Label: "record"
///
/// Description: "A record describing the registration of a single dataset or data service that is part of the catalog."
pub const RECORD: &str = "http://www.w3.org/ns/dcat#record";

// ── DataService properties ────────────────────────────────────────────────────

/// `dcat:endpointURL` — The root location or primary endpoint of the service.
///
/// Label: "endpoint URL"
///
/// Description: "The root location or primary endpoint of the service (a Web-resolvable IRI)."
pub const ENDPOINT_URL: &str = "http://www.w3.org/ns/dcat#endpointURL";

/// `dcat:endpointDescription` — A description of the service endpoint.
///
/// Label: "description of service endpoint"
///
/// Description: "A description of the service endpoint, including its operations, parameters etc."
pub const ENDPOINT_DESCRIPTION: &str = "http://www.w3.org/ns/dcat#endpointDescription";

/// `dcat:servesDataset` — A dataset that this `DataService` can distribute.
///
/// Label: "serves dataset"
///
/// Description: "A collection of data that this `DataService` can distribute."
pub const SERVES_DATASET: &str = "http://www.w3.org/ns/dcat#servesDataset";

// ── Temporal/spatial ──────────────────────────────────────────────────────────

/// `dcat:temporal` — The temporal coverage of the dataset.
///
/// Label: "temporal coverage"
///
/// Description: "The temporal period that the dataset covers."
pub const TEMPORAL: &str = "http://www.w3.org/ns/dcat#temporal";

/// `dcat:spatial` — The geographic area covered by the dataset.
///
/// Label: "spatial/geographical coverage"
///
/// Description: "The geographical area covered by the dataset."
pub const SPATIAL: &str = "http://www.w3.org/ns/dcat#spatial";

// ── Provenance ────────────────────────────────────────────────────────────────

/// `dcat:version` — The version indicator for a resource.
///
/// Label: "version"
///
/// Description: "The version indicator (name or identifier) of a resource."
pub const VERSION: &str = "http://www.w3.org/ns/dcat#version";

/// `dcat:previousVersion` — The previous version of the resource.
///
/// Label: "previous version"
///
/// Description: "The previous version of a resource in a lineage."
pub const PREVIOUS_VERSION: &str = "http://www.w3.org/ns/dcat#previousVersion";

/// `dcat:hasCurrentVersion` — The current version of the resource.
///
/// Label: "has current version"
///
/// Description: "This resource has a more specific, versioned resource with equivalent content."
pub const HAS_CURRENT_VERSION: &str = "http://www.w3.org/ns/dcat#hasCurrentVersion";

/// `dcat:hasVersion` — A related resource that is a version of the described resource.
///
/// Label: "has version"
///
/// Description: "A related resource that is a version, edition, or adaptation of the described resource."
pub const HAS_VERSION: &str = "http://www.w3.org/ns/dcat#hasVersion";

/// `dcat:isVersionOf` — A related resource of which the described resource is a version.
///
/// Label: "is version of"
///
/// Description: "A related resource of which the described resource is a version, edition, or adaptation."
pub const IS_VERSION_OF: &str = "http://www.w3.org/ns/dcat#isVersionOf";

/// `dcat:first` — The first resource in an ordered collection or series.
///
/// Label: "first"
///
/// Description: "The first resource in an ordered collection or series of resources, to which the current resource belongs."
pub const FIRST: &str = "http://www.w3.org/ns/dcat#first";

/// `dcat:last` — The last resource in an ordered collection or series.
///
/// Label: "last"
///
/// Description: "The last resource in an ordered collection or series of resources, to which the current resource belongs."
pub const LAST: &str = "http://www.w3.org/ns/dcat#last";

/// `dcat:next` — The next resource in an ordered collection or series.
///
/// Label: "next"
///
/// Description: "The next resource in an ordered collection or series of resources."
pub const NEXT: &str = "http://www.w3.org/ns/dcat#next";

/// `dcat:prev` — The previous resource in an ordered collection or series.
///
/// Label: "previous"
///
/// Description: "The previous resource in an ordered collection or series of resources."
pub const PREV: &str = "http://www.w3.org/ns/dcat#prev";
