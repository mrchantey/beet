//! A [`SceneBody`] to and from a world: the body reflected against a type
//! registry into the [`DynamicTemplate`] that spawns it, and the keying of
//! entity references both ways.

use super::CfgPrune;
use super::ComponentSlot;
use super::DeferredTemplate;
use super::DynamicTemplate;
use super::DynamicTemplateEntity;
use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use bevy::ecs::entity::EntityGeneration;
use bevy::ecs::entity::EntityIndex;
use bevy::ecs::entity::EntityMapper;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectDeserialize;
use bevy_reflect::ReflectFromReflect;
use bevy_reflect::TypeInfo;
use bevy_reflect::TypeRegistration;
use bevy_reflect::TypeRegistry;
use bevy_reflect::serde::SerializationData;
use bevy_reflect::serde::TypedReflectDeserializer;
use bevy_reflect::structs::DynamicStruct;
use core::cell::RefCell;
use indexmap::IndexSet;
use serde::de::DeserializeSeed;

impl DynamicTemplate {
	/// Reflect `body` against `world`'s type registry into the
	/// [`DynamicTemplate`] that spawns it.
	///
	/// A key resolves as a full type path, else a short one. A key naming a
	/// template is its invocation, a deferred slot expanded when the entity
	/// builds, and one naming a resource patches the live resource where it
	/// stands. A component key that resolves as neither is left off its entity
	/// and held, value and all, in the entity's
	/// [`Unregistered`]. One of the body's own resources has no entity to hold
	/// it, so an unresolved one is an error, as is any registered type whose
	/// value does not read.
	///
	/// A branch whose [`Cfg`] this binary does not meet is left out, its
	/// tombstone in its place. A value reads in its type's serde form, else as
	/// markup would have authored it, so what a lean binary held by name
	/// resolves in a full one.
	pub fn reflect(world: &World, body: &SceneBody) -> Result<Self> {
		Self::reflect_document(world, body, None)
	}

	/// [`reflect`](Self::reflect) `body` as the document at `path` of its
	/// package: every [`DocPath`] it holds is rewritten package absolute.
	pub fn reflect_at(
		world: &World,
		body: &SceneBody,
		path: &FwdPath,
	) -> Result<Self> {
		Self::reflect_document(world, body, Some(path))
	}

	/// [`reflect`](Self::reflect) a document as it was read, the template
	/// answering for where its entities were written ([`SceneSource`]).
	pub fn read(world: &World, source: Arc<DocumentSource>) -> Result<Self> {
		let mut template = Self::reflect_placed(
			world,
			source.body(),
			source.path(),
			Some(&source),
		)?;
		template.source = Some(source);
		template.xok()
	}

	/// [`reflect`](Self::reflect) `body`, as the document at `path` where it
	/// has one.
	pub fn reflect_document(
		world: &World,
		body: &SceneBody,
		path: Option<&FwdPath>,
	) -> Result<Self> {
		Self::reflect_placed(world, body, path, None)
	}

	/// Reflect `body`, an error in an entity naming where `source` says it was
	/// written, else its key.
	fn reflect_placed(
		world: &World,
		body: &SceneBody,
		path: Option<&FwdPath>,
		source: Option<&DocumentSource>,
	) -> Result<Self> {
		let app_registry = world
			.get_resource::<AppTypeRegistry>()
			.ok_or_else(|| {
				bevyhow!("a scene body reflects against an `AppTypeRegistry`")
			})?
			.clone();
		let registry = app_registry.read();
		let (body, excluded) = CfgPrune::new(body).prune(world)?;
		let keys = EntityKeys::new(body.entities.keys().cloned())
			.with_excluded(excluded);

		let resources = body
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

		let entities = body
			.entities
			.iter()
			.map(|(key, body)| {
				Self::reflect_entity(key, body, &keys, &registry).map_err(
					|err| match source.and_then(|source| source.position(key)) {
						Some(position) => bevyhow!("{position}: {err}"),
						None => bevyhow!("entity `{key}`: {err}"),
					},
				)
			})
			.collect::<Result<Vec<_>>>()?;

		let mut template = Self {
			resources,
			entities,
			source: None,
		};
		if let Some(path) = path {
			template.resolve_paths(path)?;
		}
		template.xok()
	}

