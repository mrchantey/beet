use crate::prelude::*;

/// A record's address, `at://<did>/<collection>/<rkey>`.
///
/// Always the record form and always a did authority: a handle may move to
/// another account, so an address beet stores or compares names the identity
/// that cannot.
///
/// ```
/// # use beet_core::prelude::*;
/// let uri = AtUri::parse(
/// 	"at://did:plc:hnv7bd4gtxrf7iigjo22qukp/app.bsky.feed.post/3mw72aaeuj22n",
/// )
/// .unwrap();
/// uri.collection().as_str().xpect_eq("app.bsky.feed.post");
/// uri.rkey().as_str().xpect_eq("3mw72aaeuj22n");
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct AtUri {
	did: Did,
	collection: Nsid,
	rkey: Rkey,
}

impl AtUri {
	/// The scheme every at uri carries.
	pub const SCHEME: &'static str = "at://";

	/// The address of `rkey` in `collection` of `did`'s repo.
	pub fn new(did: Did, collection: Nsid, rkey: Rkey) -> Self {
		Self {
			did,
			collection,
			rkey,
		}
	}

	/// Strict parse of the `at://<did>/<collection>/<rkey>` record form.
	pub fn parse(uri: &str) -> Result<Self> {
		let Some(rest) = uri.strip_prefix(Self::SCHEME) else {
			bevybail!("`{uri}` is not an at uri: expected the `at://` scheme");
		};
		let mut parts = rest.split('/');
		match (parts.next(), parts.next(), parts.next(), parts.next()) {
			(Some(did), Some(collection), Some(rkey), None) => Self::new(
				Did::parse(did)?,
				Nsid::parse(collection)?,
				Rkey::parse(rkey)?,
			)
			.xok(),
			_ => bevybail!(
				"`{uri}` is not a record address: expected \
				 `at://<did>/<collection>/<rkey>`"
			),
		}
	}

	/// The repo holding the record.
	pub fn did(&self) -> &Did { &self.did }

	/// The record's collection.
	pub fn collection(&self) -> &Nsid { &self.collection }

	/// The record's key within its collection.
	pub fn rkey(&self) -> &Rkey { &self.rkey }
}

impl core::fmt::Display for AtUri {
	fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(
			formatter,
			"{}{}/{}/{}",
			Self::SCHEME,
			self.did,
			self.collection,
			self.rkey
		)
	}
}

impl core::str::FromStr for AtUri {
	type Err = BevyError;
	fn from_str(uri: &str) -> Result<Self> { Self::parse(uri) }
}

impl TryFrom<SmolStr> for AtUri {
	type Error = BevyError;
	fn try_from(uri: SmolStr) -> Result<Self> { Self::parse(&uri) }
}

impl From<AtUri> for SmolStr {
	fn from(uri: AtUri) -> SmolStr { SmolStr::new(uri.to_string()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	const URI: &str = "at://did:plc:vgxy56shhs3t3b2mu2avizjf/site.standard.document/3mw72aaeuj22n";

	#[crate::test]
	fn round_trips() { AtUri::parse(URI).unwrap().to_string().xpect_eq(URI); }

	#[crate::test]
	fn rejects_malformed() {
		AtUri::parse("did:plc:vgxy56shhs3t3b2mu2avizjf/a.b.c/rkey")
			.unwrap_err()
			.to_string()
			.xpect_contains("scheme");
		AtUri::parse("at://did:plc:vgxy56shhs3t3b2mu2avizjf/a.b.c")
			.unwrap_err()
			.to_string()
			.xpect_contains("record address");
		AtUri::parse("at://did:plc:vgxy56shhs3t3b2mu2avizjf/a.b.c/rkey/extra")
			.unwrap_err();
		AtUri::parse("at://beet.org/a.b.c/rkey")
			.unwrap_err()
			.to_string()
			.xpect_contains("not a did");
	}
}
