//! XSD (XML Schema Definition) datatype terms.
//!
//! Namespace: `http://www.w3.org/2001/XMLSchema#`
//! Reference: <https://www.w3.org/TR/xmlschema11-2/>

/// XSD namespace IRI (trailing `#`).
pub const NS: &str = "http://www.w3.org/2001/XMLSchema#";

// ── String types ─────────────────────────────────────────────────────────────

/// `xsd:string` — Character strings in XML.
///
/// Label: "string"
///
/// Description: "The datatype corresponding to character strings in XML."
pub const STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// `xsd:normalizedString` — Whitespace-normalised string (no CR/LF/tab).
///
/// Label: "normalizedString"
///
/// Description: "A string that does not contain carriage return, line feed, or tab characters."
pub const NORMALIZED_STRING: &str = "http://www.w3.org/2001/XMLSchema#normalizedString";

/// `xsd:token` — Tokenised string (no leading/trailing/internal doubled whitespace).
///
/// Label: "token"
///
/// Description: "A normalized string with no leading or trailing white space and no internal sequences of two or more spaces."
pub const TOKEN: &str = "http://www.w3.org/2001/XMLSchema#token";

/// `xsd:language` — A valid XML language tag (e.g. `en`, `fr-BE`).
///
/// Label: "language"
///
/// Description: "A string that represents a natural language identifier as defined by RFC 3066."
pub const LANGUAGE: &str = "http://www.w3.org/2001/XMLSchema#language";

/// `xsd:Name` — An XML Name production value.
///
/// Label: "Name"
///
/// Description: "A string that is a valid XML Name."
pub const NAME: &str = "http://www.w3.org/2001/XMLSchema#Name";

/// `xsd:NCName` — An XML non-colonised Name.
///
/// Label: "`NCName`"
///
/// Description: "A string that is a valid XML non-colonized name (no colon allowed)."
pub const NC_NAME: &str = "http://www.w3.org/2001/XMLSchema#NCName";

/// `xsd:NMTOKEN` — An XML NMTOKEN production value.
///
/// Label: "NMTOKEN"
///
/// Description: "A string that is a valid XML NMTOKEN (name token)."
pub const NMTOKEN: &str = "http://www.w3.org/2001/XMLSchema#NMTOKEN";

// ── Boolean ───────────────────────────────────────────────────────────────────

/// `xsd:boolean` — The Boolean values `true` and `false`.
///
/// Label: "boolean"
///
/// Description: "The datatype corresponding to the two-valued Boolean logic: true and false."
pub const BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

// ── Numeric — decimal hierarchy ───────────────────────────────────────────────

/// `xsd:decimal` — Arbitrary-precision decimal numbers.
///
/// Label: "decimal"
///
/// Description: "Arbitrary-precision decimal numbers."
pub const DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";

/// `xsd:integer` — Arbitrary-size integer numbers.
///
/// Label: "integer"
///
/// Description: "Arbitrary-size integer numbers derived from decimal."
pub const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

/// `xsd:long` — 64-bit signed integer (-9223372036854775808 to 9223372036854775807).
///
/// Label: "long"
///
/// Description: "A signed 64-bit integer."
pub const LONG: &str = "http://www.w3.org/2001/XMLSchema#long";

/// `xsd:int` — 32-bit signed integer (-2147483648 to 2147483647).
///
/// Label: "int"
///
/// Description: "A signed 32-bit integer."
pub const INT: &str = "http://www.w3.org/2001/XMLSchema#int";

/// `xsd:short` — 16-bit signed integer (-32768 to 32767).
///
/// Label: "short"
///
/// Description: "A signed 16-bit integer."
pub const SHORT: &str = "http://www.w3.org/2001/XMLSchema#short";

/// `xsd:byte` — 8-bit signed integer (-128 to 127).
///
/// Label: "byte"
///
/// Description: "A signed 8-bit integer."
pub const BYTE: &str = "http://www.w3.org/2001/XMLSchema#byte";

