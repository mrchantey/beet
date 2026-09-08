//! Application-friendly URL type for routing and request construction.
//!
//! [`Url`] is a structured URI-reference: scheme, authority, rootedness, path
//! segments, query params and fragment, each held **decoded** and re-encoded on
//! [`Display`](core::fmt::Display). So `url.path()` yields the text an author
//! meant (`my file.txt`) while `url.to_string()` yields the text a wire wants
//! (`my%20file.txt`), and the round trip is stable.
//!
//! Rootedness is a field of its own, because `blog/post-1` and `/blog/post-1`
//! are different references and a type that cannot tell them apart cannot carry
//! a relative link. An authority always implies a rooted path.
//!
//! Three entry points, by how much they promise:
//!
//! - [`Url::coerce`] never fails, repairing what it can (a raw space becomes
//!   `%20`). For internal strings that are already valid.
//! - [`Url::parse`] rejects what cannot be repaired: an empty input or an ASCII
//!   control character. The authoring seam, ie the [`LiteralParser`] entry.
//! - [`Url::parse_absolute`] additionally requires a scheme and an authority.
//!   For a url that will be resolved against nothing, ie a sitemap `<loc>`.
//!
//! Data URIs (RFC 2397) and the other non-hierarchical schemes (`mailto:`,
//! `tel:`, `javascript:`, ...) hold their payload verbatim as a single opaque
//! path segment, neither split on `/` nor percent-coded.
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//! let url = Url::coerce("https://example.com/api/users?limit=10#results");
//! assert_eq!(url.scheme(), &Scheme::Https);
//! assert_eq!(url.authority(), Some("example.com"));
//! assert_eq!(url.path(), &["api", "users"]);
//! assert!(url.is_rooted());
//! assert_eq!(url.get_param("limit"), Some("10"));
//! assert_eq!(url.fragment(), Some("results"));
//! assert_eq!(url.to_string(), "https://example.com/api/users?limit=10#results");
//! ```

use crate::prelude::*;
use alloc::borrow::Cow;

/// An application-friendly URI-reference. See the [module docs](self) for the
/// model and the three parsing entry points.
///
/// Reflect-opaque: a url is a scalar to whoever authors one, not a struct to
/// edit field by field, so a schema UI renders it as a text input and the
/// [`LiteralParser`] owns its spelling. Its serde form is that same string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(opaque)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Url {
	/// ie `https://`, `mailto:`
	scheme: Scheme,
	/// ie `example.com`
	authority: Option<SmolStr>,
	/// Whether the path begins at `/`. See [`is_rooted`](Self::is_rooted).
	root: bool,
	/// ie `path/to/page`
	path: Vec<SmolStr>,
	/// ie `?foo=bar&bazz`
	params: MultiMap<SmolStr, SmolStr>,
	/// ie `#some-heading`
	fragment: Option<SmolStr>,
}

/// The root url `/`: rooted, with no scheme, authority, path, params or
/// fragment.
impl Default for Url {
	fn default() -> Self { Self::ROOT }
}

impl Url {
	/// The root url `/`, ie a rooted empty path.
	pub const ROOT: Url = Url {
		scheme: Scheme::None,
		authority: None,
		root: true,
		path: Vec::new(),
		params: MultiMap::new(),
		fragment: None,
	};

	/// Read `input` as a url, repairing whatever it can rather than failing.
	///
	/// A raw space (or any other character illegal unescaped) is percent-coded
	/// on the way out, and a malformed `%` escape is kept as a literal `%` and
	/// re-emitted as `%25`. Use it for a string this codebase produced; use
	/// [`parse`](Self::parse) for one a human wrote.
	pub fn coerce(input: impl AsRef<str>) -> Self {
		let input = input.as_ref();

		// a data URI is fully opaque: `#`, `?` and `/` inside the payload are
		// content, not delimiters, so it short-circuits every rule below.
		if let Some(payload) = input.strip_prefix("data:") {
			return Self {
				scheme: Scheme::Data,
				authority: None,
				root: false,
				path: match payload.is_empty() {
					true => Vec::new(),
					false => vec![payload.into()],
				},
				params: default(),
				fragment: None,
			};
		}

		// fragment first, then query: each ends the part before it
		let (before_fragment, fragment) = match input.split_once('#') {
			Some((before, fragment)) if !fragment.is_empty() => {
				(before, Some(percent::decode(fragment, false)))
			}
			Some((before, _)) => (before, None),
			None => (input, None),
		};
		let (before_query, query) = match before_fragment.split_once('?') {
			Some((before, query)) => (before, Some(query)),
			None => (before_fragment, None),
		};
		let params = query.map(Self::parse_query_string).unwrap_or_default();

		let (scheme, rest) = split_scheme(before_query);

		// a hierarchical scheme's authority runs to the first `/`; a
		// non-hierarchical one has none and keeps its payload in the path.
		let (authority, path) = match (&scheme, scheme.is_hierarchical()) {
			(Scheme::None, _) => (None, rest),
			(_, false) => (None, rest),
			(_, true) => match rest.split_once('/') {
				Some((authority, path)) if !authority.is_empty() => {
					(Some(SmolStr::from(authority)), path)
				}
				// the whole rest is the authority, with no path at all
				_ if !rest.is_empty() && !rest.starts_with('/') => {
					(Some(SmolStr::from(rest)), "")
				}
				_ => (None, rest),
			},
		};

		Self {
			// an authority is always followed by a rooted path, and a
			// non-hierarchical payload is neither rooted nor relative
			root: authority.is_some() || path.starts_with('/'),
			path: match scheme.is_hierarchical() || scheme == Scheme::None {
				true => Self::split_path(path),
				// opaque payload: one segment, verbatim
				false if path.is_empty() => Vec::new(),
				false => vec![path.into()],
			},
			scheme,
			authority,
			params,
			fragment,
		}
	}

