//! A DynamoDB table deploy block, for the analytics store's remote backend.
//!
//! Uses the generated [`AwsDynamodbTableDetails`] binding (in `bindings/aws_common.rs`).
use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;

/// A DynamoDB table with a single string hash key, provisioned pay-per-request.
///
/// Mirrors [`S3BucketBlock`](crate::prelude::S3BucketBlock): its `label` becomes
/// the stage-prefixed table name (`stack.resource_ident(label)`), and the deploy
/// hands that name to the running binary via an env var, so the two agree on the
/// name without deriving it independently.
#[derive(Debug, Clone, Serialize, Deserialize, Component)]
#[component(immutable, on_add = on_add_dynamo_table_block)]
pub struct DynamoTableBlock {
	/// The unprefixed table label (eg `analytics`).
	label: SmolStr,
	/// The hash (partition) key attribute name.
	hash_key: SmolStr,
}

impl DynamoTableBlock {
	/// A table `label` keyed by a string `id` hash key (the [`TableStoreRow`]
	/// primary key the analytics store writes).
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			hash_key: "id".into(),
		}
	}

	/// The stage-prefixed table name this block creates, resolved against `stack`.
	pub fn table_name(&self, stack: &Stack) -> String {
		stack
			.resource_ident(self.label.clone())
			.primary_identifier()
			.to_string()
	}
}

/// Inserts the [`ErasedBlock`] so the deploy config collects this table.
fn on_add_dynamo_table_block(mut world: DeferredWorld, cx: HookContext) {
	ErasedBlock::on_add::<DynamoTableBlock>(world.reborrow(), cx);
}

impl Block for DynamoTableBlock {
	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		config: &mut terra::Config,
	) -> Result {
		let table = ResourceDef::new_primary(
			stack.resource_ident(self.label.clone()),
			AwsDynamodbTableDetails {
				billing_mode: Some("PAY_PER_REQUEST".into()),
				hash_key: Some(self.hash_key.clone()),
				attribute: Some(vec![
					AwsDynamodbTableResourceBlockTypeAttribute {
						name: self.hash_key.clone(),
						r#type: "S".into(),
					},
				]),
				region: Some(stack.aws_region().clone()),
				..default()
			},
		);
		config.add_resource(&table)?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The block emits an `aws_dynamodb_table` with a stage-prefixed name, an `id`
	/// string hash key, and pay-per-request billing.
	#[beet_core::test]
	fn emits_dynamodb_table() {
		let (stack, _dir) = Stack::default_local();
		let mut config = stack.create_config();
		let mut world = World::new();
		DynamoTableBlock::new("analytics")
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&mut config,
			)
			.unwrap();
		config
			.to_json()
			.to_string()
			.as_str()
			.xpect_contains("aws_dynamodb_table")
			.xpect_contains("PAY_PER_REQUEST")
			.xpect_contains("analytics")
			.xpect_contains("hash_key")
			.xpect_contains("\"id\"");
	}
}
