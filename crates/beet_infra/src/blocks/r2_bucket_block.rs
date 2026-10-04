use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A Cloudflare R2 bucket, declared once like an [`S3BucketBlock`] and created
/// by the same apply, in a different VENDOR's account. That is the whole reason
/// it exists: every other store a stack declares lives in the one AWS account,
/// so a lost credential or a lost account takes the record and every copy of it
/// together. A cold copy here is the one that does not go with them.
///
/// Authored directly from markup, ie `<R2BucketBlock label="cold-backups"
/// location="weur"/>`. The `<app>--<stage>--<label>` name composes through the
/// ancestor [`Stack`] exactly as an S3 bucket's does, so a consumer names it by
/// label and never by a second composition.
///
/// ## The credential is parked, and the apply never mints it
///
/// An R2 bucket is reached over the S3 api with an api token, and no IAM policy
/// can grant that. So one is minted for this bucket alone: an account-owned
/// token scoped to its objects and nothing else in the account, whose id IS the
/// S3 access key id and whose value's SHA-256 IS the secret access key. Both
/// are parked as `SecureString` parameters under the stack's secret prefix,
/// exactly as the mail box parks its SES relay pair, and this block's
/// [grants](Block::grants) name those two parameters rather than the bucket, so
/// an AWS compute lowers them to `ssm:GetParameter` on exactly those.
///
/// **`cloudflare/mint` does that minting, not the apply**, and the difference
/// is the whole reason the seam exists. Creating a token needs `Account API
/// Tokens Write`, which can mint any token the account can hold, a wider one
/// included; Cloudflare has no permissions boundary to cap that with and no mfa
/// condition to put on it, so the only thing that keeps it out of reach is that
/// no credential a deploy reads carries it. An apply that minted this token
/// would put it in every deploy credential of every repo declaring a bucket.
/// So the rare operator verb mints it with a credential that exists in no
/// document, the apply creates the bucket and reads nothing, and
/// [`missing_credential`](Self::missing_credential) names the verb when the
/// parked pair is not there.
///
/// ## What R2 does not have
///
/// Object versioning. A deletion or an overwrite is final, so the retention
/// story is entirely [`expire_prefixes`](Self::expire_prefixes) plus the
/// writer's own discipline (copy, never sync). The credential that writes here
/// can also delete here, and this block does not pretend otherwise: it is the
/// off-account copy, not the off-everything one.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedStoreBlock::on_insert::<Self>,
	on_remove = ErasedStoreBlock::on_remove
)]
pub struct R2BucketBlock {
	label: SmolStr,
	/// The location HINT, ie `weur`: where R2 places the bucket on a
	/// best-effort basis, honoured only at creation. For a cold copy this is
	/// the point: a store meant to survive a regional event should not sit in
	/// the region it is a copy of. Empty lets R2 choose, which is near the
	/// caller, which is the wrong answer for a backup.
	#[set_with(into)]
	location: SmolStr,
	/// Expiries scoped to a key prefix, see [`PrefixExpiry`]. The whole
	/// retention story for a bucket with no versioning: a prefix not named
	/// here is kept forever.
	expire_prefixes: Vec<PrefixExpiry>,
}

impl Default for R2BucketBlock {
	fn default() -> Self { Self::new("") }
}

impl R2BucketBlock {
	/// The [`AccessGrant::kind`] this bucket declares, lowered by an AWS
	/// compute to a read of the parked credential rather than to a bucket
	/// policy it cannot write.
	pub const ACCESS_KIND: &'static str = "r2_bucket";

	/// The S3 api's region for every R2 bucket.
	pub const REGION: &'static str = "auto";

