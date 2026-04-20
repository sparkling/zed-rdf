//! FOAF (Friend of a Friend) vocabulary terms.
//!
//! Namespace: `http://xmlns.com/foaf/0.1/`
//! Reference: <http://xmlns.com/foaf/spec/>

/// FOAF namespace IRI (trailing `/`).
pub const NS: &str = "http://xmlns.com/foaf/0.1/";

// ── Core classes ──────────────────────────────────────────────────────────────

/// `foaf:Agent` — An agent (person, group, software or physical artifact).
///
/// Label: "Agent"
///
/// Description: "An agent (eg. person, group, software or physical artifact)."
pub const AGENT: &str = "http://xmlns.com/foaf/0.1/Agent";

/// `foaf:Person` — A person.
///
/// Label: "Person"
///
/// Description: "A person."
pub const PERSON: &str = "http://xmlns.com/foaf/0.1/Person";

/// `foaf:Organization` — An organization.
///
/// Label: "Organization"
///
/// Description: "An organization."
pub const ORGANIZATION: &str = "http://xmlns.com/foaf/0.1/Organization";

/// `foaf:Group` — A collection of individual agents.
///
/// Label: "Group"
///
/// Description: "A collection of individual agents."
pub const GROUP: &str = "http://xmlns.com/foaf/0.1/Group";

/// `foaf:Document` — A document.
///
/// Label: "Document"
///
/// Description: "A document."
pub const DOCUMENT: &str = "http://xmlns.com/foaf/0.1/Document";

/// `foaf:Image` — An image.
///
/// Label: "Image"
///
/// Description: "An image."
pub const IMAGE: &str = "http://xmlns.com/foaf/0.1/Image";

/// `foaf:Project` — A project (a collective endeavour of some kind).
///
/// Label: "Project"
///
/// Description: "A project (a collective endeavour of some kind)."
pub const PROJECT: &str = "http://xmlns.com/foaf/0.1/Project";

/// `foaf:PersonalProfileDocument` — A personal profile RDF document.
///
/// Label: "`PersonalProfileDocument`"
///
/// Description: "A personal profile RDF document."
pub const PERSONAL_PROFILE_DOCUMENT: &str =
    "http://xmlns.com/foaf/0.1/PersonalProfileDocument";

/// `foaf:OnlineAccount` — An online account.
///
/// Label: "Online Account"
///
/// Description: "An online account."
pub const ONLINE_ACCOUNT: &str = "http://xmlns.com/foaf/0.1/OnlineAccount";

/// `foaf:OnlineChatAccount` — An online chat account.
///
/// Label: "Online Chat Account"
///
/// Description: "An online chat account."
pub const ONLINE_CHAT_ACCOUNT: &str = "http://xmlns.com/foaf/0.1/OnlineChatAccount";

/// `foaf:OnlineEcommerceAccount` — An online e-commerce account.
///
/// Label: "Online E-commerce Account"
///
/// Description: "An online e-commerce account."
pub const ONLINE_ECOMMERCE_ACCOUNT: &str =
    "http://xmlns.com/foaf/0.1/OnlineEcommerceAccount";

/// `foaf:OnlineGamingAccount` — An online gaming account.
///
/// Label: "Online Gaming Account"
///
/// Description: "An online gaming account."
pub const ONLINE_GAMING_ACCOUNT: &str =
    "http://xmlns.com/foaf/0.1/OnlineGamingAccount";

// ── Identification properties ─────────────────────────────────────────────────

/// `foaf:name` — A name for some thing.
///
/// Label: "name"
///
/// Description: "A name for some thing."
pub const NAME: &str = "http://xmlns.com/foaf/0.1/name";

/// `foaf:title` — A title (Mr, Mrs, Dr, etc.).
///
/// Label: "title"
///
/// Description: "A title (Mr, Mrs, Dr, etc)."
pub const TITLE: &str = "http://xmlns.com/foaf/0.1/title";

/// `foaf:nick` — A short informal nickname characterising an agent.
///
/// Label: "nickname"
///
/// Description: "A short informal nickname characterising an agent (includes login identifiers, IRC and other chat nicknames)."
pub const NICK: &str = "http://xmlns.com/foaf/0.1/nick";

/// `foaf:firstName` — The first name of a person.
///
/// Label: "firstName"
///
/// Description: "The first name of a person."
pub const FIRST_NAME: &str = "http://xmlns.com/foaf/0.1/firstName";

/// `foaf:lastName` — The last name of a person.
///
/// Label: "lastName"
///
/// Description: "The last name of a person."
pub const LAST_NAME: &str = "http://xmlns.com/foaf/0.1/lastName";

/// `foaf:givenName` — The given name of some person.
///
/// Label: "Given name"
///
/// Description: "The given name of some person."
pub const GIVEN_NAME: &str = "http://xmlns.com/foaf/0.1/givenName";

