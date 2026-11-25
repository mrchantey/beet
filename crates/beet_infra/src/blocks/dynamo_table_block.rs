//! A DynamoDB table deploy block: a table store's remote backend, in the
//! single-table layout [`DynamoStore`] reads and writes.
//!
//! Uses the generated [`AwsDynamodbTableDetails`] binding (in `bindings/aws_dynamo.rs`).
use crate::bindings::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// A DynamoDB table in [`DynamoStore`]'s single-table layout, provisioned
/// pay-per-request: `pk`/`sk` string keys, the four generic index slots
/// (`gsi1` to `gsi4`, each projecting every attribute) a declared secondary
/// index writes into, so declaring one never provisions infrastructure, and
/// expiry on the `ttl` attribute an archived row is stamped with.
///
/// Mirrors [`S3BucketBlock`](crate::prelude::S3BucketBlock): the declaration
/// carries only its `label`, and the `<app>--<stage>--<label>` name composes at
/// resolution through the ancestor [`Stack`]. The deploy creates the table from
/// that name, and as a [`StoreBlock`] its erased root is the `dynamo://` uri
/// of the same name, so the one runtime attach lands a [`DynamoStore`] remotely
/// and a [`JsonTableStore`] over the host's local stand-in locally, rows alone
/// either way: one declaration and nothing to keep in agreement.
///
/// Authored directly from markup, ie
/// `<DynamoTableBlock bx:ref="analytics" label="analytics"/>`.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedStoreBlock::on_insert::<Self>,
	on_remove = ErasedStoreBlock::on_remove
)]
pub struct DynamoTableBlock {
	/// The unprefixed table label (eg `analytics`).
	label: SmolStr,
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
	/// The table `label`.
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
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
}

impl StoreBlock for DynamoTableBlock {
	fn kind(&self) -> StoreKind { StoreKind::Table }

	/// The composed table name pinned to the stack's declared region, so the
	/// process a deploy hands it to reaches the table the deploy created.
	fn store_uri(&self, stack: &ResolvedStack) -> Result<StoreUri> {
		StoreUri::Dynamo {
			name: self.table_name(stack).into(),
			region: Some(stack.aws_region()?.clone()),
		}
		.xok()
	}
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
	/// Emit the pay-per-request table in the single-table layout.
	fn emit(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result {
		let string =
			|name: String| AwsDynamodbTableResourceBlockTypeAttribute {
				name: name.into(),
				r#type: "S".into(),
			};
		let slots = 1..=DynamoStore::INDEX_SLOTS;
		let table = ResourceDef::new_primary(
			stack.resource_ident(self.label.clone()),
			AwsDynamodbTableDetails {
				billing_mode: Some("PAY_PER_REQUEST".into()),
				hash_key: Some("pk".into()),
				range_key: Some("sk".into()),
				attribute: Some(
					["pk".to_string(), "sk".to_string()]
						.into_iter()
						.chain(slots.clone().flat_map(|slot| {
							[format!("gsi{slot}pk"), format!("gsi{slot}sk")]
						}))
						.map(string)
						.collect(),
				),
				global_secondary_index: Some(
					slots
						.map(|slot| {
							AwsDynamodbTableResourceBlockTypeGlobalSecondaryIndex {
								name: format!("gsi{slot}").into(),
								// `key_schema`, which the provider's 6.x reads
								// in place of the index's `hash_key`/`range_key`
								key_schema: Some(vec![
									GlobalSecondaryIndexResourceBlockTypeKeySchema {
										attribute_name: format!("gsi{slot}pk").into(),
										key_type: "HASH".into(),
									},
									GlobalSecondaryIndexResourceBlockTypeKeySchema {
										attribute_name: format!("gsi{slot}sk").into(),
										key_type: "RANGE".into(),
									},
								]),
								projection_type: "ALL".into(),
								..default()
							}
						})
						.collect(),
				),
				region: Some(stack.aws_region()?.clone()),
				ttl: Some(vec![AwsDynamodbTableResourceBlockTypeTtl {
					attribute_name: Some(DynamoStore::TTL_ATTRIBUTE.into()),
					enabled: Some(true),
				}]),
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

	/// The declaration's erased root is the table uri a remote process
	/// attaches, the composed name pinned to the stack's declared region, so
	/// the generic store attach serves a table exactly as it serves a bucket.
	#[beet_core::test]
	fn projects_a_dynamo_root() {
		let mut world = World::new();
		world.init_resource::<PackageConfig>();
		let stack = world
			.spawn((
				Stack::new("app").with_stage("prod"),
				AwsRegion::new("eu-west-1"),
				children![DynamoTableBlock::new("analytics")],
			))
			.id();
		world.flush();
		let entity = world.entity(stack).get::<Children>().unwrap()[0];
		world
			.get::<ErasedStoreBlock>(entity)
			.unwrap()
			.root()
			.to_string()
			.xpect_eq("dynamo://app--prod--analytics?region=eu-west-1");
	}

	/// The block emits the single-table layout: `pk`/`sk`, the four index
	/// slots, expiry on `ttl`, pay-per-request billing.
	#[beet_core::test]
	fn emits_the_single_table_layout() {
		let json = build_json(DynamoTableBlock::new("analytics"));
		json.as_str()
			.xpect_contains("aws_dynamodb_table")
			.xpect_contains("PAY_PER_REQUEST")
			.xpect_contains(r#""hash_key":"pk""#)
			.xpect_contains(r#""range_key":"sk""#)
			.xpect_contains(
				r#""ttl":[{"attribute_name":"ttl","enabled":true}]"#,
			);
		for slot in 1..=4 {
			json.as_str()
				.xpect_contains(&format!(r#""name":"gsi{slot}""#))
				.xpect_contains(&format!(
					r#"{{"attribute_name":"gsi{slot}pk","key_type":"HASH"}}"#
				))
				.xpect_contains(&format!(
					r#"{{"attribute_name":"gsi{slot}sk","key_type":"RANGE"}}"#
				));
		}
	}
}
