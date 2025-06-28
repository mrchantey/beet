use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Load static scene if it exists.
#[cfg(feature = "serde")]
pub fn load_static_scene(world: &mut World) -> Result {
	use beet_bevy::prelude::WorldMutExt;
	use beet_utils::prelude::ReadFile;
	if let Some(static_scene_config) = world.get_resource::<StaticSceneConfig>()
	{
		if let Ok(file) = ReadFile::to_string(static_scene_config.scene_file())
		{
			world.load_scene(file)?;
		}
	}
	Ok(())
}



/// When a [`MacroIdx`] is added to an entity,
/// recusively apply each [`StaticNodeRoot`] and run [`OnSpawnTemplate`] methods
pub fn spawn_templates(world: &mut World) -> Result {
	let mut query = world.query_filtered::<(), (
		Added<MacroIdx>,
		Without<StaticNodeRoot>,
		Without<ResolvedRoot>,
	)>();
	while query.iter(world).next().is_some() {
		// println!("Running spawn_templates system");
		world.run_system_cached(apply_static_nodes)??;
		world.run_system_cached(apply_on_spawn_template)??;
	}
	Ok(())
}

pub(super) fn apply_static_nodes(
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
	attributes: Query<&Attributes>,
	mut on_spawn_templates: Query<(&ExprIdx, &mut OnSpawnTemplate)>,
) -> Result {
	for (instance, static_tree, idx) in
		instances.iter().filter_map(|(instance, idx)| {
			static_trees
				.iter()
				.find(|(_, static_idx)| *static_idx == idx)
				.map(|(static_tree, idx)| (instance, static_tree, idx))
		}) {
		trace!("Applying static nodes for{}", idx);

		// take all [`OnSpawnTemplate`] methods from the instance,
		// then entirely clear it.
		// this must be done before clearing and cloning are executed.
		// TODO attributes too?
		let mut instance_expr_map = HashMap::new();

		for child in children.iter_descendants_inclusive(instance) {
			if let Ok((idx, mut template)) = on_spawn_templates.get_mut(child) {
				instance_expr_map.insert(*idx, template.take());
			}
			for attr in attributes.iter_direct_descendants(child) {
				if let Ok((idx, mut template)) =
					on_spawn_templates.get_mut(attr)
				{
					instance_expr_map.insert(*idx, template.take());
				}
			}
		}

		// commands.run_system_cached_with(
		// 	|entity: In<Entity>, world: &mut World| {
		// 		let str = world.component_names_related::<Children>(*entity);
		// 		let str = str.iter_to_string_indented();
		// 		println!("tree: {}", str);
		// 	},
		// 	instance,
		// );

		commands
			.entity(instance)
			.despawn_related::<Children>()
			.despawn_related::<TemplateRoot>()
			.despawn_related::<Attributes>()
			// remove all components that the static tree may
			// replace
			// currently just TemplateOf but we may need to add more later
			.retain::<TemplateOf>();

		// apply the static tree
		commands
			.entity(static_tree)
			.clone_with(instance, |builder| {
				builder
					.deny::<StaticNodeRoot>()
					// a static node should not have a TemplateOf 
					// but specify for completeness
					.deny::<TemplateOf>()
					.linked_cloning(true)
					.add_observers(true);
			});

		// queue system to resolve template locations after clone
		commands.run_system_cached_with(
			apply_template_locations,
			(idx.clone(), instance, instance_expr_map),
		);
	}
	for (entity, _) in instances.iter() {
		commands.entity(entity).insert(ResolvedRoot);
	}
	Ok(())
}

