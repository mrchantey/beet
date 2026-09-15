//! The incremental pass: what changed in the world since the last frame,
//! patched into the nodes it is bound to.
use super::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;
use wasm_bindgen::JsCast;

/// Runs the DOM sink's incremental pass each frame, after the document sync
/// chain, so a painted tree follows its world.
#[derive(Default)]
pub struct DomRenderPlugin;

impl Plugin for DomRenderPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PostUpdate, sync_dom.in_set(DomRenderSet));
	}
}

/// The [`PostUpdate`] set the incremental pass runs in, for a first paint to
/// order after: a host mounting once the pass has run leaves it nothing
/// stale to patch next frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct DomRenderSet;

/// What changed since the last pass, read once per frame so each change is
/// patched exactly once.
#[derive(SystemParam)]
struct DomChanges<'w, 's> {
	/// A text node's content, or a control's value.
	values: Query<
		'w,
		's,
		Entity,
		(Changed<Value>, With<DomNode>, Without<Attribute>),
	>,
	/// An attribute added or edited.
	attributes: Query<
		'w,
		's,
		Entity,
		(Changed<Value>, With<Attribute>, With<AttributeOf>),
	>,
	/// An element's classes.
	classes: Query<'w, 's, Entity, (Changed<Classes>, With<DomNode>)>,
	/// A parent's child list, or a holder's transclusion.
	children: Query<'w, 's, Entity, Or<(Changed<Children>, Changed<Portal>)>>,
	/// Whether anything is painted at all.
	painted: Query<'w, 's, (), With<DomNode>>,
}

/// System: patch every change into the nodes it is bound to. Exclusive,
/// since a changed child list may paint fresh subtrees whose bindings the
/// next reconcile in the same pass must see.
fn sync_dom(world: &mut World, state: &mut SystemState<DomChanges>) {
	let Ok(changes) = state.get(world) else {
		return;
	};
	if changes.painted.is_empty() {
		return;
	}
	let values: Vec<Entity> = changes.values.iter().collect();
	let attributes: Vec<Entity> = changes.attributes.iter().collect();
	let classes: Vec<Entity> = changes.classes.iter().collect();
	let children: Vec<Entity> = changes.children.iter().collect();
	for entity in values {
		sync_value(world, entity);
	}
	for entity in attributes {
		sync_attribute(world, entity);
	}
	for entity in classes {
		sync_classes(world, entity);
	}
	// every changed list under one painted element reconciles that element
	// once
	let mut targets: Vec<(Entity, web_sys::Element)> = Vec::new();
	for entity in children {
		if let Some(target) = reconcile_target(world, entity)
			&& !targets.iter().any(|(known, _)| *known == target.0)
		{
			targets.push(target);
		}
	}
	for (entity, target) in targets {
		sync_children(world, entity, target);
	}
}

/// A changed [`Value`]: a text node's content, or a control's live value.
fn sync_value(world: &mut World, entity: Entity) {
	let Ok(entity_ref) = world.get_entity(entity) else {
		return;
	};
	let (Some(DomNode::Node(node)), Some(value)) =
		(entity_ref.get::<DomNode>(), entity_ref.get::<Value>())
	else {
		return;
	};
	match entity_ref.get::<Element>() {
		None => {
			if let Some(text) = node.dyn_ref::<web_sys::Text>() {
				let data = value.to_string();
				if text.data() != data {
					text.set_data(&data);
				}
			}
		}
		Some(element) if is_value_element(element.tag()) => {
			DomRenderer::set_control_value(node, value);
		}
		Some(_) => {}
	}
}

/// An attribute entity added or edited: set on its element, and bound to it.
fn sync_attribute(world: &mut World, entity: Entity) {
	let Ok(entity_ref) = world.get_entity(entity) else {
		return;
	};
	let (Some(of), Some(attribute), Some(value)) = (
		entity_ref.get::<AttributeOf>(),
		entity_ref.get::<Attribute>(),
		entity_ref.get::<Value>(),
	) else {
		return;
	};
	let element_entity = **of;
	let Some(element) = world
		.get::<DomNode>(element_entity)
		.and_then(DomNode::element)
		.cloned()
	else {
		return;
	};
	let name = attribute.to_string();
	let text = DomRenderer::text_of(value);
	let bound = entity_ref.contains::<DomNode>();
	match name.as_str() {
		"class" => sync_classes(world, element_entity),
		_ => {
			element.set_attribute(&name, &text).ok();
		}
	}
	if !bound {
		world.entity_mut(entity).insert(DomNode::attribute(element));
	}
}

/// An element's merged `class`: its `Classes` and its `class` attribute, or
/// none.
fn sync_classes(world: &mut World, entity: Entity) {
	let Some(element) = world
		.get::<DomNode>(entity)
		.and_then(DomNode::element)
		.cloned()
	else {
		return;
	};
	let class = world.with_state::<ElementQuery, _>(|elements| {
		elements
			.get(entity)
			.ok()
			.and_then(|view| view.class_attribute())
	});
	match class {
		Some(class) => {
			element.set_attribute("class", &class).ok();
		}
		None => {
			element.remove_attribute("class").ok();
		}
	}
}

