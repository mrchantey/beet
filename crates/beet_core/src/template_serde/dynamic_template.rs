//! The [`DynamicTemplate`] intermediate representation and its build path.
//!
//! A `DynamicTemplate` is the whole: an ordered list of resources and an ordered
//! list of nodes. Each node corresponds to an entity and carries an ordered list
//! of component slots, each either a resolved value or a deferred template. A
//! fully resolved save-game and a hand-authored page are the same kind of thing
//! at different points on one axis: how much is already a value versus deferred.
//!
//! `DynamicTemplate` is itself a [`Template`], so the single instantiation path
//! ([`spawn_template`](crate::prelude::WorldTemplateExt::spawn_template) /
//! [`insert_template`](crate::prelude::EntityWorldMutTemplateExt::insert_template))
//! builds it. Building walks the nodes in order, mapping each in-template
//! [`Entity`] to a real world entity through the one entity model
//! ([`SceneEntityReferences`]), applying value slots and building deferred slots
//! through the [`ReflectTemplate`](crate::prelude::ReflectTemplate) registry.
//!
//! # Entity model
//!
//! Each in-template [`Entity`] is keyed to a deterministic
//! [`SceneEntityReference`] on its [index](Entity::index), so the same id always
//! resolves to the same real entity within one build. The first node's id is
//! pinned to the walker's root; the rest, and any `Entity`-typed field, resolve
//! through [`SceneEntityReferences::get`], which spawns a placeholder on first
//! lookup and is filled when that node is built. This is the single remapping
//! model: cross-entity references and forward references resolve uniformly,
//! replacing the old standalone scene entity mapper.

use crate::prelude::*;
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
use bevy_reflect::TypeRegistration;
use bevy_reflect::TypeRegistry;

/// The serde intermediate representation: ordered resources plus an ordered list
/// of nodes, each slot a reflectable value or a named deferred template.
///
/// This is the IR for the serde formats (RON/JSON/postcard), not for markup: BSX
/// is its own syntax-tree IR ([`BsxTemplate`](crate::prelude::BsxTemplate))
/// resolved against live registries, with no serializable value form. Both
/// implement [`Template`] and the unified
/// [`TemplateLoader`](crate::prelude::TemplateLoader) dispatches between them by
/// [`MediaType`](crate::prelude::MediaType) (see
/// [`EntryTemplate`](crate::prelude::EntryTemplate)); they are not merged.
///
/// It builds a subtree into the world, so it is a [`Template`]. The first node
/// builds into the context entity (the root); the rest spawn. See the module
/// docs for the entity model and the children-order contract.
#[derive(Default)]
pub struct DynamicTemplate {
	/// Resources to write into the world, applied after all nodes are built.
	pub resources: Vec<Box<dyn PartialReflect>>,
	/// Nodes in build order. The first node becomes the root, building into the
	/// context entity; the rest spawn.
	pub nodes: Vec<DynamicTemplateNode>,
}

// `DynamicTemplate` is not `Clone` (a boxed `PartialReflect` is not), so it
// cannot pick up Bevy's blanket `Template for T: Default + Clone + Unpin`. The
// opt-out documents the intent and guards against a future `Clone`.
subtree_template!(DynamicTemplate);

/// A transient sink the loader installs to capture the real entities a
/// [`DynamicTemplate`] build maps each node to, in node order.
///
/// The loader inserts it before building and drains it after, so it learns every
/// spawned entity without a second remapping model. Absent for a plain
/// `spawn_template`, where the caller only needs the root.
#[derive(Default, Resource)]
pub(crate) struct TemplateBuildSink(pub Vec<Entity>);

/// One node of a [`DynamicTemplate`]: an entity and its ordered component slots.
pub struct DynamicTemplateNode {
	/// The in-template entity id, unique within a [`DynamicTemplate`].
	///
	/// Component slots that reference this entity (a relationship target, an
	/// `Entity`-typed field) must use this same id; it remaps to a real world
	/// entity on the build path.
	pub entity: Entity,
	/// The component slots, in order.
	pub components: Vec<ComponentSlot>,
}

/// One component slot on a node: a resolved value or a deferred template.
pub enum ComponentSlot {
	/// A concrete component value, the save-game form. Applied by reflection over
	/// the inserted-or-defaulted component.
	Value(Box<dyn PartialReflect>),
	/// A registered template carried by name plus its reflected patch, resolved
	/// at build time through the
	/// [`ReflectTemplate`](crate::prelude::ReflectTemplate) registry. This is how
	/// a function component or any world-context value is carried before it is
	/// built.
	Template(DeferredTemplate),
}

/// A deferred template slot: a registered template's short type path and the
/// reflected patch (a `DynamicStruct` over its default) to build it from.
pub struct DeferredTemplate {
	/// The registered template's short type path, the registry lookup key.
	pub name: SmolStr,
	/// The reflected patch over the template's default, typically a
	/// `DynamicStruct` carrying only the provided fields.
	pub patch: Box<dyn PartialReflect>,
}

impl Template for DynamicTemplate {
	type Output = ();