/// `xsd:nonNegativeInteger` — Integers >= 0.
///
/// Label: "nonNegativeInteger"
///
/// Description: "An integer with a minimum value of 0."
pub const NON_NEGATIVE_INTEGER: &str =
    "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";

/// `xsd:positiveInteger` — Integers > 0.
///
/// Label: "positiveInteger"
///
/// Description: "An integer with a minimum value of 1."
pub const POSITIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#positiveInteger";

/// `xsd:unsignedLong` — Unsigned 64-bit integer (0 to 18446744073709551615).
///
/// Label: "unsignedLong"
///
/// Description: "An unsigned 64-bit integer."
pub const UNSIGNED_LONG: &str = "http://www.w3.org/2001/XMLSchema#unsignedLong";

/// `xsd:unsignedInt` — Unsigned 32-bit integer (0 to 4294967295).
///
/// Label: "unsignedInt"
///
/// Description: "An unsigned 32-bit integer."
pub const UNSIGNED_INT: &str = "http://www.w3.org/2001/XMLSchema#unsignedInt";

/// `xsd:unsignedShort` — Unsigned 16-bit integer (0 to 65535).
///
/// Label: "unsignedShort"
///
/// Description: "An unsigned 16-bit integer."
pub const UNSIGNED_SHORT: &str = "http://www.w3.org/2001/XMLSchema#unsignedShort";

/// `xsd:unsignedByte` — Unsigned 8-bit integer (0 to 255).
///
/// Label: "unsignedByte"
///
/// Description: "An unsigned 8-bit integer."
pub const UNSIGNED_BYTE: &str = "http://www.w3.org/2001/XMLSchema#unsignedByte";

/// `xsd:nonPositiveInteger` — Integers <= 0.
///
/// Label: "nonPositiveInteger"
///
/// Description: "An integer with a maximum value of 0."
pub const NON_POSITIVE_INTEGER: &str =
    "http://www.w3.org/2001/XMLSchema#nonPositiveInteger";

/// `xsd:negativeInteger` — Integers < 0.
///
/// Label: "negativeInteger"
///
/// Description: "An integer with a maximum value of -1."
pub const NEGATIVE_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#negativeInteger";

// ── Numeric — floating-point ──────────────────────────────────────────────────

/// `xsd:float` — IEEE 754 32-bit floating-point.
///
/// Label: "float"
///
/// Description: "A 32-bit floating-point number as defined by IEEE 754-2008."
pub const FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";

/// `xsd:double` — IEEE 754 64-bit floating-point.
///
/// Label: "double"
///
/// Description: "A 64-bit floating-point number as defined by IEEE 754-2008."
pub const DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";

// ── Date/time ─────────────────────────────────────────────────────────────────

/// `xsd:dateTime` — ISO 8601 combined date and time.
///
/// Label: "dateTime"
///
/// Description: "A date and time value as defined by ISO 8601."
pub const DATE_TIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

/// `xsd:dateTimeStamp` — ISO 8601 date-time with mandatory timezone.
///
/// Label: "dateTimeStamp"
///
/// Description: "A date and time with a required timezone offset."
pub const DATE_TIME_STAMP: &str = "http://www.w3.org/2001/XMLSchema#dateTimeStamp";

/// `xsd:date` — ISO 8601 date (no time component).
///
/// Label: "date"
///
/// Description: "A calendar date as defined by ISO 8601."
pub const DATE: &str = "http://www.w3.org/2001/XMLSchema#date";

/// `xsd:time` — ISO 8601 time of day.
///
/// Label: "time"
///
/// Description: "A time of day as defined by ISO 8601."
pub const TIME: &str = "http://www.w3.org/2001/XMLSchema#time";

/// `xsd:gYear` — A Gregorian calendar year.
///
/// Label: "gYear"
///
/// Description: "A Gregorian calendar year."
pub const G_YEAR: &str = "http://www.w3.org/2001/XMLSchema#gYear";

/// `xsd:gYearMonth` — A Gregorian year and month.
///
/// Label: "gYearMonth"
///
/// Description: "A specific Gregorian month in a specific Gregorian year."
pub const G_YEAR_MONTH: &str = "http://www.w3.org/2001/XMLSchema#gYearMonth";

