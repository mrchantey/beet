//! Marker-based slot resolution, run synchronously inside the build walk.
//!
//! A `<Slot>` placeholder lowers to a [`SlotTarget`] marker; content routed to
//! a slot lowers to a [`SlotChild`] marker. [`resolve_slots`] matches children
//! to targets positionally over an already-built subtree, identically for the
//! `rsx!` macro, the BSX parser, and serde, because all three produce the same
//! markers in the same tree.
//!
//! Resolution is one mechanism with these behaviors (the acceptance contract):
//!
//! - A named [`SlotChild`] is inserted into the first matching [`SlotTarget`].
//! - Only direct-descendant children of a composition scope are collected, so a
//!   parent never steals a slot belonging to a nested template.
//! - Unnamed children go to the default slot.
//! - A [`SlotTarget`] is replaced by its resolved children (a fragment).
//! - Fallback children are used when no content is supplied, removed otherwise.
//! - Unconsumed children are a [`TemplateError`], naming the missing slot and
//!   listing available targets.
//! - Astro-style transfers compose through levels: a [`SlotTarget`] that is also
//!   a [`SlotChild`] re-opens as a target once filled, so deeper content flows.
//! - Resolution recurses into nested templates.

use crate::prelude::*;

/// The default slot name, used when a [`SlotTarget`] or [`SlotChild`] is unnamed.
pub const DEFAULT_SLOT: &str = "default";

/// A `<Slot>` placeholder in a built subtree.
///
/// Carries an optional slot name (the default slot when unnamed). Its children
/// are fallback content, used only when no [`SlotChild`] is routed to it.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct SlotTarget {
	/// The slot name, or `None` for the default slot.
	pub name: Option<SmolStr>,
}

impl SlotTarget {
	/// A target for the default (unnamed) slot.
	pub fn new() -> Self { Self::default() }

	/// A target for a named slot.
	pub fn named(name: impl Into<SmolStr>) -> Self {
		Self {
			name: Some(name.into()),
		}
	}

	/// The slot name this target matches, defaulting to [`DEFAULT_SLOT`].
	pub fn slot_name(&self) -> &str {
		self.name.as_deref().unwrap_or(DEFAULT_SLOT)
	}
}

/// Caller content routed to a named (or default) slot.
///
/// Carries the target slot name, defaulting to the default slot when unnamed.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
pub struct SlotChild {
	/// The target slot name, or `None` for the default slot.
	pub name: Option<SmolStr>,
}

impl SlotChild {
	/// Content routed to the default (unnamed) slot.
	pub fn new() -> Self { Self::default() }

	/// Content routed to a named slot.
	pub fn named(name: impl Into<SmolStr>) -> Self {
		Self {
			name: Some(name.into()),
		}
	}

	/// The slot name this content targets, defaulting to [`DEFAULT_SLOT`].
	pub fn slot_name(&self) -> &str {
		self.name.as_deref().unwrap_or(DEFAULT_SLOT)
	}
}

/// Resolves every [`SlotChild`] under `root` into its matching [`SlotTarget`].
///
/// Synchronous over the world: collects each composition scope's direct
/// [`SlotChild`]s and the [`SlotTarget`]s reachable in its subtree, then matches
/// them positionally. A [`SlotTarget`] is replaced by its resolved children; an
/// unfilled target keeps its fallback. An unmatched [`SlotChild`] returns an
/// error naming the missing slot and listing the available targets.
///
/// Resolution iterates to a fixpoint: a pass splices each scope's matched
/// content, which exposes deeper targets (a relay buried under another scope's
/// content becomes reachable once that content is spliced in), so multi-level
/// forwarding resolves one hop per pass until no wiring remains. Only then is
/// leftover content an error.
///
/// Returns `Err` so the walker can ride a failure onto [`TemplateError`].
pub fn resolve_slots(world: &mut World, root: Entity) -> Result {
	loop {
		let plan = world.run_system_cached_with(plan_slots, root)?;
		// nothing matched this pass: the resolution is stable, so leftover content
		// is genuinely unconsumed.
		if plan.wirings.is_empty() {
			return report_unmatched(plan);
		}
		apply_wirings(world, plan.wirings);
	}
}

/// Splices each wiring's content into its target.
fn apply_wirings(world: &mut World, wirings: Vec<SlotWiring>) {
	for SlotWiring { target, content } in wirings {
		splice_into_target(world, target, content);
	}
}

/// Surfaces the first unconsumed slot as an error, naming it and the available
/// targets in its scope.
fn report_unmatched(plan: SlotPlan) -> Result {
	if let Some(unmatched) = plan.unmatched.first() {
		bevybail!(
			"unconsumed slot content for slot {:?}; available targets: {:?}",
			unmatched.name,
			unmatched.available
		);
	}
	OK
}

