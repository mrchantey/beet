use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;





#[derive(Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Component)]
#[require(ErasedBlock=ErasedBlock::new::<Self>())]
pub struct S3BucketBlock {
	label: SmolStr,
	#[deref]
	details: AwsS3BucketDetails,
	/// add a tofu output for the bucket name
	output: bool,
	/// apply the stack default region if none set
	apply_region: bool,
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
		}
	}
	pub fn with_output(mut self, output: bool) -> Self {
		self.output = output;
		self
	}

	pub fn output_label(&self) -> String { format!("{}_bucket", self.label) }

	pub fn provider(&self, stack: &Stack) -> beet_net::prelude::S3Bucket {
		cfg_if! {
			if #[cfg(feature = "aws")] {
				let region = self.region.as_ref().unwrap_or(stack.aws_region());
				let bucket_name = stack.resource_ident(self.label.clone());
				beet_net::prelude::S3Bucket::new(
					bucket_name.primary_identifier(),
					region.clone(),
				)
			} else {
				let _ = stack;
				panic!("the `aws` feature is required for S3 bucket providers")
			}
		}
	}
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BucketDetails {
	Aws(AwsS3BucketDetails),
}
impl Into<BucketDetails> for AwsS3BucketDetails {
	fn into(self) -> BucketDetails { BucketDetails::Aws(self) }
}

impl Block for S3BucketBlock {
	fn apply_to_config(
		&self,
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
		Ok(())
	}
}
#[cfg(test)]
mod tests {
	use super::*;

	#[beet_core::test(timeout_ms = 120000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (stack, _dir) = Stack::default_local();
		let block = LambdaBlock::default();
		let mut config = stack.create_config();
		block.apply_to_config(&stack, &mut config).unwrap();
		let project = terra::Project::new(&stack, config);
		project.validate().await.unwrap();
	}
}
