use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::json;

/// An S3 bucket, declared once and read by both meanings of the declaration:
/// the deploy creates it, and the runtime attaches the store its erased half
/// names on the same entity (an S3 store remotely, an `FsStore` locally, see
/// [`ErasedStoreBlock`]).
///
/// Authored directly from markup, ie `<S3BucketBlock label="app"
/// deploy_versioned=false/>`. The label alone is declared; the
/// `<app>--<stage>--<label>` name composes at resolution through the ancestor
/// [`Stack`], so both sides read the same string. The bucket an app is served
/// from carries the [`RepoStoreBlock`] marker.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedStoreBlock::on_insert::<Self>,
	on_remove = ErasedStoreBlock::on_remove
)]
pub struct S3BucketBlock {
	label: SmolStr,
	/// add a tofu output for the bucket name
	output: bool,
	/// Allow the deploy to delete a non-empty bucket. `false` for a source of
	/// record, whose accidental teardown is unrecoverable.
	force_destroy: bool,
	/// All objects will be nested under the deploy uuid,
	/// ensuring unique files per deploy
	deploy_versioned: bool,
	/// Declare the runtime grant as read/write rather than read-only, for a
	/// bucket the deployed process stores into (a mail blob store) rather than
	/// one the deploy publishes and the process merely serves.
	runtime_write: bool,
	/// Grant anonymous `s3:GetObject` on every object and `s3:ListBucket` on the
	/// bucket (via a public-access-block that lifts the default block, plus a
	/// bucket policy). Needed when objects are served by a 301 to the public S3
	/// url, and when a credential-free `sync` hydrates a checkout from the
	/// bucket: `aws s3 sync --no-sign-request` lists before it gets, so
	/// `GetObject` alone fails at the first `ListObjectsV2`.
	public_read: bool,
	/// Keep every version of every object, so an overwrite or a delete is
	/// recoverable rather than final. For a SOURCE OF RECORD (a mail blob
	/// store) this is the difference between a bug and a loss; a bucket the
	/// deploy re-publishes into on every run wants it off, since every version
	/// is a copy of something git already has.
	///
	/// Distinct from [`deploy_versioned`](Self::deploy_versioned), which nests
	/// objects under the deploy id and is about publication, not durability.
	object_versioning: bool,
	/// Days a noncurrent version is kept before it expires, `0` keeping them
	/// forever. Only meaningful with
	/// [`object_versioning`](Self::object_versioning): versions accumulate
	/// silently and are billed like any other object, so a versioned bucket
	/// with no expiry is a bill that only grows.
	expire_noncurrent_days: i64,
	/// Days a CURRENT object is kept before it expires, `0` keeping it forever.
	///
	/// For a bucket whose contents are a rolling window rather than a record: a
	/// nightly database dump is worth keeping for a season and worth paying for
	/// forever by nobody. Distinct from
	/// [`expire_noncurrent_days`](Self::expire_noncurrent_days), which is about
	/// the versions an overwrite leaves behind; this is about the objects
	/// themselves, so it is `0` by default and a bucket that sets it is saying
	/// its contents are disposable.
	expire_days: i64,
	/// Expiries scoped to a key PREFIX, for a bucket holding datasets whose
	/// retention differs: an archive bucket keeps a sole-copy export forever
	/// while expiring nightly database dumps after a season, which
	/// [`expire_days`](Self::expire_days) (the whole bucket, no filter) cannot
	/// say. Each entry renders one filtered rule alongside the unfiltered ones.
	expire_prefixes: Vec<PrefixExpiry>,
	/// The class every object is kept in, see [`S3StorageClass`]. Declared
	/// once here and read both ways: the deploy renders a day-zero transition
	/// (or the class's minimum, for the infrequent-access classes) that moves
	/// every object already in the bucket in place, no re-upload, and a
	/// [`DirSync`] push lands new objects there directly. Current versions
	/// only: a noncurrent version expiring inside the class's minimum duration
	/// would be billed for the whole of it, more than its days at Standard.
	storage_class: S3StorageClass,
	/// The deploy layer for the bucket and its public-read pair
	/// ([`Config::STORAGE_LAYER`](terra::Config::STORAGE_LAYER) by default):
	/// the deploy syncs content into the bucket, so it converges before anything
	/// that reads it rolls.
	layer: SmolStr,
}

