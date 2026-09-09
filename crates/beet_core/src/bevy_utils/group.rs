//! Ordered membership: the [`Group`] marker, the [`MemberOf`] relationship that
//! enrolls into one, and the append-ordered [`Members`] it maintains.
//!
//! Membership is a second axis beside the hierarchy. A member is enrolled by
//! reference, so it may live anywhere in the tree, and [`Members`] reports it in
//! the order it was enrolled: bsx resolves relations in one pass in insert
//! order, so file order IS member order, and anything spawned later appends at
//! the end.
use crate::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;

/// An entity that owns an ordered list of [`Members`].
///
/// Children of a group auto-enroll, so the common case is authored as nesting:
///
/// ```
/// # use beet_core::prelude::*;
/// # let mut world = World::new();
/// let group = world
///     .spawn((Group, children![Name::new("first"), Name::new("second")]))
///     .flush();
/// world.entity(group).get::<Members>().unwrap().len().xpect_eq(2);
/// ```
///
/// A child carrying an explicit [`MemberOf`] is NOT auto-enrolled in its parent:
/// explicit membership suppresses the default, which is what lets a template
/// placed inside one group spawn a member of another.
///
/// A group carries no direction. Running one backwards is a property of the RUN,
/// see `RunGroup` in `beet_action`, so "a reverse group inside a forward run" is
/// deliberately inexpressible.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = Group::on_add)]
pub struct Group;

impl Group {
	/// The `on_add` hook: arm the world's auto-enrollment observer, then enroll
	/// any children the entity already had.
	///
	/// Two steps because a group is assembled in one of two orders. Usually the
	/// children arrive after the marker, and the observer catches them: bevy
	/// flushes a hook's commands *before* the spawning bundle's `children!`
	/// effect runs, so the observer is armed in time to see the first one. The
	/// sweep covers the other order, a `Group` inserted onto an entity that
	/// already has children.
	///
	/// The observer arms itself here rather than in a plugin so that spawning a
	/// group is the whole declaration, in any world.
	fn on_add(mut world: DeferredWorld, cx: HookContext) {
		let group = cx.entity;
		world.commands().queue(move |world: &mut World| {
			if !world.contains_resource::<AutoEnroll>() {
				world.insert_resource(AutoEnroll);
				world.add_observer(auto_enroll);
			}
			Group::enroll_children(world, group);
		});
	}

	/// Enroll every child of `group` that declared no [`MemberOf`] of its own,
	/// appending in [`Children`] order.
	///
	/// Idempotent: an enrolled child carries the relationship that excludes it
	/// from the next pass.
	fn enroll_children(world: &mut World, group: Entity) {
		let Some(children) = world
			.get_entity(group)
			.ok()
			.and_then(|entity| entity.get::<Children>())
			.map(|children| children.iter().collect::<Vec<_>>())
		else {
			return;
		};
		for child in children {
			Self::enroll(world, group, child);
		}
	}

	/// Enroll `child` in `group` unless it declared a [`MemberOf`] of its own.
	fn enroll(world: &mut World, group: Entity, child: Entity) {
		let Ok(mut child) = world.get_entity_mut(child) else {
			return;
		};
		if child.contains::<MemberOf>() {
			return;
		}
		child.insert(MemberOf(group));
	}
}

/// Enrolls this entity in a [`Group`], appending it to that group's [`Members`].
///
/// Written at the tag as a spread, `{MemberOf($deploy)}`, so a member says which
/// group it joins where it is authored. Refs are explicit: nothing resolves by
/// field-name matching or by ambient lookup.
#[derive(Debug, Clone, Copy, Deref, Component, Reflect)]
#[reflect(Component)]
#[relationship(relationship_target = Members)]
pub struct MemberOf(pub Entity);

/// The ordered members of a [`Group`], appended in enrollment order and managed
/// by [`MemberOf`].
///
/// Never `linked_spawn`: a member lives in the hierarchy, and despawning the
/// group it was enrolled in is not despawning it.
#[derive(Debug, Deref, Component, Reflect)]
#[reflect(Component)]
#[relationship_target(relationship = MemberOf)]
#[component(on_add = hook_ext::observe(assert_group))]
pub struct Members(Vec<Entity>);

/// Marks a world whose [`auto_enroll`] observer is installed, so the first
/// [`Group`] arms it and later ones do not stack a second copy.
#[derive(Resource)]
struct AutoEnroll;

/// A new child of a [`Group`] joins it, unless it declared a [`MemberOf`] of its
/// own.
fn auto_enroll(ev: On<Insert, ChildOf>, mut commands: Commands) {
	let child = ev.entity;
	commands.queue(move |world: &mut World| {
		let Some(parent) = world.get::<ChildOf>(child).map(ChildOf::parent)
		else {
			return;
		};
		if world.get::<Group>(parent).is_some() {
			Group::enroll(world, parent, child);
		}
	});
}

