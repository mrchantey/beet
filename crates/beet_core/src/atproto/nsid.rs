use super::string_primitive;
use crate::prelude::*;

/// A lexicon's name: a reversed domain authority then a name segment of
/// letters and digits starting with a letter, ie `site.standard.document`.
/// With a `#` fragment it names one def of the lexicon,
/// `org.beet.core#provenance`; without one, its `main`.
///
/// The collection of a record type and the key of a component in a record, so
/// it is const constructible through [`Nsid::new_static`].
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
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Nsid(SmolStr);

string_primitive!(Nsid);

impl Nsid {
	/// The longest nsid the protocol allows, fragment excluded.
	pub const MAX_LEN: usize = 317;

	/// A const nsid from a literal, unchecked: for a `COLLECTION` or a def's
	/// key, each covered by a test that parses it.
	pub const fn new_static(nsid: &'static str) -> Self {
		Self(SmolStr::new_static(nsid))
	}

	/// Parse an nsid with an optional `#def` fragment.
	pub fn parse(text: &str) -> Result<Self> {
		let (nsid, fragment) = match text.split_once('#') {
			Some((nsid, fragment)) => (nsid, Some(fragment)),
			None => (text, None),
		};
		if nsid.len() > Self::MAX_LEN {
			bevybail!(
				"nsid `{text}` is longer than {} characters",
				Self::MAX_LEN
			);
		}
		let segments = nsid.split('.').collect::<Vec<_>>();
		let Some((name, authority)) = segments.split_last() else {
			bevybail!("nsid `{text}` is empty");
		};
		if authority.len() < 2 {
			bevybail!(
				"nsid `{text}` needs a reversed domain and a name, ie \
				 `com.example.thing`"
			);
		}
		for (i, segment) in authority.iter().enumerate() {
			let valid = (1..=63).contains(&segment.len())
				&& segment
					.bytes()
					.all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
				&& !segment.starts_with('-')
				&& !segment.ends_with('-')
				&& !(i == 0
					&& segment.starts_with(|c: char| c.is_ascii_digit()));
			if !valid {
				bevybail!(
					"nsid `{text}` has an invalid domain segment `{segment}`"
				);
			}
		}
		for segment in core::iter::once(*name).chain(fragment) {
			if !Self::is_name(segment) {
				bevybail!(
					"nsid `{text}` has an invalid name `{segment}`: letters \
					 and digits starting with a letter"
				);
			}
		}
		Self(SmolStr::new(text)).xok()
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

	fn is_name(segment: &str) -> bool {
		(1..=63).contains(&segment.len())
			&& segment.starts_with(|c: char| c.is_ascii_alphabetic())
			&& segment.bytes().all(|byte| byte.is_ascii_alphanumeric())
	}
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
		Nsid::parse("com.example").unwrap_err();
		Nsid::parse("com.example.foo-bar").unwrap_err();
		Nsid::parse("com.example.3foo").unwrap_err();
		Nsid::parse("1com.example.foo").unwrap_err();
		Nsid::parse("com.-example.foo").unwrap_err();
		Nsid::parse("com..foo").unwrap_err();
		Nsid::parse("com.example.foo#").unwrap_err();
		Nsid::parse("com.example.foo#bar-baz").unwrap_err();
	}
}
