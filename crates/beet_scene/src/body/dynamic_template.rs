//! The [`DynamicTemplate`] intermediate representation and its build path.
//!
//! A `DynamicTemplate` is the whole: an ordered list of resources and an ordered
//! list of entities. Each entity corresponds to an entity and carries an ordered list
//! of component slots, each either a resolved value or a deferred template. A
//! fully resolved save-game and a hand-authored page are the same kind of thing
//! at different points on one axis: how much is already a value versus deferred.
//!
//! `DynamicTemplate` is itself a [`Template`], so the single instantiation path
//! ([`spawn_template`](beet_core::prelude::WorldTemplateExt::spawn_template) /
//! [`insert_template`](beet_core::prelude::EntityWorldMutTemplateExt::insert_template))
//! builds it. Building walks the entities in order, mapping each in-template
//! [`Entity`] to a real world entity through the one entity model
//! ([`SceneEntityReferences`]), applying value slots and building deferred slots
//! through the [`ReflectTemplate`](beet_core::prelude::ReflectTemplate) registry.
//!
//! # Entity model
//!
//! An entity of the template is named by its position: a slot references
//! another with the in-template [`Entity`] of that position
//! ([`DynamicTemplate::reference`]), which is keyed to a deterministic
//! [`SceneEntityReference`], so the same position always resolves to the same
//! real entity within one build. The first entity is pinned to the walker's
//! root; the rest, and any `Entity`-typed field, resolve through
//! [`SceneEntityReferences::get`], which spawns a placeholder on first lookup
//! and is filled when that entity is built. This is the single remapping
//! model: cross-entity references and forward references resolve uniformly.

use crate::prelude::*;
use alloc::sync::Arc;
use beet_core::prelude::*;
use bevy::ecs::component::ComponentCloneBehavior;
use bevy::ecs::entity::EntityMapper;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::ecs::reflect::ReflectComponent;
use bevy::ecs::reflect::ReflectResource;
use bevy::ecs::relationship::RelationshipHookMode;
use bevy::ecs::template::SceneEntityReference;
use bevy::ecs::template::SceneEntityReferences;
use bevy::ecs::template::Template;
use bevy::ecs::template::TemplateContext;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectMut;
use bevy_reflect::ReflectRef;
use bevy_reflect::TypeRegistration;
use bevy_reflect::TypeRegistry;

/// A scene reflected against a type registry: ordered resources plus an ordered
/// list of entities, each slot a reflectable value or a named deferred
/// template. What a [`SceneBody`] spawns through ([`DynamicTemplate::reflect`]).
///
/// Every document spawns through one: a parser of any format returns a
/// [`SceneBody`], and building it reflects it here. Each build reflects its
/// own, since a default is minted as a value is reflected (a thread's id) and
/// every build's is its own.
///
/// It builds a subtree into the world, so it is a [`Template`]. The first entity
/// builds into the context entity (the root); the rest spawn. See the module
/// docs for the entity model and the children-order contract.
#[derive(Default)]
pub struct DynamicTemplate {
	/// Resources to write into the world, before any entity builds, so what an
	/// entity does as it lands reads them.
	pub resources: Vec<Box<dyn PartialReflect>>,
	/// Entities in build order. The first entity becomes the root, building into the
	/// context entity; the rest spawn.
	pub entities: Vec<DynamicTemplateEntity>,
	/// What it was read from, so what it builds answers where it was written.
	pub source: Option<Arc<DocumentSource>>,
}

// `DynamicTemplate` is not `Clone` (a boxed `PartialReflect` is not), so it
// cannot pick up Bevy's blanket `Template for T: Clone + Unpin`. The
// opt-out documents the intent and guards against a future `Clone`.
subtree_template!(DynamicTemplate);

/// A transient sink the loader installs to capture the `(file key, real entity)`
/// pairing a [`DynamicTemplate`] build produces for each entity, in entity order.
///
/// The loader inserts it before building and drains it after, so it learns every
/// spawned entity without a second remapping model, and retains the pairs as the
/// document's [`TemplateEntityMap`]. Absent for a plain `spawn_template`, where
/// the caller only needs the root.
#[derive(Default, Resource)]
pub(crate) struct TemplateBuildSink(pub Vec<(EntityKey, Entity)>);

/// One entity of a [`DynamicTemplate`]: its file key and its ordered component
/// slots.
pub struct DynamicTemplateEntity {
	/// The key the entity is written under in its document, which the loader
	/// retains so a re-save writes it back.
	pub key: EntityKey,
	/// The component slots, in order. A slot referencing an entity of this
	/// template (a relationship target, an `Entity`-typed field) names it by
	/// position, [`DynamicTemplate::reference`].
	pub components: Vec<ComponentSlot>,
}