/// Only a [`Group`] can be enrolled into, so a [`MemberOf`] naming a plain
/// entity fails where the document settles rather than where the group is later
/// run.
///
/// Checked on [`Ready`] rather than when the relation lands, because a `$ref`
/// may point forward: the member resolves against a placeholder the `<Group>`
/// tag has not been built onto yet.
fn assert_group(ev: On<Ready>, groups: Query<(), With<Group>>) -> Result {
	match groups.contains(ev.entity) {
		true => Ok(()),
		false => bevybail!(
			"{} has members but is not a `Group`, so nothing can run them",
			ev.entity
		),
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn names(world: &World, group: Entity) -> Vec<String> {
		world
			.entity(group)
			.get::<Members>()
			.map(|members| {
				members
					.iter()
					.map(|member| {
						world
							.entity(member)
							.get::<Name>()
							.map(Name::to_string)
							.unwrap_or_default()
					})
					.collect()
			})
			.unwrap_or_default()
	}

	#[crate::test]
	fn children_enroll_in_file_order() {
		let mut world = World::new();
		let group = world
			.spawn((Group, children![
				Name::new("one"),
				Name::new("two"),
				Name::new("three")
			]))
			.flush();
		names(&world, group).xpect_eq(vec![
			"one".to_string(),
			"two".to_string(),
			"three".to_string(),
		]);
	}

	/// A group marked onto an entity that already has children still enrolls
	/// them: the sweep covers the order the observer cannot.
	#[crate::test]
	fn a_late_group_enrolls_the_children_it_found() {
		let mut world = World::new();
		let group = world.spawn(children![Name::new("one")]).flush();
		world.entity_mut(group).insert(Group);
		world.flush();
		names(&world, group).xpect_eq(vec!["one".to_string()]);
	}

	/// An entity spawned into a group later joins at the end, so a runtime
	/// member never displaces an authored one.
	#[crate::test]
	fn a_later_child_appends() {
		let mut world = World::new();
		let group = world.spawn((Group, children![Name::new("one")])).flush();
		world.spawn((Name::new("two"), ChildOf(group)));
		world.flush();
		names(&world, group)
			.xpect_eq(vec!["one".to_string(), "two".to_string()]);
	}

	/// Explicit membership suppresses the default: this is what lets a template
	/// placed inside one group spawn a member of another.
	#[crate::test]
	fn an_explicit_member_does_not_join_its_parent() {
		let mut world = World::new();
		let other = world.spawn(Group).flush();
		let group = world
			.spawn((Group, children![
				Name::new("mine"),
				(Name::new("theirs"), MemberOf(other)),
			]))
			.flush();
		names(&world, group).xpect_eq(vec!["mine".to_string()]);
		names(&world, other).xpect_eq(vec!["theirs".to_string()]);
	}

	/// Enrollment is by reference, so a member may live anywhere in the tree.
	#[crate::test]
	fn remote_members_enroll_in_insert_order() {
		let mut world = World::new();
		let group = world.spawn(Group).flush();
		world.spawn((Name::new("one"), MemberOf(group)));
		world.spawn((Name::new("two"), MemberOf(group)));
		world.flush();
		names(&world, group)
			.xpect_eq(vec!["one".to_string(), "two".to_string()]);
	}

	#[crate::test]
	fn an_empty_group_has_no_members() {
		let mut world = World::new();
		let group = world.spawn(Group).flush();
		names(&world, group).xpect_empty();
	}
}

#[cfg(all(test, feature = "bsx"))]
mod bsx_test {
	use crate::prelude::*;

	/// Build `markup` into a world that registers the membership types.
	fn build(markup: &str) -> (World, Entity) {
		let mut world =
			(TemplatePlugin, DocumentPlugin, MinimalTypesPlugin).into_world();
		let nodes =
			BsxNode::parse_document(markup, &BsxParseConfig::bsx()).unwrap();
		let root = world
			.spawn_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap()
			.id();
		world.flush();
		(world, root)
	}

	/// A `$ref` may point forward, so a member declared above the group it
	/// joins enrolls all the same, in the order the file declares it.
	#[crate::test]
	fn a_forward_reference_enrolls_in_file_order() {
		let (world, root) = build(
			r#"<div>
				<span {MemberOf($later)}/>
				<em {MemberOf($later)}/>
				<Group bx:ref="later"/>
			</div>"#,
		);
		let host = world.entity(root).get::<Children>().unwrap()[0];
		let children = world.entity(host).get::<Children>().unwrap();
		let (first, second, group) = (children[0], children[1], children[2]);
		world
			.entity(group)
			.get::<Members>()
			.unwrap()
			.iter()
			.collect::<Vec<_>>()
			.xpect_eq(vec![first, second]);
	}

	/// A `$ref` naming something that never became a group fails when the
	/// document settles, rather than when the group is later run.
	#[crate::test]
	#[should_panic(expected = "has members but is not a `Group`")]
	fn enrolling_into_a_non_group_fails() {
		build(r#"<div><span {MemberOf($target)}/><em bx:ref="target"/></div>"#);
	}
}