/// One slot wiring: a target and the ordered content routed into it.
struct SlotWiring {
	target: Entity,
	content: Vec<Entity>,
}

/// The materialized slot resolution: wirings to apply plus any unmatched
/// content that should surface as an error.
#[derive(Default)]
struct SlotPlan {
	wirings: Vec<SlotWiring>,
	/// Unconsumed `(slot_name, available_targets)` per composition scope.
	unmatched: Vec<UnmatchedSlot>,
}

struct UnmatchedSlot {
	name: SmolStr,
	available: Vec<SmolStr>,
}

/// Computes the wiring plan, one positional match per composition scope.
///
/// A composition scope is a distinct `ChildOf` parent of [`SlotChild`] content.
/// Each is matched independently so sibling templates never pool their slots.
fn plan_slots(
	root: In<Entity>,
	children: Query<&Children>,
	parents: Query<&ChildOf>,
	slot_children: Query<&SlotChild>,
	slot_targets: Query<&SlotTarget>,
) -> SlotPlan {
	// composition scopes: the distinct parents of routed `SlotChild` content.
	let scopes: HashSet<Entity> = children
		.iter_descendants_inclusive(*root)
		.filter(|entity| slot_children.contains(*entity))
		.filter_map(|entity| parents.get(entity).ok().map(ChildOf::parent))
		.collect();

	let mut plan = SlotPlan::default();
	for scope in scopes.iter().copied() {
		plan_scope(
			scope,
			&scopes,
			&children,
			&slot_children,
			&slot_targets,
			&mut plan,
		);
	}
	plan
}

/// Matches one composition scope's direct content to its subtree's targets.
fn plan_scope(
	scope: Entity,
	scopes: &HashSet<Entity>,
	children: &Query<&Children>,
	slot_children: &Query<&SlotChild>,
	slot_targets: &Query<&SlotTarget>,
	plan: &mut SlotPlan,
) {
	// sources: this scope's own direct content, in document order.
	let mut sources: Vec<(SmolStr, Entity)> = children
		.get(scope)
		.into_iter()
		.flat_map(Children::iter)
		.filter_map(|child| {
			slot_children
				.get(child)
				.ok()
				.map(|content| (content.slot_name().into(), child))
		})
		.collect();

	// targets: this scope's structural slots, descending its subtree but never
	// crossing into routed content or a nested scope.
	let mut targets = collect_targets(scope, scopes, children, slot_children, slot_targets);

	// fixpoint: a filled transfer target re-opens, so deeper content flows.
	let mut next = 0;
	while next < targets.len() {
		let (target_name, target) = targets[next].clone();
		next += 1;

		let mut matched = Vec::new();
		sources.retain(|(name, content)| {
			// a relay must not consume itself: its `SlotChild` side and its
			// re-opened `SlotTarget` side share one entity and slot name.
			if *name == target_name && *content != target {
				matched.push(*content);
				false
			} else {
				true
			}
		});
		if matched.is_empty() {
			continue;
		}
		// a matched transfer (a `SlotChild` that is itself a `SlotTarget`)
		// re-opens as a target named by its own slot, forwarding deeper content.
		for content in matched.iter().copied() {
			if let Ok(child_target) = slot_targets.get(content) {
				targets.push((child_target.slot_name().into(), content));
			}
		}
		plan.wirings.push(SlotWiring {
			target,
			content: matched,
		});
	}

	// any content left over is an error, except a forwarding relay (a `SlotChild`
	// that is itself a `SlotTarget`): its target resolves in a deeper scope, so an
	// unmatched relay is a pass-through, not unconsumed content.
	for (name, content) in sources {
		if slot_targets.contains(content) {
			continue;
		}
		plan.unmatched.push(UnmatchedSlot {
			name,
			available: targets.iter().map(|(name, _)| name.clone()).collect(),
		});
	}
}

/// Collects the [`SlotTarget`]s reachable from `scope`, stopping at routed
/// content and nested composition scopes so a parent never sees a nested
/// template's targets.
fn collect_targets(
	scope: Entity,
	scopes: &HashSet<Entity>,
	children: &Query<&Children>,
	slot_children: &Query<&SlotChild>,
	slot_targets: &Query<&SlotTarget>,
) -> Vec<(SmolStr, Entity)> {
	let mut targets = Vec::new();
	let mut stack = vec![scope];
	while let Some(entity) = stack.pop() {
		if entity != scope && let Ok(target) = slot_targets.get(entity) {
			// a target (including a forwarding relay that is also a `SlotChild`) is
			// visible to this scope so the parent can fill it; its children are
			// fallback or deeper content, not further targets for this scope.
			targets.push((target.slot_name().into(), entity));
			continue;
		}
		// do not descend into plain routed content or a nested scope's own subtree.
		if entity != scope
			&& (slot_children.contains(entity) || scopes.contains(&entity))
		{
			continue;
		}
		if let Ok(child_list) = children.get(entity) {
			stack.extend(child_list.iter().rev());
		}
	}
	targets
}