/// Reconcile `target`'s DOM children against the entities `entity` holds: a
/// surviving node already in place is untouched, one elsewhere moves before
/// what is there, a new one is painted, and what is left past the end is
/// gone.
fn sync_children(world: &mut World, entity: Entity, target: web_sys::Element) {
	let mut renderer = DomRenderer::new(target.clone());
	let mut expected = Vec::new();
	collect_children(world, &mut renderer, entity, &mut expected);
	let current = target.child_nodes();
	for (index, node) in expected.iter().enumerate() {
		let at = current.item(index as u32);
		if at.as_ref() == Some(node) {
			continue;
		}
		target.insert_before(node, at.as_ref()).ok();
	}
	while current.length() as usize > expected.len() {
		let Some(last) = target.last_child() else {
			break;
		};
		target.remove_child(&last).ok();
	}
}

/// The painted element `entity`'s children land in: itself when it paints as
/// one, else the nearest visual ancestor that does, crossing a transclusion
/// to its holder. `None` outside any painted tree.
fn reconcile_target(
	world: &World,
	entity: Entity,
) -> Option<(Entity, web_sys::Element)> {
	let mut current = entity;
	loop {
		if let Some(element) =
			world.get::<DomNode>(current).and_then(DomNode::element)
		{
			return Some((current, element.clone()));
		}
		current = visual_parent(world, current)?;
	}
}

/// The parent an entity renders under: its portal holder, else its
/// [`ChildOf`] parent.
fn visual_parent(world: &World, entity: Entity) -> Option<Entity> {
	world
		.get::<PortalOf>(entity)
		.and_then(|of| of.holders().first().copied())
		.or_else(|| world.get::<ChildOf>(entity).map(ChildOf::parent))
}

/// What one entity contributes to its parent's DOM child list.
enum Contribution {
	/// The node it is bound to.
	Node(web_sys::Node),
	/// Its own children, through it: a document element, a group with no
	/// node of its own.
	Children,
	/// What it transcludes.
	Portal(Entity),
	/// Nodes painted fresh.
	Paint,
	/// Nothing: an attribute's binding, never a child.
	None,
}

fn contribution(world: &World, entity: Entity) -> Contribution {
	match world.get::<DomNode>(entity) {
		Some(DomNode::Node(node)) => Contribution::Node((**node).clone()),
		Some(DomNode::Document(_)) => Contribution::Children,
		Some(DomNode::Attribute(_)) => Contribution::None,
		None => match world.get::<Portal>(entity) {
			Some(portal) => Contribution::Portal(portal.target()),
			// the document's own `<html>` is transparent; anything else with a
			// node of its own paints, and a group without one contributes its
			// children
			None if world
				.get::<Element>(entity)
				.is_some_and(|element| element.tag() != "html")
				|| world.get::<Value>(entity).is_some()
				|| world.get::<Comment>(entity).is_some() =>
			{
				Contribution::Paint
			}
			None => Contribution::Children,
		},
	}
}

/// The nodes `entity`'s children contribute to its DOM child list, in order.
fn collect_children(
	world: &mut World,
	renderer: &mut DomRenderer,
	entity: Entity,
	out: &mut Vec<web_sys::Node>,
) {
	let children: Vec<Entity> = world
		.get::<Children>(entity)
		.map(|children| children.iter().collect())
		.unwrap_or_default();
	for child in children {
		collect(world, renderer, child, out);
	}
}

/// The nodes `entity` contributes to its parent's DOM child list.
fn collect(
	world: &mut World,
	renderer: &mut DomRenderer,
	entity: Entity,
	out: &mut Vec<web_sys::Node>,
) {
	match contribution(world, entity) {
		Contribution::Node(node) => out.push(node),
		Contribution::Children => {
			collect_children(world, renderer, entity, out)
		}
		Contribution::Portal(target) => collect(world, renderer, target, out),
		Contribution::Paint => out.extend(renderer.render(world, entity)),
		Contribution::None => {}
	}
}

#[cfg(test)]
mod test {
	use super::super::test_ext::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use wasm_bindgen::JsCast;

