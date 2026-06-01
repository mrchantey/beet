//! Wires caller content into a widget's `<slot>` elements after spawn.
//!
//! The scene `rsx!` lowering tags every child of a component tag with
//! [`SlotChild`] (see `rsx_scene.rs`). After [`spawn_scene`](WorldSceneExt),
//! the composition root's [`Children`] intermix the widget's own structural
//! subtree with the caller's `SlotChild` content. This pass reads each
//! `SlotChild`'s `slot` attribute (default if absent), finds the matching
//! `<slot>` in the widget subtree, and links the two via [`SlotContainer`] so
//! the [`NodeWalker`](crate::prelude::NodeWalker) renders caller content in
//! place of the slot's fallback.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::scene::Scene;

/// [`World`] scene spawning that auto-wires caller content into widget slots.
///
/// Shadows `bevy_scene`'s `WorldSceneExt` so every `world.spawn_scene(..)` call
/// site picks up slot wiring without change: it spawns the scene, then runs
/// [`apply_slots`] over the fresh root before returning.
pub trait WorldSceneExt {
	/// Spawn `scene`, then wire any caller [`SlotChild`] content into the
	/// widget's `<slot>` elements. See [`bevy::scene::WorldSceneExt::spawn_scene`].
	fn spawn_scene<S: Scene>(
		&mut self,
		scene: S,
	) -> Result<EntityWorldMut<'_>, BevyError>;
}

impl WorldSceneExt for World {
	fn spawn_scene<S: Scene>(
		&mut self,
		scene: S,
	) -> Result<EntityWorldMut<'_>, BevyError> {
		let root = bevy::scene::WorldSceneExt::spawn_scene(self, scene)?.id();
		apply_slots(self, root);
		self.get_entity_mut(root).map_err(Into::into)
	}
}

/// Adapts an [`impl Scene`](Scene) into an [`impl Bundle`], the form file-route
/// page handlers return content in.
///
/// File-based routes are classified by their *input* signature and the
/// `render_action::*_route` constructors all spawn an `impl Bundle`. A
/// widget-using page returns `rsx!{ .. }.into_scene_bundle()` and a plain page
/// returns `rsx_direct!{ .. }` directly — both satisfy the same constructors,
/// so a single file-route collection handles either macro.
///
/// The returned [`OnSpawn`] resolves and spawns the scene (wiring its slots via
/// [`WorldSceneExt::spawn_scene`]) as a child of the entity it is added to. The
/// host entity (eg a `fixed_route` `CallerScene` render root) carries no
/// [`Element`], so it renders transparently and the scene content lands in its
/// place.
pub trait IntoSceneBundle: 'static + Send + Sync + Scene + Sized {
	/// Spawn this scene as a child of the host entity on add. See
	/// [`IntoSceneBundle`].
	fn into_scene_bundle(self) -> impl Bundle {
		OnSpawn::new(move |entity: &mut EntityWorldMut| -> Result {
			let parent = entity.id();
			entity.world_scope(move |world: &mut World| -> Result {
				let child = world.spawn_scene(self)?.id();
				world.entity_mut(child).insert(ChildOf(parent));
				Ok(())
			})
		})
	}
}
impl<S: 'static + Send + Sync + Scene> IntoSceneBundle for S {}

/// Route every caller [`SlotChild`] under `root` into its matching `<slot>`.
///
/// Each component instance is its own wiring scope: a `SlotChild` belongs to
/// the component whose root entity is its [`ChildOf`] parent, so sibling
/// widgets (eg a `<Select>` and a `<Table>` both with default slots) never
/// steal each other's content. For each such composition root the wiring is a
/// layered fixpoint:
///
/// 1. Seed the open targets with that instance's structural `<slot>` elements
///    (reachable without crossing into caller content or a nested instance).
/// 2. For each open target, take the next caller content (in source order)
///    whose `slot` attribute matches the target's `name` (default when absent).
///    Wire it via [`SlotContainer`] and pull it out of the child flow so it
///    never also renders as a plain child.
/// 3. A re-projecting relay (a `SlotChild` that is itself a `<slot name="y"
///    slot="x"/>`) reopens as a new target once consumed, so deeper content
///    flows through it. Consumed content is never reused, so a slot is never
///    stolen across composition boundaries.
///
/// Multiple contents for one slot are reparented under a single transparent
/// wrapper entity (no [`Element`]), preserving caller order, since
/// [`SlotContainer`] points at one entity.
pub fn apply_slots(world: &mut World, root: Entity) {
	let plan = world
		.run_system_cached_with(plan_slots, root)
		.unwrap_or_default();
	apply_plan(world, plan);
}

