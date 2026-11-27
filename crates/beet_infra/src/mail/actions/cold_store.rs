//! The deploy machine's view of the buckets a cold copy spans: the live
//! archive over the instance's own cloud, and the cold store over the S3 api
//! at the other vendor's endpoint under the parked token.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// One object of a bucket listing, as much of it as a verb here reads.
#[derive(Debug, Clone, PartialEq)]
pub struct ListedObject {
	pub key: String,
	/// RFC 3339, as the S3 api reports it, so a lexical comparison is a
	/// chronological one.
	pub last_modified: String,
	pub size: u64,
}

/// A bucket listing under one prefix, whichever side it came from.
#[derive(Debug, Default, Clone)]
pub struct Listing(pub Vec<ListedObject>);

impl Listing {
	/// The objects of every `ListObjectsV2` page, in page order.
	fn from_pages<'a>(pages: impl IntoIterator<Item = &'a BsxElement>) -> Self {
		pages
			.into_iter()
			.flat_map(|page| page.children_named("Contents"))
			.filter_map(|object| {
				Some(ListedObject {
					key: object.text_at("Key")?.to_string(),
					last_modified: object.text_at("LastModified")?.to_string(),
					size: object
						.text_at("Size")
						.and_then(|size| size.parse().ok())
						.unwrap_or_default(),
				})
			})
			.collect::<Vec<_>>()
			.xmap(Self)
	}

	/// The newest object whose key ends in `suffix`, by last-modified rather
	/// than by name.
	///
	/// The keys are date-ordered, so sorting by name would agree today, and
	/// stop agreeing the moment a snapshot is copied, re-uploaded or restored
	/// from an archive tier, which are exactly the circumstances a drill runs
	/// in. Filtering first stops a marker or an unrelated object winning.
	pub fn newest(&self, suffix: &str) -> Option<&ListedObject> {
		self.0
			.iter()
			.filter(|object| object.key.ends_with(suffix))
			.max_by(|left, right| left.last_modified.cmp(&right.last_modified))
	}
}

/// The live side, ie an S3 bucket the deploy machine's own credentials read.
pub struct LiveStore {
	pub region: String,
	pub bucket: String,
}

impl LiveStore {
	fn remote(&self) -> RemoteBucket {
		RemoteBucket::new(
			S3Store::new(self.bucket.clone(), self.region.clone()),
			AwsClient::new(AwsService::S3, self.region.clone()),
		)
	}

	pub async fn list(&self, prefix: &str) -> Result<Listing> {
		self.remote().list(prefix).await
	}

	pub async fn download(&self, key: &str, local: &AbsPath) -> Result {
		self.remote().download(key, local).await
	}
}

/// The cold side: the [`R2BucketBlock`] a mail box copies into, reached over
/// the S3 api at the account's endpoint under the token a human parked, whose
/// pair signs every request in place of the process's own credential.
pub struct ColdStore {
	pub bucket: String,
	remote: RemoteBucket,
}

impl ColdStore {
	/// The store `block` declares in `stack`, under the credential parked
	/// for it in `secrets`, that stack's store: the stage whose token it is,
	/// which for a drill is the SOURCE stage rather than the drill's own.
	///
	/// Missing is an error naming the apply that parks it, never a skip: a
	/// verb that quietly did nothing against an empty cold bucket is the
	/// failure the cold copy exists to close.
	pub async fn resolve(
		block: &R2BucketBlock,
		stack: &ResolvedStack,
		secrets: &SecretStore,
	) -> Result<Self> {
		let (access_key, secret_key) =
			block.parked_pair(stack, secrets).await?;
		let credentials = AwsCredentials::new(access_key, secret_key);
		let bucket = block.bucket_name(stack);
		let endpoint = block.endpoint(stack)?;
		let region = CloudflareAccount::R2_REGION;
		Self {
			remote: RemoteBucket::new(
				S3Store::new(bucket.clone(), region)
					.with_endpoint(endpoint.clone())
					.with_credentials(credentials.clone()),
				AwsClient::new(AwsService::S3, region)
					.with_endpoint(endpoint)
					.with_credentials(credentials),
			),
			bucket,
		}
		.xok()
	}

	pub async fn list(&self, prefix: &str) -> Result<Listing> {
		self.remote.list(prefix).await
	}

	pub async fn download(&self, key: &str, local: &AbsPath) -> Result {
		self.remote.download(key, local).await
	}
}

/// One bucket over the S3 api, its objects read through its [`S3Store`] and
/// its listing through a client of the same endpoint, region and credential:
/// a listing here orders by last-modified, which a [`BlobStat`] does not
/// carry.
struct RemoteBucket {
	bucket: SmolStr,
	store: BlobStore,
	client: AwsClient,
}

impl RemoteBucket {
	fn new(store: S3Store, client: AwsClient) -> Self {
		Self {
			bucket: store.bucket_name().clone(),
			store: BlobStore::new(store),
			client,
		}
	}

