//! Schema.org vocabulary terms (most common subset).
//!
//! Namespace: `https://schema.org/`
//! Reference: <https://schema.org/>

/// Schema.org namespace IRI (trailing `/`).
pub const NS: &str = "https://schema.org/";

// ── Core classes ──────────────────────────────────────────────────────────────

/// `schema:Thing` — The most generic type; every entity is a Thing.
///
/// Label: "Thing"
///
/// Description: "The most generic type of item."
pub const THING: &str = "https://schema.org/Thing";

/// `schema:Action` — An action performed by an agent.
///
/// Label: "Action"
///
/// Description: "An action performed by a direct agent and indirect participants upon a direct object."
pub const ACTION: &str = "https://schema.org/Action";

/// `schema:CreativeWork` — A creative work of any kind.
///
/// Label: "`CreativeWork`"
///
/// Description: "The most generic kind of creative work, including books, movies, photographs, software programs, etc."
pub const CREATIVE_WORK: &str = "https://schema.org/CreativeWork";

/// `schema:Event` — An event happening at a certain time and location.
///
/// Label: "Event"
///
/// Description: "An event happening at a certain time and location, such as a concert, lecture, or festival."
pub const EVENT: &str = "https://schema.org/Event";

/// `schema:Intangible` — A utility class that serves as the umbrella for a number of 'intangible' things.
///
/// Label: "Intangible"
///
/// Description: "A utility class that serves as the umbrella for a number of 'intangible' things such as quantities, structured values, etc."
pub const INTANGIBLE: &str = "https://schema.org/Intangible";

/// `schema:Organization` — An organization such as a school, NGO, corporation, club, etc.
///
/// Label: "Organization"
///
/// Description: "An organization such as a school, NGO, corporation, club, etc."
pub const ORGANIZATION: &str = "https://schema.org/Organization";

/// `schema:Person` — A person (alive, dead, undead, or fictional).
///
/// Label: "Person"
///
/// Description: "A person (alive, dead, undead, or fictional)."
pub const PERSON: &str = "https://schema.org/Person";

/// `schema:Place` — Entities that have a somewhat fixed, physical extension.
///
/// Label: "Place"
///
/// Description: "Entities that have a somewhat fixed, physical extension."
pub const PLACE: &str = "https://schema.org/Place";

/// `schema:Product` — Any offered product or service.
///
/// Label: "Product"
///
/// Description: "Any offered product or service. For example: a pair of shoes; a concert ticket; the rental of a car; a haircut; or an episode of a TV show streamed online."
pub const PRODUCT: &str = "https://schema.org/Product";

// ── Digital/tech specialisations ──────────────────────────────────────────────

/// `schema:Dataset` — A body of structured information describing some topic(s).
///
/// Label: "Dataset"
///
/// Description: "A body of structured information describing some topic(s) of interest, in machine-readable form."
pub const DATASET: &str = "https://schema.org/Dataset";

/// `schema:DataCatalog` — A collection of datasets.
///
/// Label: "`DataCatalog`"
///
/// Description: "A collection of datasets."
pub const DATA_CATALOG: &str = "https://schema.org/DataCatalog";

/// `schema:DataDownload` — A dataset in downloadable form.
///
/// Label: "`DataDownload`"
///
/// Description: "A dataset in downloadable form."
pub const DATA_DOWNLOAD: &str = "https://schema.org/DataDownload";

/// `schema:SoftwareApplication` — A software application.
///
/// Label: "`SoftwareApplication`"
///
/// Description: "A software application."
pub const SOFTWARE_APPLICATION: &str = "https://schema.org/SoftwareApplication";

/// `schema:SoftwareSourceCode` — Computer programming source code.
///
/// Label: "`SoftwareSourceCode`"
///
/// Description: "Computer programming source code. Example: Full (compile ready) solutions, code snippet samples, scripts, templates."
pub const SOFTWARE_SOURCE_CODE: &str = "https://schema.org/SoftwareSourceCode";

