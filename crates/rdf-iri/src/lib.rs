//! Main RFC 3987 IRI parser, normaliser, and RFC 3986 §5 resolver.
//!
//! This is the *main* Phase-A IRI implementation referenced by ADR-0020 §1.4.
//! The shadow at `crates/syntax/rdf-iri-shadow` is an independent second
//! implementation; both sides are compared by the `rdf-diff` harness via
//! their respective [`rdf_diff::Parser`] implementations.
//!
//! # Pinned readings
//!
//! - `docs/spec-readings/iri/percent-encoding-3986-vs-3987.md`
//!   (`IRI-PCT-001`): IRI equality is byte-for-byte Simple String
//!   Comparison after base resolution. We do **not** hex-case-fold
//!   percent-encoded octets, decode unreserved percent-encodings, fold
//!   host case, or apply Unicode NFC/NFD at parse time.
//! - `docs/spec-readings/iri/idna-host-normalisation-pin.md`: `ToASCII`
//!   (RFC 3490 Punycode + UTS 46 mapping) is applied by [`Iri::to_uri`]
//!   via the `idna` crate. Inputs `idna` rejects (disallowed code
//!   points, malformed `xn--` labels, empty host) fall through to
//!   ASCII-lowercase + percent-encode UTF-8 bytes.
//! - RFC 3986 §5.2.2 (merge) + §5.2.4 (`remove_dot_segments`, with
//!   errata 4005): applied during [`Iri::resolve`].
//! - RFC 3987 §3.1 (Converting IRIs to URIs): applied during the
//!   [`Iri::to_uri`] helper.
//!
//! # Public surface
//!
//! - [`Iri`] — validated (pre-checked) IRI reference, either absolute
//!   or relative.
//! - [`Iri::parse`] — validate bytes as an RFC 3987 IRI reference.
//! - [`Iri::normalise`] — apply the narrow normalisations the pin
//!   permits (scheme case, path dot-segment removal).
//! - [`Iri::resolve`] — RFC 3986 §5 reference resolution.
//! - [`Iri::to_uri`] — RFC 3987 §3.1 IRI → URI mapping.
//! - [`IriParser`] — zero-sized type implementing [`rdf_diff::Parser`].
//!
//! # Diagnostics
//!
//! Until `rdf-diagnostics` lands, this crate defines a local
//! [`diagnostic::Diagnostic`] stub. The swap is a follow-up PR.
//!
//! [`rdf_diff::Parser`]: rdf_diff::Parser

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod diagnostic;
mod normalise;
mod parse;
mod parser_impl;
mod resolve;
#[cfg(test)]
mod tests;

use std::fmt;

pub use diagnostic::{Diagnostic, DiagnosticCode};
pub use parser_impl::IriParser;

/// A validated RFC 3987 IRI reference.
///
/// Equality is **byte-for-byte** on the stored IRI character sequence
/// after [`Iri::resolve`]-time base resolution only; see `IRI-PCT-001`.
/// No percent-encoding normalisation, no host case folding, no Unicode
/// NFC/NFD is applied during [`Iri::parse`].
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iri {
    raw: String,
    parts: Components,
}

/// Components of an IRI reference as decomposed by [`Iri::parse`].
///
/// Offsets are byte indices into the raw IRI string. A `None` range
/// means the component is absent.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub(crate) struct Components {
    pub(crate) scheme: Option<(usize, usize)>,
    pub(crate) authority: Option<(usize, usize)>,
    pub(crate) userinfo: Option<(usize, usize)>,
    pub(crate) host: Option<(usize, usize)>,
    pub(crate) port: Option<(usize, usize)>,
    pub(crate) path: (usize, usize),
    pub(crate) query: Option<(usize, usize)>,
    pub(crate) fragment: Option<(usize, usize)>,
}

impl fmt::Debug for Iri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Iri").field(&self.raw).finish()
    }
}

impl fmt::Display for Iri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl Iri {
    /// Parse `input` as an RFC 3987 IRI reference.
    ///
    /// Accepts both absolute IRIs (with a scheme) and relative
    /// references. No normalisation is applied; the stored byte
    /// sequence equals `input` verbatim.
    ///
    /// # Errors
    ///
    /// Returns a [`Diagnostic`] whose [`DiagnosticCode`] identifies the
    /// failure class (`IRI-SYNTAX-001`, `IRI-PCT-001`, …). The
    /// `IRI-PCT-001` code is emitted on percent-encoding shape errors
    /// per the pin in `docs/spec-readings/iri/percent-encoding-3986-vs-3987.md`.
    pub fn parse(input: &str) -> Result<Self, Diagnostic> {
        parse::parse(input)
    }