	/// Builds this template into `cx`: maps every in-template entity to a real
	/// entity, applies each node's component slots, then writes resources.
	///
	/// The first node builds into `cx.entity` (the walker's root); the rest
	/// resolve through [`SceneEntityReferences`], which spawns a real entity on
	/// first lookup, so cross-entity and forward references resolve through the
	/// one entity model.
	fn build_template(&self, cx: &mut TemplateContext) -> Result<()> {
		let app_registry = cx.entity.resource::<AppTypeRegistry>().clone();
		let registry = app_registry.read();

		// pin the root: the first node's in-template entity maps to cx.entity, so
		// its components land on the walker's root rather than a fresh spawn.
		let root = cx.entity.id();
		if let Some(root_node) = self.nodes.first() {
			cx.entity_references
				.set(scene_reference(root_node.entity), root);
		}
		// the build walker exposes the template root via `TemplateBuildRoot`, so a
		// nested asset/remote value template parks its pending dependency on it.
		self.build_nodes(&registry, &app_registry, cx)
	}

	fn clone_template(&self) -> Self {
		unreachable!("DynamicTemplate is built once, never cloned")
	}
}

impl DynamicTemplate {
	/// Builds every node onto its mapped entity, in order, then writes resources.
	///
	/// Node order plus in-order `ChildOf` application (relationship hooks run) is
	/// what preserves `Children` order across a round-trip.
	fn build_nodes(
		&self,
		registry: &TypeRegistry,
		app_registry: &AppTypeRegistry,
		cx: &mut TemplateContext,
	) -> Result<()> {
		for node in &self.nodes {
			// SAFETY: only used to spawn-or-fetch the mapped placeholder entity.
			let world = unsafe { cx.entity.world_mut() };
			let entity_id = cx
				.entity_references
				.get(scene_reference(node.entity), world);
			// record the real entity for the loader, if it installed a sink.
			if let Some(mut sink) =
				world.get_resource_mut::<TemplateBuildSink>()
			{
				sink.0.push(entity_id);
			}
			build_node(node, entity_id, registry, app_registry, cx)?;
		}

		// resources last, so they are available for any reference resolution.
		// SAFETY: only used to write resources into the world.
		let world = unsafe { cx.entity.world_mut() };
		for resource in &self.resources {
			write_resource(
				world,
				resource.as_ref(),
				cx.entity_references,
				registry,
			)?;
		}
		OK
	}
}

/// Builds one node's component slots onto its mapped real entity.
///
/// Value slots apply by reflection over the inserted-or-defaulted component,
/// remapping any `Entity`-typed field through the shared entity map. Deferred
/// slots build through the registry by name, into a [`TemplateContext`] scoped
/// to this entity that shares the same reference map.
fn build_node(
	node: &DynamicTemplateNode,
	entity_id: Entity,
	registry: &TypeRegistry,
	app_registry: &AppTypeRegistry,
	cx: &mut TemplateContext,
) -> Result<()> {
	for slot in &node.components {
		match slot {
			ComponentSlot::Value(value) => {
				// SAFETY: only used to apply a component onto the mapped entity.
				let world = unsafe { cx.entity.world_mut() };
				apply_value(
					world,
					entity_id,
					value.as_ref(),
					cx.entity_references,
					registry,
				)?;
			}
			ComponentSlot::Template(deferred) => {
				// build the deferred template into a context scoped to this
				// entity, sharing the reference map so cross-node refs resolve.
				// SAFETY: only used to acquire the mapped entity for a scoped build.
				let world = unsafe { cx.entity.world_mut() };
				let mut entity = world.entity_mut(entity_id);
				let mut scoped =
					TemplateContext::new(&mut entity, cx.entity_references);
				build_template_by_name(
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
	SceneEntityReference::new((file!(), 0, 0), in_template.index_u32() as usize, 0)
}

/// An [`EntityMapper`] backed by the build's [`SceneEntityReferences`], so an
/// `Entity`-typed component field serialized as an in-template id resolves to the
/// real (possibly forward-referenced) entity. The single remapping model.
struct ReferenceMapper<'a> {
	references: &'a mut SceneEntityReferences,
	world: &'a mut World,
}

impl EntityMapper for ReferenceMapper<'_> {
	fn get_mapped(&mut self, source: Entity) -> Entity {
		self.references.get(scene_reference(source), self.world)
	}

	fn set_mapped(&mut self, source: Entity, target: Entity) {
		self.references.set(scene_reference(source), target);
	}
}

/// Applies a reflected component value onto `entity`, remapping `Entity`-typed
/// fields through `references` and running relationship hooks so `Children` and
/// other relationship mirrors rebuild in order.
fn apply_value(
	world: &mut World,
	entity: Entity,
	value: &dyn PartialReflect,
	references: &mut SceneEntityReferences,
	registry: &TypeRegistry,
) -> Result<()> {
	let reflect_component = reflect_component(registry, value)?;

	// honour clone-ignored components (eg observers), which must not be written.
	let component_id = reflect_component.register_component(world);
	// SAFETY: registered immediately above, so the info exists.
	#[expect(unsafe_code, reason = "avoids a redundant lookup")]
	let is_ignored = unsafe {
		matches!(
			world
				.components()
				.get_info_unchecked(component_id)
				.clone_behavior(),
			ComponentCloneBehavior::Ignore
		)
	};
	if is_ignored {
		return Ok(());
	}

	// the mapper reborrows the world to spawn placeholder entities for any
	// `Entity`-typed field, while `apply_or_insert_mapped` writes the component.
	// SAFETY: the mapper only spawns/looks up entities and never touches the
	// component being applied, so the two borrows do not alias the same data.
	let mapper_world = unsafe { &mut *(world as *mut World) };
	let mut mapper = ReferenceMapper {
		references,
		world: mapper_world,
	};
	// `RelationshipHookMode::Run` so applying `ChildOf` rebuilds the parent's
	// ordered `Children`, the children-order contract.
	reflect_component.apply_or_insert_mapped(
		&mut world.entity_mut(entity),
		value,
		registry,
		&mut mapper,
		RelationshipHookMode::Run,
	);
	Ok(())
}

/// Writes a reflected resource value into the world, remapping `Entity`-typed
/// fields through `references`.
fn write_resource(
	world: &mut World,
	value: &dyn PartialReflect,
	references: &mut SceneEntityReferences,
	registry: &TypeRegistry,
) -> Result<()> {
	let reflect_resource = reflect_resource(registry, value)?;
	let resource_id = reflect_resource.register_component(world);
	let entity = world
		.resource_entities()
		.get(resource_id)
		.unwrap_or_else(|| world.spawn_empty().id());

	let mapper_world = unsafe { &mut *(world as *mut World) };
	let mut mapper = ReferenceMapper {
		references,
		world: mapper_world,
	};
	reflect_resource.apply_or_insert_mapped(
		&mut world.entity_mut(entity),
		value,
		registry,
		&mut mapper,
		RelationshipHookMode::Run,
	);
	Ok(())
}

/// Resolve the [`ReflectComponent`] for a reflected component value, erroring if
/// its type is missing a represented type, unregistered, or not a component.
fn reflect_component<'a>(
	registry: &'a TypeRegistry,
	value: &dyn PartialReflect,
) -> Result<&'a ReflectComponent> {
	let registration = type_registration(registry, value)?;
	registration.data::<ReflectComponent>().ok_or_else(|| {
		bevyhow!(
			"template contains the unregistered component `{}`",
			registration.type_info().type_path()
		)
	})
}

