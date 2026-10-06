use super::string_primitive;
use crate::prelude::*;

/// An account's permanent identity, `did:plc:…` or `did:web:…`.
///
/// A handle is a name pointed at one and may move; a did never does, so every
/// address beet writes names the did. The two methods the protocol supports
/// are the only two parsed: `did:plc` with its 24 character base32 identifier,
/// `did:web` with a hostname. Resolving one to its document, and so to the PDS
/// hosting its repo, is `beet_net`'s `DidResolver`.
///
/// ```
/// # use beet_core::prelude::*;
/// let did = Did::parse("did:plc:vgxy56shhs3t3b2mu2avizjf").unwrap();
/// did.method().xpect_eq(DidMethod::Plc);
/// did.identifier().xpect_eq("vgxy56shhs3t3b2mu2avizjf");
/// Did::parse("beet.org").unwrap_err();
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Did(SmolStr);

string_primitive!(Did);

/// The did methods the protocol supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DidMethod {
	/// `did:plc`, the directory-registered method nearly every account uses.
	Plc,
	/// `did:web`, a did document served from a hostname.
	Web,
}

impl Did {
	/// The scheme every did carries.
	pub const PREFIX: &'static str = "did:";

	/// Parse a did, refusing any method but `plc` and `web` and an identifier
	/// that method could not have issued, since a typo here is an account
	/// that silently does not exist.
	pub fn parse(text: &str) -> Result<Self> {
		let Some(rest) = text.strip_prefix(Self::PREFIX) else {
			bevybail!(
				"`{text}` is not a did: an account is named like \
				 `did:plc:…`, not by a handle or a url"
			);
		};
		match rest.split_once(':') {
			Some(("plc", identifier))
				if identifier.len() == 24
					&& identifier.bytes().all(
						|byte| matches!(byte, b'a'..=b'z' | b'2'..=b'7'),
					) =>
			{
				Self(SmolStr::new(text)).xok()
			}
			Some(("plc", _)) => bevybail!(
				"`{text}` is not a did:plc: the identifier is 24 lowercase \
				 base32 characters"
			),
			Some(("web", host))
				if !host.is_empty()
					&& host.bytes().all(|byte| {
						byte.is_ascii_alphanumeric()
							|| matches!(byte, b'.' | b'-' | b'%')
					}) =>
			{
				Self(SmolStr::new(text)).xok()
			}
			Some(("web", _)) => bevybail!(
				"`{text}` is not a did:web: the identifier is a hostname, \
				 a port written `%3A`"
			),
			_ => bevybail!(
				"`{text}` is not a supported did: the protocol supports \
				 `did:plc` and `did:web`"
			),
		}
	}

	/// The method this did was issued by.
	pub fn method(&self) -> DidMethod {
		match self.0.starts_with("did:web:") {
			true => DidMethod::Web,
			false => DidMethod::Plc,
		}
	}

	/// The method-specific identifier, after `did:<method>:`.
	pub fn identifier(&self) -> &str {
		self.0.splitn(3, ':').nth(2).unwrap_or_default()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn parses_both_methods() {
		Did::parse("did:web:example.com")
			.unwrap()
			.method()
			.xpect_eq(DidMethod::Web);
		Did::parse("did:web:localhost%3A8080")
			.unwrap()
			.identifier()
			.xpect_eq("localhost%3A8080");
	}

	#[crate::test]
	fn rejects_malformed() {
		Did::parse("did:plc:alice")
			.unwrap_err()
			.to_string()
			.xpect_contains("24 lowercase");
		Did::parse("did:plc:REPLACE-ME-WITH-A-REAL-DID").unwrap_err();
		Did::parse("did:key:z6Mk")
			.unwrap_err()
			.to_string()
			.xpect_contains("supported");
		Did::parse("did:web:").unwrap_err();
		Did::parse("alice.bsky.social")
			.unwrap_err()
			.to_string()
			.xpect_contains("not a did");
	}
}