	/// Read `input` as a url, rejecting what no encoding can repair: an empty
	/// input, or an ASCII control character (a newline, a tab, a stray byte).
	///
	/// The authoring seam. A space is NOT rejected, since its repair is
	/// unambiguous: `image_url = "/assets/my card.png"` is honoured rather than
	/// scolded.
	///
	/// # Errors
	/// Errors naming the offending input, so a malformed frontmatter value
	/// fails loudly rather than landing as a url nobody meant.
	pub fn parse(input: impl AsRef<str>) -> Result<Self> {
		let input = input.as_ref();
		if input.is_empty() {
			bevybail!("invalid url: expected a url, found an empty string");
		}
		if let Some(char) = input.chars().find(|char| char.is_control()) {
			bevybail!(
				"invalid url {input:?}: contains the control character {char:?}"
			);
		}
		Self::coerce(input).xok()
	}

	/// [`parse`](Self::parse) for a url that will be resolved against nothing:
	/// it must additionally name a scheme and an authority.
	///
	/// # Errors
	/// Errors when the url is relative or names no host, which is the failure a
	/// sitemap `<loc>`, a feed `<link>` and a social-card image each need to be
	/// loud about: a crawler resolving one has no base to resolve it against.
	pub fn parse_absolute(input: impl AsRef<str>) -> Result<Self> {
		let url = Self::parse(input.as_ref())?;
		if url.scheme == Scheme::None || url.authority.is_none() {
			bevybail!(
				"invalid absolute url {:?}: expected a scheme and a host, ie `https://example.com/path`",
				input.as_ref()
			);
		}
		url.xok()
	}

	/// Create a rooted url from individual components, ie the decomposed parts
	/// of a request an http or cli transport already split for us.
	pub fn new(
		scheme: Scheme,
		authority: Option<SmolStr>,
		path: Vec<SmolStr>,
		params: MultiMap<SmolStr, SmolStr>,
		fragment: Option<SmolStr>,
	) -> Self {
		Self {
			scheme,
			authority,
			root: true,
			path,
			params,
			fragment,
		}
	}

	/// The scheme of the URL.
	pub fn scheme(&self) -> &Scheme { &self.scheme }

	/// Set the scheme.
	pub fn with_scheme(mut self, scheme: Scheme) -> Self {
		self.scheme = scheme;
		self
	}

	/// Whether the path begins at `/`, ie the reference is resolved against the
	/// origin rather than against the document that holds it.
	///
	/// The difference between `<a href="/blog/x">` and `<a href="x">`, which is
	/// what lets a relative link (a redirect target resolved against its route's
	/// scope, a markdown link beside its page) survive as a url rather than
	/// being silently rooted.
	pub fn is_rooted(&self) -> bool { self.root }

	/// Whether this url points to another origin, ie it carries an authority.
	///
	/// A bare path (`/about`, `next`) is internal; `https://other.example/x` is
	/// external. The classification a link handler uses to decide whether to
	/// navigate in-app or leave it.
	pub fn is_external(&self) -> bool { self.authority.is_some() }

	/// Set whether the path is rooted.
	pub fn with_rooted(mut self, root: bool) -> Self {
		self.root = root;
		self
	}

	/// Root the path in place, ie what a transport does to a url it is about to
	/// put on the wire, where only an absolute-path form is legal.
	pub fn set_rooted(&mut self, root: bool) -> &mut Self {
		self.root = root;
		self
	}

	/// The authority (host and optional port), if present.
	pub fn authority(&self) -> Option<&str> { self.authority.as_deref() }

	/// The authority split into host and explicit port. An ipv6 host keeps its
	/// brackets, ie `[::1]:80` -> `("[::1]", Some(80))`.
	pub fn host_and_port(&self) -> Option<(&str, Option<u16>)> {
		let authority = self.authority.as_deref()?;
		// bracketed ipv6: the port is whatever follows the closing bracket
		if let Some(end) = authority.find(']') {
			let (host, rest) = authority.split_at(end + 1);
			let port =
				rest.strip_prefix(':').and_then(|port| port.parse().ok());
			return Some((host, port));
		}
		match authority.rsplit_once(':') {
			Some((host, port)) => match port.parse().ok() {
				Some(port) => Some((host, Some(port))),
				None => Some((authority, None)),
			},
			None => Some((authority, None)),
		}
	}

	/// The host portion of the authority, without the port.
	pub fn host(&self) -> Option<&str> {
		self.host_and_port().map(|(host, _)| host)
	}

	/// The explicit port in the authority, if any.
	pub fn port(&self) -> Option<u16> {
		self.host_and_port().and_then(|(_, port)| port)
	}

