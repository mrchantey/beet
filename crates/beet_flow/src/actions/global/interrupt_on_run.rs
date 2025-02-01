use crate::prelude::*;
use bevy::prelude::*;

/// Mark a behavior as uninterruptible, the `Running` component
/// will only be removed if [`OnRunResult`] is called on it,
/// either directly or via bubbling.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct NoInterrupt;


/// Whenever [`OnRun`] is called,
/// removes [`Running`] from children unless they have a [`NoInterrupt`]
pub fn interrupt_on_run(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	should_remove: Populated<(), (With<Running>, Without<NoInterrupt>)>,
	children: Populated<&Children>,
) {
	for child in children.iter_descendants(trigger.entity()) {
		if should_remove.contains(child) {
			commands.entity(child).remove::<Running>();
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut world = World::new();

		world.add_observer(interrupt_on_run);

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
				// // only no interrupt, with running child
				// parent.spawn(NoInterrupt).with_children(|parent| {
				// 	parent.spawn(Running);
				// });
			})
			.flush_trigger(OnRun)
			.id();

		expect(
			EntityTree::new_with_world(entity, &world)
				.component_tree::<Running>(&world),
		)
		.to_be(
			TreeNode::new(Some(&Running))
				.with_leaf(None)
				.with_leaf(None)
				.with_child(TreeNode::new(None).with_leaf(None))
				.with_child(TreeNode::new(None).with_leaf(None))
				.with_child(TreeNode::new(Some(&Running)).with_leaf(None)), // .with_child(Tree::new(None).with_leaf(Some(&Running))),
		);
	}
}