/// `schema:WebPage` — A web page.
///
/// Label: "`WebPage`"
///
/// Description: "A web page. Every web page is implicitly assumed to be declared to be of type `WebPage`."
pub const WEB_PAGE: &str = "https://schema.org/WebPage";

/// `schema:WebSite` — A website (a set of related web pages).
///
/// Label: "`WebSite`"
///
/// Description: "A `WebSite` is a set of related web pages and other items typically served from a single web domain and accessible via `URLs`."
pub const WEB_SITE: &str = "https://schema.org/WebSite";

// ── Common properties ─────────────────────────────────────────────────────────

/// `schema:name` — The name of the item.
///
/// Label: "name"
///
/// Description: "The name of the item."
pub const NAME: &str = "https://schema.org/name";

/// `schema:description` — A description of the item.
///
/// Label: "description"
///
/// Description: "A description of the item."
pub const DESCRIPTION: &str = "https://schema.org/description";

/// `schema:url` — URL of the item.
///
/// Label: "url"
///
/// Description: "URL of the item."
pub const URL: &str = "https://schema.org/url";

/// `schema:identifier` — The identifier property represents any kind of identifier for a Thing.
///
/// Label: "identifier"
///
/// Description: "The identifier property represents any kind of identifier for any kind of Thing, such as `ISBNs`, GTIN codes, `UUIDs` etc."
pub const IDENTIFIER: &str = "https://schema.org/identifier";

/// `schema:image` — An image of the item.
///
/// Label: "image"
///
/// Description: "An image of the item. This can be a URL or a fully described `ImageObject`."
pub const IMAGE: &str = "https://schema.org/image";

/// `schema:alternateName` — An alias for the item.
///
/// Label: "alternateName"
///
/// Description: "An alias for the item."
pub const ALTERNATE_NAME: &str = "https://schema.org/alternateName";

/// `schema:sameAs` — URL of a reference Web page that unambiguously indicates the item's identity.
///
/// Label: "sameAs"
///
/// Description: "URL of a reference Web page that unambiguously indicates the item's identity. E.g. the URL of the item's Wikipedia page, Freebase page, or official website."
pub const SAME_AS: &str = "https://schema.org/sameAs";

/// `schema:additionalType` — An additional type for the item.
///
/// Label: "additionalType"
///
/// Description: "An additional type for the item, typically used for adding more specific types from external vocabularies in microdata syntax."
pub const ADDITIONAL_TYPE: &str = "https://schema.org/additionalType";

/// `schema:potentialAction` — Indicates a potential Action.
///
/// Label: "potentialAction"
///
/// Description: "Indicates a potential Action, which describes an idealized action in which this thing would play an 'object' role."
pub const POTENTIAL_ACTION: &str = "https://schema.org/potentialAction";

/// `schema:mainEntityOfPage` — Indicates a page that is a main content entity for this item.
///
/// Label: "mainEntityOfPage"
///
/// Description: "Indicates a page (or other `CreativeWork`) for which this thing is the main entity being described."
pub const MAIN_ENTITY_OF_PAGE: &str = "https://schema.org/mainEntityOfPage";

/// `schema:subjectOf` — A `CreativeWork` about this Thing.
///
/// Label: "subjectOf"
///
/// Description: "A `CreativeWork` or Event about this Thing."
pub const SUBJECT_OF: &str = "https://schema.org/subjectOf";

// ── People/org properties ─────────────────────────────────────────────────────

/// `schema:givenName` — Given name.
///
/// Label: "givenName"
///
/// Description: "Given name. In the U.S., the first name of a Person."
pub const GIVEN_NAME: &str = "https://schema.org/givenName";

/// `schema:familyName` — Family name.
///
/// Label: "familyName"
///
/// Description: "Family name. In the U.S., the last name of a Person."
pub const FAMILY_NAME: &str = "https://schema.org/familyName";

/// `schema:email` — Email address.
///
/// Label: "email"
///
/// Description: "Email address."
pub const EMAIL: &str = "https://schema.org/email";

/// `schema:telephone` — The telephone number.
///
/// Label: "telephone"
///
/// Description: "The telephone number."
pub const TELEPHONE: &str = "https://schema.org/telephone";

