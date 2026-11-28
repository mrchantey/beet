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
pub struct Rkey(SmolStr);

impl Rkey {
	/// The longest rkey the protocol allows.
	pub const MAX_LEN: usize = 512;

	/// The key of a one-per-repo record, ie a profile.
	pub const SELF: Self = rkey!("self");

	/// Parse an rkey, refusing any character a repo path could not hold.
	///
	/// # Errors
	/// Errors as [`check`](Self::check) refuses.
	pub fn parse(text: &str) -> Result<Self> {
		match Self::check(text) {
			Ok(()) => Self(SmolStr::new(text)).xok(),
			Err(reason) => bevybail!("rkey `{text}` {reason}"),
		}
	}

	/// The rule an rkey keeps, the one [`parse`](Self::parse) and [`rkey!`]
	/// check, else which part `text` breaks.
	pub const fn check(text: &str) -> Result<(), &'static str> {
		let bytes = text.as_bytes();
		if bytes.is_empty() || bytes.len() > Self::MAX_LEN {
			return Err("must be 1 to 512 characters");
		}
		if matches!(bytes, [b'.'] | [b'.', b'.']) {
			return Err("is a relative path segment");
		}
		let mut index = 0;
		while index < bytes.len() {
			let byte = bytes[index];
			if !(byte.is_ascii_alphanumeric()
				|| matches!(byte, b'.' | b'_' | b':' | b'~' | b'-'))
			{
				return Err("holds a character outside `A-Za-z0-9._:~-`");
			}
			index += 1;
		}
		Ok(())
	}

	/// An rkey from text [`check`](Self::check) passed, see [`rkey!`].
	#[doc(hidden)]
	pub const fn from_canonical(text: &'static str) -> Self {
		Self(SmolStr::new_static(text))
	}
}

/// An [`Rkey`] known at compile time, its text a literal or a `&'static str`
/// constant checked as it compiles.
///
/// ```
/// # use beet_core::prelude::*;
/// const PROFILE: Rkey = rkey!("self");
/// PROFILE.as_str().xpect_eq("self");
/// ```
///
/// ```compile_fail
/// # use beet_core::prelude::*;
/// rkey!("a/b");
/// ```
#[macro_export]
macro_rules! rkey {
	($text:expr) => {{
		const _: () = if let ::core::result::Result::Err(reason) =
			$crate::prelude::Rkey::check($text)
		{
			::core::panic!("{}", reason)
		};
		$crate::prelude::Rkey::from_canonical($text)
	}};
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
			.xpect_contains("outside `A-Za-z0-9._:~-`");
		Rkey::parse(&"a".repeat(513)).unwrap_err();
		Rkey::parse("nsid:app.bsky.feed.post~1").unwrap();
	}

}