	/// One entity's components as slots, the unresolved ones held.
	///
	/// The slots' order is the build's: the entity's owner first, so whatever
	/// a component does as it lands finds its ancestors, then templates, so
	/// what was authored beside one wins over what it built, then the rest. A
	/// body's components are a map, and within each group the order is their
	/// keys', which every format keeps.
	fn reflect_entity(
		key: &EntityKey,
		body: &EntityBody,
		keys: &EntityKeys,
		registry: &TypeRegistry,
	) -> Result<DynamicTemplateEntity> {
		let mut components = Vec::new();
		let mut unregistered = Unregistered::default();
		let mut entries = body.iter().collect::<Vec<_>>();
		entries.sort_by_key(|(name, _)| name.as_str());
		for (name, value) in entries {
			// a type registered as none of the three an entity holds is held
			// as a name nothing registered
			let Some(registration) = Self::registration(registry, name)
				.filter(|registration| NameKind::of(registration).is_some())
			else {
				debug!(
					"entity `{key}`: no type registered for `{name}`, left off and held unregistered"
				);
				unregistered.insert(name.clone(), value.clone());
				continue;
			};
			let read = || {
				keys.deserialize(registration, registry, value)
					.map_err(|err| bevyhow!("`{name}`: {err}"))
			};
			// a resource is entity backed, so reflects a component too
			let (order, slot) =
				if registration.data::<ReflectResource>().is_some() {
					let patch = keys.read(registration, registry, value)?;
					(2, ComponentSlot::Resource(patch))
				} else if registration.data::<ReflectTemplate>().is_some()
					&& registration.data::<ReflectComponent>().is_none()
				{
					(
						1,
						ComponentSlot::Template(DeferredTemplate {
							name: registration.type_info().type_path().into(),
							patch: read()?,
						}),
					)
				} else if registration.type_info().is::<ChildOf>()
					|| registration.type_info().is::<AttributeOf>()
				{
					(0, ComponentSlot::Value(read()?))
				} else {
					(2, ComponentSlot::Value(read()?))
				};
			components.push((order, slot));
		}
		components.sort_by_key(|(order, _)| *order);
		let mut components = components
			.into_iter()
			.map(|(_, slot)| slot)
			.collect::<Vec<_>>();
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
	pub(crate) fn registration<'a>(
		registry: &'a TypeRegistry,
		name: &str,
	) -> Option<&'a TypeRegistration> {
		registry.get_with_type_path(name).or_else(|| {
			reflect_ext::carried_registration_by_name(registry, name)
		})
	}
}

/// The entities one body's values may reference, each by its position: the
/// in-template [`Entity`] a reflected component holds in place of a key.
///
/// A body's own keys take their file positions, which is how a
/// [`DynamicTemplate`] names its entities; a key the body does not hold takes
/// the next position past them, a reference to an entity outside it.
pub(crate) struct EntityKeys {
	keys: RefCell<IndexSet<EntityKey, FixedHasher>>,
	/// The keys an exclusion took with it, each to its condition.
	excluded: HashMap<EntityKey, SmolStr>,
}

impl EntityKeys {
	/// The positions of `keys`, in order.
	pub fn new(keys: impl IntoIterator<Item = EntityKey>) -> Self {
		Self {
			keys: RefCell::new(keys.into_iter().collect()),
			excluded: default(),
		}
	}

	/// Warn of a reference to any key of `excluded`, which binds to an entity
	/// nothing builds: a consumer waiting on its components (a `StoreRef`)
	/// fails far from the cause, or never.
	pub fn with_excluded(
		mut self,
		excluded: HashMap<EntityKey, SmolStr>,
	) -> Self {
		self.excluded = excluded;
		self
	}

	/// The in-template entity standing for `key`.
	fn entity(&self, key: EntityKey) -> Option<Entity> {
		if let Some(condition) = self.excluded.get(&key) {
			warn!(
				"a reference names `{key}`, which `bx:cfg=\"{condition}\"` excluded from this binary: it binds to nothing"
			);
		}
		let (position, _) = self.keys.borrow_mut().insert_full(key);
		Entity::from_raw_u32(position as u32)
	}

	/// The key an in-template entity stands for.
	pub fn key(&self, entity: Entity) -> Option<EntityKey> {
		self.keys
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
		let reflected = self.read(registration, registry, value)?;
		registration
			.data::<ReflectFromReflect>()
			.and_then(|from_reflect| {
				from_reflect.from_reflect(reflected.as_partial_reflect())
			})
			.map(PartialReflect::into_partial_reflect)
			.unwrap_or(reflected)
			.xok()
	}