/// A single slot wiring: the `<slot>` element and the ordered caller content
/// entities routed into it.
struct SlotWiring {
	slot: Entity,
	content: Vec<Entity>,
}

/// Compute the wiring plan, one layered fixpoint per composition root, reading
/// the tree through queries so traversal stays formalized.
fn plan_slots(
	root: In<Entity>,
	children: Query<&Children>,
	parents: Query<&ChildOf>,
	slot_children: Query<(), With<SlotChild>>,
	elements: Query<&Element>,
	attribute_lists: Query<&Attributes>,
	attributes: Query<(&Attribute, &Value)>,
) -> Vec<SlotWiring> {
	let attr = |entity: Entity, key: &str| -> Option<String> {
		attribute_lists.get(entity).ok().and_then(|attrs| {
			attrs.iter().find_map(|attr| {
				attributes.get(attr).ok().and_then(|(attribute, value)| {
					(attribute.as_str() == key)
						.then(|| value.as_str().ok().map(String::from))
						.flatten()
				})
			})
		})
	};
	let is_slot = |entity: Entity| {
		elements
			.get(entity)
			.map(|el| el.tag() == "slot")
			.unwrap_or(false)
	};

	// composition roots are the distinct `ChildOf` parents of caller content;
	// each is wired independently so sibling instances don't pool their slots.
	let composition_roots: HashSet<Entity> = children
		.iter_descendants_inclusive(*root)
		.filter(|entity| slot_children.contains(*entity))
		.filter_map(|entity| {
			parents.get(entity).ok().map(|parent| parent.parent())
		})
		.collect();

	let mut plan = Vec::<SlotWiring>::new();
	for instance in composition_roots.iter().copied() {
		// sources: this instance's own caller content, in document order
		let mut sources: Vec<(Option<String>, Entity)> = children
			.get(instance)
			.into_iter()
			.flat_map(|child_list| child_list.iter())
			.filter(|entity| slot_children.contains(*entity))
			.map(|entity| (attr(entity, "slot"), entity))
			.collect();

		// targets: this instance's structural slots, descending its subtree but
		// never crossing caller content or a nested instance's subtree.
		let mut targets: Vec<(Option<String>, Entity)> = Vec::new();
		let mut stack = vec![instance];
		while let Some(entity) = stack.pop() {
			if entity != instance
				&& (slot_children.contains(entity)
					|| composition_roots.contains(&entity))
			{
				continue;
			}
			if entity != instance && is_slot(entity) {
				targets.push((attr(entity, "name"), entity));
				// a structural slot's children are fallback, not more targets
				continue;
			}
			if let Ok(child_list) = children.get(entity) {
				for child in child_list.iter().rev() {
					stack.push(child);
				}
			}
		}

		// fixpoint: match sources to open targets, relays reopen as new targets
		let mut next = 0;
		while next < targets.len() {
			let (target_name, slot) = targets[next].clone();
			next += 1;

			let mut matched = Vec::new();
			sources.retain(|(name, content)| {
				if *name == target_name {
					matched.push(*content);
					false
				} else {
					true
				}
			});
			if matched.is_empty() {
				continue;
			}

			// a matched relay (`SlotChild` that is itself a `<slot>`) reopens as
			// a target named by its own `name`, so deeper content flows through.
			for content in &matched {
				if is_slot(*content) {
					targets.push((attr(*content, "name"), *content));
				}
			}
			plan.push(SlotWiring {
				slot,
				content: matched,
			});
		}
	}
	plan
}

