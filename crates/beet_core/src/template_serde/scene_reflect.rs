//! A [`SceneBody`] to and from a world: the body reflected against a type
//! registry into the [`DynamicTemplate`] that spawns it, and the keying of
//! entity references both ways.

use super::ComponentSlot;
use super::DeferredTemplate;
use super::DynamicTemplate;
use super::DynamicTemplateEntity;
use crate::prelude::*;
use bevy::ecs::entity::EntityGeneration;
use bevy::ecs::entity::EntityIndex;
use bevy::ecs::entity::EntityMapper;
use bevy::ecs::reflect::ReflectComponent;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectFromReflect;
use bevy_reflect::TypeRegistration;
use bevy_reflect::TypeRegistry;
use bevy_reflect::serde::TypedReflectDeserializer;
use core::cell::RefCell;
use indexmap::IndexSet;
use serde::de::DeserializeSeed;

impl SceneBody {
	/// Reflect this body against `world`'s type registry into the
	/// [`DynamicTemplate`] that spawns it.
	///
	/// A key resolves as a full type path, else a short one. A key naming a
	/// template is its invocation, a deferred slot expanded when the entity
	/// builds. A component key that resolves as neither is left off its entity
	/// with a warning and held, value and all, in the entity's
	/// [`Unregistered`], unless its short name is
	/// [allowed](AllowedUnregistered), which skips it silently. A resource has
	/// no entity to hold it, so an unresolved one is an error, as is any
	/// registered type whose value does not read.
	pub fn reflect(&self, world: &World) -> Result<DynamicTemplate> {
		let app_registry = world
			.get_resource::<AppTypeRegistry>()
			.ok_or_else(|| {
				bevyhow!("a scene body reflects against an `AppTypeRegistry`")
			})?
			.clone();
		let registry = app_registry.read();
		let allowed = world.get_resource::<AllowedUnregistered>();
		let keys = EntityKeys::new(self.entities.keys().cloned());

		let resources = self
			.resources
			.iter()
			.map(|(name, value)| {
				Self::registration(&registry, name)
					.ok_or_else(|| {
						bevyhow!("resource `{name}` is not a registered type")
					})
					.and_then(|registration| {
						keys.deserialize(registration, &registry, value)
					})
			})
			.collect::<Result<Vec<_>>>()?;

		let entities = self
			.entities
			.iter()
			.map(|(key, body)| {
				Self::reflect_entity(key, body, &keys, &registry, allowed)
					.map_err(|err| bevyhow!("entity `{key}`: {err}"))
			})
			.collect::<Result<Vec<_>>>()?;

		DynamicTemplate {
			resources,
			entities,
		}
		.xok()
	}

	/// One entity's components as slots, the unresolved ones held.
	fn reflect_entity(
		key: &EntityKey,
		body: &EntityBody,
		keys: &EntityKeys,
		registry: &TypeRegistry,
		allowed: Option<&AllowedUnregistered>,
	) -> Result<DynamicTemplateEntity> {
		let mut components = Vec::new();
		let mut unregistered = Unregistered::default();
		for (name, value) in body.iter() {
			match Self::registration(registry, name) {
				Some(registration)
					if registration.data::<ReflectComponent>().is_none()
						&& registration.data::<ReflectTemplate>().is_some() =>
				{
					components.push(ComponentSlot::Template(DeferredTemplate {
						name: registration.type_info().type_path().into(),
						patch: keys.deserialize(
							registration,
							registry,
							value,
						)?,
					}))
				}
				Some(registration) => components.push(ComponentSlot::Value(
					keys.deserialize(registration, registry, value)?,
				)),
				None if allowed.is_some_and(|allowed| {
					allowed.contains(&TypeName::new(name.as_str()).short())
				}) => {}
				None => {
					warn!(
						"entity `{key}`: no type registered for `{name}`, left off and held unregistered"
					);
					unregistered.insert(name.clone(), value.clone());
				}
			}
		}
		if !unregistered.is_empty() {
			components.push(ComponentSlot::Value(Box::new(unregistered)));
		}
		DynamicTemplateEntity {
			key: key.clone(),
			components,
		}
		.xok()
	}

	/// The type a body key names: its full type path, else its short one, so a
	/// key a lean binary held by the name its markup wrote resolves in a full
	/// one.
	pub(super) fn registration<'a>(
		registry: &'a TypeRegistry,
		name: &str,
	) -> Option<&'a TypeRegistration> {
		registry
			.get_with_type_path(name)
			.or_else(|| registry.get_with_short_type_path(name))
	}
}

