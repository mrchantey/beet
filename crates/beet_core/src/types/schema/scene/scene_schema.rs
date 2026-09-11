//! [`ValueSchema::scene`]: the schema of a scene document.
use crate::prelude::*;

impl ValueSchema {
	/// The registry name of the scene schema, [`scene`](Self::scene).
	pub const SCENE: &'static str = "Scene";
	/// The registry name of the node schema, [`scene_node`](Self::scene_node).
	pub const SCENE_NODE: &'static str = "SceneNode";

	/// The schema describing a scene document as data, the `template_serde`
	/// shape: `{ resources: { type path: value }, nodes: { key: node } }`.
	///
	/// A resource is described by its key, and a node names
	/// [`scene_node`](Self::scene_node) by reference so the one node schema is
	/// what an inspector over a single entity binds. Registered under
	/// [`SCENE`](Self::SCENE) by every [`SchemaRegistry`], where a scene
	/// validates wherever it is read.
	pub fn scene() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some(Self::SCENE.into()),
			description: Some(
				"A scene: resources plus entity-keyed nodes, in child order"
					.into(),
			),
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("resources", Self::keyed_map())
					.with_label("Resources"),
				NamedFieldSchema::new(
					"nodes",
					ValueSchema::Map(MapSchema::uniform(
						ValueSchema::reference(Self::SCENE_NODE),
					)),
				)
				.with_label("Nodes"),
			],
		})
	}

	/// The schema of one scene node: an entity as the list of components it
	/// holds, `{ components: { type path: value } }`, never a struct of
	/// optionals.
	///
	/// Each component value is described by its key, so an entity inspector is
	/// the form's map arm with the add-entry key input specialized to a
	/// component picker.
	pub fn scene_node() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some(Self::SCENE_NODE.into()),
			description: Some("An entity: the components it holds".into()),
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new("components", Self::keyed_map())
					.with_label("Components"),
			],
		})
	}

	/// `{ name: value }` where each value is whatever the registry holds under
	/// its key ([`MapSchema::Keyed`]).
	fn keyed_map() -> ValueSchema { ValueSchema::Map(MapSchema::Keyed) }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Reflect)]
	#[allow(dead_code)]
	struct Health(u32);

	fn registry() -> SchemaRegistry {
		let mut registry = SchemaRegistry::default();
		registry.register_type::<Name>();
		registry.register_type::<ChildOf>();
		registry.register_type::<Health>();
		registry
	}

	/// A root with a name and one child, as the saver writes it: node keys are
	/// file keys and a `ChildOf` holds the parent's file entity bits.
	fn scene() -> Value {
		value!({
			"resources": {},
			"nodes": {
				"0": { "components": {
					"bevy_ecs::name::Name": "root",
					"beet_core::types::schema::scene::scene_schema::test::Health": 3
				} },
				"1": { "components": {
					"bevy_ecs::hierarchy::ChildOf": (EntitySchema::reference(0).unwrap())
				} }
			}
		})
	}

	/// The scene is intrinsic to a registry, so a scene document validates
	/// wherever it is read, and each component value validates against the
	/// schema registered under its key.
	#[crate::test]
	async fn a_scene_validates_by_component_key() {
		let registry = registry();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		resolver
			.schema(ValueSchema::SCENE)
			.unwrap()
			.assert_valid_in(resolver, "scene.json", &mut scene())
			.await
			.unwrap();

		let mut wrong = scene();
		wrong.as_map_mut().unwrap().insert(
			"nodes",
			value!({ "0": { "components": {
				"beet_core::types::schema::scene::scene_schema::test::Health": "full"
			} } }),
		);
		ValueSchema::reference(ValueSchema::SCENE)
			.assert_valid_in(resolver, "scene.json", &mut wrong)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Health")
			.xpect_contains("expected u64");
	}

	/// A component this binary has not registered is rejected naming the key
	/// and the node, as the template loader rejects it at build.
	#[crate::test]
	async fn an_unregistered_component_is_an_error() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference(ValueSchema::SCENE)
			.assert_valid_in(resolver, "scene.json", &mut scene())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("nodes.0.components.bevy_ecs::name::Name")
			.xpect_contains("no schema is registered");
	}

	/// A node's shape is still enforced: the scene is a struct of maps, not
	/// `Any`.
	#[crate::test]
	async fn a_node_must_hold_components() {
		let registry = registry();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference(ValueSchema::SCENE)
			.assert_valid_in(
				resolver,
				"scene.json",
				&mut value!({ "resources": {}, "nodes": { "0": {} } }),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("nodes.0.components");
	}

	/// A component's field is addressable through the scene, so a typed write
	/// into a node is checked against the component's own schema.
	#[crate::test]
	fn a_component_field_is_addressable() {
		let registry = registry();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference(ValueSchema::SCENE)
			.get_field_schema_in(resolver, &[
				FieldSegment::key("nodes"),
				FieldSegment::key("1"),
				FieldSegment::key("components"),
				FieldSegment::key("bevy_ecs::hierarchy::ChildOf"),
			])
			.unwrap()
			.xpect_eq(ValueSchema::Entity(default()));
	}
}
