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
/// ## The credential is minted by the apply and parked, not lowered
///
/// An R2 bucket is reached over the S3 api with an api token, and no IAM
/// policy can grant that. So the apply mints one: an account-owned
/// `cloudflare_account_token` scoped to this one bucket (read and write on its
/// objects, nothing else in the account), whose id IS the S3 access key id and
/// whose value's SHA-256 IS the secret access key. Both are parked as
/// `SecureString` parameters under the stack's secret prefix, exactly as the
/// mail box parks its SES relay pair, and this block's [grants](Block::grants)
/// name those two parameters rather than the bucket, so an AWS compute lowers
/// them to `ssm:GetParameter` on exactly those. SecretRotation is replacing the
/// token resource; the parameters follow it in the same apply.
///
/// That asks one thing of the deployer's own Cloudflare token: `Account API
/// Tokens: Edit`, which is the tier every deployer credential sits at (the AWS
/// deploy user mints IAM users and access keys the same way). One hand-made
/// credential per provider, the deployer's; every runtime credential is
/// minted by an apply.
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
	/// bucket, read and write, and nothing at the account level. Ids are
	/// Cloudflare's global constants (`GET /accounts/{id}/tokens/permission_groups`
	/// lists them by name); an id that stopped existing fails the apply.
	pub const ITEM_READ_PERMISSION: &'static str =
		"6a018a9f2fc74eb6b293b0c548f38b39";
	pub const ITEM_WRITE_PERMISSION: &'static str =
		"2efd5506f9c8494dacb1fa10a3e7d5b6";

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
	/// script. The apply writes the pair beside the bucket, so its absence
	/// means the apply has not run since the bucket was declared, or somebody
	/// deleted the parameter, and either way the next apply restores it.
	pub fn missing_credential(&self, stack: &ResolvedStack) -> String {
		format!(
			"the apply mints the token for {} and parks its S3 pair at {} and \
			{}; run deploy",
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

	/// The token's resource scope: this bucket, in the default jurisdiction.
	fn token_resources(&self, stack: &ResolvedStack) -> Result<String> {
		serde_json::json!({
			format!(
				"com.cloudflare.edge.r2.bucket.{}_default_{}",
				stack.cloudflare_account()?.id,
				self.bucket_name(stack)
			): "*"
		})
		.to_string()
		.xok()
	}

	/// The account-owned token the apply mints for this bucket, and the two
	/// parameters its S3 pair is parked in. The token depends on the bucket
	/// explicitly, since its policy names the bucket as a string rather than
	/// a reference.
	fn emit_token(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		bucket: &ResourceDef<CloudflareR2BucketDetails>,
	) -> Result {
		let token = ResourceDef::new_secondary(
			stack.resource_ident(format!("{}-token", self.label)),
			CloudflareAccountTokenDetails {
				account_id: stack.cloudflare_account()?.id.clone(),
				name: stack
					.resource_name(format!("{}-token", self.label))
					.into(),
				policies: vec![CloudflareAccountTokenPolicies {
					effect: "allow".into(),
					permission_groups: vec![
						CloudflareAccountTokenPoliciesPermissionGroups {
							id: Self::ITEM_READ_PERMISSION.into(),
						},
						CloudflareAccountTokenPoliciesPermissionGroups {
							id: Self::ITEM_WRITE_PERMISSION.into(),
						},
					],
					resources: self.token_resources(stack)?.into(),
				}],
				depends_on: Some(vec![bucket.address().into()]),
				..default()
			},
		);
		config.add_resource(&token)?;
		// the S3 pair, derived in-config so the secret exists nowhere but the
		// state (encrypted) and the parameter (SecureString); rotated by
		// replacing the token, which the same apply re-parks
		for (secret, value, note) in [
			(
				self.access_key_secret(),
				token.field_ref("id"),
				self.access_key_note(),
			),
			(
				self.secret_key_secret(),
				format!("${{sha256({})}}", token.field("value")),
				self.secret_key_note(),
			),
		] {
			config.add_untyped_resource(
				"aws_ssm_parameter",
				stack.resource_ident(secret.label().clone()).label(),
				&secret.parameter_resource(
					stack,
					value,
					&note,
					SecretRotation::replace(token.address()),
				),
			)?;
		}
		Ok(())
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
		self.emit_token(stack, config, &bucket)?;
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

	/// The apply mints the credential: an account-owned token scoped to this
	/// bucket's objects and nothing else, created after the bucket it names,
	/// its id and the sha256 of its value parked as the S3 pair under the
	/// stack's secret prefix, each described as a replacement of the token
	/// so a listing says how it rotates. No hand step anywhere in the path.
	#[beet_core::test]
	fn the_token_is_minted_and_parked_by_the_apply() {
		let rendered = render(cold());
		rendered
			.xpect_contains("\"cloudflare_account_token\"")
			.xpect_contains("\"name\":\"beet-infra--dev--cold-backups-token\"")
			.xpect_contains(R2BucketBlock::ITEM_READ_PERMISSION)
			.xpect_contains(R2BucketBlock::ITEM_WRITE_PERMISSION)
			.xpect_contains(
				"com.cloudflare.edge.r2.bucket.acct123_default_beet-infra--dev--cold-backups",
			)
			.xpect_contains("\"depends_on\":[\"cloudflare_r2_bucket.")
			.xpect_contains("\"aws_ssm_parameter\"")
			.xpect_contains("\"name\":\"/beet-infra/dev/cold-backups-access-key-id\"")
			.xpect_contains("\"name\":\"/beet-infra/dev/cold-backups-secret-access-key\"")
			.xpect_contains("\"type\":\"SecureString\"")
			.xpect_contains("${sha256(cloudflare_account_token.")
			.xpect_contains(
				"\"description\":\"replace:cloudflare_account_token.beet_infra__dev__cold_backups_token :: R2 token for bucket cold-backups: access key id\"",
			)
			// the resource scope is one bucket, never the account
			.xnot()
			.xpect_contains("com.cloudflare.api.account");
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