/// The entities one body's values may reference, each by its position: the
/// in-template [`Entity`] a reflected component holds in place of a key.
///
/// A body's own keys take their file positions, which is how a
/// [`DynamicTemplate`] names its entities; a key the body does not hold takes
/// the next position past them, a reference to an entity outside it.
pub(super) struct EntityKeys(RefCell<IndexSet<EntityKey, FixedHasher>>);

impl EntityKeys {
	/// The positions of `keys`, in order.
	pub fn new(keys: impl IntoIterator<Item = EntityKey>) -> Self {
		Self(RefCell::new(keys.into_iter().collect()))
	}

	/// The in-template entity standing for `key`.
	fn entity(&self, key: EntityKey) -> Option<Entity> {
		let (position, _) = self.0.borrow_mut().insert_full(key);
		Entity::from_raw_u32(position as u32)
	}

	/// The key an in-template entity stands for.
	pub fn key(&self, entity: Entity) -> Option<EntityKey> {
		self.0
			.borrow()
			.get_index(entity.index_u32() as usize)
			.cloned()
	}

	/// Read `value` as the registered type, every reference in it (a key where
	/// an [`Entity`] is expected) as that key's in-template entity. The value
	/// converts through [`ReflectFromReflect`] so it carries its concrete type
	/// rather than staying a dynamic.
	pub fn deserialize(
		&self,
		registration: &TypeRegistration,
		registry: &TypeRegistry,
		value: &Value,
	) -> Result<Box<dyn PartialReflect>> {
		let reference = |key: &str| {
			EntityKey::parse(key)
				.ok()
				.and_then(|key| self.entity(key))
				.map(Entity::to_bits)
		};
		let reflected = TypedReflectDeserializer::new(registration, registry)
			.deserialize(
				ValueDeserializer::new(value.clone())
					.with_references(&reference),
			)
			.map_err(|err| {
				bevyhow!("`{}`: {err}", registration.type_info().type_path())
			})?;
		registration
			.data::<ReflectFromReflect>()
			.and_then(|from_reflect| {
				from_reflect.from_reflect(reflected.as_partial_reflect())
			})
			.map(PartialReflect::into_partial_reflect)
			.unwrap_or(reflected)
			.xok()
	}
}

/// Finds the entity references in a component's serialized value, so each is
/// written as its target's key.
///
/// A component's serde form writes an [`Entity`] as opaque bits, and a type
/// with its own `Serialize` hides where. So the save maps each entity the
/// component references to a sentinel no world holds, serializes, and replaces
/// every sentinel it finds in the result. A value that only looks like one is
/// told apart by count: the component's own `map_entities` says how many
/// references it holds.
#[derive(Default)]
pub(super) struct ReferenceSentinels {
	/// The entities referenced, each standing at its sentinel's index.
	targets: Vec<Entity>,
	/// How many times the component references each target.
	visits: Vec<usize>,
}

impl ReferenceSentinels {
	/// A generation no entity reaches in practice, so a sentinel's bits are
	/// not a number real data holds.
	const GENERATION: u32 = 0xBEE7_5CE0;

	/// The sentinel standing for the `ordinal`th target.
	fn sentinel(ordinal: usize) -> Entity {
		Entity::from_index_and_generation(
			EntityIndex::from_raw_u32(ordinal as u32).expect(
				"a component references fewer than `u32::MAX` entities",
			),
			EntityGeneration::from_bits(Self::GENERATION),
		)
	}

	/// The target a sentinel's bits stand for.
	fn ordinal(&self, bits: u64) -> Option<usize> {
		Entity::try_from_bits(bits)
			.filter(|entity| entity.generation().to_bits() == Self::GENERATION)
			.map(|entity| entity.index_u32() as usize)
			.filter(|ordinal| *ordinal < self.targets.len())
	}

	/// `value`, a component serialized after its entities were mapped through
	/// this, with every reference replaced by its target's key in
	/// `entity_map`, minting one for a target the document has not keyed.
	pub fn keyed(
		&self,
		value: Value,
		entity_map: &mut TemplateEntityMap,
	) -> Result<Value> {
		if self.targets.is_empty() {
			return value.xok();
		}
		let mut found = vec![0; self.targets.len()];
		let value = self.key_value(value, entity_map, &mut found)?;
		if found
			.iter()
			.zip(&self.visits)
			.any(|(found, visits)| found > visits)
		{
			bevybail!(
				"a value is indistinguishable from an entity reference, so the \
				component cannot be keyed"
			);
		}
		value.xok()
	}