    /// The raw IRI character sequence, byte-for-byte.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// The scheme subcomponent, if present. ASCII lowercase is **not**
    /// applied by this accessor; callers that need canonical case must
    /// go through [`Iri::normalise`].
    #[must_use]
    pub fn scheme(&self) -> Option<&str> {
        self.parts.scheme.map(|(a, b)| &self.raw[a..b])
    }

    /// `true` iff the IRI has a scheme (i.e., is absolute).
    #[must_use]
    pub const fn is_absolute(&self) -> bool {
        self.parts.scheme.is_some()
    }

    /// Authority subcomponent, if present.
    #[must_use]
    pub fn authority(&self) -> Option<&str> {
        self.parts.authority.map(|(a, b)| &self.raw[a..b])
    }

    /// Host subcomponent, if present.
    #[must_use]
    pub fn host(&self) -> Option<&str> {
        self.parts.host.map(|(a, b)| &self.raw[a..b])
    }

    /// Path subcomponent. Always present; may be the empty string.
    #[must_use]
    pub fn path(&self) -> &str {
        let (a, b) = self.parts.path;
        &self.raw[a..b]
    }

    /// Query subcomponent, excluding the leading `?`.
    #[must_use]
    pub fn query(&self) -> Option<&str> {
        self.parts.query.map(|(a, b)| &self.raw[a..b])
    }

    /// Fragment subcomponent, excluding the leading `#`.
    #[must_use]
    pub fn fragment(&self) -> Option<&str> {
        self.parts.fragment.map(|(a, b)| &self.raw[a..b])
    }

    /// Apply the narrow normalisations permitted by `IRI-PCT-001`:
    ///
    /// - **Scheme case.** ASCII-lowercased (RFC 3986 §6.2.2.1).
    /// - **Host case.** ASCII-lowercased (RFC 3490 §4 step 2).
    ///   Already-ACE labels (`xn--…`) are ASCII by construction, so
    ///   lowercasing preserves them verbatim. Non-ASCII hosts are
    ///   **not** mutated here — `ToASCII` runs during
    ///   [`Iri::to_uri`], not during [`Iri::normalise`], because it
    ///   changes the host byte sequence and would break the pin's
    ///   "no silent normalisation" guarantee for equality callers.
    /// - **Path dot-segment removal** for absolute IRIs (RFC 3986
    ///   §5.2.4, errata 4005).
    ///
    /// **Not** applied (per the pin):
    ///
    /// - No percent-encoding hex case folding.
    /// - No percent-decoding of unreserved characters.
    /// - No Unicode NFC/NFD.
    /// - No empty-path-normalised-to-`/` rewriting when the authority
    ///   is absent (that would change meaning for `mailto:` etc.).
    ///
    /// Returns a fresh [`Iri`]; the receiver is untouched.
    #[must_use]
    pub fn normalise(&self) -> Self {
        normalise::normalise(self)
    }

    /// Resolve this reference against `base` per RFC 3986 §5. The
    /// "strict" algorithm is used (no scheme-inheritance shortcut).
    ///
    /// # Panics
    ///
    /// Panics if `base` is not absolute (has no scheme). RFC 3986 §5.1
    /// requires the base to be absolute; callers are expected to have
    /// established one.
    #[must_use]
    pub fn resolve(&self, base: &Self) -> Self {
        resolve::resolve(self, base)
    }

    /// Convert this IRI to an RFC 3986 URI per RFC 3987 §3.1.
    ///
    /// Non-ASCII characters in `iunreserved` positions (path, query,
    /// fragment, userinfo) are UTF-8 encoded and percent-escaped. Host
    /// `ireg-name` labels are passed through RFC 3490 `ToASCII` (UTS
    /// 46 strict profile) via the `idna` crate: pure-ASCII labels are
    /// lowercased locally, Unicode labels are Punycode-encoded
    /// (`xn--…`). When `idna` rejects the input (disallowed code
    /// points, empty host, malformed existing `xn--` label), we fall
    /// back to ASCII-lowercasing + percent-encoding the host's
    /// non-ASCII UTF-8 bytes — see
    /// `docs/spec-readings/iri/idna-host-normalisation-pin.md`.
    ///
    /// # Errors
    ///
    /// Returns [`Diagnostic`] when the IRI contains a control
    /// character forbidden in URIs (RFC 3986 §2.2).
    pub fn to_uri(&self) -> Result<String, Diagnostic> {
        normalise::to_uri(self)
    }
}

// -----------------------------------------------------------------------
// Internal constructors used by sibling modules.
// -----------------------------------------------------------------------

impl Iri {
    pub(crate) const fn from_raw(raw: String, parts: Components) -> Self {
        Self { raw, parts }
    }
}