	/// A changed value patches its text node in place: the same node, new
	/// data.
	#[beet_core::test(browser)]
	fn a_value_change_patches_the_same_text_node() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! { <h1>"Garden"</h1> });
		let target = paint(&mut app, host);
		let heading = element(app.world_mut(), "h1");
		let text = app.world().get::<Children>(heading).unwrap()[0];
		let node = node_of(app.world(), text);
		*app.world_mut().get_mut::<Value>(text).unwrap() =
			Value::str("Orchard");
		app.update();
		target.text_content().unwrap().xpect_eq("Orchard");
		node_of(app.world(), heading)
			.first_child()
			.unwrap()
			.xpect_eq(node);
	}

	/// A control the DOM already agrees with is left alone, caret and all;
	/// one it disagrees with is reassigned.
	#[beet_core::test(browser)]
	fn a_control_the_dom_agrees_with_is_left_alone() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<TextField field={FieldRef::new("name")}/>
		});
		set_document(&mut app, host, value!({ "name": "pete" }));
		app.update();
		paint(&mut app, host);
		let input = element(app.world_mut(), "input");
		let input = node_of(app.world(), input)
			.dyn_into::<web_sys::HtmlInputElement>()
			.unwrap();
		// the user typed ahead: the DOM holds what the world is about to
		input.set_value("peter");
		input.set_selection_range(1, 1).unwrap();
		set_document(&mut app, host, value!({ "name": "peter" }));
		app.update();
		input.selection_start().unwrap().xpect_eq(Some(1));
		// a value the DOM does not hold is written
		set_document(&mut app, host, value!({ "name": "pete" }));
		app.update();
		input.value().xpect_eq("pete");
	}

	/// A changed child list moves what moved and paints only what is new: a
	/// surviving node is the same instance, the departed one is gone.
	#[beet_core::test(browser)]
	fn a_children_change_keeps_surviving_nodes() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<ul><li>"a"</li><li>"b"</li><li>"c"</li></ul>
		});
		let target = paint(&mut app, host);
		let list = element(app.world_mut(), "ul");
		let items: Vec<Entity> =
			app.world().get::<Children>(list).unwrap().iter().collect();
		let (first, second, third) = (items[0], items[1], items[2]);
		let (node_first, node_third) =
			(node_of(app.world(), first), node_of(app.world(), third));
		// the third moves first, the second leaves, a fourth arrives last
		let fourth = app
			.world_mut()
			.spawn((Element::new("li"), children![Value::str("d")]))
			.id();
		app.world_mut().entity_mut(second).despawn();
		app.world_mut()
			.entity_mut(list)
			.detach_children(&[first, third])
			.add_children(&[third, first, fourth]);
		app.update();
		let painted = target.query_selector_all("li").unwrap();
		painted.length().xpect_eq(3);
		painted.item(0).unwrap().xpect_eq(node_third);
		painted.item(1).unwrap().xpect_eq(node_first);
		painted
			.item(2)
			.unwrap()
			.text_content()
			.unwrap()
			.xpect_eq("d");
	}

	/// A despawned entity's node leaves the document with it.
	#[beet_core::test(browser)]
	fn a_despawn_removes_the_node() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! { <ul><li>"a"</li><li>"b"</li></ul> });
		let target = paint(&mut app, host);
		let list = element(app.world_mut(), "ul");
		let second = app.world().get::<Children>(list).unwrap()[1];
		let node = node_of(app.world(), second);
		app.world_mut().entity_mut(second).despawn();
		app.update();
		target
			.query_selector_all("li")
			.unwrap()
			.length()
			.xpect_eq(1);
		node.parent_node().xpect_none();
	}

	/// An attribute the world never stated survives a reconcile of the ones
	/// it does: a `<details>` the user opened stays open while its id, its
	/// classes and its stated attributes change around it.
	#[beet_core::test(browser)]
	fn an_unstated_attribute_survives() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! {
			<details id="editor"><summary>"Scene editor"</summary></details>
		});
		let _target = paint(&mut app, host);
		let details = element(app.world_mut(), "details");
		let node = node_of(app.world(), details)
			.dyn_into::<web_sys::Element>()
			.unwrap();
		node.set_attribute("open", "").unwrap();
		// the world edits a stated attribute
		let id = attribute_of(app.world(), details, "id");
		*app.world_mut().get_mut::<Value>(id).unwrap() =
			Value::str("inspector");
		app.update();
		node.has_attribute("open").xpect_true();
		node.get_attribute("id")
			.xpect_eq(Some("inspector".to_string()));
		// classes arrive from the world
		app.world_mut()
			.entity_mut(details)
			.insert(Classes::new(["open-editor"]));
		app.update();
		node.get_attribute("class")
			.xpect_eq(Some("open-editor".to_string()));
		node.has_attribute("open").xpect_true();
		// a stated attribute leaving takes only itself
		app.world_mut().entity_mut(id).despawn();
		app.update();
		node.has_attribute("id").xpect_false();
		node.has_attribute("open").xpect_true();
	}

	/// A painted subtree re-parented under a fresh parent is reused, node
	/// and all, rather than painted again.
	#[beet_core::test(browser)]
	fn a_painted_subtree_is_reused_under_a_new_parent() {
		let mut app = dom_app();
		let host = build(&mut app, rsx! { <div><span>"keep"</span></div> });
		let target = paint(&mut app, host);
		let span = element(app.world_mut(), "span");
		let node = node_of(app.world(), span);
		let section = app
			.world_mut()
			.spawn((Element::new("section"), ChildOf(host)))
			.id();
		app.world_mut().entity_mut(span).insert(ChildOf(section));
		app.update();
		target
			.query_selector("section > span")
			.unwrap()
			.unwrap()
			.xpect_eq(node.dyn_into::<web_sys::Element>().unwrap());
		target
			.query_selector_all("span")
			.unwrap()
			.length()
			.xpect_eq(1);
	}
}