/// One component slot on an entity: a resolved value or a deferred template.
pub enum ComponentSlot {
	/// A concrete component value, the save-game form. Applied by reflection over
	/// the inserted-or-defaulted component.
	Value(Box<dyn PartialReflect>),
	/// A registered template carried by name plus its reflected patch, resolved
	/// at build time through the
	/// [`ReflectTemplate`](beet_core::prelude::ReflectTemplate) registry. This is how
	/// a function component or any world-context value is carried before it is
	/// built.
	Template(DeferredTemplate),
	/// A patch over a live resource, which a document declares where it
	/// stands: the fields it holds are written and the rest keep their values,
	/// an absent resource inserting over its type's default. The entity holds
	/// nothing of it.
	Resource(Box<dyn PartialReflect>),
}

/// A deferred template slot: a template invocation, the registered
/// template's type path and the reflected patch to build it from, expanded
/// when the slot builds and never before (design: a scene is never target
/// specific through a baked expansion).
///
/// A template is a builder: it runs once and is consumed, so a save of the
/// world it built into writes what it built and nothing of the invocation.
pub struct DeferredTemplate {
	/// The registered template's type path, full or short, the registry
	/// lookup key.
	pub name: SmolStr,
	/// The reflected patch over the template's default, typically a
	/// `DynamicStruct` carrying only the provided fields.
	pub patch: Box<dyn PartialReflect>,
}

impl Template for DynamicTemplate {
	type Output = ();

	/// Builds this template into `cx`: maps every in-template entity to a real
	/// entity, applies each entity's component slots, then writes resources.
	///
	/// The first entity builds into `cx.entity` (the walker's root); the rest
	/// resolve through [`SceneEntityReferences`], which spawns a real entity on
	/// first lookup, so cross-entity and forward references resolve through the
	/// one entity model.
	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		let app_registry = cx.entity.resource::<AppTypeRegistry>().clone();
		let registry = app_registry.read();

		// pin the root: the first entity's in-template entity maps to cx.entity, so
		// its components land on the walker's root rather than a fresh spawn.
		let root = cx.entity.id();
		if !self.entities.is_empty() {
			cx.entity_references
				.set(scene_reference(Self::reference(0)), root);
		}
		// the build walker exposes the template root via `TemplateBuildRoot`, so a
		// nested asset/remote value template parks its pending dependency on it.
		self.build_entities(&registry, &app_registry, cx)
	}

	fn clone_template(&self) -> Self {
		unreachable!("DynamicTemplate is built once, never cloned")
	}
}

impl DynamicTemplate {
	/// The in-template [`Entity`] naming the entity at `position`, what a slot
	/// holds to reference it. A position past the last names an entity outside
	/// the template, which the build spawns empty.
	pub fn reference(position: usize) -> Entity {
		Entity::from_raw_u32(position as u32)
			.expect("a template holds fewer than `u32::MAX` entities")
	}

	/// The documents this instantiates, as its [`Instance`]s name them: what
	/// must be loaded for it to build whole. A held template's count, since
	/// its holder builds it when it likes.
	pub fn instances(&self) -> Vec<RelPath> {
		let mut found = Vec::new();
		for slot in self.slots() {
			InstanceVisitor { found: &mut found }.visit(slot.value());
		}
		found.dedup();
		found
	}

	/// The props the document declares, its [`Prop`] entities.
	pub fn props(&self) -> Vec<Prop> {
		self.slots()
			.filter_map(|slot| match slot {
				ComponentSlot::Value(value) => value
					.try_as_reflect()
					.and_then(|value| value.downcast_ref::<Prop>())
					.cloned(),
				_ => None,
			})
			.collect()
	}

	fn slots(&self) -> impl Iterator<Item = &ComponentSlot> {
		self.entities
			.iter()
			.flat_map(|entity| entity.components.iter())
	}

	/// Build into `entity`, its root, answering the key of every entity it
	/// built: one use of a scene held as data, each use its own entities.
	pub fn build_keyed(
		&self,
		entity: &mut EntityWorldMut,
	) -> Result<TemplateEntityMap> {
		let outer = entity.world_scope(|world| {
			let outer = world.remove_resource::<TemplateBuildSink>();
			world.insert_resource(TemplateBuildSink::default());
			outer
		});
		let result = entity.build_template(self);
		let pairs = entity.world_scope(|world| {
			let pairs = world
				.remove_resource::<TemplateBuildSink>()
				.map(|sink| sink.0)
				.unwrap_or_default();
			if let Some(outer) = outer {
				world.insert_resource(outer);
			}
			pairs
		});
		result?;
		TemplateEntityMap::from_pairs(pairs).xok()
	}

	/// Build this onto `entity`, its root the entity itself and its file keys
	/// retained there, then extend the tree in one [`PostParseTree`] run: what
	/// [`ParseContext::parse`] does with the body it read.
	pub fn build_document(self, entity: &mut EntityWorldMut) -> Result {
		let id = entity.id();
		entity.world_scope(|world| {
			TemplateLoader::new(world).with_entity(id).build_onto(self)
		})?;
		// the tree is extended once, whatever its format: its tables numbered,
		// then whatever else is registered, ie styles or a deck's triage
		entity.insert(ParsedRoot);
		entity.world_scope(|world| {
			let _ = world.try_run_schedule(PostParseTree);
		});
		entity.remove::<ParsedRoot>();
		Ok(())
	}

