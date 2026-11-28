use crate::prelude::*;

/// A lexicon's name: a reversed domain authority then a name segment of
/// letters and digits starting with a letter, ie `site.standard.document`.
/// With a `#` fragment it names one def of the lexicon,
/// `org.beet.core#provenance`; without one, its `main`.
///
/// The collection of a record type and the key of a component in a record, so
/// it is const constructible through [`nsid!`].
///
/// ```
/// # use beet_core::prelude::*;
/// let nsid = Nsid::parse("org.beet.core#provenance").unwrap();
/// nsid.authority().xpect_eq("org.beet");
/// nsid.name().xpect_eq("core");
/// nsid.fragment().xpect_eq(Some("provenance"));
/// Nsid::parse("beet.org").unwrap_err();
/// ```
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Reflect,
	StringPrimitive,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Nsid(SmolStr);

impl Nsid {
	/// The longest nsid the protocol allows, fragment excluded.
	pub const MAX_LEN: usize = 317;

	/// Parse an nsid with an optional `#def` fragment.
	///
	/// # Errors
	/// Errors as [`check`](Self::check) refuses.
	pub fn parse(text: &str) -> Result<Self> {
		match Self::check(text) {
			Ok(()) => Self(SmolStr::new(text)).xok(),
			Err(reason) => bevybail!("nsid `{text}` {reason}"),
		}
	}

	/// The rule an nsid keeps, the one [`parse`](Self::parse) and [`nsid!`]
	/// check, else which part `text` breaks: a reversed domain of at least
	/// two segments, a name, then an optional `#` fragment naming a def.
	pub const fn check(text: &str) -> Result<(), &'static str> {
		let bytes = text.as_bytes();
		let mut hash = bytes.len();
		let mut index = 0;
		while index < bytes.len() {
			if bytes[index] == b'#' {
				hash = index;
				break;
			}
			index += 1;
		}
		if hash > Self::MAX_LEN {
			return Err("is longer than the 317 characters an nsid may hold");
		}
		// each `.` ends a domain segment, the end of the nsid its name
		let mut start = 0;
		let mut domains = 0;
		let mut index = 0;
		while index <= hash {
			if index == hash {
				if !Self::is_name(bytes, start, hash) {
					return Err("has an invalid name: 1 to 63 letters and \
						digits starting with a letter");
				}
			} else if bytes[index] == b'.' {
				if !Self::is_domain(bytes, start, index, domains == 0) {
					return Err("has an invalid domain segment: 1 to 63 \
						letters, digits or `-`, neither starting nor ending \
						with `-`, the first not starting with a digit");
				}
				domains += 1;
				start = index + 1;
			}
			index += 1;
		}
		if domains < 2 {
			return Err(
				"needs a reversed domain and a name, ie `com.example.thing`",
			);
		}
		if hash < bytes.len() && !Self::is_name(bytes, hash + 1, bytes.len())
		{
			return Err("has an invalid fragment: 1 to 63 letters and digits \
				starting with a letter");
		}
		Ok(())
	}

	/// An nsid from text [`check`](Self::check) passed, see [`nsid!`].
	#[doc(hidden)]
	pub const fn from_canonical(text: &'static str) -> Self {
		Self(SmolStr::new_static(text))
	}

	/// The reversed domain authority, ie `site.standard` of
	/// `site.standard.document`.
	pub fn authority(&self) -> &str {
		self.without_fragment()
			.rsplit_once('.')
			.map(|(authority, _)| authority)
			.unwrap_or_default()
	}

	/// The name segment, ie `document` of `site.standard.document`.
	pub fn name(&self) -> &str {
		self.without_fragment()
			.rsplit_once('.')
			.map(|(_, name)| name)
			.unwrap_or_default()
	}

	/// The def a fragment names, ie `provenance` of `org.beet.core#provenance`.
	pub fn fragment(&self) -> Option<&str> {
		self.0.split_once('#').map(|(_, fragment)| fragment)
	}

	fn without_fragment(&self) -> &str {
		self.0.split('#').next().unwrap_or_default()
	}

	/// Whether `bytes[start..end]` is a name: 1 to 63 letters and digits
	/// starting with a letter.
	const fn is_name(bytes: &[u8], start: usize, end: usize) -> bool {
		if end <= start || end - start > 63 || !bytes[start].is_ascii_alphabetic()
		{
			return false;
		}
		let mut index = start;
		while index < end {
			if !bytes[index].is_ascii_alphanumeric() {
				return false;
			}
			index += 1;
		}
		true
	}

	/// Whether `bytes[start..end]` is a domain segment: 1 to 63 letters,
	/// digits or `-`, neither starting nor ending with `-`, and the `first`
	/// not starting with a digit.
	const fn is_domain(
		bytes: &[u8],
		start: usize,
		end: usize,
		first: bool,
	) -> bool {
		if end <= start
			|| end - start > 63
			|| bytes[start] == b'-'
			|| bytes[end - 1] == b'-'
			|| (first && bytes[start].is_ascii_digit())
		{
			return false;
		}
		let mut index = start;
		while index < end {
			if !(bytes[index].is_ascii_alphanumeric() || bytes[index] == b'-') {
				return false;
			}
			index += 1;
		}
		true
	}
}

/// An [`Nsid`] known at compile time, its text a literal or a `&'static str`
/// constant checked as it compiles.
///
/// ```
/// # use beet_core::prelude::*;
/// const POST: Nsid = nsid!("app.bsky.feed.post");
/// POST.name().xpect_eq("post");
/// ```
///
/// ```compile_fail
/// # use beet_core::prelude::*;
/// nsid!("beet.org");
/// ```
#[macro_export]
macro_rules! nsid {
	($text:expr) => {{
		const _: () = if let ::core::result::Result::Err(reason) =
			$crate::prelude::Nsid::check($text)
		{
			::core::panic!("{}", reason)
		};
		$crate::prelude::Nsid::from_canonical($text)
	}};
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn parses() {
		Nsid::parse("app.bsky.feed.post")
			.unwrap()
			.authority()
			.xpect_eq("app.bsky.feed");
		Nsid::parse("com.example.fooBar2").unwrap();
		Nsid::parse("a-0.b-1.c").unwrap();
	}

	#[crate::test]
	fn rejects_malformed() {
		Nsid::parse("com.example")
			.unwrap_err()
			.to_string()
			.xpect_contains("reversed domain");
		Nsid::parse(&format!("{}.example.foo", "a".repeat(320)))
			.unwrap_err()
			.to_string()
			.xpect_contains("longer than");
		Nsid::parse("com.example.foo-bar").unwrap_err();
		Nsid::parse("com.example.3foo").unwrap_err();
		Nsid::parse("1com.example.foo").unwrap_err();
		Nsid::parse("com.-example.foo").unwrap_err();
		Nsid::parse("com..foo").unwrap_err();
		Nsid::parse("com.example.foo#").unwrap_err();
		Nsid::parse("com.example.foo#bar-baz").unwrap_err();
	}
}
