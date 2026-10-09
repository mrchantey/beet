//! [`Uri`], an absolute uri held exactly as written.
//!
//! Beside [`Url`], and the one rule between them: **a field you store or
//! compare is a `Uri`, a request you send is a `Url`.** A `Url` is the parsed
//! form a request is routed and sent by, lowercasing its scheme, re-encoding
//! its query and reading every authority as a host and port, so a value read
//! through one and written back can differ from what was read. A record field,
//! a canonical address or anything hashed must read back byte for byte, which
//! a `Uri` does; [`to_url`](Uri::to_url) is the parsed view for the moment a
//! request is sent.
use crate::atproto::string_primitive;
use crate::prelude::*;

/// Any absolute uri, a publication's `https://beet.org/blog` or a record's
/// `at://did:plc:../<collection>/<rkey>`, held exactly as written: the
/// lexicon `uri` string format. See the [module docs](self) for when to reach
/// for it over a [`Url`].
///
/// [`to_url`](Self::to_url) is the parsed view a request sends, and the
/// atproto view is [`at_uri`](Self::at_uri).
///
/// ```
/// # use beet_core::prelude::*;
/// let site = Uri::parse(
/// 	"at://did:plc:vgxy56shhs3t3b2mu2avizjf/site.standard.publication/3mw72aaeuj22n",
/// )
/// .unwrap();
/// site.scheme().xpect_eq("at");
/// site.at_uri().unwrap().rkey().as_str().xpect_eq("3mw72aaeuj22n");
/// let blog = Uri::parse("https://beet.org/blog").unwrap();
/// blog.at_uri().xpect_none();
/// blog.to_url().unwrap().path().xpect_eq(&["blog"]);
/// Uri::parse("beet.org").unwrap_err();
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Uri(pub(crate) SmolStr);

string_primitive!(Uri);

impl Uri {
	/// The longest uri the atproto lexicons allow, and the longest this holds.
	pub const MAX_LEN: usize = 8192;

	/// Parse an absolute uri: a scheme of a letter then letters, digits, `+`,
	/// `-` or `.`, a `:`, and a non-empty remainder with no whitespace.
	pub fn parse(text: &str) -> Result<Self> {
		if text.len() > Self::MAX_LEN {
			bevybail!("uri is longer than {} bytes", Self::MAX_LEN);
		}
		let Some((scheme, rest)) = text.split_once(':') else {
			bevybail!("`{text}` is not a uri: it names no scheme, ie `https:`");
		};
		let scheme_valid = scheme
			.starts_with(|c: char| c.is_ascii_alphabetic())
			&& scheme.chars().all(|c| {
				c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')
			});
		if !scheme_valid || rest.is_empty() {
			bevybail!("`{text}` is not a uri: expected `<scheme>:<rest>`");
		}
		if text.chars().any(char::is_whitespace) {
			bevybail!("uri `{text}` contains whitespace");
		}
		Self(SmolStr::new(text)).xok()
	}

	/// The scheme as written, ie `https`, `at`.
	pub fn scheme(&self) -> &str {
		self.0.split(':').next().unwrap_or_default()
	}

	/// The parsed url a request sends.
	pub fn to_url(&self) -> Result<Url> { Url::parse(&self.0) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A uri reads back exactly as written, where a `Url` would normalize.
	#[crate::test]
	fn keeps_its_text() {
		for text in [
			"HTTPS://Beet.org/Blog?b=1&a=2",
			"https://beet.org",
			"mailto:a@b.c",
		] {
			Uri::parse(text).unwrap().as_str().xpect_eq(text);
		}
	}

	#[crate::test]
	fn rejects_malformed() {
		Uri::parse("/relative").unwrap_err();
		Uri::parse("1http://x").unwrap_err();
		Uri::parse("https:").unwrap_err();
		Uri::parse("https://beet .org").unwrap_err();
	}
}