	fn key_value(
		&self,
		value: Value,
		entity_map: &mut TemplateEntityMap,
		found: &mut [usize],
	) -> Result<Value> {
		match value {
			Value::Uint(bits) => match self.ordinal(bits) {
				Some(ordinal) => {
					found[ordinal] += 1;
					entity_map.file_key(self.targets[ordinal]).reference()
				}
				None => Value::Uint(bits),
			},
			Value::List(items) => items
				.into_iter()
				.map(|item| self.key_value(item, entity_map, found))
				.collect::<Result<Vec<_>>>()?
				.xmap(Value::List),
			Value::Map(map) => map
				.into_iter()
				.map(|(key, item)| {
					// a map key is a string either way, so a reference there
					// could not be told from a number on the way back in
					if key
						.parse::<u64>()
						.ok()
						.and_then(|bits| self.ordinal(bits))
						.is_some()
					{
						bevybail!(
							"a map keyed by an entity reference cannot be saved"
						);
					}
					(key, self.key_value(item, entity_map, found)?).xok()
				})
				.collect::<Result<Map>>()?
				.xmap(Value::Map),
			other => other,
		}
		.xok()
	}
}

impl EntityMapper for ReferenceSentinels {
	fn get_mapped(&mut self, source: Entity) -> Entity {
		// a placeholder names no entity, so it is written as it stands
		if source == Entity::PLACEHOLDER {
			return source;
		}
		let ordinal = self
			.targets
			.iter()
			.position(|target| *target == source)
			.unwrap_or_else(|| {
				self.targets.push(source);
				self.visits.push(0);
				self.targets.len() - 1
			});
		self.visits[ordinal] += 1;
		Self::sentinel(ordinal)
	}

	fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;

	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Health(u32);

	/// A component a lean binary lacks.
	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Shield {
		strength: f32,
	}