	/// The explicit port, or the scheme's well-known default
	/// (see [`Scheme::default_port`]).
	pub fn port_or_default(&self) -> Option<u16> {
		self.port().or_else(|| self.scheme.default_port())
	}

	/// The last path segment's file extension, eg `Some("jpg")` for
	/// `/assets/x.jpg`, `Some("css")` for `/style.css`, `None` for a page route
	/// (`/about`, `/blog/post-6`).
	///
	/// The link handler keys off this: a link to a served file is handed off /
	/// opened rather than navigating the in-app router to a path that has no page
	/// route. An extension is a final `.` with a non-empty alphanumeric stem and
	/// suffix, so a dotted route segment without a real suffix is not mistaken for
	/// a file.
	pub fn file_extension(&self) -> Option<&str> {
		self.last_segment()
			.and_then(|segment| segment.rsplit_once('.'))
			.filter(|(stem, ext)| {
				!stem.is_empty()
					&& !ext.is_empty()
					&& ext.chars().all(|char| char.is_ascii_alphanumeric())
			})
			.map(|(_, ext)| ext)
	}

	/// Set the authority, which also roots the path: a host is always followed
	/// by an absolute path.
	pub fn with_authority(mut self, authority: impl Into<SmolStr>) -> Self {
		self.authority = Some(authority.into());
		self.root = true;
		self
	}

	/// The path segments, decoded.
	pub fn path(&self) -> &Vec<SmolStr> { &self.path }

	/// A mutable reference to the path segments.
	pub fn path_mut(&mut self) -> &mut Vec<SmolStr> { &mut self.path }

	/// Set the path segments.
	pub fn with_path(mut self, path: Vec<SmolStr>) -> Self {
		self.path = path;
		self
	}

	/// Set the path segments.
	pub fn set_path(&mut self, path: Vec<SmolStr>) -> &mut Self {
		self.path = path;
		self
	}

	/// All query parameters, decoded.
	pub fn params(&self) -> &MultiMap<SmolStr, SmolStr> { &self.params }

	/// A mutable reference to the query parameters.
	pub fn params_mut(&mut self) -> &mut MultiMap<SmolStr, SmolStr> {
		&mut self.params
	}

	/// Get the first value for a query parameter.
	pub fn get_param(&self, key: &str) -> Option<&str> {
		self.params
			.get_vec(key)
			.and_then(|vals| vals.first().map(|val| val.as_str()))
	}

	/// Check if a query parameter exists.
	pub fn has_param(&self, key: &str) -> bool { self.params.contains_key(key) }

	/// Add a query parameter, returning self for chaining.
	pub fn with_param(
		mut self,
		key: impl Into<SmolStr>,
		value: impl Into<SmolStr>,
	) -> Self {
		self.params.insert(key.into(), value.into());
		self
	}

	/// Add a flag parameter (key with no value).
	pub fn with_flag(mut self, key: impl Into<SmolStr>) -> Self {
		self.params.insert_key(key.into());
		self
	}

	/// The fragment identifier, if present.
	pub fn fragment(&self) -> Option<&str> { self.fragment.as_deref() }

	/// Set the fragment identifier.
	pub fn with_fragment(mut self, fragment: impl Into<SmolStr>) -> Self {
		self.fragment = Some(fragment.into());
		self
	}

	/// The path as it appears in the url, percent-encoded, with its leading `/`
	/// when [rooted](Self::is_rooted).
	///
	/// A rooted empty path is `/`; a relative empty path is the empty string.
	pub fn path_string(&self) -> String {
		let encoded = match self.scheme.is_opaque() {
			// an opaque payload is carried verbatim
			true => self.path.join("/"),
			false => self
				.path
				.iter()
				.map(|segment| percent::encode(segment, percent::PATH_SAFE))
				.collect::<Vec<_>>()
				.join("/"),
		};
		match self.root {
			true => format!("/{encoded}"),
			false => encoded,
		}
	}

	/// The query string built from parameters, percent-encoded.
	pub fn query_string(&self) -> String {
		Self::build_query_string(&self.params)
	}

	/// The first path segment, if any.
	pub fn first_segment(&self) -> Option<&str> {
		self.path.first().map(|seg| seg.as_str())
	}

	/// The last path segment, if any.
	pub fn last_segment(&self) -> Option<&str> {
		self.path.last().map(|seg| seg.as_str())
	}

	/// Path segments starting from the given index.
	pub fn path_from(&self, index: usize) -> &[SmolStr] {
		if index >= self.path.len() {
			&[]
		} else {
			&self.path[index..]
		}
	}

	/// Resolve `other` against this url as a base, the RFC 3986 reference
	/// resolution a browser applies to an `href`.
	///
	/// `other` wins outright when it names another scheme or another host, and
	/// a rooted `other` replaces the path. A RELATIVE `other` merges: it is
	/// appended beside this url's last segment, so `next` from `/docs/intro`
	/// resolves to `/docs/next`, and `.`/`..` segments collapse.
	pub fn join(&self, other: Url) -> Url {
		if other.scheme != Scheme::None && other.scheme != self.scheme {
			return other;
		}
		if other.authority.is_some() {
			return other;
		}
		let mut resolved = self.clone();
		resolved.params = other.params;
		resolved.fragment = other.fragment;
		resolved.path = match other.root {
			true => other.path,
			false => {
				// merge beside the base's last segment, as a relative `href` does
				let mut merged = self.path.clone();
				merged.pop();
				merged.extend(other.path);
				normalize_segments(merged)
			}
		};
		resolved
	}

