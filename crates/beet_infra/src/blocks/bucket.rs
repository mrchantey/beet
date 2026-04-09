use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;





#[derive(Debug, Clone, Serialize, Deserialize, Component)]
#[require(ErasedBlock=ErasedBlock::new::<Self>())]
pub struct BucketBlock {
	label: SmolStr,
	details: BucketDetails,
	output: bool,
}

impl BucketBlock {
	pub fn new(label: SmolStr, details: impl Into<BucketDetails>) -> Self {
		Self {
			label,
			details: details.into(),
			output: true,
		}
	}
	pub fn with_output(mut self, output: bool) -> Self {
		self.output = output;
		self
	}

	pub fn output_label(&self) -> String { format!("{}_bucket", self.label) }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BucketDetails {
	Aws(AwsS3BucketDetails),
}
impl Into<BucketDetails> for AwsS3BucketDetails {
	fn into(self) -> BucketDetails { BucketDetails::Aws(self) }
}

impl Block for BucketBlock {
	fn apply_to_config(
		&self,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		match &self.details {
			BucketDetails::Aws(details) => {
				let bucket = ResourceDef::new_primary(
					stack.resource_ident(self.label.clone()),
					details.clone(),
				);
				config.add_resource(&bucket)?;
				if self.output {
					config.add_output(self.output_label(), terra::Output {
						value: json!(bucket.field_ref("bucket")),
						description: Some(
							format!("The bucket name for {}", self.label)
								.into(),
						),
						sensitive: None,
					})?;
				}
			}
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
