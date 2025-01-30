use crate::events::OnRun;
use bevy::ecs::component::StorageType;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;

/// Very simple pre-entity relations mechanic,
/// add this as an outgoing relation to entities with actions and other components that require it.
#[derive(Debug, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct TargetEntity(pub Entity);

impl MapEntities for TargetEntity {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		**self = entity_mapper.map_entity(**self);
	}
}

/// Adds a [`TargetEntity`] that points to the root [`Parent`] of this entity.
///
/// If you need to dynamically update [`TargetEntity`] whenever you reparent a tree, add the
/// [`DynamicRootIsTargetEntity`] component to the root of the tree.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(TargetEntity(||TargetEntity(Entity::PLACEHOLDER)))]
pub struct RootIsTargetEntity;

pub fn set_root_as_target_entity(
	parents: Query<&Parent>,
	mut query: Query<(Entity, &mut TargetEntity), Added<RootIsTargetEntity>>,
) {
	for (entity, mut target_entity) in query.iter_mut() {
		let root = parents.root_ancestor(entity);
		target_entity.0 = root;
	}
}

/// If present, will monitor if this entity has its Parent changed, and if so, will repair the
/// TargetEntity of all descendants with a [`RootIsTargetEntity`] component.
#[derive(Debug, Reflect)]
#[reflect(Component)]
pub struct DynamicRootIsTargetEntity;

impl Component for DynamicRootIsTargetEntity {
	const STORAGE_TYPE: StorageType = StorageType::Table;
	fn register_component_hooks(
		hooks: &mut bevy::ecs::component::ComponentHooks,
	) {
		hooks.on_add(|mut world, entity, _component_id| {
			let mut observer = Observer::new(fix_target_on_run);
			observer.watch_entity(entity);
			world.commands().spawn(observer);
		});
	}
}

