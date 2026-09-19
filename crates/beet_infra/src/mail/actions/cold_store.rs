//! The deploy machine's view of the buckets a cold copy spans: the live
//! archive over the instance's own cloud, and the cold store over the S3 api
//! at the other vendor's endpoint under the parked token.
use crate::actions::aws_cli_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;

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
	/// Parse an `aws s3api list-objects-v2 --output json` body. An empty
	/// bucket prints nothing at all rather than an empty document, so that
	/// is a listing of nothing rather than a parse error.
	pub fn parse(body: &str) -> Result<Self> {
		if body.trim().is_empty() {
			return Ok(Self::default());
		}
		let listing: Value = serde_json::from_str(body)?;
		listing["Contents"]
			.as_array()
			.into_iter()
			.flatten()
			.filter_map(|object| {
				Some(ListedObject {
					key: object["Key"].as_str()?.to_string(),
					last_modified: object["LastModified"].as_str()?.to_string(),
					size: object["Size"].as_u64().unwrap_or_default(),
				})
			})
			.collect::<Vec<_>>()
			.xmap(Self)
			.xok()
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
	pub async fn list(&self, prefix: &str) -> Result<Listing> {
		aws_cli_ext::service("s3api", &self.region, [
			"list-objects-v2",
			"--bucket",
			&self.bucket,
			"--prefix",
			prefix,
			"--output",
			"json",
		])
		.run_async_stdout()
		.await?
		.xmap(|body| Listing::parse(&body))
	}

	pub async fn download(&self, key: &str, local: &AbsPath) -> Result {
		aws_cli_ext::service("s3", &self.region, [
			"cp",
			&format!("s3://{}/{key}", self.bucket),
			local.as_str(),
			"--only-show-errors",
		])
		.run_async()
		.await?;
		Ok(())
	}
}

/// The cold side: the [`R2BucketBlock`] a mail box copies into, reached over
/// the S3 api at the account's endpoint under the token a human parked. The
/// pair rides the child's environment rather than its argv, and the secret
/// half is declared so a failure cannot print it.
pub struct ColdStore {
	pub bucket: String,
	endpoint: String,
	access_key: String,
	secret_key: String,
}

impl ColdStore {
	/// The store `block` declares, under the credential parked for the stack
	/// `secrets` is scoped to: the stage whose token it is, which for a drill
	/// is the SOURCE stage rather than the drill's own.
	///
	/// Missing is an error naming the apply that parks it, never a skip: a
	/// verb that quietly did nothing against an empty cold bucket is the
	/// failure the cold copy exists to close.
	pub async fn resolve(
		block: &R2BucketBlock,
		secrets: &SecretStore,
	) -> Result<Self> {
		let (access_key, secret_key) = block.parked_pair(secrets).await?;
		Self {
			bucket: block.bucket_name(secrets.stack()),
			endpoint: block.endpoint(),
			access_key,
			secret_key,
		}
		.xok()
	}

	/// An `aws <service>` invocation against the cold endpoint. Region `auto`
	/// is what every R2 bucket answers to; the session token is cleared so a
	/// deploy machine holding temporary AWS credentials does not send them to
	/// the other vendor.
	fn aws<'a>(
		&self,
		service: &'a str,
		args: impl IntoIterator<Item = &'a str>,
	) -> ChildProcess {
		ChildProcess::new("aws")
			.without_env("AWS_PROFILE")
			.without_env("AWS_SESSION_TOKEN")
			.with_envs([
				("AWS_ACCESS_KEY_ID", self.access_key.as_str()),
				("AWS_SECRET_ACCESS_KEY", self.secret_key.as_str()),
			])
			.with_args(
				[service].into_iter().chain(args).map(SmolStr::from).chain([
					SmolStr::from("--endpoint-url"),
					SmolStr::from(self.endpoint.as_str()),
					SmolStr::from("--region"),
					SmolStr::from(R2BucketBlock::REGION),
				]),
			)
			.with_secret(self.secret_key.as_str())
	}

	pub async fn list(&self, prefix: &str) -> Result<Listing> {
		self.aws("s3api", [
			"list-objects-v2",
			"--bucket",
			&self.bucket,
			"--prefix",
			prefix,
			"--output",
			"json",
		])
		.run_async_stdout()
		.await?
		.xmap(|body| Listing::parse(&body))
	}

	pub async fn download(&self, key: &str, local: &AbsPath) -> Result {
		self.aws("s3", [
			"cp",
			&format!("s3://{}/{key}", self.bucket),
			local.as_str(),
			"--only-show-errors",
		])
		.run_async()
		.await?;
		Ok(())
	}

	pub async fn upload(&self, local: &AbsPath, key: &str) -> Result {
		self.aws("s3", [
			"cp",
			local.as_str(),
			&format!("s3://{}/{key}", self.bucket),
			"--only-show-errors",
		])
		.run_async()
		.await?;
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

	/// Newest by last-modified, filtered by suffix first: a marker under the
	/// prefix and a newer object under another prefix both lose, and an empty
	/// bucket's empty stdout is a listing of nothing rather than an error.
	#[beet_core::test]
	fn newest_selects_by_time_within_the_suffix() {
		let listing = Listing::parse(
			r#"{"Contents":[
				{"Key":"sqlite/2026/09/05/old.db","LastModified":"2026-09-05T14:30:00Z","Size":1},
				{"Key":"sqlite/notes.txt","LastModified":"2026-09-07T14:30:00Z","Size":1},
				{"Key":"sqlite/2026/09/06/new.db","LastModified":"2026-09-06T14:30:00Z","Size":2}
			]}"#,
		)
		.unwrap();
		listing
			.newest(".db")
			.unwrap()
			.key
			.as_str()
			.xpect_eq("sqlite/2026/09/06/new.db");
		listing.newest(".age").xpect_none();
		Listing::parse("").unwrap().0.len().xpect_eq(0);
		Listing::parse("{}").unwrap().0.len().xpect_eq(0);
	}
}