impl Default for S3BucketBlock {
	fn default() -> Self { Self::new("") }
}

impl S3BucketBlock {
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			output: true,
			force_destroy: true,
			deploy_versioned: true,
			runtime_write: false,
			public_read: false,
			object_versioning: false,
			expire_noncurrent_days: 90,
			expire_days: 0,
			expire_prefixes: Vec::new(),
			storage_class: S3StorageClass::Standard,
			layer: terra::Config::STORAGE_LAYER.into(),
		}
	}

	pub fn output_label(&self) -> String { format!("{}_bucket", self.label) }

	/// The [`AccessGrant::kind`] a bucket declares, this block's own constant so
	/// a compute lowering it never shares a vocabulary with another provider's.
	pub const ACCESS_KIND: &'static str = "s3_bucket";

	/// Returns the composed bucket name, ie `beet-site--prod--analytics`.
	pub fn bucket_name(&self, stack: &ResolvedStack) -> String {
		stack.resource_name(self.label.clone())
	}
}

impl StoreBlock for S3BucketBlock {
	/// The composed bucket name pinned to the stack's declared region, so
	/// every reader of the uri (the process a deploy bakes it into, the sync,
	/// the ledger) addresses the bucket the deploy created.
	fn store_uri(&self, stack: &ResolvedStack) -> Result<StoreUri> {
		StoreUri::S3 {
			name: self.bucket_name(stack).into(),
			path_prefix: None,
			endpoint: None,
			region: Some(stack.aws_region()?.clone()),
		}
		.xok()
	}

	fn deploy_versioned(&self) -> bool { self.deploy_versioned }

	fn storage_class(&self) -> Option<S3StorageClass> {
		self.storage_class.declared()
	}
}

impl Block for S3BucketBlock {
	fn label(&self) -> &SmolStr { &self.label }

	/// A deployed process reads the buckets declared alongside it (its site
	/// store, its assets); the deploy itself is what writes them. A bucket the
	/// process stores into declares [`runtime_write`](Self::with_runtime_write).
	fn grants(&self, stack: &ResolvedStack) -> Vec<AccessGrant> {
		let name = self.bucket_name(stack);
		vec![match self.runtime_write {
			true => AccessGrant::read_write(Self::ACCESS_KIND, name),
			false => AccessGrant::read(Self::ACCESS_KIND, name),
		}]
	}
}

impl EmitBlock for S3BucketBlock {
	fn emit(
		&self,
		stack: &ResolvedStack,
		_deployment: &Deployment,
		config: &mut terra::Config,
	) -> Result {
		self.emit(stack, config)
	}
}

impl S3BucketBlock {
	/// Emit this bucket's resources: the bucket, its optional output, and the
	/// public-read / versioning secondaries.
	fn emit(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result {
		let bucket = ResourceDef::new_primary(
			stack.resource_ident(self.label.clone()),
			AwsS3BucketDetails {
				force_destroy: Some(self.force_destroy),
				region: Some(stack.aws_region()?.clone()),
				..default()
			},
		);
		// the destination of a deploy's content sync, so it converges in the
		// layer applied ahead of the sync
		config.add_layer_resource(self.layer.clone(), &bucket)?;
		if self.output {
			config.add_output(self.output_label(), terra::Output {
				value: bucket.field_ref("bucket").into(),
				description: Some(
					format!("The bucket name for {}", self.label).into(),
				),
				sensitive: None,
			})?;
		}
		if self.public_read {
			self.emit_public_read(stack, config, &bucket)?;
		}
		if self.object_versioning {
			self.emit_versioning(stack, config, &bucket)?;
		}
		self.emit_lifecycle(stack, config, &bucket)?;
		Ok(())
	}
}

impl S3BucketBlock {
	/// The resource label of this bucket's versioning configuration, which its
	/// lifecycle rules depend on.
	fn versioning_label(&self, stack: &ResolvedStack) -> String {
		stack
			.resource_ident(format!("{}-versioning", self.label))
			.label()
			.to_string()
	}

