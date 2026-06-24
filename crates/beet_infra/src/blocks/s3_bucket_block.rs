use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::json;

#[derive(
	Debug,
	Clone,
	Get,
	Deref,
	DerefMut,
	SetWith,
	Serialize,
	Deserialize,
	Component,
)]
#[component(immutable, on_add = on_add_s3_bucket_block)]
pub struct S3BucketBlock {
	label: SmolStr,
	#[deref]
	details: AwsS3BucketDetails,
	/// add a tofu output for the bucket name
	output: bool,
	/// apply the stack default region if none set
	apply_region: bool,
	/// All objects will be nested under the deploy uuid,
	/// ensuring unique files per deploy
	deploy_versioned: bool,
	/// Grant anonymous `s3:GetObject` on every object (via a public-access-block
	/// that lifts the default block, plus a bucket policy). Needed when objects
	/// are served by a 301 to the public S3 url, eg the assets bucket.
	public_read: bool,
}

impl S3BucketBlock {
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			details: AwsS3BucketDetails {
				force_destroy: Some(true),
				..default()
			},
			apply_region: true,
			output: true,
			deploy_versioned: true,
			public_read: false,
		}
	}

	pub fn output_label(&self) -> String { format!("{}_bucket", self.label) }

	/// The [`S3Store`](beet_net::prelude::S3Store) for this bucket, resolved
	/// against `stack` (bucket name, region, and deploy-versioned subdir).
	#[cfg(feature = "aws_sdk")]
	pub fn store(&self, stack: &Stack) -> beet_net::prelude::S3Store {
		let region = self.region.as_ref().unwrap_or(stack.aws_region());
		let bucket_name = stack.resource_ident(self.label.clone());
		let mut store = beet_net::prelude::S3Store::new(
			bucket_name.primary_identifier().clone(),
			region.clone(),
		);
		if self.deploy_versioned {
			store =
				store.with_subdir(SmolPath::new(stack.deploy_id().to_string()));
		}
		store
	}
}

/// Inserts an [`ErasedBlock`] and, when the `aws_sdk` feature is enabled,
/// also inserts an [`S3Store`] (which in turn inserts a [`BlobStore`]).
fn on_add_s3_bucket_block(mut world: DeferredWorld, cx: HookContext) {
	// always insert ErasedBlock
	ErasedBlock::on_add::<S3BucketBlock>(world.reborrow(), cx);

	// when aws_sdk is available, insert S3Store
	#[cfg(feature = "aws_sdk")]
	{
		world.commands().entity(cx.entity).queue(
			move |mut entity: EntityWorldMut| -> Result {
				if let Ok(stack) = entity
					.with_state::<AncestorQuery<&Stack>, _>(|entity, query| {
						query.get(entity).cloned()
					}) {
					let block = entity.get_or_else::<S3BucketBlock>()?;
					let s3_store = block.store(&stack);
					entity.insert(s3_store);
				}
				Ok(())
			},
		);
	}
}

impl Block for S3BucketBlock {
	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		let mut details = self.details.clone();
		if self.apply_region && details.region.is_none() {
			details.region = Some(stack.aws_region().clone());
		}
		let bucket = ResourceDef::new_primary(
			stack.resource_ident(self.label.clone()),
			details,
		);
		config.add_resource(&bucket)?;
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
}

impl S3BucketBlock {
	/// Emit the public-access-block (lifting the default block on public policies)
	/// and the anonymous `s3:GetObject` bucket policy that depends on it.
	fn emit_public_read(
		&self,
		stack: &Stack,
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
		config.add_resource(&public_access)?.add_resource(&policy)?;
		Ok(())
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	/// The terraform json emitted by `block`.
	fn build_json(block: S3BucketBlock) -> String {
		let (stack, _dir) = Stack::default_local();
		let mut config = stack.create_config();
		let mut world = World::new();
		block
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&mut config,
			)
			.unwrap();
		config.to_json().to_string()
	}

	#[beet_core::test]
	fn public_read_emits_access_block_and_policy() {
		let json =
			build_json(S3BucketBlock::new("assets").with_public_read(true));
		json.as_str()
			.xpect_contains("aws_s3_bucket_public_access_block")
			.xpect_contains("aws_s3_bucket_policy")
			.xpect_contains("s3:GetObject")
			.xpect_contains("PublicReadGetObject");
	}

	#[beet_core::test]
	fn private_by_default() {
		build_json(S3BucketBlock::new("site"))
			.as_str()
			.xnot()
			.xpect_contains("aws_s3_bucket_policy");
	}
}