	/// Rewrite every [`DocPath`] this holds package absolute, as the document
	/// at `document`.
	pub(super) fn resolve_paths(&mut self, document: &RelPath) {
		for entity in &mut self.entities {
			for slot in &mut entity.components {
				let value = match slot {
					ComponentSlot::Value(value)
					| ComponentSlot::Resource(value) => value,
					ComponentSlot::Template(template) => &mut template.patch,
				};
				DocPathResolver { document }.visit(value.as_mut());
			}
		}
	}

	/// Writes resources, then builds every entity onto its mapped real entity,
	/// in order.
	///
	/// File order plus in-order `ChildOf` application (relationship hooks run) is
	/// what preserves `Children` order across a round-trip.
	fn build_entities(
		&self,
		registry: &TypeRegistry,
		app_registry: &AppTypeRegistry,
		cx: &mut TemplateContext,
	) -> Result<()> {
		{
			// SAFETY: only used to write resources into the world.
			let world = unsafe { cx.entity.world_mut() };
			let (world, mut mapper) =
				ReferenceMapper::split(world, cx.entity_references);
			for resource in &self.resources {
				write_resource(
					world,
					resource.as_ref(),
					&mut mapper,
					registry,
				)?;
			}
		}
		// before anything lands, which may make the root a use site itself
		let root = cx.entity.id();
		let use_site = use_site(cx.entity.world(), root);
		// this build's own, so what it nests (a template building a document)
		// records nothing in it
		// SAFETY: only used to take the sink resource.
		let sink = unsafe { cx.entity.world_mut() }
			.remove_resource::<TemplateBuildSink>();
		let mut built = Vec::with_capacity(self.entities.len());
		for (position, entity) in self.entities.iter().enumerate() {
			// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
			let world = unsafe { cx.entity.world_mut() };
			let entity_id = cx
				.entity_references
				.get(scene_reference(Self::reference(position)), world);
			build_entity(entity, entity_id, registry, app_registry, cx)?;
			built.push(entity_id);
		}
		// SAFETY: only used to finish the entities just built.
		let world = unsafe { cx.entity.world_mut() };
		// record the real entities for the loader, if it installed a sink
		if let Some(mut sink) = sink {
			sink.0.extend(
				self.entities
					.iter()
					.map(|entity| entity.key.clone())
					.zip(built.iter().copied()),
			);
			world.insert_resource(sink);
		}
		if let Some(source) = &self.source {
			SceneSource::land(
				world,
				root,
				source.clone(),
				self.entities
					.iter()
					.map(|entity| entity.key.clone())
					.zip(built.iter().copied()),
			);
		}
		settle_props(world, root, use_site, &built);
		self.forward_attributes(world, &built)?;
		hold_prefabs(world, &built)
	}

	/// Settle the attributes this document wrote on an entity a template
	/// built: each belongs to the root element the template built there, a
	/// `class` joining the element's own and any other key replacing its
	/// value. A template that built no element there has nowhere to put one,
	/// an error naming it.
	fn forward_attributes(
		&self,
		world: &mut World,
		built: &[Entity],
	) -> Result {
		let templates = self
			.entities
			.iter()
			.zip(built)
			.filter_map(|(entity, id)| {
				entity.components.iter().find_map(|slot| match slot {
					ComponentSlot::Template(template) => {
						Some((*id, &template.name))
					}
					_ => None,
				})
			})
			.collect::<HashMap<_, _>>();
		if templates.is_empty() {
			return OK;
		}
		for attribute in built {
			let Some((owner, template)) = world
				.get::<AttributeOf>(*attribute)
				.and_then(|owner| templates.get_key_value(&owner.get()))
			else {
				continue;
			};
			let Some(key) = world.get::<Attribute>(*attribute).cloned() else {
				continue;
			};
			if !world.entity(*owner).contains::<Element>() {
				bevybail!(
					"`<{}>` has no prop `{}` and builds no single root element to forward it to",
					TypeName::new((*template).clone()).short(),
					key.as_str()
				);
			}
			if key.as_str() == "class" {
				continue;
			}
			let replaced = world
				.get::<Attributes>(*owner)
				.into_iter()
				.flat_map(|attributes| attributes.iter())
				.filter(|other| {
					!built.contains(other)
						&& world.get::<Attribute>(*other) == Some(&key)
				})
				.collect::<Vec<_>>();
			for other in replaced {
				world.despawn(other);
			}
		}
		OK
	}
}

/// The use site a document building at `root` was used at, if any.
fn use_site(world: &World, root: Entity) -> Option<Entity> {
	ElementTraverseQuery::world_ancestors_inclusive(world, root)
		.skip(1)
		.find(|entity| world.entity(*entity).contains::<PropsDocument>())
}

