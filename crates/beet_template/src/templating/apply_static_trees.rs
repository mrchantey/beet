use core::panic;

use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::prelude::TemplateRoot;
use crate::prelude::on_spawn_template;



/// Marker to discern that this is the static template, not an instance.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct StaticTree;

pub(super) fn apply_static_trees(
	mut commands: Commands,
	instances: Populated<(Entity, &MacroIdx), Without<StaticTree>>,
	static_trees: Query<(Entity, &MacroIdx), With<StaticTree>>,
	children: Query<&Children>,
	mut on_spawn_templates: Query<(&ExprIdx, &mut OnSpawnTemplate)>,
) -> Result {
	for (instance, static_tree) in
		instances.iter().filter_map(|(instance, idx)| {
			static_trees
				.iter()
				.find(|(_, static_idx)| *static_idx == idx)
				.map(|(static_tree, _)| (instance, static_tree))
		}) {
		// recursively clear entire instance tree
		commands
			.entity(instance)
			.despawn_related::<Children>()
			.despawn_related::<TemplateRoot>()
			.despawn_related::<Attributes>()
			.clear();
		commands
			.entity(static_tree)
			.clone_with(instance, |builder| {
				builder.linked_cloning(true);
				builder.add_observers(true);
			});
		let instance_exprs: HashMap<_, _> = children
			.iter_descendants_inclusive(instance)
			.filter_map(|child| match on_spawn_templates.get_mut(child) {
				Ok((idx, mut template)) => Some((*idx, template.take())),
				Err(_) => None,
			})
			.collect();
		// resolve template locations after clone
		commands.run_system_cached_with(
			apply_template_locations,
			(instance, instance_exprs),
		);
	}
	Ok(())
}


pub(super) fn apply_template_locations(
	In((entity, mut instance_exprs)): In<(
		Entity,
		HashMap<ExprIdx, OnSpawnTemplate>,
	)>,
	mut commands: Commands,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	expr_idxs: Query<&ExprIdx>,
) {
	let all_keys = instance_exprs.keys().cloned().collect::<Vec<_>>();
	let mut get_on_spawn = |idx: &ExprIdx| {
		instance_exprs.remove(idx).unwrap_or_else(|| {
			panic!(
				"
The instance is missing an ExprIdx found in the static tree.
Expected idx: {idx}, instance idxs: {all_keys:?}
"
			);
		})
	};

	for child in children.iter_descendants_inclusive(entity) {
		if let Ok(expr_idx) = expr_idxs.get(child) {
			commands.entity(child).insert(get_on_spawn(expr_idx));
		}
		for attr in attributes.iter_direct_descendants(child) {
			if let Ok(expr_idx) = expr_idxs.get(attr) {
				commands.entity(attr).insert(get_on_spawn(expr_idx));
			}
		}
	}
	if !instance_exprs.is_empty() {
		panic!(
			"
Not all ExprIdx were applied.
The static tree is missing the following idxs found in the instance: {:?}
",
			instance_exprs.keys()
		);
	}
	commands
		.run_system_cached_with(on_spawn_template.pipe(panic_on_err), entity);
}

fn panic_on_err(In(err): In<Result>) {
	match err {
		Ok(_) => {}
		Err(e) => panic!("{e}"),
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use bevy::ecs::system::RunSystemOnce;
	use sweet::prelude::*;

	fn parse(instance: impl Bundle, tree: impl Bundle) -> String {
		let mut world = World::new();
		let instance = world.spawn(instance).insert(MacroIdx::default()).id();
		let _tree = world
			.spawn(tree)
			.insert((MacroIdx::default(), StaticTree))
			.id();
		world.run_system_once(apply_static_trees).unwrap().unwrap();
		world.run_system_once(apply_slots).ok(); // no matching entities ok
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
	}


	#[test]
	#[ignore = "temp disabled wrapping exprs in OnSpawnTemplate, needs design"]
	fn works() {
		parse(
			rsx! {<div>{7}</div>},
			// because ExprIdx matches, this should be replace with 7
			rsx! {<div><span>{()}</span><br/></div>},
		)
		.xpect()
		.to_be("<div><span>7</span><br/></div>");
	}
	#[test]
	#[ignore = "temp disabled wrapping exprs in OnSpawnTemplate, needs design"]
	fn root() { parse(rsx! {{7}}, rsx! {hello{()}}).xpect().to_be("hello7"); }

	#[test]
	#[ignore = "temp disabled wrapping exprs in OnSpawnTemplate, needs design"]
	#[should_panic = "Not all ExprIdx were applied.."]
	fn tree_missing_idx() {
		parse(rsx! {<div>{7}</div>}, rsx! {<div><br/></div>});
	}
	#[test]
	#[ignore = "temp disabled wrapping exprs in OnSpawnTemplate, needs design"]
	#[should_panic = "The instance is missing an ExprIdx.."]
	fn instance_missing_idx() {
		parse(rsx! {<div><br/></div>}, rsx! {<div>{7}</div>});
	}


	#[template]
	fn Counter(initial: u32) -> impl Bundle {
		let (count, set_count) = signal(initial);
		rsx! {
			<div>
				<span>{count}</span>
				<button onclick={move |_| set_count.update(|v| *v -= 1)}>-</button>
			</div>
		}
	}


	#[test]
	#[ignore = "temp disabled wrapping exprs in OnSpawnTemplate, needs design"]
	fn template() {
		parse(
			rsx! {<Counter initial=3/>},
			rsx! {<div><span>{()}</span><br/></div>},
		)
		.xpect()
		.to_be("<div><span>7</span><br/></div>");
	}
}