/// `foaf:familyName` — The family name of some person.
///
/// Label: "familyName"
///
/// Description: "The family name of some person."
pub const FAMILY_NAME: &str = "http://xmlns.com/foaf/0.1/familyName";

// ── Contact properties ────────────────────────────────────────────────────────

/// `foaf:mbox` — A personal mailbox (a `PrimaryTopic` of this).
///
/// Label: "personal mailbox"
///
/// Description: "A personal mailbox, ie. an Internet mailbox associated with exactly one owner, the first owner of this mailbox. This is a 'static inverse functional property', in that there is (across time and change) at most one individual that ever has any particular value for foaf:mbox."
pub const MBOX: &str = "http://xmlns.com/foaf/0.1/mbox";

/// `foaf:mbox_sha1sum` — The SHA1 sum of the URI of an Internet mailbox.
///
/// Label: "sha1sum of a personal mailbox URI name"
///
/// Description: "The sha1sum of the URI of an internet mailbox associated with exactly one owner, the first owner of the mailbox."
pub const MBOX_SHA1SUM: &str = "http://xmlns.com/foaf/0.1/mbox_sha1sum";

/// `foaf:phone` — A phone number for some agent.
///
/// Label: "phone"
///
/// Description: "A phone, specified using fully qualified tel: URI scheme (refs: <http://www.ietf.org/rfc/rfc3966.txt).>"
pub const PHONE: &str = "http://xmlns.com/foaf/0.1/phone";

/// `foaf:jabberID` — A Jabber ID for something.
///
/// Label: "jabber ID"
///
/// Description: "A jabber ID for something."
pub const JABBER_ID: &str = "http://xmlns.com/foaf/0.1/jabberID";

/// `foaf:skypeID` — A Skype ID for something.
///
/// Label: "Skype ID"
///
/// Description: "A Skype ID."
pub const SKYPE_ID: &str = "http://xmlns.com/foaf/0.1/skypeID";

// ── Web presence ──────────────────────────────────────────────────────────────

/// `foaf:homepage` — A homepage for some thing.
///
/// Label: "homepage"
///
/// Description: "A homepage for some thing."
pub const HOMEPAGE: &str = "http://xmlns.com/foaf/0.1/homepage";

/// `foaf:weblog` — A weblog of some thing (usually a Person, Group, or Organization).
///
/// Label: "weblog"
///
/// Description: "A weblog of some thing (usually a person, group, or organization)."
pub const WEBLOG: &str = "http://xmlns.com/foaf/0.1/weblog";

/// `foaf:openid` — An `OpenID` for an Agent.
///
/// Label: "openid"
///
/// Description: "An `OpenID` for an Agent."
pub const OPEN_ID: &str = "http://xmlns.com/foaf/0.1/openid";

/// `foaf:account` — Indicates an account held by this agent.
///
/// Label: "account"
///
/// Description: "Indicates an account held by this agent."
pub const ACCOUNT: &str = "http://xmlns.com/foaf/0.1/account";

/// `foaf:accountServiceHomepage` — Indicates a homepage of the service providing the account.
///
/// Label: "account service homepage"
///
/// Description: "Indicates a homepage of the service provide for this online account."
pub const ACCOUNT_SERVICE_HOMEPAGE: &str =
    "http://xmlns.com/foaf/0.1/accountServiceHomepage";

/// `foaf:accountName` — Indicates the name (identifier) associated with this online account.
///
/// Label: "account name"
///
/// Description: "Indicates the name (identifier) associated with this online account."
pub const ACCOUNT_NAME: &str = "http://xmlns.com/foaf/0.1/accountName";

// ── Social properties ─────────────────────────────────────────────────────────

/// `foaf:knows` — A person known by this person.
///
/// Label: "knows"
///
/// Description: "A person known by this person (indicating some level of reciprocated interaction between the parties)."
pub const KNOWS: &str = "http://xmlns.com/foaf/0.1/knows";

/// `foaf:member` — Indicates a member of a Group.
///
/// Label: "member"
///
/// Description: "Indicates a member of a Group."
pub const MEMBER: &str = "http://xmlns.com/foaf/0.1/member";

/// `foaf:interest` — A page about a topic of interest to this person.
///
/// Label: "interest"
///
/// Description: "A page about a topic of interest to this person."
pub const INTEREST: &str = "http://xmlns.com/foaf/0.1/interest";

/// `foaf:currentProject` — A current project this person works on.
///
/// Label: "current project"
///
/// Description: "A current project this person works on."
pub const CURRENT_PROJECT: &str = "http://xmlns.com/foaf/0.1/currentProject";

/// `foaf:pastProject` — A project this person has previously worked on.
///
/// Label: "past project"
///
/// Description: "A project this person has previously worked on."
pub const PAST_PROJECT: &str = "http://xmlns.com/foaf/0.1/pastProject";

/// `foaf:fundedBy` — An organization funding a project or person.
///
/// Label: "funded by"
///
/// Description: "An organization funding a project or person."
pub const FUNDED_BY: &str = "http://xmlns.com/foaf/0.1/fundedBy";