/// `xsd:gMonth` — A Gregorian month.
///
/// Label: "gMonth"
///
/// Description: "A Gregorian month that recurs every year."
pub const G_MONTH: &str = "http://www.w3.org/2001/XMLSchema#gMonth";

/// `xsd:gMonthDay` — A Gregorian month-day combination.
///
/// Label: "gMonthDay"
///
/// Description: "A Gregorian date that recurs every year."
pub const G_MONTH_DAY: &str = "http://www.w3.org/2001/XMLSchema#gMonthDay";

/// `xsd:gDay` — A Gregorian day of the month.
///
/// Label: "gDay"
///
/// Description: "A Gregorian day that recurs every month."
pub const G_DAY: &str = "http://www.w3.org/2001/XMLSchema#gDay";

/// `xsd:duration` — ISO 8601 duration.
///
/// Label: "duration"
///
/// Description: "A duration of time as defined by ISO 8601."
pub const DURATION: &str = "http://www.w3.org/2001/XMLSchema#duration";

/// `xsd:yearMonthDuration` — Duration restricted to years and months.
///
/// Label: "yearMonthDuration"
///
/// Description: "A duration expressed in years and months only."
pub const YEAR_MONTH_DURATION: &str =
    "http://www.w3.org/2001/XMLSchema#yearMonthDuration";

/// `xsd:dayTimeDuration` — Duration restricted to days, hours, minutes, seconds.
///
/// Label: "dayTimeDuration"
///
/// Description: "A duration expressed in days, hours, minutes, and seconds only."
pub const DAY_TIME_DURATION: &str = "http://www.w3.org/2001/XMLSchema#dayTimeDuration";

// ── Binary ────────────────────────────────────────────────────────────────────

/// `xsd:base64Binary` — Base64-encoded binary data.
///
/// Label: "base64Binary"
///
/// Description: "Binary data encoded using Base64 encoding."
pub const BASE64_BINARY: &str = "http://www.w3.org/2001/XMLSchema#base64Binary";

/// `xsd:hexBinary` — Hex-encoded binary data.
///
/// Label: "hexBinary"
///
/// Description: "Binary data represented as a sequence of hexadecimal digits."
pub const HEX_BINARY: &str = "http://www.w3.org/2001/XMLSchema#hexBinary";

// ── URI/IRI ───────────────────────────────────────────────────────────────────

/// `xsd:anyURI` — A URI reference.
///
/// Label: "anyURI"
///
/// Description: "A Uniform Resource Identifier reference."
pub const ANY_URI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";

// ── Miscellaneous ─────────────────────────────────────────────────────────────

/// `xsd:QName` — An XML qualified name (prefix:local).
///
/// Label: "`QName`"
///
/// Description: "A qualified name in XML namespace-qualified form."
pub const Q_NAME: &str = "http://www.w3.org/2001/XMLSchema#QName";

/// `xsd:NOTATION` — An XML NOTATION production value.
///
/// Label: "NOTATION"
///
/// Description: "A set of `QNames`."
pub const NOTATION: &str = "http://www.w3.org/2001/XMLSchema#NOTATION";

/// `xsd:anyAtomicType` — The root of all atomic simple types.
///
/// Label: "anyAtomicType"
///
/// Description: "The base type for all atomic XSD types."
pub const ANY_ATOMIC_TYPE: &str = "http://www.w3.org/2001/XMLSchema#anyAtomicType";

/// `xsd:anySimpleType` — The root of all simple types.
///
/// Label: "anySimpleType"
///
/// Description: "The abstract base type for all XSD simple types."
pub const ANY_SIMPLE_TYPE: &str = "http://www.w3.org/2001/XMLSchema#anySimpleType";

/// `xsd:anyType` — The root of the entire XSD type hierarchy.
///
/// Label: "anyType"
///
/// Description: "The abstract base type for all XSD types, both simple and complex."
pub const ANY_TYPE: &str = "http://www.w3.org/2001/XMLSchema#anyType";