/// Settle every [`DocumentPath::Props`] binding among `built` to `use_site`, so
/// one slotted into another document reads the props of the document that
/// wrote it.
///
/// A document no use instantiates, ie a page or an entry, was passed nothing,
/// so its bindings read the defaults of its own [`Prop`]s, held by an entity
/// related to `root` so it never renders.
fn settle_props(
	world: &mut World,
	root: Entity,
	use_site: Option<Entity>,
	built: &[Entity],
) {
	let bound = built
		.iter()
		.copied()
		.filter(|entity| {
			world.get_entity(*entity).is_ok_and(|entity| {
				let field = entity.get::<FieldRef>().map(|it| &it.document);
				let source =
					entity.get::<SourceFieldRef>().map(|it| &it.document);
				[field, source].contains(&Some(&DocumentPath::Props))
			})
		})
		.collect::<Vec<_>>();
	if bound.is_empty() {
		return;
	}
	let props = use_site.unwrap_or_else(|| {
		let defaults = built
			.iter()
			.filter_map(|entity| world.get::<Prop>(*entity))
			.filter_map(|prop| {
				prop.default
					.clone()
					.map(|default| (prop.name.clone(), default))
			})
			.collect::<Map>();
		world
			.spawn((
				AttributeOf::new(root),
				Document::new(Value::Map(defaults)),
				PropsDocument,
			))
			.id()
	});
	for entity in bound {
		let mut entity = world.entity_mut(entity);
		if let Some(field) = entity
			.get::<FieldRef>()
			.filter(|field| field.document == DocumentPath::Props)
			.cloned()
		{
			entity.insert(field.with_document(DocumentPath::Entity(props)));
		}
		if let Some(mut source) = entity.get_mut::<SourceFieldRef>()
			&& source.document == DocumentPath::Props
		{
			source.document = DocumentPath::Entity(props);
		}
	}
}

/// Hold every prefab among `built`, its own content's slots filled first, so
/// nothing of it shows or runs where it is held.
fn hold_prefabs(world: &mut World, built: &[Entity]) -> Result {
	let prefabs = built
		.iter()
		.copied()
		.filter(|entity| {
			world
				.get_entity(*entity)
				.is_ok_and(|entity| entity.contains::<Prefab>())
		})
		.collect::<Vec<_>>();
	for prefab in prefabs {
		SlotTarget::resolve_all(world, prefab)?;
		Prefab::hold(world, prefab);
	}
	OK
}

impl ComponentSlot {
	/// The value the slot builds from.
	fn value(&self) -> &dyn PartialReflect {
		match self {
			Self::Value(value) | Self::Resource(value) => value.as_ref(),
			Self::Template(template) => template.patch.as_ref(),
		}
	}
}

/// Collects the documents a reflected value's [`Instance`]s name.
struct InstanceVisitor<'a> {
	found: &'a mut Vec<RelPath>,
}

impl InstanceVisitor<'_> {
	fn visit(&mut self, value: &dyn PartialReflect) {
		if let Some(value) = value.try_as_reflect() {
			if let Some(instance) = value.downcast_ref::<Instance>() {
				self.found.push(instance.document.rel_path());
				return;
			}
			if let Some(held) = value.downcast_ref::<HeldTemplate>() {
				self.found.extend(held.instances());
				return;
			}
		}
		match value.reflect_ref() {
			ReflectRef::Struct(value) => {
				for field in (0..value.field_len())
					.filter_map(|index| value.field_at(index))
				{
					self.visit(field);
				}
			}
			ReflectRef::TupleStruct(value) => {
				for field in value.iter_fields() {
					self.visit(field);
				}
			}
			ReflectRef::List(value) => {
				for item in value.iter() {
					self.visit(item);
				}
			}
			ReflectRef::Enum(value) => {
				for field in value.iter_fields() {
					self.visit(field.value());
				}
			}
			_ => {}
		}
	}
}

/// Rewrites every [`DocPath`] of a reflected value package absolute.
pub(super) struct DocPathResolver<'a> {
	pub(super) document: &'a RelPath,
}

impl DocPathResolver<'_> {
	pub(super) fn visit(&self, value: &mut dyn PartialReflect) {
		if let Some(value) = value.try_as_reflect_mut() {
			if let Some(path) = value.downcast_mut::<DocPath>() {
				*path = path.resolve(self.document).into();
				return;
			}
			// a held template is a document of its own, resolved as it builds
			if let Some(held) = value.downcast_mut::<HeldTemplate>() {
				held.set_document(self.document);
				return;
			}
		}
		match value.reflect_mut() {
			ReflectMut::Struct(value) => {
				for index in 0..value.field_len() {
					value.field_at_mut(index).map(|field| self.visit(field));
				}
			}
			ReflectMut::TupleStruct(value) => {
				for index in 0..value.field_len() {
					value.field_mut(index).map(|field| self.visit(field));
				}
			}
			ReflectMut::Tuple(value) => {
				for index in 0..value.field_len() {
					value.field_mut(index).map(|field| self.visit(field));
				}
			}
			ReflectMut::List(value) => {
				for index in 0..value.len() {
					value.get_mut(index).map(|item| self.visit(item));
				}
			}
			ReflectMut::Array(value) => {
				for index in 0..value.len() {
					value.get_mut(index).map(|item| self.visit(item));
				}
			}
			ReflectMut::Enum(value) => {
				for index in 0..value.field_len() {
					value.field_at_mut(index).map(|field| self.visit(field));
				}
			}
			_ => {}
		}
	}
}

