use super::string_primitive;
use crate::prelude::*;

/// A record key: 1 to 512 characters of `A-Za-z0-9._:~-`, never `.` or `..`,
/// so never a slash, and a path segment wherever a repo is laid out as files.
///
/// ```
/// # use beet_core::prelude::*;
/// Rkey::parse("3mw72aaeuj22n").unwrap();
/// Rkey::parse("self").unwrap();
/// Rkey::parse("a/b").unwrap_err();
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Rkey(SmolStr);

string_primitive!(Rkey);

impl Rkey {
	/// The longest rkey the protocol allows.
	pub const MAX_LEN: usize = 512;

	/// The key of a one-per-repo record, ie a profile.
	pub const SELF: Self = Self(SmolStr::new_static("self"));

	/// Parse an rkey, refusing any character a repo path could not hold.
	pub fn parse(text: &str) -> Result<Self> {
		if !(1..=Self::MAX_LEN).contains(&text.len()) {
			bevybail!(
				"rkey `{text}` must be 1 to {} characters",
				Self::MAX_LEN
			);
		}
		if text == "." || text == ".." {
			bevybail!("rkey `{text}` is a relative path segment");
		}
		if let Some(char) = text.chars().find(|char| {
			!(char.is_ascii_alphanumeric()
				|| matches!(char, '.' | '_' | ':' | '~' | '-'))
		}) {
			bevybail!(
				"rkey `{text}` contains `{char}`: only `A-Za-z0-9._:~-` are allowed"
			);
		}
		Self(SmolStr::new(text)).xok()
	}
}

impl From<Tid> for Rkey {
	fn from(tid: Tid) -> Self { Self(SmolStr::new(tid.to_string())) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn rejects_malformed() {
		Rkey::parse("").unwrap_err();
		Rkey::parse(".").unwrap_err();
		Rkey::parse("..").unwrap_err();
		Rkey::parse("a b")
			.unwrap_err()
			.to_string()
			.xpect_contains("` `");
		Rkey::parse(&"a".repeat(513)).unwrap_err();
		Rkey::parse("nsid:app.bsky.feed.post~1").unwrap();
	}
}