	/// The permission groups the bucket's token holds: objects in this one
	/// bucket, read and write, and nothing at the account level. Named through
	/// [`TokenPermission`], the one table every Cloudflare group beet uses
	/// lives in, so the id and what it grants are read in one place.
	pub const ITEM_PERMISSIONS: &'static [TokenPermission] = &[
		TokenPermission::BUCKET_ITEM_READ,
		TokenPermission::BUCKET_ITEM_WRITE,
	];

	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			location: SmolStr::default(),
			expire_prefixes: Vec::new(),
		}
	}

	/// The composed bucket name, ie `beetmash-mail--prod--cold-backups`.
	pub fn bucket_name(&self, stack: &ResolvedStack) -> String {
		stack.resource_name(self.label.clone())
	}

	/// The S3-compatible endpoint every client dials, composed from the
	/// stack's [`CloudflareAccount`] rather than typed by a consumer.
	pub fn endpoint(&self, stack: &ResolvedStack) -> Result<String> {
		format!(
			"https://{}.r2.cloudflarestorage.com",
			stack.cloudflare_account()?.id
		)
		.xok()
	}

	/// Where the token's S3 access key id is parked, ie
	/// `/beetmash-mail/prod/cold-backups-access-key-id`.
	pub fn access_key_secret(&self) -> SecretRef {
		SecretRef::new(format!("{}-access-key-id", self.label))
	}

	/// Where the token's S3 secret access key is parked.
	pub fn secret_key_secret(&self) -> SecretRef {
		SecretRef::new(format!("{}-secret-access-key", self.label))
	}

	/// The note the access key id is parked with.
	pub fn access_key_note(&self) -> String {
		format!("R2 token for bucket {}: access key id", self.label)
	}

	/// The note the secret access key is parked with.
	pub fn secret_key_note(&self) -> String {
		format!("R2 token for bucket {}: secret access key", self.label)
	}

	/// What a consumer says when the parked pair is missing, worded once.
	/// Free of quotes and backticks, since one of those consumers is a shell
	/// script. The verb that mints parks the pair, so its absence means it has
	/// not run since the bucket was declared, or somebody deleted the
	/// parameter, and either way running it again restores the pair without
	/// touching the bucket.
	pub fn missing_credential(&self, stack: &ResolvedStack) -> String {
		format!(
			"beet cloudflare/mint mints the token for {} and parks its S3 pair \
			at {} and {}; run it with the mint token in the environment",
			self.bucket_name(stack),
			self.access_key_secret().name(stack),
			self.secret_key_secret().name(stack),
		)
	}

	/// The S3 pair the apply parked for this bucket, read from the stack's
	/// secret store (the stack being the store's): what a deploy-machine
	/// process reaches the bucket with, since its own credentials are the
	/// other vendor's. Missing is an error naming the apply that parks it,
	/// never a skip: a verb that quietly did nothing against an empty bucket
	/// is the failure a cold copy exists to close.
	#[cfg(feature = "vault")]
	pub async fn parked_pair(
		&self,
		secrets: &SecretStore,
	) -> Result<(String, String)> {
		let access_key = secrets.get(&self.access_key_secret()).await?;
		let secret_key = secrets.get(&self.secret_key_secret()).await?;
		match (access_key, secret_key) {
			(Some(access_key), Some(secret_key)) => {
				(access_key, secret_key).xok()
			}
			_ => bevybail!(
				"no cold credential at {} and {}: {}",
				secrets.address(&self.access_key_secret()),
				secrets.address(&self.secret_key_secret()),
				self.missing_credential(secrets.stack())
			),
		}
	}

	/// The bucket over the S3 api under the parked pair
	/// ([`parked_pair`](Self::parked_pair)): the store a deploy-machine
	/// process reads and writes the bucket through. The runtime attach lands
	/// a store under the process's ambient credentials, which are AWS's, so
	/// a verb targeting this bucket resolves this instead. Native, as the
	/// S3 client is.
	#[cfg(all(
		feature = "vault",
		feature = "aws_sdk",
		not(target_arch = "wasm32")
	))]
	pub async fn parked_store(
		&self,
		secrets: &SecretStore,
	) -> Result<BlobStore> {
		let (access_key, secret_key) = self.parked_pair(secrets).await?;
		S3Store::from_uri(&self.store_uri(secrets.stack())?)?
			.with_credentials(S3Credentials::new(access_key, secret_key))
			.xmap(BlobStore::new)
			.xok()
	}

	/// The name the bucket's own token carries at the account, ie
	/// `beetmash-mail--prod--cold-backups-token`: how the verb that mints finds
	/// the one it already made.
	pub fn token_name(&self, stack: &ResolvedStack) -> String {
		stack.resource_name(format!("{}-token", self.label))
	}

	/// The policy the bucket's own token carries, ready for
	/// `POST /accounts/{id}/tokens`: read and write on the objects of this one
	/// bucket, and nothing else in the account. One place says what the token
	/// grants, so the verb that mints it posts a declaration rather than
	/// composing one.
	pub fn token_policies(
		&self,
		stack: &ResolvedStack,
	) -> Result<serde_json::Value> {
		serde_json::json!([{
			"effect": "allow",
			"resources": self.token_resources(stack)?,
			"permission_groups": Self::ITEM_PERMISSIONS
				.iter()
				.map(|permission| serde_json::json!({ "id": permission.id() }))
				.collect::<Vec<_>>(),
		}])
		.xok()
	}

	/// The S3 secret access key a token value derives to: its lowercase hex
	/// SHA-256, which is what R2 expects and the only half of the pair that is
	/// computed rather than read. Pinned by a test, because a derivation that
	/// changed shape would park a credential that authenticates nowhere and
	/// nothing would say so until a copy failed.
	pub fn derive_secret_key(token_value: &str) -> String {
		digest_ext::hex::<sha2::Sha256>(token_value.as_bytes())
	}

	/// The token's resource scope: this bucket, in the default jurisdiction.
	fn token_resources(
		&self,
		stack: &ResolvedStack,
	) -> Result<serde_json::Value> {
		serde_json::json!({
			format!(
				"com.cloudflare.edge.r2.bucket.{}_default_{}",
				stack.cloudflare_account()?.id,
				self.bucket_name(stack)
			): "*"
		})
		.xok()
	}

	/// Rejects a declaration the provider would reject, at config time.
	pub fn validate(&self) -> Result {
		if !self.location.is_empty()
			&& !Self::LOCATIONS.contains(&self.location.as_str())
		{
			bevybail!(
				"r2 bucket '{}' names location '{}', which is not one of {}",
				self.label,
				self.location,
				Self::LOCATIONS.join(", ")
			);
		}
		for expiry in &self.expire_prefixes {
			expiry.validate()?;
		}
		PrefixExpiry::assert_unique_ids(
			&self.label,
			self.rule_ids().iter().map(String::as_str),
		)
	}

	/// The location hints R2 accepts, checked at render because a typo here
	/// creates the bucket somewhere else and the hint is honoured exactly once.
	pub const LOCATIONS: &'static [&'static str] =
		&["apac", "eeur", "enam", "weur", "wnam", "oc"];

	fn rule_ids(&self) -> Vec<String> {
		self.expire_prefixes
			.iter()
			.map(PrefixExpiry::rule_id)
			.collect()
	}

	/// The age transition a [`PrefixExpiry`] renders as: R2 counts in
	/// seconds.
	fn rule(expiry: &PrefixExpiry) -> CloudflareR2BucketLifecycleRules {
		CloudflareR2BucketLifecycleRules {
			id: expiry.rule_id().into(),
			enabled: true,
			conditions: CloudflareR2BucketLifecycleRulesConditions {
				prefix: expiry.prefix().clone(),
			},
			delete_objects_transition: Some(
				CloudflareR2BucketLifecycleRulesDeleteObjectsTransition {
					condition: Some(
						CloudflareR2BucketLifecycleRulesDeleteObjectsTransitionCondition {
							r#type: "Age".into(),
							max_age: Some(expiry.expire_days() * 24 * 60 * 60),
							date: None,
						},
					),
				},
			),
			abort_multipart_uploads_transition: None,
			storage_class_transitions: None,
		}
	}
}