/// Builds one entity's component slots onto its mapped real entity.
///
/// Value slots apply by reflection over the inserted-or-defaulted component,
/// remapping any `Entity`-typed field through the shared entity map. Deferred
/// slots build through the registry by name, into a [`TemplateContext`] scoped
/// to this entity that shares the same reference map.
fn build_entity(
	entity: &DynamicTemplateEntity,
	entity_id: Entity,
	registry: &TypeRegistry,
	app_registry: &AppTypeRegistry,
	cx: &mut TemplateContext,
) -> Result<()> {
	for slot in &entity.components {
		match slot {
			ComponentSlot::Value(value) => {
				// SAFETY: only used to apply a component onto the mapped entity.
				let world = unsafe { cx.entity.world_mut() };
				// what a body's reflection held is inserted as it stands, so
				// holding needs no registration of the marker itself, a name
				// this world minted as a runtime component inserted as one
				if let Some(unregistered) = value
					.try_as_reflect()
					.and_then(|value| value.downcast_ref::<Unregistered>())
				{
					DynamicComponents::insert_held(
						&mut world.entity_mut(entity_id),
						unregistered.clone(),
					);
					continue;
				}
				let (world, mut mapper) =
					ReferenceMapper::split(world, cx.entity_references);
				insert_value(
					world,
					entity_id,
					value.as_ref(),
					&mut mapper,
					registry,
				)?;
			}
			ComponentSlot::Resource(patch) => {
				// SAFETY: only used to patch the resource, entity backed
				// itself, never the entity being built.
				let world = unsafe { cx.entity.world_mut() };
				patch_resource(world, patch.as_ref(), registry)?;
			}
			ComponentSlot::Template(deferred) => {
				// build the deferred template into a context scoped to this
				// entity, sharing the reference map so cross-entity refs resolve.
				// SAFETY: only used to acquire the mapped entity for a scoped build.
				let world = unsafe { cx.entity.world_mut() };
				let mut entity = world.entity_mut(entity_id);
				let mut scoped =
					TemplateContext::new(&mut entity, cx.entity_references);
				ReflectTemplate::build_named(
					app_registry,
					&deferred.name,
					deferred.patch.as_ref(),
					&mut scoped,
				)?;
			}
		}
	}
	Ok(())
}

/// The deterministic [`SceneEntityReference`] for an in-template entity, keyed on
/// its index at this source location, so a reference is traceable when debugging.
fn scene_reference(in_template: Entity) -> SceneEntityReference {
	SceneEntityReference::new(
		(file!(), 0, 0),
		in_template.index_u32() as usize,
		0,
	)
}

/// An [`EntityMapper`] backed by the build's [`SceneEntityReferences`], so an
/// `Entity`-typed component field serialized as an in-template id resolves to the
/// real (possibly forward-referenced) entity. The single remapping model.
struct ReferenceMapper<'a> {
	references: &'a mut SceneEntityReferences,
	world: &'a mut World,
}

impl<'a> ReferenceMapper<'a> {
	/// The world to apply into and a mapper spawning placeholders in that same
	/// world for any `Entity`-typed field met on the way.
	///
	/// SAFETY: the mapper only spawns/looks up entities and never touches the
	/// component being applied, so the two borrows do not alias the same data.
	fn split(
		world: &'a mut World,
		references: &'a mut SceneEntityReferences,
	) -> (&'a mut World, Self) {
		let mapper_world = unsafe { &mut *(world as *mut World) };
		(world, Self {
			references,
			world: mapper_world,
		})
	}
}

impl EntityMapper for ReferenceMapper<'_> {
	fn get_mapped(&mut self, source: Entity) -> Entity {
		// a placeholder names no entity, so none is spawned for it
		if source == Entity::PLACEHOLDER {
			return source;
		}
		self.references.get(scene_reference(source), self.world)
	}

	fn set_mapped(&mut self, source: Entity, target: Entity) {
		self.references.set(scene_reference(source), target);
	}
}