	/// Push a path segment to the end of the path.
	pub fn push(mut self, segment: impl Into<SmolStr>) -> Self {
		self.path.push(segment.into());
		self
	}

	/// Redirect this URL onto another's destination: a copy of `self` whose
	/// scheme, authority and path are taken from `other`, keeping `self`'s query
	/// params and fragment.
	///
	/// Used to forward a request to a new target without losing its existing
	/// query/body semantics, ie pointing a `run` command at a remote device:
	/// the caller's params survive, only the destination changes.
	pub fn forward(&self, other: &Url) -> Url {
		Url {
			scheme: other.scheme.clone(),
			authority: other.authority.clone(),
			root: other.root,
			path: other.path.clone(),
			params: self.params.clone(),
			fragment: self.fragment.clone(),
		}
	}

	/// Split an encoded path string into decoded segments, dropping the empty
	/// ones a `//` or a trailing `/` leaves behind.
	pub fn split_path(path: &str) -> Vec<SmolStr> {
		path.split('/')
			.filter(|segment| !segment.is_empty())
			.map(|segment| percent::decode(segment, false))
			.collect()
	}

	/// Parse an encoded query string into decoded parameters. A pair with no
	/// `=` is a flag, ie a key with an empty value.
	pub fn parse_query_string(query: &str) -> MultiMap<SmolStr, SmolStr> {
		let mut params = MultiMap::default();
		for pair in query.split('&').filter(|pair| !pair.is_empty()) {
			let (key, value) = match pair.split_once('=') {
				Some((key, value)) => (key, value),
				None => (pair, ""),
			};
			// `+` is a space in a query, the form-encoding convention every
			// browser writes and every server reads
			params.insert(
				percent::decode(key, true),
				percent::decode(value, true),
			);
		}
		params
	}

	/// Build an encoded query string from parameters. A parameter with an empty
	/// value is written as a bare flag.
	///
	/// Keys are sorted, so the same params always render the same string:
	/// a url is used as a cache key and read in snapshots, and the backing map
	/// iterates in hash order. Repeated values under one key keep their order,
	/// which is the only order that carries meaning.
	pub fn build_query_string(params: &MultiMap<SmolStr, SmolStr>) -> String {
		let mut pairs: Vec<_> = params.iter_all().collect();
		pairs.sort_by(|(left, _), (right, _)| left.cmp(right));
		pairs
			.into_iter()
			.flat_map(|(key, values)| {
				values.iter().map(move |value| {
					let key = percent::encode(key, percent::QUERY_SAFE);
					match value.is_empty() {
						true => key,
						false => format!(
							"{key}={}",
							percent::encode(value, percent::QUERY_SAFE)
						),
					}
				})
			})
			.collect::<Vec<_>>()
			.join("&")
	}
}

/// Split a leading `scheme:` off `input`, along with the `//` that may follow
/// it. A scheme is an alphanumeric word (plus `+-.`) before the first `:`, so a
/// relative path carrying a colon is not mistaken for one.
fn split_scheme(input: &str) -> (Scheme, &str) {
	let Some((maybe_scheme, rest)) = input.split_once(':') else {
		return (Scheme::None, input);
	};
	let is_scheme = !maybe_scheme.is_empty()
		&& maybe_scheme.bytes().all(|byte| {
			byte.is_ascii_alphanumeric()
				|| byte == b'+'
				|| byte == b'-'
				|| byte == b'.'
		});
	match is_scheme {
		true => (
			Scheme::from_str(maybe_scheme),
			rest.strip_prefix("//").unwrap_or(rest),
		),
		false => (Scheme::None, input),
	}
}

/// Collapse the `.` and `..` segments a merged relative reference leaves
/// behind. A `..` past the root is dropped, as a browser drops it.
fn normalize_segments(segments: Vec<SmolStr>) -> Vec<SmolStr> {
	let mut out: Vec<SmolStr> = Vec::with_capacity(segments.len());
	for segment in segments {
		match segment.as_str() {
			"." => {}
			".." => {
				out.pop();
			}
			_ => out.push(segment),
		}
	}
	out
}

/// Percent-coding, the wire form of every character a url component may not
/// carry literally.
///
/// Each component has its own safe set, since `&` delimits a query pair but is
/// ordinary in a path, and `?` begins a query but is ordinary in a fragment.
mod percent {
	use crate::prelude::*;

	/// Legal unescaped in a path segment: the RFC 3986 unreserved set, the
	/// sub-delims, and the `:@` a segment may carry. `/` is absent, so a
	/// segment can never split itself in two.
	pub const PATH_SAFE: &str = "-._~!$&'()*+,;=:@";
	/// Legal unescaped in a query key or value: the path set minus the `&=+`
	/// that delimit pairs and encode spaces, plus the `/?` that are ordinary
	/// once a query has begun.
	pub const QUERY_SAFE: &str = "-._~!$'()*,;:@/?";
	/// Legal unescaped in a fragment: the query set plus `&=+`, none of which
	/// delimit anything after the `#`.
	pub const FRAGMENT_SAFE: &str = "-._~!$&'()*+,;=:@/?";

