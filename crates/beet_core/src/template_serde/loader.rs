//! Deserializing bytes into a [`DynamicTemplate`] and building it through the
//! [`spawn_template`](WorldTemplateExt::spawn_template) path.
//!
//! [`TemplateLoader`] dispatches by [`MediaType`] to the right serde format,
//! producing a [`DynamicTemplate`], then builds it. Every spawned entity is
//! collected through the build sink, never a second remapping model. When a
//! target entity is given, the spawned roots are tracked as [`TemplateNodeOf`] of it
//! rather than reparented, so its existing children survive.

use crate::prelude::*;

/// Deserializes template bytes into the world and builds the result.
///
/// With a target entity (via [`TemplateLoader::with_entity`] or
/// [`TemplateLoader::new_entity`]) the spawned roots are tracked as
/// [`TemplateNodes`] of that entity.
pub struct TemplateLoader<'a> {
	world: &'a mut World,
	/// If set, spawned roots are tracked as [`TemplateNodeOf`] of this entity.
	entity: Option<Entity>,
}

impl<'a> TemplateLoader<'a> {
	/// Creates a loader for the given world.
	pub fn new(world: &'a mut World) -> Self {
		Self {
			world,
			entity: None,
		}
	}

	/// Creates a loader for the world containing the given entity, tracking
	/// spawned roots as [`TemplateNodeOf`] of it.
	pub fn new_entity(entity: EntityWorldMut<'a>) -> Self {
		let id = entity.id();
		Self {
			world: entity.into_world_mut(),
			entity: Some(id),
		}
	}

	/// Tracks spawned roots as [`TemplateNodeOf`] of the given entity.
	pub fn with_entity(mut self, entity: Entity) -> Self {
		self.entity = Some(entity);
		self
	}

	/// Deserializes from [`MediaBytes`] into the world, dispatching by media type.
	pub fn load(self, bytes: &MediaBytes) -> Result<Vec<Entity>> {
		let template = self.deserialize(bytes)?;
		self.build(template)
	}

	#[cfg(any(feature = "ron", feature = "json", feature = "postcard"))]
	fn deserialize(&self, bytes: &MediaBytes) -> Result<super::DynamicTemplate> {
		use super::serde::DynamicTemplateDeserializer;
		use serde::de::DeserializeSeed;
		let type_registry = self.world.resource::<AppTypeRegistry>().clone();
		let registry = type_registry.read();
		match bytes.media_type() {
			MediaType::Ron => {
				cfg_if! {
					if #[cfg(feature = "ron")] {
						let text = bytes.as_utf8()?;
						let mut de = ron::de::Deserializer::from_str(text)?;
						DynamicTemplateDeserializer { type_registry: &registry }
							.deserialize(&mut de)?
							.xok()
					} else {
						bevybail!("The `ron` feature is required for RON loading")
					}
				}
			}
			MediaType::Json => {
				cfg_if! {
					if #[cfg(feature = "json")] {
						let mut de =
							serde_json::Deserializer::from_slice(bytes.bytes());
						DynamicTemplateDeserializer { type_registry: &registry }
							.deserialize(&mut de)?
							.xok()
					} else {
						bevybail!("The `json` feature is required for JSON loading")
					}
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				cfg_if! {
					if #[cfg(feature = "postcard")] {
						let mut de =
							postcard::Deserializer::from_bytes(bytes.bytes());
						DynamicTemplateDeserializer { type_registry: &registry }
							.deserialize(&mut de)?
							.xok()
					} else {
						bevybail!("The `postcard` feature is required for postcard loading")
					}
				}
			}
			other => {
				bevybail!("Unsupported media type for template loading: {other}")
			}
		}
	}

	#[cfg(not(any(feature = "ron", feature = "json", feature = "postcard")))]
	fn deserialize(
		&self,
		_bytes: &MediaBytes,
	) -> Result<super::DynamicTemplate> {
		bevybail!(
			"No serde format feature enabled; enable `ron`, `json`, or `postcard`"
		)
	}

	/// Builds the template through [`spawn_template`](WorldTemplateExt::spawn_template),
	/// collecting every spawned entity via the build sink.
	fn build(self, template: super::DynamicTemplate) -> Result<Vec<Entity>> {
		let entity = self.entity;
		// install the sink so the build records every real entity it maps to.
		self.world.insert_resource(TemplateBuildSink::default());
		self.world.spawn_template(template)?;
		let spawned = self
			.world
			.remove_resource::<TemplateBuildSink>()
			.map(|sink| sink.0)
			.unwrap_or_default();

		// in entity mode the spawned roots (no `ChildOf`) are tracked as
		// `TemplateNodeOf` of the target rather than reparented, preserving its
		// existing children.
		if let Some(parent) = entity {
			for spawned_entity in spawned.iter() {
				if !self.world.entity(*spawned_entity).contains::<ChildOf>() {
					self.world
						.entity_mut(*spawned_entity)
						.insert(TemplateNodeOf(parent));
				}
			}
		}

		// reflect inserts settle per-entity during the build, so per-insert
		// observers run before relationships like `ChildOf` are whole. Signal
		// completion now the hierarchy is settled, so listeners can react (eg
		// rebuilding a `RouteTree`) before any async work runs.
		self.world.trigger(LoadTemplateSerde {
			entities: spawned.clone(),
		});

		Ok(spawned)
	}
}