/// Replaces a [`SlotTarget`]'s children with `content`, in order.
///
/// The target keeps its identity (a transparent fragment) but loses its
/// fallback children and its [`SlotTarget`] marker; the routed content becomes
/// its children, each its own entity, with its [`SlotChild`] marker stripped.
fn splice_into_target(world: &mut World, target: Entity, content: Vec<Entity>) {
	// drop the fallback children: they only render when no content is supplied.
	let fallback: Vec<Entity> = world
		.entity(target)
		.get::<Children>()
		.into_iter()
		.flat_map(|children| children.iter())
		.collect();
	for child in fallback {
		world.entity_mut(child).despawn();
	}
	// route each content entity in as a child, stripping its routing marker.
	for child in content.iter().copied() {
		world.entity_mut(child).remove::<SlotChild>().insert(ChildOf(target));
	}
	// the target is now a transparent fragment: drop its placeholder marker.
	world.entity_mut(target).remove::<SlotTarget>();
}

#[cfg(test)]
mod test {
	use super::*;

	/// Spawns a `<div>`-like marker entity carrying `Name` for assertion.
	fn node(world: &mut World, name: &str) -> Entity {
		world.spawn(Name::new(name.to_string())).id()
	}

	/// Names of an entity's children, in order.
	fn child_names(world: &World, entity: Entity) -> Vec<String> {
		world
			.entity(entity)
			.get::<Children>()
			.into_iter()
			.flat_map(|children| children.iter())
			.filter_map(|child| {
				world.entity(child).get::<Name>().map(|n| n.to_string())
			})
			.collect()
	}

	#[beet_core::test]
	fn named_and_default() {
		let mut world = World::new();
		// scope with a default target and a named "header" target.
		let scope = node(&mut world, "scope");
		let default_target = world
			.spawn((SlotTarget::new(), ChildOf(scope)))
			.id();
		let header_target = world
			.spawn((SlotTarget::named("header"), ChildOf(scope)))
			.id();
		// caller content: default body + named header.
		let body = world.spawn((Name::new("body"), SlotChild::new(), ChildOf(scope))).id();
		let title =
			world.spawn((Name::new("title"), SlotChild::named("header"), ChildOf(scope))).id();

		resolve_slots(&mut world, scope).unwrap();

		child_names(&world, default_target).xpect_eq(vec!["body".to_string()]);
		child_names(&world, header_target).xpect_eq(vec!["title".to_string()]);
		// markers stripped.
		world.entity(body).contains::<SlotChild>().xpect_false();
		world.entity(title).contains::<SlotChild>().xpect_false();
		world.entity(default_target).contains::<SlotTarget>().xpect_false();
	}

	#[beet_core::test]
	fn fallback_when_unfilled() {
		let mut world = World::new();
		let scope = node(&mut world, "scope");
		let target = world.spawn((SlotTarget::named("header"), ChildOf(scope))).id();
		world.spawn((Name::new("fallback"), ChildOf(target)));
		// a different slot is filled so the scope is a valid composition scope.
		let default_target = world.spawn((SlotTarget::new(), ChildOf(scope))).id();
		world.spawn((Name::new("body"), SlotChild::new(), ChildOf(scope)));

		resolve_slots(&mut world, scope).unwrap();

		// header keeps its fallback; default is filled.
		child_names(&world, target).xpect_eq(vec!["fallback".to_string()]);
		child_names(&world, default_target).xpect_eq(vec!["body".to_string()]);
	}

	#[beet_core::test]
	fn multi_child_order_preserved() {
		let mut world = World::new();
		let scope = node(&mut world, "scope");
		let target = world.spawn((SlotTarget::new(), ChildOf(scope))).id();
		for name in ["one", "two", "three"] {
			world.spawn((Name::new(name), SlotChild::new(), ChildOf(scope)));
		}

		resolve_slots(&mut world, scope).unwrap();

		child_names(&world, target).xpect_eq(vec![
			"one".to_string(),
			"two".to_string(),
			"three".to_string(),
		]);
	}