/// Inserts a reflected component value onto `entity` as it builds, remapping
/// `Entity`-typed fields through `mapper`. An insert rather than a patch: a
/// component a template built or another requires is replaced, and whatever
/// watches its insertion sees the value the document wrote.
fn insert_value(
	world: &mut World,
	entity: Entity,
	value: &dyn PartialReflect,
	mapper: &mut dyn EntityMapper,
	registry: &TypeRegistry,
) -> Result<()> {
	let registration = type_registration(registry, value)?;
	let reflect_component = reflect_component(registry, value)?;
	// a value that clones to its own type is mapped and inserted whole; one
	// that cannot is applied as a load does
	let Ok(mut owned) = reflect_ext::clone_reflect_value(value, registration)
	else {
		return apply_value(world, entity, value, mapper, registry);
	};
	let Some(reflect) = owned.try_as_reflect_mut() else {
		return apply_value(world, entity, value, mapper, registry);
	};
	if is_clone_ignored(world, reflect_component) {
		return OK;
	}
	let references = references(reflect.as_partial_reflect());
	let mut mapper = CountingMapper { mapper, mapped: 0 };
	reflect_component.map_entities(reflect, &mut mapper);
	if mapper.mapped < references {
		bevybail!(
			"`{}` holds an entity its `map_entities` never visits, so a reference in it binds to nothing: mark the field `#[entities]`",
			registration.type_info().type_path()
		);
	}
	reflect_component.insert(
		&mut world.entity_mut(entity),
		owned.as_ref(),
		registry,
	);
	OK
}

/// How many entities a reflected value references.
fn references(value: &dyn PartialReflect) -> usize {
	if let Some(entity) = value.try_downcast_ref::<Entity>() {
		return usize::from(*entity != Entity::PLACEHOLDER);
	}
	match value.reflect_ref() {
		ReflectRef::Struct(value) => value
			.iter_fields()
			.map(|(_, field)| references(field))
			.sum(),
		ReflectRef::TupleStruct(value) => {
			value.iter_fields().map(references).sum()
		}
		ReflectRef::Tuple(value) => value.iter_fields().map(references).sum(),
		ReflectRef::List(value) => value.iter().map(references).sum(),
		ReflectRef::Array(value) => value.iter().map(references).sum(),
		ReflectRef::Map(value) => value
			.iter()
			.map(|(key, value)| references(key) + references(value))
			.sum(),
		ReflectRef::Set(value) => value.iter().map(references).sum(),
		ReflectRef::Enum(value) => value
			.iter_fields()
			.map(|field| references(field.value()))
			.sum(),
		_ => 0,
	}
}

/// An [`EntityMapper`] counting what it is asked to map, so a component's own
/// `map_entities` is held to every reference it holds.
struct CountingMapper<'a> {
	mapper: &'a mut dyn EntityMapper,
	mapped: usize,
}

impl EntityMapper for CountingMapper<'_> {
	fn get_mapped(&mut self, source: Entity) -> Entity {
		self.mapped += 1;
		self.mapper.get_mapped(source)
	}

	fn set_mapped(&mut self, source: Entity, target: Entity) {
		self.mapper.set_mapped(source, target);
	}
}

/// Writes a reflected patch over the live resource of its type, the fields it
/// lacks keeping their values, or inserts it over the type's default, tied to
/// the build root creating it ([`SceneResource`]).
fn patch_resource(
	world: &mut World,
	patch: &dyn PartialReflect,
	registry: &TypeRegistry,
) -> Result {
	use bevy::ecs::reflect::ReflectFromWorld;
	use bevy_reflect::ReflectFromReflect;
	use bevy_reflect::std_traits::ReflectDefault;
	let registration = type_registration(registry, patch)?;
	let reflect_resource = reflect_resource(registry, patch)?;
	let component_id = reflect_resource.register_component(world);
	if let Some(entity) = world.resource_entities().get(component_id) {
		reflect_resource.apply(&mut world.entity_mut(entity), patch);
		return OK;
	}
	// inserting an unconstructible type panics, so it is refused here
	let constructible = registration.data::<ReflectDefault>().is_some()
		|| registration.data::<ReflectFromWorld>().is_some()
		|| registration.data::<ReflectFromReflect>().is_some_and(
			|from_reflect| from_reflect.from_reflect(patch).is_some(),
		);
	if !constructible {
		let type_path = registration.type_info().type_path();
		bevybail!(
			"the resource `{type_path}` is not in the world and cannot be constructed from its fields, add `#[reflect(Default)]` or insert the resource first"
		);
	}
	// created by the document: its scene's teardown, and only that scene's,
	// removes the resource with it
	let root = world.get_resource::<TemplateBuildRoot>().map(|root| **root);
	let entity = world.spawn_empty().id();
	reflect_resource.insert(&mut world.entity_mut(entity), patch, registry);
	if let Some(root) = root {
		world.entity_mut(entity).insert(SceneResource { root });
	}
	OK
}

