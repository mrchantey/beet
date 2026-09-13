//! [`ValueSchema::scene`]: the schema of a scene document.
use crate::prelude::*;

impl ValueSchema {
	/// The registry name of the scene schema, [`scene`](Self::scene).
	pub const SCENE: &'static str = "Scene";
	/// The registry name of the entity schema,
	/// [`scene_entity`](Self::scene_entity).
	pub const SCENE_ENTITY: &'static str = "SceneEntity";

	/// The schema describing a scene document as data, the `template_serde`
	/// shape: `{ resources: { type path: value }, entities: { key: entity } }`.
	///
	/// A resource is described by its key, and an entity names
	/// [`scene_entity`](Self::scene_entity) by reference so the one entity schema
	/// is what an inspector over a single entity binds. Registered under
	/// [`SCENE`](Self::SCENE) by every [`SchemaRegistry`], where a scene
	/// validates wherever it is read.
	pub fn scene() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some(Self::SCENE.into()),
			description: Some(
				"A scene: resources plus file-keyed entities, in child order"
					.into(),
			),
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new(
					SceneEntities::RESOURCES,
					Self::keyed_map(),
				)
				.with_label("Resources"),
				NamedFieldSchema::new(
					SceneEntities::ENTITIES,
					ValueSchema::Map(MapSchema::uniform(
						ValueSchema::reference(Self::SCENE_ENTITY),
					)),
				)
				.with_label("Entities"),
			],
		})
	}

	/// The schema of one scene entity: the list of components it
	/// holds, `{ components: { type path: value } }`, never a struct of
	/// optionals.
	///
	/// Each component value is described by its key, so an entity inspector is
	/// the form's map arm with the add-entry key input specialized to a
	/// component picker.
	pub fn scene_entity() -> ValueSchema {
		ValueSchema::Struct(StructSchema {
			name: Some(Self::SCENE_ENTITY.into()),
			description: Some("An entity: the components it holds".into()),
			allow_additional: false,
			fields: vec![
				NamedFieldSchema::new(
					SceneEntities::COMPONENTS,
					Self::keyed_map(),
				)
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

	const HEALTH: &str =
		"beet_core::types::schema::scene::scene_schema::test::Health";
	const CHILD_OF: &str = "bevy_ecs::hierarchy::ChildOf";

	fn registry() -> SchemaRegistry {
		let mut registry = SchemaRegistry::default();
		registry.register_type::<Name>();
		registry.register_type::<ChildOf>();
		registry.register_type::<Health>();
		registry
	}

	/// A root with a name and one child, as the saver writes it: entity keys are
	/// file keys and a `ChildOf` holds the parent's file entity bits.
	fn scene() -> Value {
		SceneEntities::scene([
			(
				0,
				Map::new([
					("bevy_ecs::name::Name", Value::str("root")),
					(HEALTH, Value::Uint(3)),
				]),
			),
			(
				1,
				Map::new([(CHILD_OF, EntitySchema::reference(0).unwrap())]),
			),
		])
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

		let mut wrong =
			SceneEntities::scene([(0, Map::new([(HEALTH, "full")]))]);
		ValueSchema::reference(ValueSchema::SCENE)
			.assert_valid_in(resolver, "scene.json", &mut wrong)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("Health")
			.xpect_contains("expected u64");
	}

	/// A component this binary has not registered is rejected naming the key
	/// and the entity, as the template loader rejects it at build.
	#[crate::test]
	async fn an_unregistered_component_is_an_error() {
		let registry = SchemaRegistry::default();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference(ValueSchema::SCENE)
			.assert_valid_in(resolver, "scene.json", &mut scene())
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains(format!("entities.0.components.{HEALTH}"))
			.xpect_contains("no schema is registered");
	}

	/// An entity's shape is still enforced: the scene is a struct of maps, not
	/// `Any`.
	#[crate::test]
	async fn an_entity_must_hold_components() {
		let registry = registry();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference(ValueSchema::SCENE)
			.assert_valid_in(
				resolver,
				"scene.json",
				&mut value!({ "resources": {}, "entities": { "0": {} } }),
			)
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("entities.0.components");
	}

	/// A component's field is addressable through the scene, so a typed write
	/// into an entity is checked against the component's own schema.
	#[crate::test]
	fn a_component_field_is_addressable() {
		let registry = registry();
		let resolver = SchemaResolver::default().with_schemas(&registry);
		ValueSchema::reference(ValueSchema::SCENE)
			.get_field_schema_in(
				resolver,
				&SceneEntities::entity_path(1)
					.with_pushed(SceneEntities::COMPONENTS)
					.with_pushed(CHILD_OF),
			)
			.unwrap()
			.into_owned()
			.xpect_eq(ValueSchema::Entity(default()));
	}
}
