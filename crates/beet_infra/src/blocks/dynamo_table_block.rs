//! A DynamoDB table deploy block, for the analytics store's remote backend.
//!
//! Uses the generated [`AwsDynamodbTableDetails`] binding (in `bindings/aws_dynamo.rs`).
use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A DynamoDB table with a single string hash key, provisioned pay-per-request.
///
/// Mirrors [`S3BucketBlock`](crate::prelude::S3BucketBlock): the declaration
/// carries only its `label`, and the `<app>--<stage>--<label>` name composes at
/// resolution through the ancestor [`Stack`]. The deploy creates the table from
/// that name and the runtime attaches a store for the same name off the same
/// entity, so there is one declaration and nothing to keep in agreement.
///
/// Authored directly from markup, ie
/// `<DynamoTableBlock bx:ref="analytics" label="analytics"/>`.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_add = ErasedBlock::on_add::<DynamoTableBlock>)]
pub struct DynamoTableBlock {
	/// The unprefixed table label (eg `analytics`).
	label: SmolStr,
	/// The hash (partition) key attribute name.
	hash_key: SmolStr,
	/// Override the region this table lives in, which otherwise resolves from
	/// the ancestor [`Stack`]. The runtime store and the tofu resource read the
	/// one resolved value rather than the runtime falling back to an environment
	/// the deploy never saw.
	#[set_with(unwrap_option, into)]
	region: Option<SmolStr>,
	/// The deploy layer ([`Config::STORAGE_LAYER`](terra::Config::STORAGE_LAYER)
	/// by default): the runtime writes to this table from its first request, and
	/// nothing in the tofu graph orders the table before the service (the name
	/// crosses to the task env as a literal, not a field ref), so the layer is
	/// what makes it exist before the service that names it rolls.
	layer: SmolStr,
}

impl Default for DynamoTableBlock {
	fn default() -> Self { Self::new("") }
}

impl DynamoTableBlock {
	/// A table `label` keyed by a string `id` hash key (the [`TableStoreRow`]
	/// primary key the analytics store writes).
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
			hash_key: "id".into(),
			region: None,
			layer: terra::Config::STORAGE_LAYER.into(),
		}
	}

	/// The composed table name this block declares, ie `beet-site--prod--analytics`.
	pub fn table_name(&self, stack: &Stack) -> String {
		stack.resource_name(self.label.clone())
	}

	/// The region this table lives in: its own override, else `stack`'s.
	pub fn resolved_region(&self, stack: &Stack) -> SmolStr {
		self.region.clone().unwrap_or_else(|| stack.region())
	}
}

/// Observer: attach the runtime meaning of a declared table, a store provider
/// materializing the [`TableStore`] a consumer reaches through
/// [`StoreRef`]. Registered by [`InfraPlugin`] rather than hooked on the
/// component, so a build without a backend carries the declaration and nothing
/// else.
///
/// [`ServiceAccess::Remote`] (a deployed process) resolves the DynamoDB table
/// the deploy created; [`ServiceAccess::Local`] backs the same declaration with
/// a workspace directory, so one markup declaration runs both ways.
///
/// Deferred through the command queue because the ancestry a scope resolves
/// against lands with the rest of the scene, after this insertion.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn attach_table_store(
	ev: On<Add, DynamoTableBlock>,
	mut commands: Commands,
) {
	commands.entity(ev.entity).queue(
		|mut entity: EntityWorldMut| -> Result {
			let block = entity.get_or_else::<DynamoTableBlock>()?.clone();
			let stack = entity
				.with_state::<StackQuery, _>(|entity, stacks| {
					stacks.resolve(entity)
				});
			match BootstrapConfig::get().service_access {
				ServiceAccess::Remote => {
					cfg_if! {
						if #[cfg(feature = "aws_sdk")] {
							entity.insert(beet_net::prelude::DynamoStore::new(
								block.table_name(&stack),
								block.resolved_region(&stack),
							));
						} else {
							bevybail!(
								"the table declared as `{}` resolves to the remote `{}`, but this binary has no `aws_sdk` backend to reach it",
								block.label(),
								block.table_name(&stack)
							);
						}
					}
				}
				ServiceAccess::Local => {
					entity.insert(FsStore::new(
						ServiceAccess::local_store_dir(block.label().as_str())
							.into_abs(),
					));
				}
			}
			Ok(())
		},
	);
}

impl Block for DynamoTableBlock {
	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &Stack,
		_deployment: &Deployment,
		_access: &AccessGrants,
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
				region: Some(self.resolved_region(stack)),
				..default()
			},
		);
		// see the `layer` field: nothing else orders the table before the service
		config.add_layer_resource(self.layer.clone(), &table)?;
		Ok(())
	}

	/// A table is declared to be recorded to, so the process that declared it
	/// reads and writes it.
	fn runtime_access(&self, stack: &Stack) -> Vec<AccessGrant> {
		vec![AccessGrant::read_write(AccessResource::DynamoTable {
			name: self.table_name(stack),
			region: self.resolved_region(stack),
		})]
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The block emits an `aws_dynamodb_table` with a stage-prefixed name, an `id`
	/// string hash key, and pay-per-request billing.
	#[beet_core::test]
	fn emits_dynamodb_table() {
		let (stack, deployment, _dir) = Stack::default_local();
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		DynamoTableBlock::new("analytics")
			.apply_to_config(
				&world.spawn(()).as_readonly(),
				&stack,
				&deployment,
				&default(),
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