/// `schema:address` — Physical address of the item.
///
/// Label: "address"
///
/// Description: "Physical address of the item."
pub const ADDRESS: &str = "https://schema.org/address";

/// `schema:affiliation` — An organization that this person is affiliated with.
///
/// Label: "affiliation"
///
/// Description: "An organization that this person is affiliated with. For example, a school/university, a club, or a team."
pub const AFFILIATION: &str = "https://schema.org/affiliation";

/// `schema:author` — The author of this content.
///
/// Label: "author"
///
/// Description: "The author of this content or rating. Please note that author is special in that HTML 5 provides a special mechanism for indicating authorship via the rel tag."
pub const AUTHOR: &str = "https://schema.org/author";

/// `schema:creator` — The creator/author of this `CreativeWork`.
///
/// Label: "creator"
///
/// Description: "The creator/author of this `CreativeWork`. This is the same as the Author property for most cases."
pub const CREATOR: &str = "https://schema.org/creator";

/// `schema:editor` — Specifies the Person who edited the `CreativeWork`.
///
/// Label: "editor"
///
/// Description: "Specifies the Person who edited the `CreativeWork`."
pub const EDITOR: &str = "https://schema.org/editor";

/// `schema:publisher` — The publisher of the creative work.
///
/// Label: "publisher"
///
/// Description: "The publisher of the creative work."
pub const PUBLISHER: &str = "https://schema.org/publisher";

// ── Date/time properties ──────────────────────────────────────────────────────

/// `schema:dateCreated` — The date on which the `CreativeWork` was created.
///
/// Label: "dateCreated"
///
/// Description: "The date on which the `CreativeWork` was created or the item was added to a `DataFeed`."
pub const DATE_CREATED: &str = "https://schema.org/dateCreated";

/// `schema:dateModified` — The date on which the `CreativeWork` was most recently modified.
///
/// Label: "dateModified"
///
/// Description: "The date on which the `CreativeWork` was most recently modified or when the item's entry was modified within a `DataFeed`."
pub const DATE_MODIFIED: &str = "https://schema.org/dateModified";

/// `schema:datePublished` — Date of first broadcast/publication.
///
/// Label: "datePublished"
///
/// Description: "Date of first broadcast/publication."
pub const DATE_PUBLISHED: &str = "https://schema.org/datePublished";

// ── License/rights ────────────────────────────────────────────────────────────

/// `schema:license` — A license document that applies to this content.
///
/// Label: "license"
///
/// Description: "A license document that applies to this content, typically indicated by URL."
pub const LICENSE: &str = "https://schema.org/license";

/// `schema:copyrightYear` — The year during which the claimed copyright for the `CreativeWork` was first asserted.
///
/// Label: "copyrightYear"
///
/// Description: "The year during which the claimed copyright for the `CreativeWork` was first asserted."
pub const COPYRIGHT_YEAR: &str = "https://schema.org/copyrightYear";

/// `schema:copyrightHolder` — The party holding the legal copyright to the `CreativeWork`.
///
/// Label: "copyrightHolder"
///
/// Description: "The party holding the legal copyright to the `CreativeWork`."
pub const COPYRIGHT_HOLDER: &str = "https://schema.org/copyrightHolder";

// ── Keywords/subjects ─────────────────────────────────────────────────────────

/// `schema:keywords` — Keywords or tags used to describe the content.
///
/// Label: "keywords"
///
/// Description: "Keywords or tags used to describe this content. Multiple entries in a keywords list are typically delimited by commas."
pub const KEYWORDS: &str = "https://schema.org/keywords";

/// `schema:about` — The subject matter of the content.
///
/// Label: "about"
///
/// Description: "The subject matter of the content."
pub const ABOUT: &str = "https://schema.org/about";

/// `schema:inLanguage` — The language of the content or performance.
///
/// Label: "inLanguage"
///
/// Description: "The language of the content or performance or used in an action."
pub const IN_LANGUAGE: &str = "https://schema.org/inLanguage";
