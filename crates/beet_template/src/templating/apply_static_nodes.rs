use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// When a [`MacroIdx`] is added to an entity,
/// recusively apply each [`StaticNodeRoot`] and run [`OnSpawnTemplate`] methods
pub fn spawn_templates(world: &mut World) -> Result {
	let mut query = world.query_filtered::<(), (
		Added<MacroIdx>,
		Without<StaticNodeRoot>,
		Without<ResolvedRoot>,
	)>();

	while query.iter(world).next().is_some() {
		world.run_system_cached(apply_lit_nodes)??;
		world.run_system_cached(run_on_spawn_template)??;
	}
	Ok(())
}

/// Static nodes are created by statically analyzing a file,
/// so they should not be rendered directly, and only used for template reloading.
#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct StaticNodeRoot;

#[derive(Default, Component, Reflect)]
#[reflect(Default, Component)]
pub struct ResolvedRoot;

pub(super) fn apply_lit_nodes(
	mut commands: Commands,
	instances: Populated<
		(Entity, &MacroIdx),
		(
			Added<MacroIdx>,
			Without<StaticNodeRoot>,
			Without<ResolvedRoot>,
		),
	>,
	static_trees: Query<(Entity, &MacroIdx), With<StaticNodeRoot>>,
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
				builder
					.deny::<StaticNodeRoot>()
					.linked_cloning(true)
					.add_observers(true);
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
	for (entity, _) in instances.iter() {
		commands.entity(entity).insert(ResolvedRoot);
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
			.spawn((tree, StaticNodeRoot))
			.insert(MacroIdx::default())
			.id();

		world.run_system_once(spawn_templates).unwrap().unwrap();
		world.run_system_once(apply_slots).ok(); // no matching entities ok
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
	}


	#[test]
	fn works() {
		parse(
			rsx! {<main>{7}</main>},
			// because ExprIdx matches, this should be replace with 7
			rsx! {<div><span>{()}</span><br/></div>},
		)
		.xpect()
		.to_be("<div><span>7</span><br/></div>");
	}
	#[test]
	fn root() { parse(rsx! {{7}}, rsx! {hello{()}}).xpect().to_be("hello7"); }

	#[test]
	#[should_panic = "Not all ExprIdx were applied.."]
	fn tree_missing_idx() {
		parse(rsx! {<div>{7}</div>}, rsx! {<div><br/></div>});
	}
	#[test]
	#[should_panic = "The instance is missing an ExprIdx.."]
	fn instance_missing_idx() {
		parse(rsx! {<div><br/></div>}, rsx! {<div>{7}</div>});
	}


	#[test]
	fn template() {
		#[template]
		fn MyTemplate(initial: u32) -> impl Bundle {
			rsx! {{initial+2}}
		}
		parse(
			rsx! {<MyTemplate initial=3/>},
			(NodeTag::new("div"), ElementNode::open(), children![(
				ExprIdx(0u32),
				NodeTag(String::from("Counter")),
				FragmentNode,
				TemplateNode,
			)]),
		)
		.xpect()
		.to_be("<div>5</div>");
	}
}
