use crate::prelude::*;

/// A record's address, `at://<did>/<collection>/<rkey>` in an account's public
/// repo, or `at://<did>/space/<type>/<key>/<author>/<collection>/<rkey>`
/// within one member's repo of a space, the literal `space` segment telling
/// the two apart ([`RepoScope`]).
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
/// uri.scope().xpect_eq(RepoScope::Public);
/// ```
#[derive(
	Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct AtUri {
	did: Did,
	scope: RepoScope,
	collection: Nsid,
	rkey: Rkey,
}

impl AtUri {
	/// The scheme every at uri carries.
	pub const SCHEME: &'static str = "at://";

	/// The address of `rkey` in `collection` of `did`'s public repo.
	pub fn new(did: Did, collection: Nsid, rkey: Rkey) -> Self {
		Self::scoped(did, RepoScope::Public, collection, rkey)
	}

	/// The address of `rkey` in `collection` of `did`'s repo `scope`.
	pub fn scoped(
		did: Did,
		scope: RepoScope,
		collection: Nsid,
		rkey: Rkey,
	) -> Self {
		Self {
			did,
			scope,
			collection,
			rkey,
		}
	}

	/// Strict parse of the record form, public or within a space.
	pub fn parse(uri: &str) -> Result<Self> {
		let Some(rest) = uri.strip_prefix(Self::SCHEME) else {
			bevybail!("`{uri}` is not an at uri: expected the `at://` scheme");
		};
		let parts = rest.split('/').collect::<Vec<_>>();
		match parts.as_slice() {
			[did, collection, rkey] => Self::new(
				Did::parse(did)?,
				Nsid::parse(collection)?,
				Rkey::parse(rkey)?,
			)
			.xok(),
			[
				did,
				RepoScope::SPACE_SEGMENT,
				r#type,
				key,
				author,
				collection,
				rkey,
			] => Self::scoped(
				Did::parse(did)?,
				RepoScope::space(
					SpaceRef::new(Nsid::parse(r#type)?, *key)?,
					Did::parse(author)?,
				),
				Nsid::parse(collection)?,
				Rkey::parse(rkey)?,
			)
			.xok(),
			_ => bevybail!(
				"`{uri}` is not a record address: expected \
				 `at://<did>/<collection>/<rkey>` or \
				 `at://<did>/space/<type>/<key>/<author>/<collection>/<rkey>`"
			),
		}
	}

	/// The account holding the record.
	pub fn did(&self) -> &Did { &self.did }

	/// The repo of the account holding the record.
	pub fn scope(&self) -> &RepoScope { &self.scope }

	/// The record's collection.
	pub fn collection(&self) -> &Nsid { &self.collection }

	/// The record's key within its collection.
	pub fn rkey(&self) -> &Rkey { &self.rkey }
}

impl core::fmt::Display for AtUri {
	fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(formatter, "{}{}/", Self::SCHEME, self.did)?;
		if let RepoScope::Space { space, author } = &self.scope {
			write!(
				formatter,
				"{}/{space}/{author}/",
				RepoScope::SPACE_SEGMENT
			)?;
		}
		write!(formatter, "{}/{}", self.collection, self.rkey)
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

/// The atproto view of a [`Uri`]: a lexicon `uri` field names a record or a
/// web page, and this reads which.
impl Uri {
	/// The record this uri addresses, when it is an `at://` record address.
	pub fn at_uri(&self) -> Option<AtUri> {
		self.as_str()
			.starts_with(AtUri::SCHEME)
			.then(|| AtUri::parse(self.as_str()).ok())
			.flatten()
	}
}

impl From<AtUri> for Uri {
	fn from(uri: AtUri) -> Self { Self(uri.into()) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	const URI: &str = "at://did:plc:vgxy56shhs3t3b2mu2avizjf/site.standard.document/3mw72aaeuj22n";

	#[crate::test]
	fn round_trips() { AtUri::parse(URI).unwrap().to_string().xpect_eq(URI); }

	#[crate::test]
	fn round_trips_the_space_form() {
		let uri = "at://did:plc:vgxy56shhs3t3b2mu2avizjf/space/org.beet.space.package/todos/did:plc:vgxy56shhs3t3b2mu2avizjf/org.beet.branch/todos:main";
		let parsed = AtUri::parse(uri).unwrap();
		parsed
			.scope()
			.space_ref()
			.unwrap()
			.xpect_eq(SpaceRef::package("todos").unwrap());
		parsed.collection().as_str().xpect_eq("org.beet.branch");
		parsed.to_string().xpect_eq(uri);
		AtUri::parse(
			"at://did:plc:vgxy56shhs3t3b2mu2avizjf/space/org.beet.space.package/todos/org.beet.branch/todos:main",
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("record address");
	}

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