	/// Emit object versioning, an UNTYPED resource: `aws_s3_bucket_versioning`
	/// has no generated binding, and the inline `versioning` argument the
	/// `aws_s3_bucket` schema still carries is unconfigurable from provider 4 on.
	fn emit_versioning(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		bucket: &ResourceDef<AwsS3BucketDetails>,
	) -> Result {
		config.add_untyped_resource(
			"aws_s3_bucket_versioning",
			&self.versioning_label(stack),
			&json!({
				"bucket": bucket.field_ref("id"),
				"versioning_configuration": { "status": "Enabled" },
			}),
		)?;
		Ok(())
	}

	/// Emit the one lifecycle configuration holding the class transition and
	/// every declared expiry, or nothing when the bucket declares none. Also
	/// UNTYPED, and one resource for all rules, since two configurations on one
	/// bucket would fight for the same address.
	///
	/// Rules are emitted transition first, then the expiries unfiltered-first
	/// then by prefix, in declaration order, with the noncurrent-version sweep
	/// last; each carries an id derived from what it does rather than from its
	/// position.
	fn emit_lifecycle(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		bucket: &ResourceDef<AwsS3BucketDetails>,
	) -> Result {
		// one rule per expiry, since they answer different questions and a
		// bucket may want any of them alone
		let mut rules = Vec::new();
		// the class moves objects in place, so declaring it on a bucket already
		// holding objects migrates them without a re-upload
		if let Some(class) = self.storage_class.declared() {
			rules.push(json!({
				"id": "transition-objects",
				"status": "Enabled",
				"filter": {},
				"transition": {
					"days": class.min_transition_days(),
					"storage_class": class.as_str(),
				},
			}));
		}
		if self.expire_days > 0 {
			rules.push(json!({
				"id": "expire-objects",
				"status": "Enabled",
				"filter": {},
				"expiration": { "days": self.expire_days },
			}));
		}
		for prefix in &self.expire_prefixes {
			rules.push(Self::prefix_rule(prefix)?);
		}
		// versions only accumulate on a bucket that keeps them, so the sweep is
		// meaningless (and the `depends_on` below unsatisfiable) without it
		if self.object_versioning && self.expire_noncurrent_days > 0 {
			rules.push(json!({
				"id": "expire-noncurrent-versions",
				"status": "Enabled",
				// every object: provider 6 requires a filter or a prefix, and
				// an empty filter is how "all of them" is spelled.
				"filter": {},
				"noncurrent_version_expiration": {
					"noncurrent_days": self.expire_noncurrent_days
				},
			}));
		}
		if rules.is_empty() {
			return Ok(());
		}
		self.assert_unique_rule_ids(&rules)?;
		let mut resource = json!({
			"bucket": bucket.field_ref("id"),
			"rule": rules,
		});
		if self.object_versioning {
			// versioning must be on before a rule can talk about noncurrent
			// versions
			resource["depends_on"] = json!([format!(
				"aws_s3_bucket_versioning.{}",
				self.versioning_label(stack)
			)]);
		}
		config.add_untyped_resource(
			"aws_s3_bucket_lifecycle_configuration",
			stack
				.resource_ident(format!("{}-lifecycle", self.label))
				.label(),
			&resource,
		)?;
		Ok(())
	}

	/// The S3 lifecycle rule a [`PrefixExpiry`] renders as, failing the render
	/// on a declaration AWS would reject.
	fn prefix_rule(expiry: &PrefixExpiry) -> Result<serde_json::Value> {
		expiry.validate()?;
		json!({
			"id": expiry.rule_id(),
			"status": "Enabled",
			"filter": { "prefix": expiry.prefix() },
			"expiration": { "days": expiry.expire_days() },
		})
		.xok()
	}