	/// A component whose own serde hides its references from reflection, and
	/// which holds one twice beside a number.
	#[derive(
		Debug,
		Clone,
		PartialEq,
		Component,
		Reflect,
		MapEntities,
		Serialize,
		Deserialize,
	)]
	#[reflect(Component, MapEntities, Serialize, Deserialize)]
	struct Links {
		#[entities]
		first: Entity,
		#[entities]
		rest: Vec<Entity>,
		weight: u64,
	}

	/// A resource a body carries.
	#[derive(Debug, Default, Clone, PartialEq, Resource, Reflect)]
	#[reflect(Resource, Default)]
	struct Score(u32);

	/// A template a body invokes: its expansion inserts [`Health`] on the
	/// entity and spawns a named child, neither of which a save writes.
	#[derive(Debug, Default, Clone, Reflect)]
	#[reflect(Default)]
	struct Badge {
		text: String,
	}
	subtree_template!(Badge);
	impl GetTemplateSchema for Badge {}
	impl bevy::ecs::template::Template for Badge {
		type Output = ();
		fn build_template(
			&self,
			cx: &mut bevy::ecs::template::TemplateContext,
		) -> Result<()> {
			let root = cx.entity.id();
			cx.entity.insert(Health(9));
			// SAFETY: only used to spawn an unrelated child entity.
			let world = unsafe { cx.entity.world_mut() };
			world.spawn((Name::new(self.text.clone()), ChildOf(root)));
			OK
		}
		fn clone_template(&self) -> Self { self.clone() }
	}

	const HEALTH: &str =
		"beet_core::template_serde::scene_reflect::test::Health";
	const SHIELD: &str =
		"beet_core::template_serde::scene_reflect::test::Shield";
	const LINKS: &str = "beet_core::template_serde::scene_reflect::test::Links";

	/// A world registering `Health`, `Links`, `Score` and the `Badge`
	/// template, and `Shield` when `full`.
	fn world(full: bool) -> World {
		let mut app = App::new();
		app.add_plugins((TemplatePlugin, MinimalTypesPlugin))
			.install_type::<Badge>();
		let world = core::mem::take(app.world_mut());
		{
			let mut registry = world.resource::<AppTypeRegistry>().write();
			registry.register::<Health>();
			registry.register::<Links>();
			registry.register::<Score>();
			if full {
				registry.register::<Shield>();
			}
		}
		world
	}

	fn save(
		world: &mut World,
		root: Entity,
		media_type: MediaType,
	) -> MediaBytes {
		TemplateSaver::new()
			.save_roots(world, media_type, [root])
			.unwrap()
	}

	fn load(world: &mut World, bytes: &MediaBytes) -> Entity {
		TemplateLoader::new(world).load(bytes).unwrap()[0]
	}

	/// A root with a shielded child, saved by a world registering everything.
	fn shielded() -> MediaBytes {
		let mut world = world(true);
		let root = world
			.spawn((Name::new("root"), children![(Health(3), Shield {
				strength: 1.3
			})]))
			.flush();
		save(&mut world, root, MediaType::Json)
	}

	/// The document reads as its author would write it: string keys in file
	/// order, a reference its target's key, a float the number it shows.
	#[crate::test]
	fn writes_the_wire_shape() {
		shielded().as_utf8().unwrap().xpect_snapshot();
	}

	/// Every structured format round trips through the saver and the loader.
	#[cfg(all(feature = "postcard", feature = "toml"))]
	#[crate::test]
	fn round_trips_every_format() {
		for media_type in
			[MediaType::Json, MediaType::Postcard, MediaType::Toml]
		{
			let mut source = world(true);
			let root = source
				.spawn((Name::new("root"), children![(Health(3), Shield {
					strength: 1.3
				})]))
				.flush();
			let bytes = save(&mut source, root, media_type.clone());

			let mut loaded = world(true);
			let root = load(&mut loaded, &bytes);
			let child = loaded.entity(root).get::<Children>().unwrap()[0];
			loaded.get::<Name>(root).unwrap().as_str().xpect_eq("root");
			loaded.get::<Health>(child).unwrap().xpect_eq(Health(3));
			loaded
				.get::<Shield>(child)
				.unwrap()
				.xpect_eq(Shield { strength: 1.3 });
			// and the reload re-saves the bytes it read
			save(&mut loaded, root, media_type).xpect_eq(bytes);
		}
	}

	/// A body holding a `Value`, an entity reference, a resource and a template
	/// invocation spawns the invocation expanded, and a save of that world is
	/// a snapshot, output only: what the template built and never the
	/// invocation, so loading the snapshot builds nothing a second time and
	/// re-saves it byte identical through json and postcard.
	#[cfg(feature = "postcard")]
	#[crate::test]
	fn a_snapshot_holds_what_a_template_built() {
		let badge = "beet_core::template_serde::scene_reflect::test::Badge";
		let value = "beet_core::types::value::value::Value";
		let score = "beet_core::template_serde::scene_reflect::test::Score";
		let authored = MediaBytes::new_json(format!(
			r#"{{"$type":"org.beet.scene#body","resources":{{"{score}":4}},"entities":{{"0":{{"{badge}":{{"text":"hi"}},"{LINKS}":{{"first":"1","rest":[],"weight":2}}}},"1":{{"{value}":"text","bevy_ecs::hierarchy::ChildOf":"0"}}}}}}"#
		));
		// the body's resource is saved beside its entities
		let save = |world: &mut World, root: Entity, media_type: MediaType| {
			TemplateSaver::new()
				.extract_resources()
				.allow_resource::<Score>()
				.save_roots(world, media_type, [root])
				.unwrap()
		};
		// the reference lands on the text child, whatever key it saved under
		let expect_built = |world: &World, root: Entity| {
			world.get::<Health>(root).unwrap().xpect_eq(Health(9));
			world.resource::<Score>().clone().xpect_eq(Score(4));
			world
				.entity(root)
				.get::<Children>()
				.unwrap()
				.len()
				.xpect_eq(2);
			world
				.get::<Value>(world.get::<Links>(root).unwrap().first)
				.unwrap()
				.xpect_eq(Value::Str("text".into()));
		};
		let mut loaded = world(true);
		let root = load(&mut loaded, &authored);
		expect_built(&loaded, root);
		let json = save(&mut loaded, root, MediaType::Json);
		json.as_utf8()
			.unwrap()
			.xref()
			.xpect_contains(HEALTH)
			.xpect_contains("bevy_ecs::name::Name")
			.xpect_contains(score)
			.xnot()
			.xpect_contains(badge);

		for media_type in [MediaType::Json, MediaType::Postcard] {
			let bytes = json
				.deserialize::<SceneBody>()
				.unwrap()
				.to_bytes(media_type.clone())
				.unwrap();
			let mut reloaded = world(true);
			let root = load(&mut reloaded, &bytes);
			expect_built(&reloaded, root);
			save(&mut reloaded, root, media_type).xpect_eq(bytes);
		}
	}

	/// References hidden behind a type's own serde are still written as keys,
	/// a repeated one each time, and a number beside them stays a number.
	#[crate::test]
	fn references_round_trip_as_keys() {
		let mut source = world(true);
		let root = source.spawn(Name::new("root")).id();
		let (a, b) = (
			source.spawn((Name::new("a"), ChildOf(root))).id(),
			source.spawn((Name::new("b"), ChildOf(root))).id(),
		);
		source.entity_mut(root).insert(Links {
			first: b,
			rest: vec![a, b, Entity::PLACEHOLDER],
			weight: 7,
		});
		let bytes = save(&mut source, root, MediaType::Json);
		let scene = bytes.deserialize::<Value>().unwrap();
		let links = SceneEntities::entity_value(&scene, 0)
			.unwrap()
			.get(LINKS)
			.unwrap();
		links.get("first").unwrap().as_str().unwrap().xpect_eq("2");
		let rest = links.get("rest").unwrap();
		rest.get_index(0).unwrap().as_str().unwrap().xpect_eq("1");
		rest.get_index(1).unwrap().as_str().unwrap().xpect_eq("2");
		links.get("weight").unwrap().as_u64().unwrap().xpect_eq(7);
		SceneEntities::entity_value(&scene, 1)
			.unwrap()
			.get("bevy_ecs::hierarchy::ChildOf")
			.unwrap()
			.as_str()
			.unwrap()
			.xpect_eq("0");

		let mut loaded = world(true);
		let root = load(&mut loaded, &bytes);
		let children = loaded.entity(root).get::<Children>().unwrap().to_vec();
		loaded.get::<Links>(root).unwrap().clone().xpect_eq(Links {
			first: children[1],
			rest: vec![children[0], children[1], Entity::PLACEHOLDER],
			weight: 7,
		});
	}

	/// A binary lacking a component's plugin spawns the entity without it and
	/// holds the value, so its re-save is the document it read, and a binary
	/// with the plugin resolves it.
	#[crate::test]
	fn an_unregistered_component_is_held() {
		let original = shielded();

		let mut lean = world(false);
		let root = load(&mut lean, &original);
		let child = lean.entity(root).get::<Children>().unwrap()[0];
		lean.get::<Health>(child).unwrap().xpect_eq(Health(3));
		let held = lean.get::<Unregistered>(child).unwrap();
		held.keys()
			.map(SmolStr::as_str)
			.collect::<Vec<_>>()
			.xpect_eq(vec![SHIELD]);
		held.get(SHIELD)
			.unwrap()
			.clone()
			.xpect_eq(Value::Map(Map::new([("strength", 1.3)])));
		// a strict caller refuses it, naming the component
		lean.with_state::<UnregisteredQuery, _>(|query| query.refuse(root))
			.unwrap_err()
			.to_string()
			.xpect_contains(SHIELD);

		// the lean re-save is byte identical, the marker never written
		let resaved = save(&mut lean, root, MediaType::Json);
		resaved.clone().xpect_eq(original);
		resaved
			.as_utf8()
			.unwrap()
			.xref()
			.xnot()
			.xpect_contains("Unregistered");

		let mut full = world(true);
		let root = load(&mut full, &resaved);
		let child = full.entity(root).get::<Children>().unwrap()[0];
		full.get::<Shield>(child)
			.unwrap()
			.xpect_eq(Shield { strength: 1.3 });
		full.get::<Unregistered>(child).xpect_none();
		full.with_state::<UnregisteredQuery, _>(|query| query.refuse(root))
			.unwrap();
	}

	/// A key held by the short name markup wrote resolves by it, and an
	/// allowed name is skipped without a trace.
	#[crate::test]
	fn a_short_name_resolves_and_an_allowed_one_is_skipped() {
		let body = |name: &str| {
			MediaBytes::new_json(format!(
				r#"{{"$type":"org.beet.scene#body","entities":{{"0":{{"{name}":4}}}}}}"#
			))
		};
		let mut world = world(false);
		let root = load(&mut world, &body("Health"));
		world.get::<Health>(root).unwrap().xpect_eq(Health(4));

		let mut app = App::new();
		app.add_plugins((TemplatePlugin, MinimalTypesPlugin))
			.install(Unregistered::allow("Shield"));
		let root = load(app.world_mut(), &body(SHIELD));
		app.world().get::<Unregistered>(root).xpect_none();
	}

	/// A registered type whose value does not read is an error naming the
	/// entity key and the type path, never a skip.
	#[crate::test]
	fn a_malformed_value_is_an_error() {
		let mut world = world(false);
		TemplateLoader::new(&mut world)
			.load(&MediaBytes::new_json(format!(
				r#"{{"$type":"org.beet.scene#body","entities":{{"main":{{"{HEALTH}":"full"}}}}}}"#
			)))
			.unwrap_err()
			.to_string()
			.xpect_contains("entity `main`")
			.xpect_contains(HEALTH);
	}
}