	/// `text` with every character outside the unreserved set and `safe`
	/// written as its `%XX` bytes.
	pub fn encode(text: &str, safe: &str) -> String {
		let mut out = String::with_capacity(text.len());
		for char in text.chars() {
			if char.is_ascii_alphanumeric() || safe.contains(char) {
				out.push(char);
			} else {
				let mut buffer = [0u8; 4];
				for byte in char.encode_utf8(&mut buffer).as_bytes() {
					out.push_str(&format!("%{byte:02X}"));
				}
			}
		}
		out
	}

	/// `text` with its `%XX` escapes decoded, and (when `plus_is_space`) `+`
	/// read as a space, the form-encoding convention a query carries.
	///
	/// Lenient: a malformed escape (`%zz`, a trailing `%`) is kept as the
	/// literal text it is, which [`encode`] then writes back as `%25zz`. Better
	/// a stable round trip than a rejected url at a seam that cannot fail.
	pub fn decode(text: &str, plus_is_space: bool) -> SmolStr {
		if !text.contains('%') && !(plus_is_space && text.contains('+')) {
			return SmolStr::new(text);
		}
		let bytes = text.as_bytes();
		let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
		let mut index = 0;
		while index < bytes.len() {
			match bytes[index] {
				b'+' if plus_is_space => {
					out.push(b' ');
					index += 1;
				}
				b'%' if index + 2 < bytes.len() => {
					match hex_pair(bytes[index + 1], bytes[index + 2]) {
						Some(byte) => {
							out.push(byte);
							index += 3;
						}
						None => {
							out.push(b'%');
							index += 1;
						}
					}
				}
				byte => {
					out.push(byte);
					index += 1;
				}
			}
		}
		// the decoded bytes may not be valid utf8 (a `%FF`), in which case the
		// text stands as written rather than becoming a replacement character
		match String::from_utf8(out) {
			Ok(decoded) => SmolStr::new(decoded),
			Err(_) => SmolStr::new(text),
		}
	}

	/// The byte two hex digits name, `None` if either is not a hex digit.
	fn hex_pair(high: u8, low: u8) -> Option<u8> {
		let digit = |byte: u8| (byte as char).to_digit(16);
		Some((digit(high)? * 16 + digit(low)?) as u8)
	}
}

impl core::fmt::Display for Url {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		match (&self.scheme, &self.authority) {
			(Scheme::None, _) => write!(formatter, "{}", self.path_string())?,
			// an opaque scheme writes `scheme:payload`, never `scheme://`
			(scheme, _) if scheme.is_opaque() => {
				write!(formatter, "{scheme}:{}", self.path.join("/"))?
			}
			(scheme, Some(authority)) => write!(
				formatter,
				"{scheme}://{authority}{}",
				self.path_string()
			)?,
			(scheme, None) => {
				write!(formatter, "{scheme}://{}", self.path_string())?
			}
		};

		let query = self.query_string();
		if !query.is_empty() {
			write!(formatter, "?{query}")?;
		}
		if let Some(fragment) = &self.fragment {
			write!(
				formatter,
				"#{}",
				percent::encode(fragment, percent::FRAGMENT_SAFE)
			)?;
		}
		Ok(())
	}
}

/// A url's serde form is the string it displays as, so a scene, a frontmatter
/// block and a json payload all carry `"https://beet.org/blog"` rather than a
/// struct nobody authored.
#[cfg(feature = "serde")]
impl serde::Serialize for Url {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Url {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Self, D::Error> {
		let text = alloc::string::String::deserialize(deserializer)?;
		Ok(Url::coerce(text))
	}
}

impl From<&str> for Url {
	fn from(value: &str) -> Self { Url::coerce(value) }
}

impl From<String> for Url {
	fn from(value: String) -> Self { Url::coerce(value) }
}
impl From<&String> for Url {
	fn from(value: &String) -> Self { Url::coerce(value) }
}
impl From<SmolStr> for Url {
	fn from(value: SmolStr) -> Self { Url::coerce(value.as_str()) }
}
impl From<&Url> for Url {
	fn from(value: &Url) -> Self { value.clone() }
}

/// A logical path is a RELATIVE url: [`SmolPath`] strips leading slashes by
/// design, so rooting one here would invent a distinction it never carried.
impl From<SmolPath> for Url {
	fn from(value: SmolPath) -> Url {
		Url::coerce(value.as_str()).with_rooted(false)
	}
}

impl From<Cow<'_, str>> for Url {
	fn from(value: Cow<'_, str>) -> Self { Url::coerce(value) }
}
impl From<&Cow<'_, str>> for Url {
	fn from(value: &Cow<'_, str>) -> Self { Url::coerce(value) }
}

/// The transport scheme of a URL.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Scheme {
	/// No scheme specified, ie an absolute or relative path.
	#[default]
	None,
	/// `http`
	Http,
	/// `https`
	Https,
	/// `file`
	File,
	/// `ws`
	Ws,
	/// `wss`
	Wss,
	/// `data` — inline data URIs (RFC 2397).
	Data,
	/// `mailto` — email addresses.
	MailTo,
	/// `tel` — telephone numbers.
	Tel,
	/// `javascript` — inline script execution.
	JavaScript,
	/// `blob` — binary large object references.
	Blob,
	/// `cid` — content identifiers (RFC 2392).
	Cid,
	/// `about` — browser internal pages, ie `about:blank`.
	About,
	/// `chrome` — browser internal pages.
	Chrome,
	/// A scheme not covered by the named variants.
	Other(String),
}