	/// See [`PrefixExpiry::assert_unique_ids`].
	fn assert_unique_rule_ids(&self, rules: &[serde_json::Value]) -> Result {
		PrefixExpiry::assert_unique_ids(
			&self.label,
			rules.iter().filter_map(|rule| rule["id"].as_str()),
		)
	}
}

impl S3BucketBlock {
	/// Emit the public-access-block (lifting the default block on public policies)
	/// and the anonymous read bucket policy that depends on it.
	fn emit_public_read(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		bucket: &ResourceDef<AwsS3BucketDetails>,
	) -> Result {
		let public_access = ResourceDef::new_secondary(
			stack.resource_ident(format!("{}-public-access", self.label)),
			AwsS3BucketPublicAccessBlockDetails {
				bucket: bucket.field_ref("id").into(),
				block_public_acls: Some(false),
				block_public_policy: Some(false),
				ignore_public_acls: Some(false),
				restrict_public_buckets: Some(false),
				..default()
			},
		);
		let policy = ResourceDef::new_secondary(
			stack.resource_ident(format!("{}-policy", self.label)),
			AwsS3BucketPolicyDetails {
				bucket: bucket.field_ref("id").into(),
				policy: json!({
					"Version": "2012-10-17",
					"Statement": [{
						"Sid": "PublicReadGetObject",
						"Effect": "Allow",
						"Principal": "*",
						"Action": "s3:GetObject",
						"Resource": format!("{}/*", bucket.field_ref("arn"))
					}, {
						// a credential-free `sync` lists before it gets
						"Sid": "PublicListBucket",
						"Effect": "Allow",
						"Principal": "*",
						"Action": "s3:ListBucket",
						"Resource": bucket.field_ref("arn").to_string()
					}]
				})
				.to_string()
				.into(),
				// the policy is rejected until the public-access-block lifts the
				// account/bucket default block on public policies.
				depends_on: Some(vec![
					format!(
						"aws_s3_bucket_public_access_block.{}",
						public_access.ident().label()
					)
					.into(),
				]),
				..default()
			},
		);
		// the bucket's own layer: a reader that finds the bucket but not yet its
		// public policy is the same failure as finding no bucket
		config
			.add_layer_resource(self.layer.clone(), &public_access)?
			.add_layer_resource(self.layer.clone(), &policy)?;
		Ok(())
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	/// The config `block` renders, and the stack it was resolved against.
	fn build_config(block: S3BucketBlock) -> (ResolvedStack, terra::Config) {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(block);
		});
		let (stack, _deployment, config) = scope.finish().unwrap();
		(stack, config)
	}

	/// The terraform json rendered by `block`.
	fn build_json(block: S3BucketBlock) -> String {
		build_config(block).1.to_json_string().unwrap()
	}

	/// Both grants matter: `GetObject` serves the objects, `ListBucket` lets a
	/// credential-free `sync` enumerate them (it lists before it gets).
	#[beet_core::test]
	fn public_read_emits_access_block_and_policy() {
		let json =
			build_json(S3BucketBlock::new("assets").with_public_read(true));
		json.as_str()
			.xpect_contains("aws_s3_bucket_public_access_block")
			.xpect_contains("aws_s3_bucket_policy")
			.xpect_contains("s3:GetObject")
			.xpect_contains("PublicReadGetObject")
			.xpect_contains("s3:ListBucket")
			.xpect_contains("PublicListBucket");
	}

	/// The region is the STACK's now that the block's own field is an override,
	/// and the emitted value is the one the live buckets already carry: a moved
	/// region replaces every physical resource.
	#[beet_core::test]
	fn region_resolves_from_the_stack() {
		build_json(S3BucketBlock::new("app"))
			.as_str()
			.xpect_contains("\"region\":\"us-west-2\"");
		// ..and an address spread on the block itself is the nearest, so it
		// wins over its stack's
		RenderScope::test_json(|parent| {
			parent.spawn((
				S3BucketBlock::new("app"),
				AwsRegion::new("eu-west-1"),
			));
		})
		.as_str()
		.xpect_contains("\"region\":\"eu-west-1\"");
	}

	/// A bucket under a stack declaring no region fails at its erased half,
	/// naming the spread, rather than landing a store the deploy would not
	/// have created.
	#[beet_core::test]
	#[should_panic = "declares no aws region"]
	fn a_bucket_under_an_unaddressed_stack_is_loud() {
		RenderScope::test_render_stack(Stack::new("beet_infra"), |parent| {
			parent.spawn(S3BucketBlock::new("app"));
		});
	}