	#[beet_core::test]
	fn unconsumed_child_errors() {
		let mut world = World::new();
		let scope = node(&mut world, "scope");
		// only a default target, but content targets "header".
		world.spawn((SlotTarget::new(), ChildOf(scope)));
		world.spawn((Name::new("title"), SlotChild::named("header"), ChildOf(scope)));

		let err = resolve_slots(&mut world, scope).unwrap_err();
		err.to_string().xpect_contains("header");
		err.to_string().xpect_contains("default");
	}

	#[beet_core::test]
	fn no_slot_stealing_between_siblings() {
		let mut world = World::new();
		let root = node(&mut world, "root");
		// two sibling scopes, each a default target with its own content.
		let scope_a = world.spawn((Name::new("a"), ChildOf(root))).id();
		let target_a = world.spawn((SlotTarget::new(), ChildOf(scope_a))).id();
		world.spawn((Name::new("contentA"), SlotChild::new(), ChildOf(scope_a)));
		let scope_b = world.spawn((Name::new("b"), ChildOf(root))).id();
		let target_b = world.spawn((SlotTarget::new(), ChildOf(scope_b))).id();
		world.spawn((Name::new("contentB"), SlotChild::new(), ChildOf(scope_b)));

		resolve_slots(&mut world, root).unwrap();

		child_names(&world, target_a).xpect_eq(vec!["contentA".to_string()]);
		child_names(&world, target_b).xpect_eq(vec!["contentB".to_string()]);
	}

	#[beet_core::test]
	fn transfer_forwards_through_level() {
		let mut world = World::new();
		// Astro-style re-projection in a single composition scope, mirroring a
		// built `Outer` whose body is `<Inner><Slot name="header"
		// bx:slot="header"/></Inner>`: after the build splices Outer's caller
		// content into the Inner scope, that scope's direct children are the
		// transfer relay and the caller content, both targeting "header".
		let scope = node(&mut world, "scope");
		// the inner template's structural header target.
		let inner_header = world
			.spawn((SlotTarget::named("header"), ChildOf(scope)))
			.id();
		// the transfer relay: routed into inner ("header"), and itself a
		// re-opening "header" target carrying the forwarded content.
		world.spawn((
			SlotChild::named("header"),
			SlotTarget::named("header"),
			ChildOf(scope),
		));
		// Outer's caller content for "header", spliced into the same scope.
		world.spawn((
			Name::new("title"),
			SlotChild::named("header"),
			ChildOf(scope),
		));

		resolve_slots(&mut world, scope).unwrap();

		// the forwarded title lands in inner's header target's subtree, having
		// flowed through the re-opening relay rather than being left unconsumed.
		let title_under_header = world
			.entity(inner_header)
			.get::<Children>()
			.into_iter()
			.flat_map(|children| children.iter())
			.flat_map(|child| {
				core::iter::once(child).chain(
					world
						.entity(child)
						.get::<Children>()
						.into_iter()
						.flat_map(|grandchildren| {
							grandchildren.iter().collect::<Vec<_>>()
						}),
				)
			})
			.any(|entity| {
				world
					.entity(entity)
					.get::<Name>()
					.map(|name| name.as_str() == "title")
					.unwrap_or(false)
			});
		title_under_header.xpect_true();
	}

	#[beet_core::test]
	fn relay_does_not_consume_itself() {
		// a relay whose `SlotChild` and `SlotTarget` sides share a slot name must
		// forward sibling content rather than swallowing it into its own target.
		let mut world = World::new();
		let scope = node(&mut world, "scope");
		// the deeper structural target the relay forwards into.
		let inner = world.spawn((SlotTarget::named("head"), ChildOf(scope))).id();
		// the relay: routed as "head" content, and itself a re-opening "head" target.
		world.spawn((
			SlotChild::named("head"),
			SlotTarget::named("head"),
			ChildOf(scope),
		));
		// caller content for "head", which must flow through the relay, not be
		// eaten by the relay matching its own name.
		world.spawn((Name::new("meta"), SlotChild::named("head"), ChildOf(scope)));

		resolve_slots(&mut world, scope).unwrap();

		// the meta reached the inner target's subtree (via the relay).
		let reached = world
			.entity(inner)
			.get::<Children>()
			.into_iter()
			.flat_map(|children| children.iter())
			.flat_map(|child| {
				core::iter::once(child).chain(
					world
						.entity(child)
						.get::<Children>()
						.into_iter()
						.flat_map(|grandchildren| {
							grandchildren.iter().collect::<Vec<_>>()
						}),
				)
			})
			.any(|entity| {
				world
					.entity(entity)
					.get::<Name>()
					.map(|name| name.as_str() == "meta")
					.unwrap_or(false)
			});
		reached.xpect_true();
	}
}
