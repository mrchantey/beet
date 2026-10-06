//! Resolving the two names an account has: a did to the document saying where
//! its repo lives, and a handle to the did it points at right now.
use crate::client::provider_send;
use crate::prelude::*;
use beet_core::prelude::*;

/// Resolves a [`Did`] to its [`DidDocument`]: a `did:plc` through the PLC
/// directory, a `did:web` from its host's `/.well-known/did.json`.
///
/// The did is permanent and the document is where it says what moves: the
/// PDS hosting the repo and the handle it claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DidResolver {
	/// The PLC directory a `did:plc` resolves through.
	pub plc_directory: SmolStr,
}

impl Default for DidResolver {
	fn default() -> Self {
		Self {
			plc_directory: SmolStr::new_static(Self::PLC_DIRECTORY),
		}
	}
}

impl DidResolver {
	/// The public PLC directory.
	pub const PLC_DIRECTORY: &'static str = "https://plc.directory";

	/// The document `did` resolves to.
	pub async fn resolve(&self, did: &Did) -> Result<DidDocument> {
		let url = match did.method() {
			DidMethod::Plc => format!("{}/{did}", self.plc_directory),
			DidMethod::Web => format!(
				"https://{}/.well-known/did.json",
				did.identifier().replace("%3A", ":")
			),
		};
		let document = provider_send::send(Request::get(url))
			.await?
			.into_result()
			.await
			.map_err(|err| bevyhow!("resolving {did}: {err}"))?
			.json::<DidDocument>()
			.await?;
		if document.id != *did {
			bevybail!(
				"resolving {did} answered the document of {}",
				document.id
			);
		}
		document.xok()
	}
}

/// The parts of a did document atproto reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DidDocument {
	/// The did this document describes.
	pub id: Did,
	/// The names the account claims, ie `at://pete.beet.org`.
	#[serde(default, rename = "alsoKnownAs")]
	pub also_known_as: Vec<SmolStr>,
	/// The services the account names, its PDS among them.
	#[serde(default)]
	pub service: Vec<DidService>,
}

/// One service a did document names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DidService {
	/// The service id, `#atproto_pds` for the PDS.
	pub id: SmolStr,
	/// The service type, `AtprotoPersonalDataServer` for the PDS.
	#[serde(rename = "type")]
	pub r#type: SmolStr,
	/// Where the service answers.
	#[serde(rename = "serviceEndpoint")]
	pub service_endpoint: SmolStr,
}

impl DidDocument {
	/// The PDS hosting this account's repo, ie
	/// `https://chalciporus.us-west.host.bsky.network`.
	pub fn pds_endpoint(&self) -> Result<SmolStr> {
		self.service
			.iter()
			.find(|service| {
				service.id.ends_with("#atproto_pds")
					&& service.r#type == "AtprotoPersonalDataServer"
			})
			.map(|service| {
				SmolStr::new(service.service_endpoint.trim_end_matches('/'))
			})
			.ok_or_else(|| {
				bevyhow!("the did document of {} names no PDS", self.id)
			})
	}

	/// The handle this account claims, ie `pete.beet.org`.
	pub fn handle(&self) -> Option<&str> {
		self.also_known_as
			.iter()
			.find_map(|name| name.strip_prefix("at://"))
	}
}

/// Resolves a handle to the did it points at right now.
///
/// The one direction that has to be re-asked: a did is permanent and a handle
/// is a name pointed at one, so a handle may move to another account, or stop
/// resolving entirely when its dns record goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleResolver {
	/// The walk every resolver makes, with no service between: the
	/// `_atproto.<handle>` TXT record over DNS-over-HTTPS at `doh`, else the
	/// handle's `https://<handle>/.well-known/atproto-did`. Works wherever an
	/// http request does, a browser included.
	Direct {
		/// A DNS-over-HTTPS endpoint answering `application/dns-json`.
		doh: SmolStr,
	},
	/// `com.atproto.identity.resolveHandle` on an xrpc service, an AppView or
	/// a PDS: the answer that service's clients see, cache included.
	Service {
		/// The service host, ie [`HandleResolver::PUBLIC_APPVIEW`].
		host: SmolStr,
	},
}

impl Default for HandleResolver {
	fn default() -> Self {
		Self::Direct {
			doh: SmolStr::new_static(Self::CLOUDFLARE_DOH),
		}
	}
}

