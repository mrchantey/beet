use beet_common::prelude::*;
use bevy::prelude::*;



/// Marker to discern that this is the static template, not an instance.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct StaticTree;


pub(super) fn apply_static_trees(
	_: TempNonSendMarker,
	mut commands: Commands,
	instances: Populated<(Entity, &MacroIdx), Without<StaticTree>>,
	static_trees: Query<(Entity, &MacroIdx), With<StaticTree>>,
) {
	for (instance, static_tree) in
		instances.iter().filter_map(|(instance, idx)| {
			static_trees
				.iter()
				.find(|(_, static_idx)| *static_idx == idx)
				.map(|(static_tree, _)| (instance, static_tree))
		}) {
		commands.entity(instance).clear();
		commands
			.entity(static_tree)
			.clone_with(instance, |builder| {
				builder.linked_cloning(true);
			});
		println!("here");
	}
}





#[cfg(test)]
mod test {
	use super::*;
	use crate::as_beet::*;
	use bevy::ecs::system::RunSystemOnce;
	use sweet::prelude::*;

	#[test]
	#[ignore = "TODO OnSpawnTemplate"]
	fn works() {
		let instance = rsx! {<div>{7}</div>};
		// because ExprIdx matches, this should be replace with 7
		let tree = rsx! {<div><span>{()}</span><br/></div>};
		let mut world = World::new();
		let instance = world.spawn(instance).insert(MacroIdx::default()).id();
		let _tree = world
			.spawn(tree)
			.insert((MacroIdx::default(), StaticTree))
			.id();
		world.run_system_once(apply_static_trees).unwrap();
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
			.xpect()
			.to_be("<div><span>7</span><br/></div>");
	}
}
