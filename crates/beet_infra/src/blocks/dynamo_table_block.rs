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
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>)]
pub struct DynamoTableBlock {
	/// The unprefixed table label (eg `analytics`).
	label: SmolStr,
	/// The hash (partition) key attribute name.
	hash_key: SmolStr,
	/// The attribute DynamoDB reads a row's expiry from, ie `ttl`. Unset (the
	/// default) keeps every row forever.
	///
	/// A row whose attribute holds a unix SECOND in the past is deleted, free,
	/// within a couple of days; a row without the attribute is never touched, so
	/// enabling this expires nothing on its own — the writer decides what
	/// carries a stamp, and for analytics nothing is stamped until its cold
	/// archive and its daily aggregate both exist.
	///
	/// Turning it on is an in-place update to a live table: it is a sub-resource
	/// of the table rather than one of the fields (the name, the key schema) a
	/// change to would replace it.
	#[set_with(unwrap_option, into)]
	ttl: Option<SmolStr>,
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
			ttl: None,
			region: None,
			layer: terra::Config::STORAGE_LAYER.into(),
		}
	}

	/// The [`AccessGrant::kind`] a table declares, this block's own constant so
	/// a compute lowering it never shares a vocabulary with another provider's.
	pub const ACCESS_KIND: &'static str = "dynamo_table";

	/// The composed table name this block declares, ie `beet-site--prod--analytics`.
	pub fn table_name(&self, stack: &ResolvedStack) -> String {
		stack.resource_name(self.label.clone())
	}

	/// The region this table lives in: its own override, else `stack`'s.
	pub fn resolved_region(&self, stack: &ResolvedStack) -> SmolStr {
		self.region
			.clone()
			.unwrap_or_else(|| stack.region().clone())
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
	fn label(&self) -> &SmolStr { &self.label }

	/// A table is declared to be recorded to, so the process that declared it
	/// reads and writes it.
	fn grants(&self, stack: &ResolvedStack) -> Vec<AccessGrant> {
		vec![AccessGrant::read_write(
			Self::ACCESS_KIND,
			self.table_name(stack),
		)]
	}
}

impl EmitBlock for DynamoTableBlock {
	fn emit(
		&self,
		stack: &ResolvedStack,
		_deployment: &Deployment,
		config: &mut terra::Config,
	) -> Result {
		self.emit(stack, config)
	}
}

impl DynamoTableBlock {
	/// Emit the pay-per-request table, with its ttl sub-resource when declared.
	fn emit(
		&self,
		stack: &ResolvedStack,
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
				// absent unless declared, so a table that expires nothing
				// renders exactly as it did before this field existed
				ttl: self.ttl.clone().map(|attribute_name| {
					vec![AwsDynamodbTableResourceBlockTypeTtl {
						attribute_name: Some(attribute_name),
						enabled: Some(true),
					}]
				}),
				..default()
			},
		);
		// see the `layer` field: nothing else orders the table before the service
		config.add_layer_resource(self.layer.clone(), &table)?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The terraform json `block` emits.
	fn build_json(block: DynamoTableBlock) -> String {
		RenderScope::test_json(|parent| {
			parent.spawn(block);
		})
	}

	/// Enabling expiry is an IN-PLACE update to a live table: the ttl block is
	/// the only difference in the rendered resource, and none of the fields a
	/// change to would replace the table (its name, its key schema, its
	/// attributes, its billing mode) moves.
	///
	/// The live analytics table holds the only copy of anything not yet
	/// archived, so a replacement here is data loss on a green deploy.
	#[beet_core::test]
	fn enabling_ttl_is_an_in_place_change() {
		let before = build_json(DynamoTableBlock::new("analytics"));
		let after =
			build_json(DynamoTableBlock::new("analytics").with_ttl("ttl"));
		before
			.as_str()
			.xnot()
			.xpect_contains("ttl")
			.xpect_contains("PAY_PER_REQUEST");
		after.as_str().xpect_contains(
			r#""ttl":[{"attribute_name":"ttl","enabled":true}]"#,
		);
		// everything else is byte-identical: removing the ttl block from the
		// rendered json gives back exactly the config that is already deployed
		after
			.replace(r#","ttl":[{"attribute_name":"ttl","enabled":true}]"#, "")
			.xpect_eq(before);
	}

	/// The declaration contributes its read/write grant through the schedule's
	/// declare pass, so a compute lowering the pool sees the table.
	#[beet_core::test]
	fn declares_a_read_write_grant() {
		let (scope, _dir) = RenderScope::test_render(|parent| {
			parent.spawn(DynamoTableBlock::new("analytics"));
		});
		let stack = scope.stack().clone();
		scope
			.access()
			.to_vec()
			.xpect_eq(vec![AccessGrant::read_write(
				DynamoTableBlock::ACCESS_KIND,
				stack.resource_name("analytics"),
			)]);
	}

	/// The block emits an `aws_dynamodb_table` with a stage-prefixed name, an `id`
	/// string hash key, and pay-per-request billing.
	#[beet_core::test]
	fn emits_dynamodb_table() {
		build_json(DynamoTableBlock::new("analytics"))
			.as_str()
			.xpect_contains("aws_dynamodb_table")
			.xpect_contains("PAY_PER_REQUEST")
			.xpect_contains("analytics")
			.xpect_contains("hash_key")
			.xpect_contains("\"id\"");
	}
}