	/// [`deserialize`](Self::deserialize) holding only the fields `value` does,
	/// so a partial value stays a patch.
	///
	/// Reflect serde completes every value it reads over its type's default,
	/// so a struct is read here field by field, each in its own serde form or
	/// as markup would have authored it: what a lean binary held by name, or a
	/// type no serde form can write.
	pub fn read(
		&self,
		registration: &TypeRegistration,
		registry: &TypeRegistry,
		value: &Value,
	) -> Result<Box<dyn PartialReflect>> {
		let (TypeInfo::Struct(info), Value::Map(fields), None) = (
			registration.type_info(),
			value,
			registration.data::<ReflectDeserialize>(),
		) else {
			return self.read_whole(registration, registry, value);
		};
		let skipped = registration.data::<SerializationData>();
		let mut patch = DynamicStruct::default();
		for (name, value) in fields {
			let index = info.index_of(name).ok_or_else(|| {
				bevyhow!(
					"`{}` has no field `{name}`: {}",
					registration.type_info().type_path(),
					match info.field_names() {
						[] => "it has none".to_string(),
						names => format!("its fields are {}", names.join(", ")),
					}
				)
			})?;
			let field = info.field_at(index).expect("indexed above");
			if skipped.is_some_and(|data| data.is_field_skipped(index)) {
				continue;
			}
			let registration =
				registry.get(field.type_id()).ok_or_else(|| {
					bevyhow!(
						"`{}` is not registered",
						field.type_path_table().path()
					)
				})?;
			patch.insert_boxed(
				name.as_str(),
				self.read_whole(registration, registry, value)
					.map_err(|err| bevyhow!("field `{name}`: {err}"))?,
			);
		}
		patch.set_represented_type(Some(registration.type_info()));
		Ok(Box::new(patch))
	}

	/// Read `value` as a whole value of the registered type, in its serde form
	/// or as markup would have authored it.
	fn read_whole(
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
		TypedReflectDeserializer::new(registration, registry)
			.deserialize(
				ValueDeserializer::new(value.clone())
					.with_references(&reference),
			)
			.map_err(|err| {
				bevyhow!("`{}`: {err}", registration.type_info().type_path())
			})
			.or_else(|err| {
				self.read_authored(registration, registry, value)
					.map_err(|_| err)
			})
	}

	/// Read `value` as a literal markup authored.
	fn read_authored(
		&self,
		registration: &TypeRegistration,
		registry: &TypeRegistry,
		value: &Value,
	) -> Result<Box<dyn PartialReflect>> {
		LiteralReflect::resolve(
			&DataLiteral::from(value.clone()),
			Some(registration.type_info()),
			registry,
			&mut |key: &str| {
				EntityKey::parse(key)
					.ok()
					.and_then(|key| self.entity(key))
					.unwrap_or(Entity::PLACEHOLDER)
			},
		)
	}
}

/// The entities standing for references while a value serializes.
///
/// A component's serde form writes an [`Entity`] as opaque bits, and a type
/// with its own `Serialize` hides where. So each entity a value references is
/// a sentinel no world holds, and after serializing every sentinel found in
/// the result is replaced by the key it stands for.
struct Sentinel;

impl Sentinel {
	/// A generation no entity reaches in practice, so a sentinel's bits are
	/// not a number real data holds.
	const GENERATION: u32 = 0xBEE7_5CE0;

	/// The sentinel standing for the `ordinal`th target.
	fn entity(ordinal: usize) -> Entity {
		Entity::from_index_and_generation(
			EntityIndex::from_raw_u32(ordinal as u32).expect(
				"a component references fewer than `u32::MAX` entities",
			),
			EntityGeneration::from_bits(Self::GENERATION),
		)
	}

	/// The target a sentinel's bits stand for, among `count`.
	fn ordinal(bits: u64, count: usize) -> Option<usize> {
		Entity::try_from_bits(bits)
			.filter(|entity| entity.generation().to_bits() == Self::GENERATION)
			.map(|entity| entity.index_u32() as usize)
			.filter(|ordinal| *ordinal < count)
	}