fn fix_target_on_run(
	t: Trigger<OnRun>,
	q_parent_changed: Query<
		(),
		(With<DynamicRootIsTargetEntity>, Changed<Parent>),
	>,
	mut q_children: Query<&mut TargetEntity, With<RootIsTargetEntity>>,
	children_query: Query<&Children>,
	parent_query: Query<&Parent>,
) {
	if !q_parent_changed.contains(t.entity()) {
		// This means that the parent hasn't changed since the last time OnRun was triggered
		// for this entity - so we avoid descending the hierarchy.
		//
		// Changed<> has the desired behavior because this system is specific to an entity,
		// not a global observer.
		return;
	}
	let entity = t.entity();
	let root = parent_query.root_ancestor(entity);
	for child in children_query.iter_descendants(entity) {
		if let Ok(mut target_entity) = q_children.get_mut(child) {
			target_entity.0 = root;
		}
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;

	// "A" doesn't have RootIsTargetEntity
	#[derive(Clone, Component, Debug, Reflect, Action)]
	#[require(Name(||Name::new("A")))]
	struct A;

	// "B" has a RootIsTargetEntity
	#[derive(Clone, Component, Debug, Reflect, Action)]
	#[require(Name(||Name::new("B")))]
	#[require(RootIsTargetEntity)]
	struct B;

	// "C" will be given a TargetEntity manually, to ensure it does not get trampled.
	#[derive(Clone, Component, Debug, Reflect, Action)]
	#[require(Name(||Name::new("C")))]
	struct C;

	// "D" has a RootIsTargetEntity, and is a level deeper in the tree
	#[derive(Clone, Component, Debug, Reflect, Action)]
	#[require(Name(||Name::new("D")))]
	#[require(RootIsTargetEntity)]
	struct D;

	#[derive(Resource, Default)]
	struct TestState {
		a_ran: bool,
		b_ran: bool,
		c_ran: bool,
		d_ran: bool,
	}

	// generates a Trigger<OnRun> function that asserts that the TargetEntity is as expected
	#[allow(clippy::type_complexity)]
	fn on_run_asserter(
		target_entity_should_be: Option<Entity>,
	) -> impl Fn(
		Trigger<OnRun>,
		Query<(&Name, Option<&TargetEntity>)>,
		Commands,
		ResMut<TestState>,
	) {
		move |t: Trigger<OnRun>,
		      q: Query<(&Name, Option<&TargetEntity>)>,
		      mut commands: Commands,
		      mut test_state: ResMut<TestState>| {
			let Ok((name, target_entity)) = q.get(t.entity()) else {
				panic!("OnRun trigger failed to find entity");
			};
			println!(
				"OnRun triggered for: {name:?}, target_entity: {target_entity:?}, expected: {target_entity_should_be:?}"
			);
			let target_entity = target_entity.map(|te| te.0);
			assert_eq!(
				target_entity,
				target_entity_should_be,
				"{} TargetEntity should be {target_entity_should_be:?}, but was {target_entity:?} (for: {name:?})",
				t.entity()
			);
			if name.as_str() == "A" {
				test_state.a_ran = true;
			} else if name.as_str() == "B" {
				test_state.b_ran = true;
			} else if name.as_str() == "C" {
				test_state.c_ran = true;
			} else if name.as_str() == "D" {
				test_state.d_ran = true;
			} else {
				panic!("OnRun doesn't handle this entity Name: {name:?}");
			}
			commands.trigger_targets(OnRunResult::success(), t.entity());
		}
	}

	fn setup_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			// bevy::log::LogPlugin::default(),
			LifecyclePlugin,
		))
		.init_resource::<TestState>();
		app
	}

	/// Tests that TargetEntity is set correctly when using RootIsTargetEntity, assuming
	/// the hierarchy is statically set at spawn time.
	#[test]
	fn test_static_target_entity() {
		let mut app = setup_app();
		// spawn an entity to set manually as the TargetEntity for C
		let target_for_c = app.world_mut().spawn(Name::new("C's Target")).id();
		// spawn a standalone behaviour tree, and run it.
		// We would expect "B", having "RootIsTargetEntity", to have a TargetEntity of the tree root
		// when OnRun is triggered.
		let bt = app
			.world_mut()
			.spawn((Name::new("root"), SequenceFlow))
			.with_children(|parent| {
				let tree_root = parent.parent_entity();
				parent.spawn(A).observe(on_run_asserter(None));
				parent.spawn(B).observe(on_run_asserter(Some(tree_root)));
				parent
					.spawn((C, TargetEntity(target_for_c)))
					.observe(on_run_asserter(Some(target_for_c)));
				parent.spawn(SequenceFlow).with_children(|parent| {
					parent.spawn(D).observe(on_run_asserter(Some(tree_root)));
				});
			})
			.id();
		// there needs to be a world update for the system that sets TargetEntity from RootIsTargetEntity to run
		app.update();
		// check everything is as expected
		let (a_target, b_target, c_target, d_target) =
			app.fetch_target_entity_components(bt);
		// A doesn't have RootIsTargetEntity, so shouldn't have a TargetEntity.
		assert!(a_target.is_none(), "A should not have TargetEntity");
		// B and D should have been fixed to use the new parent entity
		assert_eq!(
			b_target,
			Some(&TargetEntity(bt)),
			"B's TargetEntity should be the root of the behavior tree"
		);
		assert_eq!(
			d_target,
			Some(&TargetEntity(bt)),
			"D's TargetEntity should be the root of the behavior tree"
		);
		// C should have been left alone, it was given a TargetEntity manually.
		assert_eq!(
			c_target,
			Some(&TargetEntity(target_for_c)),
			"C's TargetEntity should be the manually set target"
		);
		// this will run the assertions in the OnRun triggers
		app.world_mut().trigger_targets(OnRun, bt);
		app.update();
		// verify the observers ran
		let test_state = app.world().resource::<TestState>();
		assert!(test_state.a_ran, "A's OnRun should have run");
		assert!(test_state.b_ran, "B's OnRun should have run");
		assert!(test_state.c_ran, "C's OnRun should have run");
		assert!(test_state.d_ran, "D's OnRun should have run");
	}

	/// Tests that TargetEntity is set correctly when using DynamicRootIsTargetEntity, by spawning
	/// the behavior tree, then later adding it as a child to a new parent.
	#[test]
	fn test_dynamic_target_entity() {
		let mut app = setup_app();
		// spawn an entity to set manually as the TargetEntity for C
		let target_for_c = app.world_mut().spawn(Name::new("C's Target")).id();
		// spawn the character entity with no children
		// we'll spawn the behaviour tree, then add it as a child of the character.
		let character =
			app.world_mut().spawn(Name::new("Character Entity")).id();
		println!("character: {character:?}");
		// spawn a standalone behaviour tree, then add it as a child to the character.
		// We would expect "B", having "RootIsTargetEntity", to have a TargetEntity of the character,
		// because of the presence of the DynamicRootIsTargetEntity component.
		let bt = app
			.world_mut()
			.spawn((Name::new("root"), SequenceFlow, DynamicRootIsTargetEntity))
			.with_children(|parent| {
				parent.spawn(A).observe(on_run_asserter(None));
				parent.spawn(B).observe(on_run_asserter(Some(character)));
				parent
					.spawn((C, TargetEntity(target_for_c)))
					.observe(on_run_asserter(Some(target_for_c)));
				parent.spawn(SequenceFlow).with_children(|parent| {
					parent.spawn(D).observe(on_run_asserter(Some(character)));
				});
			})
			.id();
		println!("bt, before reparenting: {bt:?}");
		// An update now will set the TargetEntity of B to "bt", the *current* root ancestor of B.
		// Doing it to verify the presence of DynamicRootIsTargetEntity hasn't broken this.
		app.update();
		let b = app.world().entity(bt).nth_child(1).unwrap();
		let b_target = app.world().get::<TargetEntity>(b);
		assert_eq!(
			b_target,
			Some(&TargetEntity(bt)),
			"B's TargetEntity should be the tree root before reparenting"
		);
		// Note that we haven't triggered OnRun on the tree yet, because our OnRun observer is
		// going to assert that B's TargetEntity == character. So we'll reparent the tree now:
		app.world_mut().entity_mut(bt).set_parent(character);
		// Repairing the TargetEntity happens when OnRun is triggered:
		app.world_mut().trigger_targets(OnRun, bt);
		// check the TargetEntity of B and D is now the character
		let (a_target, b_target, c_target, d_target) =
			app.fetch_target_entity_components(bt);
		// A doesn't have RootIsTargetEntity, so shouldn't have a TargetEntity.
		assert!(a_target.is_none(), "A should not have TargetEntity");
		// B and D should have been fixed to use the new parent entity
		assert_eq!(
			b_target,
			Some(&TargetEntity(character)),
			"B's TargetEntity should be the character we reparented to"
		);
		assert_eq!(
			d_target,
			Some(&TargetEntity(character)),
			"D's TargetEntity should be the character we reparented to"
		);
		// C should have been left alone, it was given a TargetEntity manually.
		assert_eq!(
			c_target,
			Some(&TargetEntity(target_for_c)),
			"C's TargetEntity should be the manually set target"
		);
		// update so the observer can run, and also do their assertions
		app.update();
		// verify the observers ran
		let test_state = app.world().resource::<TestState>();
		assert!(test_state.a_ran, "A's OnRun should have run");
		assert!(test_state.b_ran, "B's OnRun should have run");
		assert!(test_state.c_ran, "C's OnRun should have run");
		assert!(test_state.d_ran, "D's OnRun should have run");
	}

	trait FetchTargetEntityComponents {
		fn fetch_target_entity_components(
			&self,
			root: Entity,
		) -> (
			Option<&TargetEntity>,
			Option<&TargetEntity>,
			Option<&TargetEntity>,
			Option<&TargetEntity>,
		);
	}

	impl FetchTargetEntityComponents for App {
		fn fetch_target_entity_components(
			&self,
			root: Entity,
		) -> (
			Option<&TargetEntity>,
			Option<&TargetEntity>,
			Option<&TargetEntity>,
			Option<&TargetEntity>,
		) {
			let a = self.world().entity(root).nth_child(0).unwrap();
			let b = self.world().entity(root).nth_child(1).unwrap();
			let c = self.world().entity(root).nth_child(2).unwrap();
			let d_parent = self.world().entity(root).nth_child(3).unwrap();
			let d = self.world().entity(d_parent).nth_child(0).unwrap();
			(
				self.world().get::<TargetEntity>(a),
				self.world().get::<TargetEntity>(b),
				self.world().get::<TargetEntity>(c),
				self.world().get::<TargetEntity>(d),
			)
		}
	}

	trait NthChild {
		fn nth_child(&self, n: usize) -> Option<Entity>;
	}

	impl NthChild for EntityRef<'_> {
		fn nth_child(&self, n: usize) -> Option<Entity> {
			self.get::<Children>()
				.and_then(|children| children.get(n).copied())
		}
	}
}