/// Marks the backing entity of a resource a document declared, recording the
/// template build root that created it, so scene teardown (`despawn_scene`)
/// removes the resource *with that scene* and a rebuild reinserts it fresh from
/// markup rather than patching stale live state.
///
/// The root is carried rather than implied so teardown is scoped: with two scenes
/// loaded, despawning one must not take the other's resources with it. A
/// teardown removes only the resources whose root is among the roots it is
/// despawning, and treats a resource whose root has already gone as orphaned (it
/// goes too, since nothing can rebuild it).
///
/// Only set when the document *creates* the resource inside a template build; a
/// resource that pre-existed (a plugin default) keeps its own lifetime and the
/// document only patches it. Edge case: two scenes declaring the same resource
/// share one backing entity (the first creates and owns it, the second only
/// patches), so tearing down the creating scene removes the resource for both.
#[derive(Debug, Component, Reflect)]
#[reflect(Component)]
pub struct SceneResource {
	/// The template build root whose scene created this resource.
	pub root: Entity,
}

/// Applies a reflected component value onto `entity`, remapping `Entity`-typed
/// fields through `mapper` and running relationship hooks so `Children` and
/// other relationship mirrors rebuild in order.
///
/// In place: a mutable component is patched where it sits, so the entity, its
/// other components and any runtime state they hold survive; an immutable one
/// (a relationship) is reinserted, which is what reparents.
pub(super) fn apply_value(
	world: &mut World,
	entity: Entity,
	value: &dyn PartialReflect,
	mapper: &mut dyn EntityMapper,
	registry: &TypeRegistry,
) -> Result<()> {
	let reflect_component = reflect_component(registry, value)?;

	if is_clone_ignored(world, reflect_component) {
		return OK;
	}
	// `RelationshipHookMode::Run` so applying `ChildOf` rebuilds the parent's
	// ordered `Children`, the children-order contract.
	reflect_component.apply_or_insert_mapped(
		&mut world.entity_mut(entity),
		value,
		registry,
		mapper,
		RelationshipHookMode::Run,
	);
	Ok(())
}

/// Whether a component is never cloned (eg observers), so must not be written.
fn is_clone_ignored(
	world: &mut World,
	reflect_component: &ReflectComponent,
) -> bool {
	let component_id = reflect_component.register_component(world);
	world
		.components()
		.get_info(component_id)
		.is_some_and(|info| {
			matches!(info.clone_behavior(), ComponentCloneBehavior::Ignore)
		})
}

/// Writes a reflected resource value into the world, remapping `Entity`-typed
/// fields through `mapper`.
pub(super) fn write_resource(
	world: &mut World,
	value: &dyn PartialReflect,
	mapper: &mut dyn EntityMapper,
	registry: &TypeRegistry,
) -> Result<()> {
	let reflect_resource = reflect_resource(registry, value)?;
	let resource_id = reflect_resource.register_component(world);
	let entity = world
		.resource_entities()
		.get(resource_id)
		.unwrap_or_else(|| world.spawn_empty().id());
	reflect_resource.apply_or_insert_mapped(
		&mut world.entity_mut(entity),
		value,
		registry,
		mapper,
		RelationshipHookMode::Run,
	);
	Ok(())
}

/// Resolve the [`ReflectComponent`] for a reflected component value, erroring if
/// its type is missing a represented type, unregistered, or not a component.
pub(super) fn reflect_component<'a>(
	registry: &'a TypeRegistry,
	value: &dyn PartialReflect,
) -> Result<&'a ReflectComponent> {
	let registration = type_registration(registry, value)?;
	registration.data::<ReflectComponent>().ok_or_else(|| {
		bevyhow!(
			"template contains `{}`, which is registered but reflects no \
			component: add `#[reflect(Component)]`",
			registration.type_info().type_path()
		)
	})
}

/// Resolve the [`ReflectComponent`] backing a reflected resource value.
pub(super) fn reflect_resource<'a>(
	registry: &'a TypeRegistry,
	value: &dyn PartialReflect,
) -> Result<&'a ReflectComponent> {
	let registration = type_registration(registry, value)?;
	if registration.data::<ReflectResource>().is_none() {
		bevybail!(
			"template contains the unregistered resource `{}`",
			registration.type_info().type_path()
		);
	}
	// ReflectResource existing implies ReflectComponent also exists.
	registration
		.data::<ReflectComponent>()
		.expect("ReflectComponent is depended on by ReflectResource")
		.xok()
}

/// Resolve a reflected value's [`TypeRegistration`], erroring with guidance.
fn type_registration<'a>(
	registry: &'a TypeRegistry,
	value: &dyn PartialReflect,
) -> Result<&'a TypeRegistration> {
	let Some(type_info) = value.get_represented_type_info() else {
		bevybail!(
			"template contains dynamic type `{}` without a represented type, \
			consider setting it with `set_represented_type`",
			value.reflect_type_path()
		);
	};
	registry.get(type_info.type_id()).ok_or_else(|| {
		bevyhow!(
			"template contains the reflected type `{}` but it was not found in \
			the type registry, consider registering it with \
			`app.register_type::<T>()`",
			type_info.type_path()
		)
	})
}