	/// `value` with every sentinel among `count` replaced by its `reference`.
	fn replace(
		value: Value,
		count: usize,
		reference: &mut dyn FnMut(usize) -> Value,
	) -> Result<Value> {
		match value {
			Value::Uint(bits) => match Self::ordinal(bits, count) {
				Some(ordinal) => reference(ordinal),
				None => Value::Uint(bits),
			},
			Value::List(items) => items
				.into_iter()
				.map(|item| Self::replace(item, count, reference))
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
						.and_then(|bits| Self::ordinal(bits, count))
						.is_some()
					{
						bevybail!(
							"a map keyed by an entity reference cannot be saved"
						);
					}
					(key, Self::replace(item, count, reference)?).xok()
				})
				.collect::<Result<Map>>()?
				.xmap(Value::Map),
			other => other,
		}
		.xok()
	}
}

/// The keys a value being written references, each standing as an entity
/// until it has serialized.
#[derive(Default)]
pub(crate) struct KeySentinels(Vec<EntityKey>);

impl KeySentinels {
	/// The entity standing for `key`.
	pub fn entity(&mut self, key: EntityKey) -> Entity {
		let ordinal = self
			.0
			.iter()
			.position(|target| *target == key)
			.unwrap_or_else(|| {
				self.0.push(key);
				self.0.len() - 1
			});
		Sentinel::entity(ordinal)
	}

	/// `value` with every reference written as its key.
	pub fn keyed(&self, value: Value) -> Result<Value> {
		match self.0.is_empty() {
			true => value.xok(),
			false => Sentinel::replace(value, self.0.len(), &mut |ordinal| {
				self.0[ordinal].reference()
			}),
		}
	}
}

/// Finds the entity references in a component's serialized value, so each is
/// written as its target's key. A value that only looks like a [`Sentinel`] is
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
		let value =
			Sentinel::replace(value, self.targets.len(), &mut |ordinal| {
				found[ordinal] += 1;
				entity_map.file_key(self.targets[ordinal]).reference()
			})?;
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
		Sentinel::entity(ordinal)
	}

	fn set_mapped(&mut self, _source: Entity, _target: Entity) {}
}