impl Scheme {
	/// Parse a scheme from a string.
	pub fn from_str(scheme: &str) -> Self {
		match scheme.to_ascii_lowercase().as_str() {
			"http" => Self::Http,
			"https" => Self::Https,
			"file" => Self::File,
			"ws" => Self::Ws,
			"wss" => Self::Wss,
			"data" => Self::Data,
			"mailto" => Self::MailTo,
			"tel" => Self::Tel,
			"javascript" => Self::JavaScript,
			"blob" => Self::Blob,
			"cid" => Self::Cid,
			"about" => Self::About,
			"chrome" => Self::Chrome,
			"" => Self::None,
			other => Self::Other(other.to_string()),
		}
	}

	/// The canonical string representation of the scheme.
	pub fn as_str(&self) -> &str {
		match self {
			Self::None => "",
			Self::Http => "http",
			Self::Https => "https",
			Self::File => "file",
			Self::Ws => "ws",
			Self::Wss => "wss",
			Self::Data => "data",
			Self::MailTo => "mailto",
			Self::Tel => "tel",
			Self::JavaScript => "javascript",
			Self::Blob => "blob",
			Self::Cid => "cid",
			Self::About => "about",
			Self::Chrome => "chrome",
			Self::Other(scheme) => scheme.as_str(),
		}
	}

	/// Whether this is an HTTP-based scheme.
	pub fn is_http(&self) -> bool { matches!(self, Self::Http | Self::Https) }

	/// Whether this is a WebSocket scheme.
	pub fn is_ws(&self) -> bool { matches!(self, Self::Ws | Self::Wss) }

	/// The scheme's well-known default port, if any.
	pub fn default_port(&self) -> Option<u16> {
		match self {
			Self::Http | Self::Ws => Some(80),
			Self::Https | Self::Wss => Some(443),
			_ => None,
		}
	}

	/// Whether this scheme uses TLS.
	pub fn is_secure(&self) -> bool { matches!(self, Self::Https | Self::Wss) }

	/// Whether this scheme uses a hierarchical authority (host) component.
	///
	/// Non-hierarchical schemes like `mailto:`, `tel:`, `data:`, `about:`,
	/// `blob:` place their content directly in the path with no authority.
	pub fn is_hierarchical(&self) -> bool {
		matches!(
			self,
			Self::Http
				| Self::Https
				| Self::File | Self::Ws
				| Self::Wss | Self::Chrome
		)
	}

	/// Whether this scheme's payload is opaque, ie carried verbatim as one path
	/// segment, neither split on `/` nor percent-coded.
	///
	/// Every named scheme but [`None`](Self::None) that is not
	/// [hierarchical](Self::is_hierarchical): a `data:` payload, a `mailto:`
	/// address and a `javascript:` expression are content, not paths.
	pub fn is_opaque(&self) -> bool {
		!self.is_hierarchical() && *self != Self::None
	}
}