/// Apply a wiring plan: link each slot to its content and remove the content
/// from the root's child flow. Multiple contents wrap in one transparent entity.
fn apply_plan(world: &mut World, plan: Vec<SlotWiring>) {
	for SlotWiring { slot, content } in plan {
		// strip the routed content's `slot` attribute so it does not render,
		// and detach it from its current parent's child flow
		for entity in &content {
			remove_slot_attribute(world, *entity);
			world.entity_mut(*entity).remove::<ChildOf>();
		}

		let container = match content.as_slice() {
			[single] => *single,
			_ => {
				// several contents share one slot: a transparent wrapper (no
				// `Element`) holds them in order; the walker renders its
				// children without emitting a tag.
				let wrapper = world.spawn_empty().id();
				for entity in &content {
					world.entity_mut(*entity).insert(ChildOf(wrapper));
				}
				wrapper
			}
		};
		world.entity_mut(slot).insert(SlotContainer::new(container));
	}
}

/// Remove the `slot` attribute entity from `entity`, so routed content does not
/// render a stray `slot="…"` attribute.
fn remove_slot_attribute(world: &mut World, entity: Entity) {
	let Some(slot_attr) = world
		.entity(entity)
		.get::<Attributes>()
		.into_iter()
		.flat_map(|attrs| attrs.iter())
		.find(|attr| {
			world
				.entity(*attr)
				.get::<Attribute>()
				.map(|attribute| attribute.as_str() == "slot")
				.unwrap_or(false)
		})
	else {
		return;
	};
	world.entity_mut(slot_attr).despawn();
}

#[cfg(test)]
mod test {
	// explicit import so `spawn_scene` resolves to beet_ui's slot-wiring
	// trait rather than the `bevy::scene` one glob-imported via
	// `beet_core::prelude` (both share the `WorldSceneExt` name).
	use crate::prelude::WorldSceneExt;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A widget with default + named slots and fallback content in each.
	#[scene]
	fn Panel() -> impl Scene {
		rsx! {
			<section>
				<header><slot name="title">"Fallback Title"</slot></header>
				<div><slot>"Fallback Body"</slot></div>
			</section>
		}
	}