// ── Document/image properties ─────────────────────────────────────────────────

/// `foaf:topic` — A topic of some page or document.
///
/// Label: "topic"
///
/// Description: "A topic of some page or document."
pub const TOPIC: &str = "http://xmlns.com/foaf/0.1/topic";

/// `foaf:primaryTopic` — The primary topic of some page or document.
///
/// Label: "primary topic"
///
/// Description: "The primary topic of some page or document."
pub const PRIMARY_TOPIC: &str = "http://xmlns.com/foaf/0.1/primaryTopic";

/// `foaf:isPrimaryTopicOf` — A document that this thing is the primary topic of.
///
/// Label: "is primary topic of"
///
/// Description: "A document that this thing is the primary topic of."
pub const IS_PRIMARY_TOPIC_OF: &str = "http://xmlns.com/foaf/0.1/isPrimaryTopicOf";

/// `foaf:page` — A page or document about this thing.
///
/// Label: "page"
///
/// Description: "A page or document about this thing."
pub const PAGE: &str = "http://xmlns.com/foaf/0.1/page";

/// `foaf:depiction` — A depiction of some thing.
///
/// Label: "depiction"
///
/// Description: "A depiction of some thing."
pub const DEPICTION: &str = "http://xmlns.com/foaf/0.1/depiction";

/// `foaf:depicts` — Something depicted in this image.
///
/// Label: "depicts"
///
/// Description: "Something depicted in this representation."
pub const DEPICTS: &str = "http://xmlns.com/foaf/0.1/depicts";

/// `foaf:thumbnail` — A thumbnail image of the main depiction.
///
/// Label: "thumbnail"
///
/// Description: "A thumbnail for a document or image."
pub const THUMBNAIL: &str = "http://xmlns.com/foaf/0.1/thumbnail";

/// `foaf:img` — An image that can be used to represent some thing.
///
/// Label: "image"
///
/// Description: "An image that can be used to represent some thing (ie. those depictions which are particularly representative of something, eg. one's photo on a homepage)."
pub const IMG: &str = "http://xmlns.com/foaf/0.1/img";

// ── Miscellaneous ─────────────────────────────────────────────────────────────

/// `foaf:maker` — An agent that made this thing.
///
/// Label: "maker"
///
/// Description: "An agent that made this thing."
pub const MAKER: &str = "http://xmlns.com/foaf/0.1/maker";

/// `foaf:made` — Something that was made by this agent.
///
/// Label: "made"
///
/// Description: "Something that was made by this agent."
pub const MADE: &str = "http://xmlns.com/foaf/0.1/made";

/// `foaf:logo` — A logo representing some thing.
///
/// Label: "logo"
///
/// Description: "A logo representing some thing."
pub const LOGO: &str = "http://xmlns.com/foaf/0.1/logo";

/// `foaf:tipjar` — A tipjar document for this agent.
///
/// Label: "tipjar"
///
/// Description: "A tipjar document for this agent, describing means for payment and reward."
pub const TIPJAR: &str = "http://xmlns.com/foaf/0.1/tipjar";

/// `foaf:sha1` — A SHA1 hash of some thing.
///
/// Label: "sha1sum (hex)"
///
/// Description: "A sha1sum hash, in hex."
pub const SHA1: &str = "http://xmlns.com/foaf/0.1/sha1";

/// `foaf:based_near` — A location that something is based near.
///
/// Label: "based near"
///
/// Description: "A location that something is based near, for some broadly human notion of near."
pub const BASED_NEAR: &str = "http://xmlns.com/foaf/0.1/based_near";

/// `foaf:gender` — The gender of this Agent.
///
/// Label: "gender"
///
/// Description: "The gender of this Agent (typically but not necessarily 'male' or 'female')."
pub const GENDER: &str = "http://xmlns.com/foaf/0.1/gender";

/// `foaf:age` — The age in years of some agent.
///
/// Label: "age"
///
/// Description: "The age in years of some agent."
pub const AGE: &str = "http://xmlns.com/foaf/0.1/age";

/// `foaf:birthday` — The birthday of this Agent.
///
/// Label: "birthday"
///
/// Description: "The birthday of this Agent, represented in mm-dd string form, eg. '12-31'."
pub const BIRTHDAY: &str = "http://xmlns.com/foaf/0.1/birthday";

/// `foaf:dnaChecksum` — A checksum for the DNA of some thing.
///
/// Label: "DNA checksum"
///
/// Description: "A checksum for the DNA of some thing. Joke."
pub const DNA_CHECKSUM: &str = "http://xmlns.com/foaf/0.1/dnaChecksum";

/// `foaf:membershipClass` — Indicates the class of individuals that are a member of a Group.
///
/// Label: "membershipClass"
///
/// Description: "Indicates the class of individuals that are a member of a Group."
pub const MEMBERSHIP_CLASS: &str = "http://xmlns.com/foaf/0.1/membershipClass";