impl Block for R2BucketBlock {
	fn label(&self) -> &SmolStr { &self.label }

	/// A process beside this bucket reads the parked S3 pair and nothing
	/// else: the bucket itself answers to the token, not to the process's
	/// identity.
	fn grants(&self, stack: &ResolvedStack) -> Vec<AccessGrant> {
		vec![
			AccessGrant::read(
				Self::ACCESS_KIND,
				self.access_key_secret().name(stack),
			),
			AccessGrant::read(
				Self::ACCESS_KIND,
				self.secret_key_secret().name(stack),
			),
		]
	}
}

impl StoreBlock for R2BucketBlock {
	/// The bucket over the S3 api at the account's endpoint, which is how
	/// every non-Worker client reaches R2.
	fn store_uri(&self, stack: &ResolvedStack) -> Result<StoreUri> {
		StoreUri::S3 {
			name: self.bucket_name(stack).into(),
			path_prefix: None,
			endpoint: Some(self.endpoint(stack)?.into()),
			region: Some(Self::REGION.into()),
		}
		.xok()
	}
}

impl EmitBlock for R2BucketBlock {
	fn emit(
		&self,
		stack: &ResolvedStack,
		_deployment: &Deployment,
		config: &mut terra::Config,
	) -> Result {
		self.validate()?;
		ensure_cloudflare_provider(config)?;
		let account_id = stack.cloudflare_account()?.id.clone();
		let bucket = ResourceDef::new_primary(
			stack.resource_ident(self.label.clone()),
			CloudflareR2BucketDetails {
				account_id: account_id.clone(),
				location: (!self.location.is_empty())
					.then(|| self.location.clone()),
				..default()
			},
		);
		config.add_layer_resource(terra::Config::STORAGE_LAYER, &bucket)?;
		if self.expire_prefixes.is_empty() {
			return Ok(());
		}
		config.add_resource(&ResourceDef::new_secondary(
			stack.resource_ident(format!("{}-lifecycle", self.label)),
			CloudflareR2BucketLifecycleDetails {
				account_id,
				bucket_name: bucket.field_ref("name").into(),
				rules: Some(
					self.expire_prefixes.iter().map(Self::rule).collect(),
				),
				..default()
			},
		))?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn cold() -> R2BucketBlock {
		R2BucketBlock::new("cold-backups")
			.with_location("weur")
			.with_expire_prefixes(vec![PrefixExpiry::new("sqlite/", 180)])
	}

	/// The test stack with the account the bucket lands in.
	fn stack() -> (ResolvedStack, Deployment, crate::types::TestWorkDir) {
		let (stack, deployment, dir) = ResolvedStack::default_local();
		(
			stack.with_cloudflare_account(CloudflareAccount::new("acct123")),
			deployment,
			dir,
		)
	}

	fn render(block: R2BucketBlock) -> String {
		let (stack, deployment, _dir) = stack();
		let mut config = deployment.create_config(&stack);
		block.emit(&stack, &deployment, &mut config).unwrap();
		config.to_json_string().unwrap()
	}

	/// The bucket lands in the account the stack declares, at the composed
	/// name, with its hint; the lifecycle counts the declared days in seconds
	/// and names the bucket by reference rather than by a second composition.
	#[beet_core::test]
	fn renders_the_bucket_and_its_lifecycle() {
		render(cold())
			.xpect_contains("\"cloudflare_r2_bucket\"")
			.xpect_contains("\"account_id\":\"acct123\"")
			.xpect_contains("\"name\":\"beet-infra--dev--cold-backups\"")
			.xpect_contains("\"location\":\"weur\"")
			.xpect_contains("\"cloudflare_r2_bucket_lifecycle\"")
			.xpect_contains("\"id\":\"expire-sqlite\"")
			.xpect_contains("\"prefix\":\"sqlite/\"")
			.xpect_contains(&format!("\"max_age\":{}", 180 * 86400))
			.xpect_contains("\"type\":\"Age\"")
			.xpect_contains("\"bucket_name\":\"${cloudflare_r2_bucket.");
	}

	/// No versioning means no unfiltered sweep to render: a bucket declaring
	/// no expiry renders no lifecycle at all rather than an empty one.
	#[beet_core::test]
	fn no_expiry_renders_no_lifecycle() {
		render(cold().with_expire_prefixes(Vec::new()))
			.xnot()
			.xpect_contains("cloudflare_r2_bucket_lifecycle");
	}

	/// The apply creates the bucket and NOTHING else: no token, and no
	/// parameter holding one. A rendered `cloudflare_account_token` would put
	/// `Account API Tokens Write` into the deploy credential of every repo that
	/// declares a bucket, which is the one group no deploy credential may have,
	/// so `cloudflare/mint` mints it out of band and the config never names it.
	#[beet_core::test]
	fn the_apply_mints_no_credential() {
		render(cold())
			.xnot()
			.xpect_contains("cloudflare_account_token")
			.xnot()
			.xpect_contains("aws_ssm_parameter")
			.xnot()
			.xpect_contains("sha256");
	}

	/// What the out-of-band mint addresses, which is the apply's own
	/// composition read back: one token name per bucket, scoped to that
	/// bucket's objects and never to the account.
	#[beet_core::test]
	fn the_token_is_named_and_scoped_by_the_bucket() {
		let (stack, ..) = stack();
		let block = cold();
		block
			.token_name(&stack)
			.xpect_eq("beet-infra--dev--cold-backups-token");
		block
			.token_policies(&stack)
			.unwrap()
			.to_string()
			.as_str()
			.xpect_contains(
				"com.cloudflare.edge.r2.bucket.acct123_default_beet-infra--dev--cold-backups",
			)
			.xpect_contains(TokenPermission::BUCKET_ITEM_READ.id())
			.xpect_contains(TokenPermission::BUCKET_ITEM_WRITE.id())
			.xnot()
			.xpect_contains("com.cloudflare.api.account");
	}

	/// The secret half is a derivation, so it is pinned: lowercase hex SHA-256
	/// of the token value, the standard digest R2 expects, checked against a
	/// published vector rather than against itself.
	#[beet_core::test]
	fn the_secret_key_is_the_hex_sha256_of_the_token() {
		R2BucketBlock::derive_secret_key("test").xpect_eq(
			"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
		);
	}

	/// The grants name the parked credential, never the bucket: an AWS
	/// compute cannot write a policy R2 would read, but it can let the box
	/// read the two parameters the token lives in.
	#[beet_core::test]
	fn grants_read_on_the_parked_credential() {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		cold().grants(&stack).xpect_eq(vec![
			AccessGrant::read(
				R2BucketBlock::ACCESS_KIND,
				"/beet-infra/dev/cold-backups-access-key-id",
			),
			AccessGrant::read(
				R2BucketBlock::ACCESS_KIND,
				"/beet-infra/dev/cold-backups-secret-access-key",
			),
		]);
	}

	/// The hint is honoured exactly once, at creation, so a misspelt one is
	/// a bucket in the wrong place forever: it fails at render instead; and
	/// a stack declaring no account fails naming the spread.
	#[beet_core::test]
	fn a_bad_location_or_a_missing_account_is_loud() {
		cold()
			.with_location("sydney")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("sydney")
			.xpect_contains("weur");
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		cold()
			.emit(&stack, &deployment, &mut deployment.create_config(&stack))
			.unwrap_err()
			.to_string()
			.xpect_contains("CloudflareAccount");
	}

	/// The store uri is the S3 api at the account's endpoint, which is what
	/// every client that is not a Worker binding dials.
	#[beet_core::test]
	fn the_store_uri_dials_the_account_endpoint() {
		let (stack, _deployment, _dir) = stack();
		match cold().store_uri(&stack).unwrap() {
			StoreUri::S3 {
				name,
				endpoint,
				region,
				..
			} => {
				name.as_str().xpect_eq("beet-infra--dev--cold-backups");
				endpoint
					.unwrap()
					.as_str()
					.xpect_eq("https://acct123.r2.cloudflarestorage.com");
				region.unwrap().as_str().xpect_eq("auto");
			}
			other => panic!("expected an S3 uri, got {other:?}"),
		}
	}
}