#[cfg(test)]
mod test {
	use super::*;
	use bevy::reflect::structs::DynamicStruct;

	#[derive(Component, Reflect, Default, PartialEq, Debug)]
	#[reflect(Component)]
	struct Score(u32);

	/// A component holding a forward reference to another in-template entity.
	#[derive(Component, Reflect, MapEntities, Debug)]
	#[reflect(Component, MapEntities)]
	struct Target(#[entities] Entity);

	impl Default for Target {
		fn default() -> Self { Self(Entity::PLACEHOLDER) }
	}

	/// A template registered by name that spawns a child labelled by its field,
	/// reading the field from its reflected patch. Exercises the deferred slot.
	#[derive(Default, Clone, Reflect)]
	#[reflect(Default)]
	struct Label {
		text: String,
	}
	subtree_template!(Label);
	impl GetTemplateSchema for Label {}
	impl Template for Label {
		type Output = ();
		fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
			let text = self.text.clone();
			let root = cx.entity.id();
			// SAFETY: only used to spawn an unrelated child entity.
			let world = unsafe { cx.entity.world_mut() };
			world.spawn((Name::new(text), ChildOf(root)));
			OK
		}
		fn clone_template(&self) -> Self { self.clone() }
	}

	fn world() -> World {
		let mut app = App::new();
		app.add_plugins(ScenePlugin).install_types::<Label>();
		let world = core::mem::take(app.world_mut());
		{
			let registry = world.resource::<AppTypeRegistry>();
			let mut registry = registry.write();
			registry.register::<Score>();
			registry.register::<Target>();
			registry.register::<Name>();
			registry.register::<ChildOf>();
		}
		world
	}

	/// A value slot from a concrete component, via `to_dynamic`.
	fn value(component: impl bevy_reflect::PartialReflect) -> ComponentSlot {
		ComponentSlot::Value(component.to_dynamic().unwrap())
	}

	/// The entity at `position`, holding `components`.
	fn entity(
		position: u32,
		components: Vec<ComponentSlot>,
	) -> DynamicTemplateEntity {
		DynamicTemplateEntity {
			key: EntityKey::from(position),
			components,
		}
	}

	#[beet_core::test]
	fn builds_value_entity_onto_root() {
		let mut world = world();
		let template = DynamicTemplate {
			resources: vec![],
			source: None,
			entities: vec![entity(0, vec![value(Score(7))])],
		};
		let root = world.spawn_template(template).unwrap().id();
		world
			.entity(root)
			.get::<Score>()
			.unwrap()
			.xpect_eq(Score(7));
	}

	#[beet_core::test]
	fn builds_children_in_order() {
		let mut world = world();
		// root entity 0, three children 1,2,3, each holding ChildOf(0).
		let parent = DynamicTemplate::reference(0);
		let mut entities = vec![entity(0, vec![])];
		for (index, label) in ["a", "b", "c"].into_iter().enumerate() {
			entities.push(entity(index as u32 + 1, vec![
				value(ChildOf(parent)),
				value(Name::new(label)),
			]));
		}
		let root = world
			.spawn_template(DynamicTemplate {
				resources: vec![],
				source: None,
				entities,
			})
			.unwrap()
			.id();

		let children: Vec<String> = world
			.entity(root)
			.get::<Children>()
			.unwrap()
			.iter()
			.map(|child| world.entity(child).get::<Name>().unwrap().to_string())
			.collect();
		children.xpect_eq(vec![
			"a".to_string(),
			"b".to_string(),
			"c".to_string(),
		]);
	}

	#[beet_core::test]
	fn resolves_forward_reference() {
		let mut world = world();
		// entity 0 holds Target(1), a forward reference to entity 1 which carries a
		// recognisable Name. The placeholder spawned on first lookup is filled
		// when entity 1 builds.
		let template = DynamicTemplate {
			resources: vec![],
			source: None,
			entities: vec![
				entity(0, vec![value(Target(DynamicTemplate::reference(1)))]),
				entity(1, vec![value(Name::new("referent"))]),
			],
		};
		let root = world.spawn_template(template).unwrap().id();
		let referenced = world.entity(root).get::<Target>().unwrap().0;
		world
			.entity(referenced)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("referent");
	}

	#[beet_core::test]
	fn resolves_deferred_template() {
		let mut world = world();
		// a deferred `Label` slot, carried by name plus a `DynamicStruct` patch,
		// resolves against the live registry and builds its subtree at build time.
		let mut patch = DynamicStruct::default();
		patch.insert("text", "deferred".to_string());
		let template = DynamicTemplate {
			resources: vec![],
			source: None,
			entities: vec![entity(0, vec![ComponentSlot::Template(
				DeferredTemplate {
					name: "Label".into(),
					patch: Box::new(patch),
				},
			)])],
		};
		let root = world.spawn_template(template).unwrap().id();
		let child = world.entity(root).get::<Children>().unwrap()[0];
		world
			.entity(child)
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("deferred");
	}
}
