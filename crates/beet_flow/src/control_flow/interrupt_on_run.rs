use crate::prelude::*;
use bevy::prelude::*;

/// Mark a behavior as uninterruptible, the `Running` component
/// will only be removed if [`OnResult`] is called on it,
/// either directly or via bubbling.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component)]
pub struct NoInterrupt;


/// Whenever [`OnRun`] is called,
/// removes [`Running`] from children unless they have a [`NoInterrupt`].
/// Unlike [`interrupt_on_result`], this does not remove the `Running` component
/// from the action entity.
pub(super) fn interrupt_on_run<T: RunPayload>(
	ev: Trigger<OnRunAction<T>>,
	mut commands: Commands,
	should_remove: Populated<(), (With<Running>, Without<NoInterrupt>)>,
	children: Populated<&Children>,
) {
	let action = ev.resolve_action();
	for child in children
		.iter_descendants(action)
		.filter(|child| should_remove.contains(*child))
	{
		commands.entity(child).remove::<Running>();
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			// root is running
			.spawn(Running::default())
			.with_children(|parent| {
				// not running
				parent.spawn_empty();
				// running
				parent.spawn(Running::default());
				// running with child
				parent.spawn(Running::default()).with_children(|parent| {
					parent.spawn(Running::default());
				});
				//only child running
				parent.spawn_empty().with_children(|parent| {
					parent.spawn(Running::default());
				});
				// running with no interrupt, with running child
				parent
					.spawn((Running::default(), NoInterrupt))
					.with_children(|parent| {
						parent.spawn(Running::default());
					});
				// // only no interrupt, with running child
				// parent.spawn(NoInterrupt).with_children(|parent| {
				// 	parent.spawn(Running::default());
				// });
			})
			.flush_trigger(OnRun::local())
			.id();

		EntityTree::new_with_world(entity, &world)
			.component_tree::<Running>(&world)
			.xpect()
			.to_be(
				TreeNode::new(Some(&Running::new(Entity::from_raw(10))))
					.with_leaf(None)
					.with_leaf(None)
					.with_child(TreeNode::new(None).with_leaf(None))
					.with_child(TreeNode::new(None).with_leaf(None))
					.with_child(
						TreeNode::new(Some(&Running::new(Entity::from_raw(
							17,
						))))
						.with_leaf(None),
					), // .with_child(Tree::new(None).with_leaf(Some(&Running))),
			);
	}
}