impl core::fmt::Display for Scheme {
	fn fmt(
		&self,
		formatter: &mut core::fmt::Formatter<'_>,
	) -> core::fmt::Result {
		write!(formatter, "{}", self.as_str())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	// -- Scheme tests --

	#[crate::test]
	fn scheme_parsing() {
		Scheme::from_str("http").xpect_eq(Scheme::Http);
		Scheme::from_str("HTTPS").xpect_eq(Scheme::Https);
		Scheme::from_str("file").xpect_eq(Scheme::File);
		Scheme::from_str("ws").xpect_eq(Scheme::Ws);
		Scheme::from_str("wss").xpect_eq(Scheme::Wss);
		Scheme::from_str("data").xpect_eq(Scheme::Data);
		Scheme::from_str("mailto").xpect_eq(Scheme::MailTo);
		Scheme::from_str("tel").xpect_eq(Scheme::Tel);
		Scheme::from_str("javascript").xpect_eq(Scheme::JavaScript);
		Scheme::from_str("blob").xpect_eq(Scheme::Blob);
		Scheme::from_str("cid").xpect_eq(Scheme::Cid);
		Scheme::from_str("about").xpect_eq(Scheme::About);
		Scheme::from_str("chrome").xpect_eq(Scheme::Chrome);
		Scheme::from_str("").xpect_eq(Scheme::None);
		Scheme::from_str("custom")
			.xpect_eq(Scheme::Other("custom".to_string()));
	}

	#[crate::test]
	fn scheme_classification() {
		Scheme::Http.is_http().xpect_true();
		Scheme::Https.is_http().xpect_true();
		Scheme::Ws.is_http().xpect_false();
		Scheme::Ws.is_ws().xpect_true();
		Scheme::Https.is_secure().xpect_true();
		Scheme::Http.is_secure().xpect_false();
		Scheme::Http.to_string().xpect_eq("http");
		Scheme::None.to_string().xpect_eq("");
		for scheme in [
			Scheme::Http,
			Scheme::Https,
			Scheme::File,
			Scheme::Ws,
			Scheme::Wss,
			Scheme::Chrome,
		] {
			scheme.is_hierarchical().xpect_true();
			scheme.is_opaque().xpect_false();
		}
		for scheme in [
			Scheme::Blob,
			Scheme::MailTo,
			Scheme::Tel,
			Scheme::Data,
			Scheme::About,
			Scheme::Cid,
			Scheme::JavaScript,
		] {
			scheme.is_hierarchical().xpect_false();
			scheme.is_opaque().xpect_true();
		}
		// a scheme-less reference is neither: it is a plain path
		Scheme::None.is_opaque().xpect_false();
	}

	// -- Url parsing tests --

	#[crate::test]
	fn parses_full_url() {
		let url = Url::coerce("https://example.com/api/users?limit=10#results");
		url.scheme().clone().xpect_eq(Scheme::Https);
		url.authority().unwrap().xpect_eq("example.com");
		url.path()
			.clone()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		url.is_rooted().xpect_true();
		url.is_external().xpect_true();
		url.get_param("limit").unwrap().xpect_eq("10");
		url.fragment().unwrap().xpect_eq("results");
	}

	#[crate::test]
	fn parses_the_shorthands() {
		// the `//` after the scheme is optional
		let url = Url::coerce("http:example.com/api/users");
		url.scheme().clone().xpect_eq(Scheme::Http);
		url.authority().unwrap().xpect_eq("example.com");
		url.path()
			.clone()
			.xpect_eq(vec!["api".to_string(), "users".to_string()]);
		// an authority with no path at all
		let url = Url::coerce("https://example.com");
		url.authority().unwrap().xpect_eq("example.com");
		url.path().xpect_empty();
		url.is_rooted().xpect_true();
		// a port rides the authority
		Url::coerce("http://localhost:8080/api")
			.authority()
			.unwrap()
			.xpect_eq("localhost:8080");
		// an empty fragment is no fragment
		Url::coerce("/path#").fragment().xpect_none();
		// `file:` has no host, so its path starts at the third slash
		let url = Url::coerce("file:///home/user/doc.txt");
		url.scheme().clone().xpect_eq(Scheme::File);
		url.authority().xpect_none();
		url.last_segment().unwrap().xpect_eq("doc.txt");
	}

	/// The distinction a type without a `root` field cannot carry: an authority
	/// implies a rooted path, a leading `/` declares one, and anything else is
	/// a reference resolved against the document that holds it.
	#[crate::test]
	fn tracks_rootedness() {
		Url::coerce("/blog/post-1").is_rooted().xpect_true();
		Url::coerce("blog/post-1").is_rooted().xpect_false();
		Url::coerce("https://beet.org/blog")
			.is_rooted()
			.xpect_true();
		// ..and it survives the round trip in both directions
		Url::coerce("blog/post-1")
			.to_string()
			.xpect_eq("blog/post-1");
		Url::coerce("/blog/post-1")
			.to_string()
			.xpect_eq("/blog/post-1");
		// a rooted empty path is the root itself; a relative one is nothing
		Url::default().to_string().xpect_eq("/");
		Url::default().with_rooted(false).to_string().xpect_eq("");
	}

	/// Components are held decoded and written encoded, so a segment carrying a
	/// space, a slash or a `&` survives the round trip intact.
	#[crate::test]
	fn round_trips_percent_encoding() {
		let url = Url::coerce("/assets/my%20card.png");
		url.last_segment().unwrap().xpect_eq("my card.png");
		url.to_string().xpect_eq("/assets/my%20card.png");
		// a raw space is repaired rather than carried
		Url::coerce("/assets/my card.png")
			.to_string()
			.xpect_eq("/assets/my%20card.png");
		// a `/` inside a segment can never split it back in two
		Url::default().push("a/b").to_string().xpect_eq("/a%2Fb");
		// query values decode `%XX` and the form-encoded `+`, and render back
		// sorted by key so the same params always render the same string
		let url = Url::coerce("/search?q=tom+%26+jerry&empty");
		url.get_param("q").unwrap().xpect_eq("tom & jerry");
		url.has_param("empty").xpect_true();
		url.to_string()
			.xpect_eq("/search?empty&q=tom%20%26%20jerry");
		// non-ascii is utf8 bytes, and decodes back
		let url = Url::coerce("/caf\u{e9}");
		url.to_string().xpect_eq("/caf%C3%A9");
		Url::coerce("/caf%C3%A9")
			.last_segment()
			.unwrap()
			.xpect_eq("caf\u{e9}");
		// a malformed escape stays literal, then encodes to a stable form
		Url::coerce("/a%zz").to_string().xpect_eq("/a%25zz");
	}

	/// The opaque schemes carry their payload verbatim: never split on `/`,
	/// never percent-coded, and `?`/`#` inside a data payload are content.
	#[crate::test]
	fn keeps_opaque_payloads_verbatim() {
		let url = Url::coerce("data:text/plain;base64,SGVsbG8=");
		url.scheme().clone().xpect_eq(Scheme::Data);
		url.path()
			.clone()
			.xpect_eq(vec!["text/plain;base64,SGVsbG8=".to_string()]);
		let raw = "data:text/html,<h1>Hello!</h1><p>not-query-param=no</p>";
		Url::coerce(raw).to_string().xpect_eq(raw);
		for raw in [
			"about:blank",
			"mailto:user@example.com",
			"tel:+1-555-0100",
			"javascript:void(0)",
			"cid:part1@example.com",
		] {
			Url::coerce(raw).to_string().xpect_eq(raw);
		}
		// a mailto still carries an ordinary query
		let url = Url::coerce("mailto:user@example.com?subject=Hello");
		url.get_param("subject").unwrap().xpect_eq("Hello");
		url.to_string()
			.xpect_eq("mailto:user@example.com?subject=Hello");
		// `blob:` keeps its origin in the opaque payload, not the authority
		let url = Url::coerce("blob:https://example.com/abc-123");
		url.authority().xpect_none();
		url.to_string().xpect_eq("blob:https://example.com/abc-123");
		// `chrome:` IS hierarchical, so its first segment is a host
		Url::coerce("chrome://settings/privacy")
			.authority()
			.unwrap()
			.xpect_eq("settings");
	}

	/// The three entry points, by what each promises.
	#[crate::test]
	fn gates_authored_urls() {
		// `parse` rejects only what no encoding repairs
		Url::parse("/blog").unwrap().to_string().xpect_eq("/blog");
		Url::parse("/my file.png")
			.unwrap()
			.to_string()
			.xpect_eq("/my%20file.png");
		Url::parse("")
			.unwrap_err()
			.to_string()
			.xpect_contains("empty string");
		Url::parse("/a\nb")
			.unwrap_err()
			.to_string()
			.xpect_contains("control character");
		// `parse_absolute` additionally demands a scheme and a host
		Url::parse_absolute("https://beet.org/blog")
			.unwrap()
			.is_external()
			.xpect_true();
		for relative in ["/blog", "blog", "beet.org/blog"] {
			Url::parse_absolute(relative)
				.unwrap_err()
				.to_string()
				.xpect_contains("expected a scheme and a host");
		}
	}

	#[crate::test]
	fn file_extension_classification() {
		let extension =
			|url: &str| Url::coerce(url).file_extension().map(str::to_string);
		// served files carry a real extension
		extension("/assets/blog/x.jpg").unwrap().xpect_eq("jpg");
		extension("/style.css").unwrap().xpect_eq("css");
		extension("/index.html").unwrap().xpect_eq("html");
		// page routes do not
		extension("/about").xpect_none();
		extension("/blog/post-6").xpect_none();
		extension("/").xpect_none();
		// a leading-dot segment is not an extension (empty stem)
		extension("/.gitignore").xpect_none();
	}

	/// Reference resolution: another origin wins outright, a rooted path
	/// replaces, and a relative one merges beside the base's last segment.
	#[crate::test]
	fn resolves_references() {
		let base = Url::coerce("https://beet.org/docs/intro?stale=1");
		let join = |href: &str| base.join(Url::coerce(href)).to_string();
		join("next").xpect_eq("https://beet.org/docs/next");
		join("/blog").xpect_eq("https://beet.org/blog");
		join("../blog/x").xpect_eq("https://beet.org/blog/x");
		join("./sibling").xpect_eq("https://beet.org/docs/sibling");
		join("https://other.example/x").xpect_eq("https://other.example/x");
		// the base's query does not leak onto the resolved reference
		join("next?fresh=1").xpect_eq("https://beet.org/docs/next?fresh=1");
	}

	#[crate::test]
	fn forward_keeps_query_swaps_destination() {
		// caller's params survive, destination scheme/authority/path are replaced.
		Url::coerce("/run/led?bright=on#top")
			.forward(&Url::coerce("https://device.local:8080/led"))
			.to_string()
			.xpect_eq("https://device.local:8080/led?bright=on#top");
	}

	#[crate::test]
	fn builder_chaining() {
		Url::default()
			.with_scheme(Scheme::Https)
			.with_authority("example.com")
			.with_path(vec!["api".into()])
			.with_param("key", "val")
			.with_fragment("top")
			.to_string()
			.xpect_eq("https://example.com/api?key=val#top");
		Url::default()
			.with_flag("verbose")
			.has_param("verbose")
			.xpect_true();
		let url: Url = "https://example.com/path".into();
		url.scheme().clone().xpect_eq(Scheme::Https);
	}

	#[crate::test]
	fn splits_paths_and_queries() {
		Url::split_path("").xpect_empty();
		Url::split_path("/").xpect_empty();
		Url::split_path("//").xpect_empty();
		Url::split_path("/a//b/")
			.xpect_eq(vec!["a".to_string(), "b".to_string()]);
		Url::coerce("/api/users/123")
			.path_string()
			.xpect_eq("/api/users/123");
		Url::default().path_string().xpect_eq("/");
		let url = Url::coerce("/api/users/123");
		url.first_segment().unwrap().xpect_eq("api");
		url.last_segment().unwrap().xpect_eq("123");
		url.path_from(1).xpect_eq(["users", "123"]);
		url.path_from(10).len().xpect_eq(0);
	}
}