#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

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

	const HEALTH: &str = "beet_scene::body::scene_reflect::test::Health";
	const SHIELD: &str = "beet_scene::body::scene_reflect::test::Shield";
	const LINKS: &str = "beet_scene::body::scene_reflect::test::Links";

	/// A world registering `Health`, `Links`, `Score` and the `Badge`
	/// template, and `Shield` when `full`.
	fn world(full: bool) -> World {
		let mut app = App::new();
		app.add_plugins((ScenePlugin, MinimalTypesPlugin))
			.install_types::<Badge>();
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
	#[beet_core::test]
	fn writes_the_wire_shape() {
		shielded().as_utf8().unwrap().xpect_snapshot();
	}

	/// Every structured format round trips through the saver and the loader.
	#[cfg(all(feature = "postcard", feature = "toml"))]
	#[beet_core::test]
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
	#[beet_core::test]
	fn a_snapshot_holds_what_a_template_built() {
		let badge = "beet_scene::body::scene_reflect::test::Badge";
		let value = "beet_core::types::value::value::Value";
		let score = "beet_scene::body::scene_reflect::test::Score";
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
	#[beet_core::test]
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
	#[beet_core::test]
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

	/// A key held by the short name markup wrote resolves by it.
	#[beet_core::test]
	fn a_short_name_resolves() {
		let mut world = world(false);
		let root = load(
			&mut world,
			&MediaBytes::new_json(
				r#"{"$type":"org.beet.scene#body","entities":{"0":{"Health":4}}}"#,
			),
		);
		world.get::<Health>(root).unwrap().xpect_eq(Health(4));
	}

	/// A registered type whose value does not read is an error naming the
	/// entity key and the type path, never a skip.
	#[beet_core::test]
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

#[cfg(test)]
mod declaration_test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A nested struct field, so the read covers a declaration that is not
	/// flat.
	#[derive(Debug, Default, Clone, PartialEq, Reflect)]
	struct Nested {
		label: Option<String>,
		order: Option<u32>,
	}

	/// A page-metadata shaped component.
	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Meta {
		title: Option<String>,
		draft: bool,
		order: Option<u32>,
		nested: Nested,
		created: Option<Date>,
		at: Option<Timestamp>,
	}

	/// A type the test world never registers.
	#[derive(Debug, Default, Clone, PartialEq, Reflect)]
	struct Foreign {
		note: Option<String>,
	}

	/// A second root component, proving a root declares whatever it holds
	/// rather than one blessed type.
	#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
	#[reflect(Component, Default)]
	struct Extra {
		note: Option<String>,
	}

	fn world() -> World {
		let world = ScenePlugin::world();
		let mut registry = world.resource::<AppTypeRegistry>().write();
		registry.register::<Meta>();
		registry.register::<Extra>();
		drop(registry);
		world
	}

	/// The root components `markup` declares, lowered against `parsers`.
	fn declared(parsers: &DocParsers, markup: &str) -> Result<EntityBody> {
		parsers.root_components(&MediaBytes::new_bsx(markup), None)
	}

	/// A root spread reads without spawning, whether the document lowered
	/// against the types or against none, as a codegen scan does.
	#[beet_core::test]
	fn reads_root_spreads() {
		let world = world();
		let registry = world.resource::<AppTypeRegistry>().read();
		for parsers in [DocParsers::of(&world), ScenePlugin::parsers()] {
			let get = |markup: &str| {
				BodyComponents::get::<Meta>(
					&declared(&parsers, markup)?,
					&registry,
				)
			};
			// the declared fields land, the rest fill from `Default`
			get(r#"<Fragment {Meta{title:"Blog", nested: Nested{order: 1}}}><h1>hi</h1></Fragment>"#)
				.unwrap()
				.unwrap()
				.xpect_eq(Meta {
					title: Some("Blog".into()),
					nested: Nested {
						label: None,
						order: Some(1),
					},
					..default()
				});
			// a `YYYY-MM-DD` string coerces to the day it names
			get(r#"<Fragment {Meta{created:"2025-09-06"}}/>"#)
				.unwrap()
				.unwrap()
				.created
				.unwrap()
				.to_string()
				.xpect_eq("2025-09-06");
			// ..while an instant takes no string at all
			get(r#"<Fragment {Meta{at:"2025-09-06"}}/>"#).xpect_err();
			// a tuple spread names it alongside others
			get(r#"<Fragment {(Meta{draft: true}, Extra)}/>"#)
				.unwrap()
				.unwrap()
				.draft
				.xpect_true();
			// only the root declares; a nested spread is content, and so is
			// the root's own element
			get(r#"<main><div {Meta{title:"nested"}}/></main>"#)
				.unwrap()
				.xpect_none();
			declared(&parsers, r#"<main class="a"/>"#)
				.unwrap()
				.is_empty()
				.xpect_true();
			get(r#"<Fragment {Meta{order:"not-a-number"}}/>"#).xpect_err();
			// a declaration that cannot resolve is loud, never a default
			let foreign =
				declared(&parsers, r#"<Fragment {Foreign{note:"hi"}}/>"#)
					.unwrap();
			BodyComponents::get::<Foreign>(&foreign, &registry)
				.unwrap_err()
				.to_string()
				.xpect_contains("absent from the registry");
		}
	}

	#[beet_core::test]
	fn inserts_every_declaration() {
		let mut world = world();
		let declared = declared(
			&DocParsers::of(&world),
			r#"<Fragment {(Meta{title:"Blog"}, Extra{note:"hi"}, Missing)}/>"#,
		)
		.unwrap();
		let entity = world.spawn_empty().id();
		BodyComponents::insert(&declared, &mut world.entity_mut(entity), None)
			.unwrap();
		world
			.get::<Meta>(entity)
			.unwrap()
			.title
			.as_deref()
			.xpect_eq(Some("Blog"));
		world
			.get::<Extra>(entity)
			.unwrap()
			.note
			.as_deref()
			.xpect_eq(Some("hi"));
	}

	/// A filename-implied default fills a field the document left out, and never
	/// invents a declaration the document never made.
	#[beet_core::test]
	fn declare_default_fills_undeclared_fields() {
		let world = world();
		let registry = world.resource::<AppTypeRegistry>().read();
		let mut body = declared(
			&DocParsers::of(&world),
			r#"<Fragment {Meta{title:"Blog"}}/>"#,
		)
		.unwrap();
		body.declare_default("Meta", "order", Value::Uint(3));
		// an explicitly declared field wins over the default
		body.declare_default("Meta", "title", Value::str("other"));
		let meta = BodyComponents::get::<Meta>(&body, &registry)
			.unwrap()
			.unwrap();
		meta.order.unwrap().xpect_eq(3);
		meta.title.as_deref().unwrap().xpect_eq("Blog");
		// an undeclared component stays undeclared
		let mut none = EntityBody::default();
		none.declare_default("Meta", "order", Value::Uint(3));
		BodyComponents::get::<Meta>(&none, &registry)
			.unwrap()
			.xpect_none();
	}
}