	/// Every object under `prefix`, paged through `ListObjectsV2`.
	async fn list(&self, prefix: &str) -> Result<Listing> {
		let mut pages = Vec::new();
		let mut continuation_token: Option<String> = None;
		loop {
			let request = AwsRequest::get("ListObjectsV2", "/")
				.with_bucket(self.bucket.clone())
				.with_query("list-type", "2")
				.with_query("prefix", prefix);
			let request = match continuation_token.take() {
				Some(token) => request.with_query("continuation-token", token),
				None => request,
			};
			let page = self.client.send_xml(request).await?;
			continuation_token = (page.text_at("IsTruncated") == Some("true"))
				.then(|| page.text_at("NextContinuationToken"))
				.flatten()
				.map(ToString::to_string);
			pages.push(page);
			if continuation_token.is_none() {
				break;
			}
		}
		Listing::from_pages(&pages).xok()
	}

	/// The object at `key`, written to `local`.
	async fn download(&self, key: &str, local: &AbsPath) -> Result {
		let bytes = self.store.get(&FwdPath::new(key)?).await?;
		fs_ext::write_async(local, bytes).await?;
		Ok(())
	}
}

impl ListedObject {
	/// sha256 of a downloaded object, hex, for the byte-for-byte comparisons
	/// the cold verbs make: two downloads are compared by digest rather than
	/// held in memory together.
	pub fn digest(path: &AbsPath) -> Result<String> {
		digest_ext::hex_file::<sha2::Sha256>(path)?.xok()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	/// A `ListObjectsV2` page holding `objects`, continued by `next`.
	fn page(objects: &[(&str, &str, u64)], next: Option<&str>) -> String {
		let contents = objects
			.iter()
			.map(|(key, modified, size)| {
				format!(
					"<Contents><Key>{key}</Key><LastModified>{modified}</LastModified>\
					<ETag>&quot;abc&quot;</ETag><Size>{size}</Size>\
					<StorageClass>STANDARD</StorageClass></Contents>"
				)
			})
			.collect::<String>();
		let truncation = match next {
			Some(next) => format!(
				"<IsTruncated>true</IsTruncated>\
				<NextContinuationToken>{next}</NextContinuationToken>"
			),
			None => "<IsTruncated>false</IsTruncated>".to_string(),
		};
		format!(
			"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
			<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">\
			<Name>cold</Name><Prefix>sqlite/</Prefix><KeyCount>{}</KeyCount>\
			<MaxKeys>1000</MaxKeys>{truncation}{contents}</ListBucketResult>",
			objects.len()
		)
	}

	/// A listing is every page of `ListObjectsV2` under the prefix, at the
	/// R2 endpoint path style and signed for `auto` as the parked pair, the
	/// continuation token carried from one page to the next.
	#[beet_core::test]
	async fn lists_every_page_under_the_prefix() {
		let pages = Arc::new(Mutex::new(
			vec![
				page(
					&[(
						"sqlite/2026/09/05/old.db",
						"2026-09-05T14:30:00.000Z",
						1,
					)],
					Some("token-1"),
				),
				page(
					&[
						("sqlite/notes.txt", "2026-09-07T14:30:00.000Z", 1),
						(
							"sqlite/2026/09/06/new.db",
							"2026-09-06T14:30:00.000Z",
							2,
						),
					],
					None,
				),
			]
			.into_iter(),
		));
		let urls = Arc::new(Mutex::new(Vec::new()));
		let seen = urls.clone();
		let client =
			AwsClient::new(AwsService::S3, CloudflareAccount::R2_REGION)
				.with_endpoint("https://acct.r2.cloudflarestorage.com")
				.with_credentials(AwsCredentials::new("AKID", "secret"))
				.with_transport(move |request| {
					let (pages, seen) = (pages.clone(), seen.clone());
					Box::pin(async move {
						(*request.method()).xpect_eq(HttpMethod::Get);
						request
							.headers
							.first_raw("authorization")
							.unwrap()
							.xpect_contains("Credential=AKID/")
							.xpect_contains("/auto/s3/aws4_request");
						seen.lock().unwrap().push(request.uri());
						Response::ok()
							.with_body(pages.lock().unwrap().next().unwrap())
							.xok()
					})
				});
		let bucket = RemoteBucket::new(S3Store::new("cold", "auto"), client);
		let listing = bucket.list("sqlite/").await.unwrap();

		let urls = urls.lock().unwrap().clone();
		urls.len().xpect_eq(2);
		urls[0]
			.as_str()
			.xpect_starts_with("https://acct.r2.cloudflarestorage.com/cold/?")
			.xpect_contains("list-type=2")
			.xpect_contains("prefix=sqlite/")
			.xnot()
			.xpect_contains("continuation-token");
		urls[1]
			.as_str()
			.xpect_contains("continuation-token=token-1");

		listing.0.len().xpect_eq(3);
		listing.0[2].size.xpect_eq(2);
		// newest by last-modified, filtered by suffix first: a marker under
		// the prefix that is newer still loses
		listing
			.newest(".db")
			.unwrap()
			.key
			.as_str()
			.xpect_eq("sqlite/2026/09/06/new.db");
		listing.newest(".age").xpect_none();
	}

	/// An empty bucket answers a page with no `Contents`, a listing of
	/// nothing rather than an error.
	#[beet_core::test]
	fn an_empty_page_is_an_empty_listing() {
		Listing::from_pages(
			&[BsxElement::parse_xml(&page(&[], None)).unwrap()],
		)
		.0
		.len()
		.xpect_eq(0);
	}
}