	/// The runtime grant defaults to read (deploy publishes, process serves) and
	/// escalates to read/write only when the block says the process stores into
	/// the bucket, so a lowering compute block can tell the two apart.
	#[beet_core::test]
	fn runtime_write_escalates_the_grant() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(S3BucketBlock::new("app"));
			parent.spawn(
				S3BucketBlock::new("mail-blobs").with_runtime_write(true),
			);
		});
		let stack = scope.stack().clone();
		scope.access().to_vec().xpect_eq(vec![
			AccessGrant::read(
				S3BucketBlock::ACCESS_KIND,
				stack.resource_name("app"),
			),
			AccessGrant::read_write(
				S3BucketBlock::ACCESS_KIND,
				stack.resource_name("mail-blobs"),
			),
		]);
	}

	/// A source of record keeps its versions, and expires the noncurrent ones so
	/// the bill does not grow forever. Both resources are separate from the
	/// bucket, since the inline arguments the `aws_s3_bucket` schema still
	/// carries have been unconfigurable since provider 4.
	/// A rolling window rather than a record: a bucket of nightly dumps expires
	/// the dumps themselves, not merely the versions an overwrite leaves. Both
	/// rules ride one configuration, since a bucket wanting both would
	/// otherwise have two resources fighting for the same address.
	#[beet_core::test]
	fn expiring_objects_is_its_own_rule() {
		let json = build_json(
			S3BucketBlock::new("archive")
				.with_object_versioning(true)
				.with_expire_days(180),
		);
		json.as_str()
			.xpect_contains("\"id\":\"expire-objects\"")
			.xpect_contains("\"days\":180")
			.xpect_contains("\"id\":\"expire-noncurrent-versions\"");
		// ..and an unversioned bucket that expires nothing declares no rule
		build_json(S3BucketBlock::new("app"))
			.as_str()
			.xnot()
			.xpect_contains("aws_s3_bucket_lifecycle_configuration");
	}

	/// An expiry is not a versioning feature: a bucket whose contents are a
	/// rolling window expires them whether or not it keeps the versions an
	/// overwrite leaves, and the noncurrent sweep is what versioning gates.
	#[beet_core::test]
	fn expiry_does_not_require_versioning() {
		build_json(S3BucketBlock::new("scratch").with_expire_days(7))
			.as_str()
			.xpect_contains("aws_s3_bucket_lifecycle_configuration")
			.xpect_contains("\"id\":\"expire-objects\"")
			.xnot()
			.xpect_contains("expire-noncurrent-versions")
			.xnot()
			.xpect_contains("aws_s3_bucket_versioning");
	}

	/// The archive profile: one bucket holding datasets whose retention
	/// differs, so the dumps under one prefix expire while the sole-copy
	/// exports beside them are kept forever. A whole-bucket `expire_days`
	/// cannot say that, and a second bucket per retention window would multiply
	/// buckets for a writer convention.
	#[beet_core::test]
	fn prefix_rules_scope_an_expiry() {
		let json = build_json(
			S3BucketBlock::new("archive")
				.with_object_versioning(true)
				.with_expire_prefixes(vec![PrefixExpiry::new("sqlite/", 180)]),
		);
		json.as_str()
			// the filtered rule, named for what it expires
			.xpect_contains("\"id\":\"expire-sqlite\"")
			.xpect_contains("\"prefix\":\"sqlite/\"")
			.xpect_contains("\"days\":180")
			// ..riding the same configuration as the noncurrent sweep
			.xpect_contains("\"id\":\"expire-noncurrent-versions\"")
			// ..and nothing expires the prefixes it did not name
			.xnot()
			.xpect_contains("\"id\":\"expire-objects\"");
	}

	/// Both rejections are render-time, ie before any tofu invocation: AWS
	/// rejects a zero-day expiry and two rules sharing an id, and a deploy is a
	/// slow place to learn either.
	#[beet_core::test]
	fn a_prefix_rule_that_says_nothing_is_loud() {
		RenderScope::test_render(|parent| {
			parent.spawn(
				S3BucketBlock::new("archive").with_expire_prefixes(vec![
					PrefixExpiry::new("sqlite/", 0),
				]),
			);
		})
		.0
		.finish()
		.unwrap_err()
		.to_string()
		.xpect_contains("expires nothing");
		// ..and two prefixes sanitizing to one id would collide at apply time
		RenderScope::test_render(|parent| {
			parent.spawn(S3BucketBlock::new("archive").with_expire_prefixes(
				vec![
					PrefixExpiry::new("logs/", 30),
					PrefixExpiry::new("logs-", 30),
				],
			));
		})
		.0
		.finish()
		.unwrap_err()
		.to_string()
		.xpect_contains("expire-logs");
	}

	#[beet_core::test]
	fn object_versioning_emits_versioning_and_lifecycle() {
		let json = build_json(
			S3BucketBlock::new("mail-blobs").with_object_versioning(true),
		);
		json.as_str()
			.xpect_contains("aws_s3_bucket_versioning")
			.xpect_contains("\"status\":\"Enabled\"")
			.xpect_contains("aws_s3_bucket_lifecycle_configuration")
			.xpect_contains("\"noncurrent_days\":90");
		// ..and neither is emitted for a bucket the deploy simply re-publishes
		build_json(S3BucketBlock::new("app"))
			.as_str()
			.xnot()
			.xpect_contains("aws_s3_bucket_versioning");
	}

	/// Keeping versions forever is expressible, since a compliance copy is a
	/// real requirement; it just is not the default.
	#[beet_core::test]
	fn zero_days_keeps_every_version() {
		build_json(
			S3BucketBlock::new("archive")
				.with_object_versioning(true)
				.with_expire_noncurrent_days(0),
		)
		.as_str()
		.xpect_contains("aws_s3_bucket_versioning")
		.xnot()
		.xpect_contains("aws_s3_bucket_lifecycle_configuration");
	}

	/// A class declared on the bucket is a day-zero transition rule, so the
	/// objects already in it move in place rather than by re-upload, riding
	/// the same configuration as the expiries; the infrequent-access classes
	/// wait the 30 days AWS requires at Standard first.
	#[beet_core::test]
	fn a_storage_class_is_a_transition_rule() {
		let json = build_json(
			S3BucketBlock::new("archive")
				.with_object_versioning(true)
				.with_storage_class(S3StorageClass::GlacierIr),
		);
		json.as_str()
			.xpect_contains("\"id\":\"transition-objects\"")
			.xpect_contains("\"storage_class\":\"GLACIER_IR\"")
			.xpect_contains("\"days\":0")
			.xpect_contains("\"id\":\"expire-noncurrent-versions\"");
		build_json(
			S3BucketBlock::new("archive")
				.with_storage_class(S3StorageClass::StandardIa),
		)
		.as_str()
		.xpect_contains("\"storage_class\":\"STANDARD_IA\"")
		.xpect_contains("\"days\":30");
		// ..and the default class is the bucket's own, needing no rule
		build_json(S3BucketBlock::new("app"))
			.as_str()
			.xnot()
			.xpect_contains("transition-objects");
	}

	#[beet_core::test]
	fn private_by_default() {
		build_json(S3BucketBlock::new("site"))
			.as_str()
			.xnot()
			.xpect_contains("aws_s3_bucket_policy");
	}

	/// A bucket and the resources that make it readable default to the `storage`
	/// layer (the deploy syncs content into it, so it converges before anything
	/// that reads it rolls). The address is the `type.label` form `-target` takes.
	#[beet_core::test]
	fn declares_storage_layer() {
		let (stack, config) = build_config(S3BucketBlock::new("app"));
		config
			.layer_targets("storage")
			.unwrap()
			.to_vec()
			.xpect_eq(vec![format!(
				"aws_s3_bucket.{}",
				stack.resource_ident("app").label()
			)]);
		// the bucket, plus its public-access block and policy
		build_config(S3BucketBlock::new("assets").with_public_read(true))
			.1
			.layer_targets("storage")
			.unwrap()
			.len()
			.xpect_eq(3);
	}

	/// The layer assignment is a field, so a route can re-cut its layers, and an
	/// undeclared layer is a loud error naming the declared ones: a typo that
	/// silently converged nothing would race exactly as an unordered deploy does.
	#[beet_core::test]
	fn layer_is_overridable_and_typos_are_loud() {
		let (_stack, config) =
			build_config(S3BucketBlock::new("app").with_layer("data"));
		config.layer_targets("data").unwrap().len().xpect_eq(1);
		config
			.layer_targets("storage")
			.unwrap_err()
			.to_string()
			.xpect_contains("no resources declare layer 'storage'")
			.xpect_contains("data");
	}
}
