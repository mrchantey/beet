use crate::prelude::*;
use beet_core::prelude::*;

/// The container entity tracking the content for the slot.
///
/// When a [`NodeWalker`] visits an entity with this component,
/// it recurses into the content entity instead of the entity's
/// [`Children`], which are considered the default slot content.
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
#[relationship(relationship_target = SlotContent)]
pub struct SlotContainer(pub Entity);

impl SlotContainer {
	pub fn new(content: Entity) -> Self { Self(content) }
}

/// The content entity to be inserted at the point of a
/// [`SlotContainer`].
///
/// Slot content tracks multiple containers, allowing for
/// a static scene to be concurrently rendered inside of
/// multiple containers.
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
#[relationship_target(relationship = SlotContainer)]
pub struct SlotContent(Vec<Entity>);


/// Finds a named `<slot>` entity in a tree.
///
/// Traverses descendants of `root` looking for an [`Element`] with
/// tag `slot` and a `name` attribute matching `slot_name`.
pub fn find_named_slot(
	world: &World,
	entity: Entity,
	slot_name: &str,
) -> Option<Entity> {
	let entity_ref = world.entity(entity);

	// check if this entity is a <slot> element with matching name
	if let Some(element) = entity_ref.get::<Element>() {
		if element.tag() == "slot" {
			if let Some(attrs) = entity_ref.get::<Attributes>() {
				for attr_entity in attrs.iter() {
					let attr_ref = world.entity(attr_entity);
					if let (Some(attr), Some(value)) =
						(attr_ref.get::<Attribute>(), attr_ref.get::<Value>())
					{
						if **attr == "name"
							&& value.as_str().ok() == Some(slot_name)
						{
							return Some(entity);
						}
					}
				}
			}
		}
	}

	// recurse into children
	if let Some(children) = entity_ref.get::<Children>() {
		for child in children.iter() {
			if let Some(found) = find_named_slot(world, child, slot_name) {
				return Some(found);
			}
		}
	}

	None
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Build a `<slot name="..."/>` entity with the given name.
	fn spawn_named_slot(world: &mut World, name: &str) -> Entity {
		let slot = world.spawn(Element::new("slot")).id();
		world.spawn((
			Attribute::new("name"),
			Value::Str(name.into()),
			AttributeOf::new(slot),
		));
		slot
	}

	#[test]
	fn finds_named_slot_at_root() {
		let mut world = World::new();
		let slot = spawn_named_slot(&mut world, "main");
		find_named_slot(&world, slot, "main")
			.unwrap()
			.xpect_eq(slot);
	}

	#[test]
	fn finds_named_slot_in_children() {
		let mut world = World::new();
		let root = world.spawn(Element::new("div")).id();
		let header = world.spawn((Element::new("header"), ChildOf(root))).id();
		let ul = world.spawn((Element::new("ul"), ChildOf(root))).id();
		let slot = spawn_named_slot(&mut world, "nav");
		world.entity_mut(slot).insert(ChildOf(ul));
		// suppress unused warning
		let _ = header;
		find_named_slot(&world, root, "nav").unwrap().xpect_eq(slot);
	}

	#[test]
	fn returns_none_for_missing_slot() {
		let mut world = World::new();
		let root = world.spawn(Element::new("div")).id();
		world.spawn((Element::new("p"), ChildOf(root)));
		find_named_slot(&world, root, "main").xpect_none();
	}

	#[test]
	fn ignores_non_slot_elements() {
		let mut world = World::new();
		// a <div name="main"> should NOT match
		let div = world.spawn(Element::new("div")).id();
		world.spawn((
			Attribute::new("name"),
			Value::Str("main".into()),
			AttributeOf::new(div),
		));
		find_named_slot(&world, div, "main").xpect_none();
	}

	#[test]
	fn relationship_adds_slot_content() {
		let mut world = World::new();
		let content = world.spawn_empty().id();
		let container = world.spawn(SlotContainer::new(content)).id();
		world.flush();

		world
			.entity(content)
			.get::<SlotContent>()
			.unwrap()
			.iter()
			.collect::<Vec<_>>()
			.contains(&container)
			.xpect_true();
	}

	#[test]
	fn walker_renders_slot_content() {
		let mut world = World::new();

		// content entity: <em>slotted</em>
		let content = world.spawn(Element::new("em")).id();
		world.spawn((Value::Str("slotted".into()), ChildOf(content)));

		// layout: <div><slot name="main"/></div>
		let root = world.spawn(Element::new("div")).id();
		let slot = spawn_named_slot(&mut world, "main");
		world
			.entity_mut(slot)
			.insert((SlotContainer::new(content), ChildOf(root)));

		let html = HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string();

		// the slot element itself should be transparent
		html.xpect_contains("<em>slotted</em>")
			.xnot()
			.xpect_contains("<slot");
	}

	#[test]
	fn walker_renders_default_content_without_slot_container() {
		let mut world = World::new();

		// <div><slot name="main"><p>default</p></slot></div>
		let root = world.spawn(Element::new("div")).id();
		let slot = spawn_named_slot(&mut world, "main");
		world.entity_mut(slot).insert(ChildOf(root));
		let default_p = world.spawn(Element::new("p")).id();
		world.spawn((Value::Str("default".into()), ChildOf(default_p)));
		world.entity_mut(default_p).insert(ChildOf(slot));

		let html = HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string();

		// without SlotContainer the <slot> renders normally with children
		html.xpect_contains("<slot")
			.xpect_contains("<p>default</p>");
	}
}