impl HandleResolver {
	/// Cloudflare's DNS-over-HTTPS json endpoint.
	pub const CLOUDFLARE_DOH: &'static str =
		"https://cloudflare-dns.com/dns-query";
	/// The public unauthenticated Bluesky AppView, the resolver the app itself
	/// asks.
	pub const PUBLIC_APPVIEW: &'static str = "https://public.api.bsky.app";
	/// The DNS record type of a TXT record.
	const TXT: u16 = 16;

	/// Ask `host`'s `resolveHandle`, see [`HandleResolver::Service`].
	pub fn service(host: impl Into<SmolStr>) -> Self {
		Self::Service { host: host.into() }
	}

	/// The did `handle` resolves to; a handle nothing points at is an error
	/// rather than an empty answer.
	pub async fn resolve(&self, handle: &str) -> Result<Did> {
		let handle = handle.trim_start_matches('@').to_ascii_lowercase();
		match self {
			Self::Service { host } => provider_send::send(
				Request::get(format!(
					"{host}/xrpc/com.atproto.identity.resolveHandle"
				))
				.with_param("handle", &handle),
			)
			.await?
			.into_result()
			.await
			.map_err(|err| {
				bevyhow!("resolving @{handle} through {host}: {err}")
			})?
			.json::<ResolveHandleResponse>()
			.await?
			.did
			.xok(),
			Self::Direct { doh } => match Self::dns(doh, &handle).await? {
				Some(did) => did.xok(),
				None => Self::well_known(&handle).await,
			},
		}
	}

	/// The did the `_atproto` TXT record names, `None` when there is none.
	/// Two records is an error: resolvers do not merge them, so the handle
	/// resolves to neither.
	async fn dns(doh: &str, handle: &str) -> Result<Option<Did>> {
		let name = format!("_atproto.{handle}");
		let answer = provider_send::send(
			Request::get(doh)
				.with_param("name", &name)
				.with_param("type", "TXT")
				.with_header_raw("accept", "application/dns-json"),
		)
		.await?
		.into_result()
		.await?
		.json::<DnsAnswer>()
		.await?;
		let dids = answer
			.answer
			.iter()
			.filter(|record| record.r#type == Self::TXT)
			.filter_map(|record| {
				record.data.trim_matches('"').strip_prefix("did=")
			})
			.map(Did::parse)
			.collect::<Result<Vec<_>>>()?;
		match dids.as_slice() {
			[] => Ok(None),
			[did] => Ok(Some(did.clone())),
			_ => bevybail!(
				"`{name}` holds {} did records, so @{handle} resolves to none \
				 of them",
				dids.len()
			),
		}
	}

	/// The did the handle's own host serves.
	async fn well_known(handle: &str) -> Result<Did> {
		let body = provider_send::send(Request::get(format!(
			"https://{handle}/.well-known/atproto-did"
		)))
		.await?
		.into_result()
		.await
		.map_err(|err| {
			bevyhow!(
				"@{handle} does not resolve: no `_atproto.{handle}` TXT \
				 record and no `/.well-known/atproto-did`: {err}"
			)
		})?
		.text()
		.await?;
		Did::parse(body.trim()).map_err(|err| {
			bevyhow!(
				"@{handle} does not resolve: no `_atproto.{handle}` TXT \
				 record, and `/.well-known/atproto-did` answers no did: {err}"
			)
		})
	}
}

/// The `com.atproto.identity.resolveHandle` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveHandleResponse {
	/// The did the handle points at.
	pub did: Did,
}

/// A DNS-over-HTTPS json answer, the parts a TXT lookup reads.
#[derive(Debug, Deserialize)]
struct DnsAnswer {
	#[serde(default, rename = "Answer")]
	answer: Vec<DnsRecord>,
}

#[derive(Debug, Deserialize)]
struct DnsRecord {
	#[serde(rename = "type")]
	r#type: u16,
	data: SmolStr,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Pete's did document as the PLC directory answered it.
	#[beet_core::test]
	fn reads_a_did_document() {
		let document = serde_json::from_str::<DidDocument>(
			r##"{"@context":["https://www.w3.org/ns/did/v1"],"id":"did:plc:hnv7bd4gtxrf7iigjo22qukp","alsoKnownAs":["at://pete.beet.org"],"verificationMethod":[],"service":[{"id":"#atproto_pds","type":"AtprotoPersonalDataServer","serviceEndpoint":"https://cordyceps.us-west.host.bsky.network"}]}"##,
		)
		.unwrap();
		document
			.pds_endpoint()
			.unwrap()
			.xpect_eq("https://cordyceps.us-west.host.bsky.network");
		document.handle().xpect_eq(Some("pete.beet.org"));
	}
}