/// Triggered after a [`TemplateLoader`] builds a whole batch of deserialized
/// entities into the world.
///
/// Distinct from the per-root
/// [`LoadTemplate`](crate::prelude::LoadTemplate) lifecycle event: this is the
/// batch-completion signal carrying every entity the loader spawned. Reflect
/// loads insert components one entity at a time, so per-insert observers run
/// before relationships settle; listeners use this to react to a completed load
/// synchronously, against the fully-formed hierarchy.
#[derive(Debug, Clone, Event)]
pub struct LoadTemplateSerde {
	/// The entities spawned by this load.
	pub entities: Vec<Entity>,
}

/// Added to entities spawned by the loader to track their target entity.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship(relationship_target = TemplateNodes)]
pub struct TemplateNodeOf(pub Entity);

/// Tracks the entities spawned into a target via [`TemplateLoader::with_entity`].
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship_target(relationship = TemplateNodeOf, linked_spawn)]
pub struct TemplateNodes(Vec<Entity>);

#[cfg(all(test, feature = "ron"))]
mod test {
	use crate::prelude::*;

	fn serde_world() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.register_type::<Name>();
		// the relationship that carries hierarchy through a round-trip, plus its
		// `Children` mirror: registering `Children` would let a naive extractor
		// serialize it and double-apply the hierarchy, so this guards that the
		// builder skips the mirror and rebuilds it from `ChildOf` hooks.
		app.register_type::<ChildOf>();
		app.register_type::<Children>();
		app.init();
		app.update();
		app
	}

	#[crate::test]
	fn round_trip_ron() {
		let mut app = serde_world();
		let bytes = TemplateSaver::new_all(app.world())
			.save(app.world(), MediaType::Ron)
			.unwrap();
		bytes.as_utf8().unwrap().xref().xpect_contains("Time");
		TemplateLoader::new(app.world_mut()).load(&bytes).unwrap();
	}

	#[crate::test]
	fn entity_scope() {
		let mut app = serde_world();
		let entity = app.world_mut().spawn(Name::new("Root")).id();
		app.world_mut()
			.entity_mut(entity)
			.with_child(Name::new("Child"));

		let bytes = TemplateSaver::new()
			.with_entity_tree(app.world(), entity)
			.save(app.world(), MediaType::Ron)
			.unwrap();
		let text = bytes.as_utf8().unwrap();
		text.xref().xpect_contains("Root");
		text.xref().xpect_contains("Child");
	}

	#[crate::test]
	fn loads_into_entity_adds_template_of() {
		let mut app = serde_world();
		let child = app.world_mut().spawn(Name::new("TemplateChild")).id();
		let bytes = TemplateSaver::new()
			.with_entities([child])
			.save(app.world(), MediaType::Ron)
			.unwrap();

		let target = app.world_mut().spawn(Name::new("Target")).id();
		let spawned = TemplateLoader::new(app.world_mut())
			.with_entity(target)
			.load(&bytes)
			.unwrap();

		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<TemplateNodeOf>()
			.unwrap()
			.0
			.xpect_eq(target);
		app.world()
			.entity(spawned[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("TemplateChild");
	}

	#[crate::test]
	fn loads_into_entity_preserves_existing_children() {
		let mut app = serde_world();
		let child = app.world_mut().spawn(Name::new("TemplateChild")).id();
		let bytes = TemplateSaver::new()
			.with_entities([child])
			.save(app.world(), MediaType::Ron)
			.unwrap();

		let target = app
			.world_mut()
			.spawn((Name::new("Target"), children![Name::new("OldChild")]))
			.id();
		app.world()
			.entity(target)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(1);

		let spawned = TemplateLoader::new(app.world_mut())
			.with_entity(target)
			.load(&bytes)
			.unwrap();

		// existing children are preserved.
		let children: Vec<Entity> = app
			.world()
			.entity(target)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		children.len().xpect_eq(1);
		app.world()
			.entity(children[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("OldChild");

		// spawned entities carry `TemplateNodeOf`, not `ChildOf`.
		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<TemplateNodeOf>()
			.unwrap()
			.0
			.xpect_eq(target);
	}

	/// Children order survives a full save then load round-trip: a parent with
	/// three ordered children is asserted in order after rebuilding.
	#[crate::test]
	fn children_order_round_trips() {
		let mut app = serde_world();
		let root = app
			.world_mut()
			.spawn((
				Name::new("parent"),
				children![
					Name::new("a"),
					Name::new("b"),
					Name::new("c"),
				],
			))
			.id();
		let bytes = TemplateSaver::new()
			.with_entity_tree(app.world(), root)
			.save(app.world(), MediaType::Ron)
			.unwrap();

		let mut app = serde_world();
		let spawned =
			TemplateLoader::new(app.world_mut()).load(&bytes).unwrap();
		// the rebuilt parent is the spawned entity named "parent".
		let parent = spawned
			.iter()
			.copied()
			.find(|entity| {
				app.world()
					.entity(*entity)
					.get::<Name>()
					.map(|name| name.as_str() == "parent")
					.unwrap_or(false)
			})
			.unwrap();
		let order: Vec<String> = app
			.world()
			.entity(parent)
			.get::<Children>()
			.unwrap()
			.iter()
			.map(|child| {
				app.world().entity(child).get::<Name>().unwrap().to_string()
			})
			.collect();
		order.xpect_eq(vec![
			"a".to_string(),
			"b".to_string(),
			"c".to_string(),
		]);
	}
}