	/// Render `scene` to an HTML string after spawn + slot wiring.
	fn render(scene: impl Scene) -> String {
		let mut world = scene_ext::test_world();
		let root = world.spawn_scene(scene).unwrap().id();
		HtmlRenderer::new()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn default_slot_receives_caller_content() {
		let html = render(rsx! { <Panel><p>"Body"</p></Panel> });
		html.as_str().xpect_contains("<p>Body</p>");
		html.as_str().xnot().xpect_contains("Fallback Body");
	}

	#[beet_core::test]
	fn named_slot_receives_caller_content() {
		let html =
			render(rsx! { <Panel><h1 slot="title">"Title"</h1></Panel> });
		html.as_str().xpect_contains("<h1>Title</h1>");
		html.as_str().xnot().xpect_contains("Fallback Title");
	}

	#[beet_core::test]
	fn unfilled_slot_keeps_fallback() {
		// only the default slot is filled; the named slot keeps its fallback
		let html = render(rsx! { <Panel><p>"Body"</p></Panel> });
		html.as_str().xpect_contains("Fallback Title");
		html.as_str().xpect_contains("<p>Body</p>");
	}

	#[beet_core::test]
	fn caller_content_not_double_rendered() {
		// a routed child must appear exactly once: in place of its (now
		// transparent) slot, never also as a plain child of the root. With both
		// slots filled, no literal <slot> element survives either.
		let html = render(rsx! {
			<Panel>
				<h1 slot="title">"Title"</h1>
				<p>"Body"</p>
			</Panel>
		});
		html.matches("<p>Body</p>").count().xpect_eq(1);
		html.matches("<h1>Title</h1>").count().xpect_eq(1);
		html.as_str().xnot().xpect_contains("<slot");
		html.as_str().xnot().xpect_contains("slot=");
	}

	#[beet_core::test]
	fn preserves_caller_order_and_multiplicity() {
		// several children target the same default slot; order is preserved
		let html = render(rsx! {
			<Panel>
				<p>"one"</p>
				<p>"two"</p>
				<p>"three"</p>
			</Panel>
		});
		html.find("one")
			.unwrap()
			.xpect_less_than(html.find("two").unwrap());
		html.find("two")
			.unwrap()
			.xpect_less_than(html.find("three").unwrap());
	}

	#[beet_core::test]
	fn nested_reprojection_through_layers() {
		// Outer wraps Panel and re-projects its own `title` slot into Panel's
		// `title` slot via `<slot name="title" slot="title"/>`. Caller content
		// targeting `title` must flow through both layers.
		#[scene]
		fn Outer() -> impl Scene {
			rsx! {
				<Panel>
					<slot name="title" slot="title"/>
					<slot/>
				</Panel>
			}
		}

		let html = render(rsx! {
			<Outer>
				<h1 slot="title">"Deep Title"</h1>
				<p>"Deep Body"</p>
			</Outer>
		});
		html.as_str().xpect_contains("<h1>Deep Title</h1>");
		html.as_str().xpect_contains("<p>Deep Body</p>");
		html.as_str().xnot().xpect_contains("Fallback");
	}

	#[beet_core::test]
	fn sibling_instances_do_not_steal_slots() {
		// two sibling widgets with default slots, nested in a plain element.
		// each must only receive its own content (regression: the first default
		// slot greedily consumed both widgets' content).
		let html = render(rsx! {
			<div>
				<Select name="a"><option>"AlphaOpt"</option></Select>
				<Panel><p>"PanelBody"</p></Panel>
			</div>
		});
		// the option stays in the select, the paragraph in the panel
		html.as_str()
			.xpect_contains("<select name=\"a\"")
			.xpect_contains("<option>AlphaOpt</option>");
		let select_open = html.find("<select").unwrap();
		let select_close = html.find("</select>").unwrap();
		let alpha = html.find("AlphaOpt").unwrap();
		let panel_body = html.find("PanelBody").unwrap();
		// AlphaOpt inside the select, PanelBody outside it
		alpha.xpect_greater_than(select_open);
		alpha.xpect_less_than(select_close);
		panel_body.xpect_greater_than(select_close);
	}

	#[beet_core::test]
	fn production_select_routes_options() {
		// the `Select` widget exposes a default slot for `<option>`s
		let html = render(rsx! {
			<Select name="fruit">
				<option>"Apple"</option>
				<option>"Banana"</option>
			</Select>
		});
		html.as_str().xpect_contains("<option>Apple</option>");
		html.as_str().xpect_contains("<option>Banana</option>");
		html.as_str().xnot().xpect_contains("<slot");
	}

	#[beet_core::test]
	fn production_table_routes_head_and_body() {
		// `Table` has `head`, default, and `foot` slots; rows route by `slot`
		let html = render(rsx! {
			<Table>
				<tr slot="head"><th>"Name"</th></tr>
				<tr><td>"Ada"</td></tr>
			</Table>
		});
		// head rows land in <thead>, default rows in <tbody>
		let thead = html.find("<thead>").unwrap();
		let tbody = html.find("<tbody>").unwrap();
		html.find("Name").unwrap().xpect_greater_than(thead);
		html.find("Name").unwrap().xpect_less_than(tbody);
		html.find("Ada").unwrap().xpect_greater_than(tbody);
		// the filled head/default slots are gone; only the unfilled `foot`
		// slot survives, rendering its (empty) fallback
		html.matches("<slot").count().xpect_eq(1);
		html.as_str().xpect_contains("<slot name=\"foot\">");
	}
}
