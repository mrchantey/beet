use crate::prelude::*;
use beet_core::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// Load snippet scene if it exists.
// temp whole file until fine-grained loading is implemented
#[cfg(feature = "serde")]
pub fn load_all_file_snippets(world: &mut World) -> Result {
	use beet_core::prelude::WorldMutExt;
	use beet_utils::prelude::ReadFile;
	let config = world.resource::<WorkspaceConfig>();

	let file = config.snippets_dir().into_abs().join("snippets.ron");
	let file = ReadFile::to_string(file)?;
	world.load_scene(file)?;

	Ok(())
}
#[cfg(feature = "serde")]
pub fn load_all_file_snippets_fine_grained(world: &mut World) -> Result {
	use beet_core::prelude::WorldMutExt;
	use beet_utils::prelude::ReadFile;
	let config = world.resource::<WorkspaceConfig>();
	use beet_utils::prelude::ReadDir;

	let files = ReadDir::files_recursive(config.snippets_dir().into_abs())?;
	let num_files = files.len();
	let start = std::time::Instant::now();
	// TODO fine-grained loading with watcher

	// TODO store this in a resource for hooking up with fine-grained loading
	let mut snippet_entity_map = Default::default();
	for file in files {
		let file = ReadFile::to_string(file)?;
		{
			world.load_scene_with(file, &mut snippet_entity_map)?;
		}
	}
	debug!(
		"Loaded {} file snippets in {}ms",
		num_files,
		start.elapsed().as_millis()
	);
	Ok(())
}



/// When a [`MacroIdx`] is added to an entity,
/// recusively apply each [`StaticNodeRoot`] and run [`OnSpawnTemplate`] methods
pub fn apply_snippets_to_instances(world: &mut World) -> Result {
	let mut query = world
		.query_filtered::<(), (Added<InstanceRoot>, Without<ResolvedRoot>)>();
	while query.iter(world).next().is_some() {
		// println!("Running spawn_templates system");
		if let Ok(result) = world.run_system_cached(apply_rsx_snippets) {
			result?;
		};
		if let Ok(result) = world.run_system_cached(apply_on_spawn_template) {
			result?;
		}
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
	for (instance_root, snippet_root, macro_idx) in
		instances
			.iter()
			.filter_map(|(instance, instance_macro_idx)| {
				rsx_snippets
					.iter()
					.find(|(_, snippet_macro_idx)| {
						*snippet_macro_idx == instance_macro_idx
					})
					.map(|(rsx_snippets, macro_idx)| {
						(instance, rsx_snippets, macro_idx)
					})
			}) {
		trace!("Applying snippets for {} at {}", instance_root, macro_idx);

		// take all [`OnSpawnTemplate`] methods from the instance,
		// then entirely clear it.
		// this must be done before clearing and cloning are executed.
		// TODO attributes too?
		let mut instance_expr_map = HashMap::new();

		for child in children.iter_descendants_inclusive(instance_root) {
			// onspawntemplate in block node position
			if let Ok((idx, mut on_spawn)) = on_spawn_templates.get_mut(child) {
				instance_expr_map.insert(*idx, on_spawn.take());
			}
			for attr in attributes.iter_direct_descendants(child) {
				// onspawntemplate in attribute position
				if let Ok((idx, mut on_spawn)) =
					on_spawn_templates.get_mut(attr)
				{
					instance_expr_map.insert(*idx, on_spawn.take());
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

		// apply the snippet tree
		commands
			.entity(snippet_root)
			.clone_with(instance_root, |builder| {
				builder
					.deny::<(BeetRoot, RsxSnippetRoot)>()
					.linked_cloning(true)
					.add_observers(true);
			});

		// if the snippet root is a ChildOf that would override the instance,
		// we need to reapply it.
		if let Ok(child_of) = deny_root.get(instance_root) {
			if let Some(child_of) = child_of {
				commands.entity(instance_root).insert(child_of.clone());
			}
		}

		// queue system to resolve template locations after clone
		commands.run_system_cached_with(
			apply_template_locations,
			(macro_idx.clone(), instance_root, instance_expr_map),
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
	let instance_keys = instance_exprs.keys().cloned().collect::<Vec<_>>();
	let mut consumed_keys = Vec::new();
	let mut get_on_spawn = |idx: &ExprIdx| {
		let out = instance_exprs.remove(idx).unwrap_or_else(|| {
			panic!(
				"
				Error resolving rsx snippet for macro at {macro_idx}
				The instance is missing an ExprIdx found in the snippet.
				Instance idxs: 	{instance_keys:?}
				Consumed idxs: 	{consumed_keys:?}
				Expected idx: 	{idx}
				"
			);
		});
		consumed_keys.push(idx.clone());
		out
	};

	for child in children.iter_descendants_inclusive(entity) {
		if let Ok(expr_idx) = expr_idxs.get(child) {
			commands.entity(child).insert(get_on_spawn(expr_idx));
		}
		// an instance template node does not have attributes,
		// they are instead applied as props.
		// but a snippet template node does, we can ignore them
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
Error resolving rsx snippet for macro at {macro_idx}
Not all ExprIdx were applied.
The rsx snippet is missing idxs found in the instance:
Instance idxs: 	{instance_keys:?}
Consumed idxs: 	{consumed_keys:?}
Remaining idxs: {:?}
",
			instance_exprs.keys().map(|idx| idx.0).collect::<Vec<_>>()
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

		let _snippet = world
			.spawn((rsx! {<span/>}, RsxSnippetRoot))
			.remove::<InstanceRoot>()
			.insert(MacroIdx::default())
			.id();

		world
			.run_system_once(apply_snippets_to_instances)
			.unwrap()
			.unwrap();

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

		// convert an instance to a snippet with the same MacroIdx
		let _snippet = world
			.spawn((RsxSnippetRoot, rsx_snippet))
			.remove::<InstanceRoot>()
			.insert(MacroIdx::default())
			.id();

		world
			.run_system_once(apply_snippets_to_instances)
			.unwrap()
			.unwrap();
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
	fn combinator_block_nodes() {
		parse(
			rsx_combinator! {"<main>{7}</main>"},
			// because ExprIdx matches, this should be replace with 7
			rsx_combinator! {"<div><span>{()}</span><br/></div>"},
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
	fn events() {
		// didnt panic
		parse(rsx! {<main onclick={||{}}/>}, rsx! {<main oninput={||{}}/>})
			.xpect()
			.to_be("<main oninput/>");
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

		world
			.run_system_once(apply_snippets_to_instances)
			.unwrap()
			.unwrap();
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
			.xpect()
			.to_be("<main><span/></main>");
	}
}
