use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::json;

/// An S3 bucket, declared once and read by both meanings of the declaration:
/// the deploy creates it, and (with the `aws_sdk` backend) the runtime attaches
/// an [`S3Store`](beet_net::prelude::S3Store) for it on the same entity.
///
/// Authored directly from markup, ie `<S3BucketBlock label="app"
/// deploy_versioned=false/>`. The label alone is declared; the
/// `<app>--<stage>--<label>` name composes at resolution through the ancestor
/// [`Stack`], so both sides read the same string.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_add = ErasedBlock::on_add::<S3BucketBlock>)]
pub struct S3BucketBlock {
	label: SmolStr,
	/// Override the region this bucket lives in, which otherwise resolves from
	/// the ancestor [`Stack`].
	#[set_with(unwrap_option, into)]
	region: Option<SmolStr>,
	/// add a tofu output for the bucket name
	output: bool,
	/// Allow the deploy to delete a non-empty bucket. `false` for a source of
	/// record, whose accidental teardown is unrecoverable.
	force_destroy: bool,
	/// All objects will be nested under the deploy uuid,
	/// ensuring unique files per deploy
	deploy_versioned: bool,
	/// Grant anonymous `s3:GetObject` on every object and `s3:ListBucket` on the
	/// bucket (via a public-access-block that lifts the default block, plus a
	/// bucket policy). Needed when objects are served by a 301 to the public S3
	/// url, and when a credential-free `sync` hydrates a checkout from the
	/// bucket: `aws s3 sync --no-sign-request` lists before it gets, so
	/// `GetObject` alone fails at the first `ListObjectsV2`.
	public_read: bool,
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
			region: None,
			output: true,
			force_destroy: true,
			deploy_versioned: true,
			public_read: false,
			layer: terra::Config::STORAGE_LAYER.into(),
		}
	}

	pub fn output_label(&self) -> String { format!("{}_bucket", self.label) }

	/// The region this bucket lives in: its own override, else `stack`'s.
	pub fn resolved_region(&self, stack: &ResolvedStack) -> SmolStr {
		self.region
			.clone()
			.unwrap_or_else(|| stack.region().clone())
	}

	/// The [`S3Store`](beet_net::prelude::S3Store) for this bucket, resolved
	/// against `stack` (the composed bucket name and the region). A
	/// deploy-versioned bucket also nests under the deploy id, which only a
	/// [`Deployment`] carries, so pass one through when there is one.
	#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
	pub fn store(
		&self,
		stack: &ResolvedStack,
		deploy_id: Option<&Uuid>,
	) -> beet_net::prelude::S3Store {
		let store = beet_net::prelude::S3Store::new(
			stack.resource_name(self.label.clone()),
			self.resolved_region(stack),
		);
		match (self.deploy_versioned, deploy_id) {
			(true, Some(deploy_id)) => {
				store.with_subdir(SmolPath::new(deploy_id.to_string()))
			}
			_ => store,
		}
	}

	/// The store for this bucket as a deploy declares it, ie including the
	/// per-deploy subdir a versioned bucket nests under.
	#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
	pub fn stack_store(
		&self,
		stack: &ResolvedStack,
		deployment: &Deployment,
	) -> beet_net::prelude::S3Store {
		self.store(stack, Some(deployment.deploy_id()))
	}
}

/// Observer: attach the runtime meaning of a declared bucket, the
/// [`S3Store`](beet_net::prelude::S3Store) (which in turn inserts a
/// [`BlobStore`]) for the name the deploy creates. Registered by [`InfraPlugin`]
/// rather than hooked on the component, so a build without the SDK carries the
/// declaration and nothing else.
///
/// Deferred through the command queue because the ancestry a scope resolves
/// against lands with the rest of the scene, after this insertion.
#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
pub(crate) fn attach_s3_store(
	ev: On<Add, S3BucketBlock>,
	mut commands: Commands,
) {
	commands
		.entity(ev.entity)
		.queue(|mut entity: EntityWorldMut| -> Result {
			let block = entity.get_or_else::<S3BucketBlock>()?.clone();
			let store = entity.with_state::<StackQuery, _>(|entity, stacks| {
				let deploy_id = stacks.deployment().deploy_id().clone();
				block.store(&stacks.resolve(entity), Some(&deploy_id))
			});
			entity.insert(store);
			Ok(())
		});
}

impl Block for S3BucketBlock {
	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &ResolvedStack,
		_deployment: &Deployment,
		_access: &AccessGrants,
		config: &mut terra::Config,
	) -> Result {
		let bucket = ResourceDef::new_primary(
			stack.resource_ident(self.label.clone()),
			AwsS3BucketDetails {
				force_destroy: Some(self.force_destroy),
				region: Some(self.resolved_region(stack)),
				..default()
			},
		);
		// the destination of a deploy's content sync, so it converges in the
		// layer applied ahead of the sync
		config.add_layer_resource(self.layer.clone(), &bucket)?;
		if self.output {
			config.add_output(self.output_label(), terra::Output {
				value: json!(bucket.field_ref("bucket")),
				description: Some(
					format!("The bucket name for {}", self.label).into(),
				),
				sensitive: None,
			})?;
		}
		if self.public_read {
			self.emit_public_read(stack, config, &bucket)?;
		}
		Ok(())
	}

	/// A deployed process reads the buckets declared alongside it (its site
	/// store, its assets); the deploy itself is what writes them.
	fn runtime_access(&self, stack: &ResolvedStack) -> Vec<AccessGrant> {
		vec![AccessGrant::read(AccessResource::S3Bucket {
			name: stack.resource_name(self.label.clone()),
		})]
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

	/// The config `block` emits, and the throwaway stack it was resolved against.
	fn build_config(block: S3BucketBlock) -> (ResolvedStack, terra::Config) {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&deployment,
				&default(),
				&mut config,
			)
			.unwrap();
		(stack, config)
	}

	/// The terraform json emitted by `block`.
	fn build_json(block: S3BucketBlock) -> String {
		build_config(block).1.to_json().to_string()
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
		// ..and an override on the block wins over its stack
		build_json(S3BucketBlock::new("app").with_region("eu-west-1"))
			.as_str()
			.xpect_contains("\"region\":\"eu-west-1\"");
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