/// A system queued after [`apply_static_nodes`],
fn apply_template_locations(
	In((macro_idx, entity, mut instance_exprs)): In<(
		MacroIdx,
		Entity,
		HashMap<ExprIdx, OnSpawnTemplate>,
	)>,
	mut commands: Commands,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	expr_idxs: Query<&ExprIdx>,
	templates: Query<&TemplateNode>,
) {
	let all_keys = instance_exprs.keys().cloned().collect::<Vec<_>>();
	let mut get_on_spawn = |idx: &ExprIdx| {
		instance_exprs.remove(idx).unwrap_or_else(|| {
			panic!(
				"
Error resolving static node for macro at {macro_idx}
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
		let is_template = templates.get(child).is_ok();
		if !is_template {
			for attr in attributes.iter_direct_descendants(child) {
				if let Ok(expr_idx) = expr_idxs.get(attr) {
					commands.entity(attr).insert(get_on_spawn(expr_idx));
				}
			}
		}
	}
	if !instance_exprs.is_empty() {
		panic!(
			"
Error resolving static node for macro at {macro_idx}
Not all ExprIdx were applied.
The static tree is missing the following idxs found in the instance: {:?}
",
			instance_exprs.keys()
		);
	}
}


/// more tests in static_scene_roundtrip.rs
#[cfg(test)]
mod test {
	use super::*;
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use bevy::ecs::system::RunSystemOnce;
	use sweet::prelude::*;

	fn parse(instance: impl Bundle, static_node: impl Bundle) -> String {
		let mut world = World::new();
		let instance = world.spawn(instance).insert(MacroIdx::default()).id();
		let _tree = world
			.spawn((static_node, StaticNodeRoot))
			.insert(MacroIdx::default())
			.id();

		world.run_system_once(spawn_templates).unwrap().unwrap();
		world.run_system_once(apply_slots).ok(); // no matching entities ok
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
	}

	#[test]
	#[should_panic = "Not all ExprIdx were applied.."]
	fn static_node_missing_idx() {
		parse(rsx! {<div>{7}</div>}, rsx! {<div><br/></div>});
	}
	#[test]
	#[should_panic = "The instance is missing an ExprIdx.."]
	fn instance_missing_idx() {
		parse(rsx! {<div><br/></div>}, rsx! {<div>{7}</div>});
	}

	#[test]
	fn block_nodes() {
		parse(
			rsx! {<main>{7}</main>},
			// because ExprIdx matches, this should be replace with 7
			rsx! {<div><span>{()}</span><br/></div>},
		)
		.xpect()
		.to_be("<div><span>7</span><br/></div>");
	}
	#[test]
	fn attributes() {
		parse(
			rsx! {<main key={7}/>},
			rsx! {<div><span key={()}></span><br/></div>},
		)
		.xpect()
		.to_be("<div><span key=\"7\"></span><br/></div>");
	}
	#[test]
	fn root() { parse(rsx! {{7}}, rsx! {hello{()}}).xpect().to_be("hello7"); }

	#[template]
	fn MyTemplate(initial: u32) -> impl Bundle {
		rsx! {{initial}}
	}
	#[template]
	fn SomeOtherName() -> impl Bundle { () }

	#[test]
	fn template_simple() {
		parse(
			rsx! {<MyTemplate initial=3/>},
			// the name doesnt matter, a <SomeTitleCase/> is treated the same as
			// any other block {}
			rsx! {<span><SomeOtherName/></span>},
		)
		.xpect()
		.to_be("<span>3</span>");
	}
	#[test]
	fn template_expr_attr() {
		let val = 5;
		parse(
			// attributes are resolved here, there is only one ExprIdx
			// in this tree
			rsx! {<MyTemplate initial=val/>},
			// this is something like the static/tokens representation
			// of a template, ie attributes have not been resolved yet,
			// this test ensures we dont try to resolve them
			(
				NodeTag(String::from("MyTemplate")),
				ExprIdx(0u32),
				FragmentNode,
				TemplateNode,
				related! {Attributes[
						(
							AttributeKey::new("initial"),
							ExprIdx(1u32)
						)
				]},
			),
		)
		.xpect()
		.to_be("5");
	}
}