/// Resolve the [`ReflectComponent`] backing a reflected resource value.
fn reflect_resource<'a>(
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
		let mut world = TemplatePlugin::world();
		world.register_template::<Label>();
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
		ComponentSlot::Value(component.to_dynamic())
	}

	/// A node id from a raw in-template index.
	fn entity(index: u32) -> Entity { Entity::from_raw_u32(index).unwrap() }

	#[crate::test]
	fn builds_value_node_onto_root() {
		let mut world = world();
		let template = DynamicTemplate {
			resources: vec![],
			nodes: vec![DynamicTemplateNode {
				entity: entity(0),
				components: vec![value(Score(7))],
			}],
		};
		let root = world.spawn_template(template).unwrap().id();
		world
			.entity(root)
			.get::<Score>()
			.unwrap()
			.xpect_eq(Score(7));
	}

	#[crate::test]
	fn builds_children_in_order() {
		let mut world = world();
		// root entity 0, three children 1,2,3, each holding ChildOf(0).
		let parent = entity(0);
		let mut nodes = vec![DynamicTemplateNode {
			entity: parent,
			components: vec![],
		}];
		for (index, label) in ["a", "b", "c"].into_iter().enumerate() {
			nodes.push(DynamicTemplateNode {
				entity: entity(index as u32 + 1),
				components: vec![
					value(ChildOf(parent)),
					value(Name::new(label)),
				],
			});
		}
		let root = world
			.spawn_template(DynamicTemplate {
				resources: vec![],
				nodes,
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

	#[crate::test]
	fn resolves_forward_reference() {
		let mut world = world();
		// node 0 holds Target(1), a forward reference to node 1 which carries a
		// recognisable Name. The placeholder spawned on first lookup is filled
		// when node 1 builds.
		let template = DynamicTemplate {
			resources: vec![],
			nodes: vec![
				DynamicTemplateNode {
					entity: entity(0),
					components: vec![value(Target(entity(1)))],
				},
				DynamicTemplateNode {
					entity: entity(1),
					components: vec![value(Name::new("referent"))],
				},
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

	#[crate::test]
	fn resolves_deferred_template() {
		let mut world = world();
		// a deferred `Label` slot, carried by name plus a `DynamicStruct` patch,
		// resolves against the live registry and builds its subtree at build time.
		let mut patch = DynamicStruct::default();
		patch.insert("text", "deferred".to_string());
		let template = DynamicTemplate {
			resources: vec![],
			nodes: vec![DynamicTemplateNode {
				entity: entity(0),
				components: vec![ComponentSlot::Template(DeferredTemplate {
					name: "Label".into(),
					patch: Box::new(patch),
				})],
			}],
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
