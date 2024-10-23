use crate::prelude::*;
use bevy::prelude::*;

/// Interruptable, this is recursive so children are also uninteruptable
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct NoInterrupt;


/// Whenever [`OnRun`] is called, this observer ensures no children are running
/// Only recurses children that have [`Running`] and do not have [`NoInterrupt`]
pub fn interrupt_running(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	children: Query<&Children>,
	no_interrupt: Query<(), With<NoInterrupt>>,
	running: Query<(), (With<Running>, Without<NoInterrupt>)>,
) {
	ChildrenExt::visit_or_cancel(trigger.entity(), &children, |entity| {
		// skip the entity that just started running
		if entity == trigger.entity() {
			return true;
		}
		if running.contains(entity) {
			commands.entity(entity).remove::<Running>();
		}
		false == no_interrupt.contains(entity)
	});
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();

		world.add_observer(interrupt_running);

		let entity = world
			// root is running
			.spawn(Running)
			.with_children(|parent| {
				// not running
				parent.spawn_empty();
				// running
				parent.spawn(Running);
				// running with child
				parent.spawn(Running).with_children(|parent| {
					parent.spawn(Running);
				});
				//only child running
				parent.spawn_empty().with_children(|parent| {
					parent.spawn(Running);
				});
				// running with no interrupt, with running child
				parent
					.spawn((Running, NoInterrupt))
					.with_children(|parent| {
						parent.spawn(Running);
					});
				// only no interrupt, with running child
				parent.spawn(NoInterrupt).with_children(|parent| {
					parent.spawn(Running);
				});
			})
			.flush_trigger(OnRun)
			.id();

		expect(
			EntityTree::new_with_world(entity, &world)
				.component_tree::<Running>(&world),
		)
		.to_be(
			Tree::new(Some(&Running))
				.with_leaf(None)
				.with_leaf(None)
				.with_child(Tree::new(None).with_leaf(None))
				.with_child(Tree::new(None).with_leaf(None))
				.with_child(Tree::new(Some(&Running)).with_leaf(Some(&Running)))
				.with_child(Tree::new(None).with_leaf(Some(&Running))),
		)?;
		Ok(())
	}
}
