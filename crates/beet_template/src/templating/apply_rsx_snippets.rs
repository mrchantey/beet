use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Load static scene if it exists.
#[cfg(feature = "serde")]
pub fn load_file_snippets(world: &mut World) -> Result {
	use beet_bevy::prelude::WorldMutExt;
	use beet_utils::prelude::ReadFile;
	if let Some(config) = world.get_resource::<WorkspaceConfig>() {
		if let Ok(file) = ReadFile::to_string(config.scene_file().into_abs()) {
			world.load_scene(file)?;
		}
	}
	Ok(())
}



/// When a [`MacroIdx`] is added to an entity,
/// recusively apply each [`StaticNodeRoot`] and run [`OnSpawnTemplate`] methods
pub fn spawn_templates(world: &mut World) -> Result {
	let mut query = world
		.query_filtered::<(), (Added<InstanceRoot>, Without<ResolvedRoot>)>();
	while query.iter(world).next().is_some() {
		// println!("Running spawn_templates system");
		if let Ok(result) = world.run_system_cached(apply_rsx_snippets) {
			result?;
		};
		world.run_system_cached(apply_on_spawn_template)??;
	}
	Ok(())
}

pub(super) fn apply_rsx_snippets(
	mut commands: Commands,
	instances: Populated<
		(Entity, &MacroIdx),
		(Added<InstanceRoot>, Without<ResolvedRoot>),
	>,
	rsx_snippets: Query<(Entity, &MacroIdx), With<RsxSnippetRoot>>,
	children: Query<&Children>,
	// types that we want to deny clone for the root only, not children
	deny_root: Query<Option<&ChildOf>>,
	attributes: Query<&Attributes>,
	mut on_spawn_templates: Query<(&ExprIdx, &mut OnSpawnTemplate)>,
) -> Result {
	for (instance_root, static_root, idx) in
		instances.iter().filter_map(|(instance, idx)| {
			rsx_snippets
				.iter()
				.find(|(_, static_idx)| *static_idx == idx)
				.map(|(rsx_snippets, idx)| (instance, rsx_snippets, idx))
		}) {
		trace!("Applying static nodes for{}", idx);

		// take all [`OnSpawnTemplate`] methods from the instance,
		// then entirely clear it.
		// this must be done before clearing and cloning are executed.
		// TODO attributes too?
		let mut instance_expr_map = HashMap::new();

		for child in children.iter_descendants_inclusive(instance_root) {
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
		// 		println!("instance:\n{}", str);
		// 	},
		// 	instance_root,
		// );
		// commands.run_system_cached_with(
		// 	|entity: In<Entity>, world: &mut World| {
		// 		let str = world.component_names_related::<Children>(*entity);
		// 		let str = str.iter_to_string_indented();
		// 		println!("static:\n{}", str);
		// 	},
		// 	static_root,
		// );

		commands
			.entity(instance_root)
			.despawn_related::<Children>()
			.despawn_related::<TemplateRoot>()
			.despawn_related::<Attributes>()
			.retain::<(
				BeetRoot,
				InstanceRoot,
				HtmlDocument,
				HtmlFragment,
				ChildOf,
				TemplateOf,
			)>();

		// apply the static tree
		commands
			.entity(static_root)
			.clone_with(instance_root, |builder| {
				builder
					.deny::<(BeetRoot, RsxSnippetRoot)>()
					.linked_cloning(true)
					.add_observers(true);
			});

		// if the static root is a ChildOf that would override the instance,
		// we need to reapply it.
		if let Ok(child_of) = deny_root.get(instance_root) {
			if let Some(child_of) = child_of {
				commands.entity(instance_root).insert(child_of.clone());
			}
		}

		// queue system to resolve template locations after clone
		commands.run_system_cached_with(
			apply_template_locations,
			(idx.clone(), instance_root, instance_expr_map),
		);
	}
	for (entity, _) in instances.iter() {
		commands.entity(entity).insert(ResolvedRoot);
	}
	Ok(())
}

/// A system queued after [`apply_rsx_snippets`],
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
	let mut taken_keys = Vec::new();
	let mut get_on_spawn = |idx: &ExprIdx| {
		let out = instance_exprs.remove(idx).unwrap_or_else(|| {
			panic!(
				"
				Error resolving static node for macro at {macro_idx}
				The instance is missing an ExprIdx found in the static tree.
				Expected idx: 	{idx}
				Instance idxs: 	{all_keys:?}
				Taken idxs: 		{taken_keys:?}
				"
			);
		});
		taken_keys.push(idx.clone());
		out
	};

	for child in children.iter_descendants_inclusive(entity) {
		if let Ok(expr_idx) = expr_idxs.get(child) {
			commands.entity(child).insert(get_on_spawn(expr_idx));
		}
		// an instance template node does not have attributes,
		// they are instead applied as props.
		// but a static template node does, we can ignore them
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
All idxs: 		{all_keys:?}
Taken idxs: 	{taken_keys:?}
",
			instance_exprs.keys(),
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

	#[test]
	fn retains_parent() {
		let mut world = World::new();

		let child = world.spawn(rsx! {<div/>}).insert(MacroIdx::default()).id();
		let parent = world.spawn(rsx! {<main></main>}).id();
		let main = world.entity(parent).get::<Children>().unwrap()[0];
		world.entity_mut(main).add_child(child);

		let _tree = world
			.spawn((rsx! {<span/>}, RsxSnippetRoot))
			.remove::<InstanceRoot>()
			.insert(MacroIdx::default())
			.id();

		world.run_system_once(spawn_templates).unwrap().unwrap();

		// let frag = world
		// 	.component_names_related::<Children>(parent)
		// 	.iter_to_string_indented();
		// println!("frag: {}", frag);

		world
			.run_system_once_with(render_fragment, parent)
			.unwrap()
			.xpect()
			.to_be("<main><span/></main>");
	}

	fn parse(instance: impl Bundle, rsx_snippet: impl Bundle) -> String {
		let mut world = World::new();
		let instance = world.spawn(instance).insert(MacroIdx::default()).id();
		let _tree = world
			.spawn((RsxSnippetRoot, rsx_snippet))
			.remove::<InstanceRoot>()
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
	fn rsx_snippet_missing_idx() {
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
	fn iterators() {
		parse(
			rsx! {<main>{["a","b","c"]}</main>},
			// because ExprIdx matches, this should be replace with 7
			rsx! {<div><span>{()}</span><br/></div>},
		)
		.xpect()
		.to_be("<div><span>abc</span><br/></div>");
	}
	#[test]
	fn attribute_values() {
		parse(
			rsx! {<main key={7}/>},
			rsx! {<div><span key={()}></span><br/></div>},
		)
		.xpect()
		.to_be("<div><span key=\"7\"></span><br/></div>");
	}
	#[test]
	fn attribute_blocks() {
		#[derive(Buildable, AttributeBlock)]
		struct Foo {
			key: u32,
		}
		parse(
			rsx! {<main {Foo{key:9}}/>},
			rsx! {<div><span {()}></span><br/></div>},
		)
		.xpect()
		.to_be("<div><span key=\"9\"></span><br/></div>");
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
	#[test]
	#[ignore = "not sure how to test this this"]
	fn bundle_templates() {
		let bundle = MyTemplate { initial: 3 };

		let idx1 = MacroIdx::new_file_line_col(file!(), line!(), column!());
		let idx2 = MacroIdx::new_file_line_col(file!(), line!(), column!());

		let mut world = World::new();
		let child = world
			.spawn(bundle.into_node_bundle())
			.insert(idx2.clone())
			.id();
		let instance = world
			.spawn((
				InstanceRoot,
				idx1.clone(),
				NodeTag(String::from("main")),
				ElementNode::open(),
			))
			.add_child(child)
			.insert(idx1.clone())
			.id();

		let _tree1 = world
			.spawn((rsx! {<span>{}</span>}, RsxSnippetRoot))
			.insert(idx1)
			.id();
		let _tree2 = world
			.spawn((rsx! {<span>{}</span>}, RsxSnippetRoot))
			.insert(idx2)
			.id();

		world.run_system_once(spawn_templates).unwrap().unwrap();
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
			.xpect()
			.to_be("<main><span/></main>");
	}
}
