use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::WorldMutExt;
use beet_core::prelude::*;
use beet_utils::prelude::ReadDir;
use beet_utils::prelude::ReadFile;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;



/// Load snippet scene if it exists.
// temp whole file until fine-grained loading is implemented
pub fn load_all_file_snippets(world: &mut World) -> Result {
	let config = world.resource::<WorkspaceConfig>();
	let file = config.snippets_dir().into_abs().join("snippets.ron");
	let file = ReadFile::to_string(file)?;
	world.load_scene(file)?;
	Ok(())
}
pub fn load_all_file_snippets_fine_grained(world: &mut World) -> Result {
	let config = world.resource::<WorkspaceConfig>();

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



/// When a [`SnippetRoot`] is added to an entity,
/// recusively apply each [`StaticRoot`]
pub fn apply_rsx_snippets(world: &mut World) -> Result {
	let mut query = world.query_filtered::<Entity, With<InstanceRoot>>();
	let mut visited = Vec::new();

	loop {
		let mut any_visited = false;
		for entity in query.iter(world).collect::<Vec<_>>() {
			if visited.contains(&entity) {
				continue;
			}
			visited.push(entity);
			any_visited = true;
			world.run_system_cached_with(apply_static_rsx, entity)??;
			world.run_system_cached_with(
				flush_on_spawn_deferred_recursive,
				entity,
			)??;
		}
		if !any_visited {
			break;
		}
	}

	Ok(())
}

fn maybe_panic(result: In<Result<()>>) {
	if let Err(err) = result.0 {
		panic!("apply_rsx_snippets: {err}");
	}
}


fn apply_static_rsx(
	In(instance_root): In<Entity>,
	mut commands: Commands,
	instances: Query<&SnippetRoot>,
	rsx_snippets: Query<(Entity, &SnippetRoot), With<StaticRoot>>,
	children: Query<&Children>,
	// types that we want to deny clone for the root only, not children
	deny_root: Query<Option<&ChildOf>>,
	attributes: Query<&Attributes>,
	mut expressions: Query<(&ExprIdx, &mut OnSpawnDeferred)>,
) -> Result {
	let Ok(instance_loc) = instances.get(instance_root) else {
		return Ok(());
	};
	let Some((static_root, snippet_root)) =
		rsx_snippets.iter().find(|(_, static_loc)| {
			// println!("compare:\n{}\n{}", static_loc, instance_loc);
			*static_loc == instance_loc
		})
	else {
		return Ok(());
	};

	trace!(
		"Applying snippets for {} at {}",
		instance_root, snippet_root
	);

	// take all [`OnSpawnDeferred`] methods from the instance,
	// then entirely clear it.
	// this must be done before clearing and cloning are executed.
	// TODO attributes too?
	let mut instance_expr_map = HashMap::new();

	// commands.run_system_cached_with(log_component_names, instance_root);
	// commands.run_system_cached_with(log_component_names, static_root);


	// take all [`OnSpawnDeferred`] methods from the instance,
	// then entirely clear it.
	// this must be done before clearing and cloning are executed.
	for child in children.iter_descendants_inclusive(instance_root) {
		// node exprs
		if let Ok((idx, mut on_spawn)) = expressions.get_mut(child) {
			instance_expr_map.insert(*idx, on_spawn.take());
		}
		for attr in attributes.iter_direct_descendants(child) {
			// attribute exprs
			if let Ok((idx, mut on_spawn)) = expressions.get_mut(attr) {
				instance_expr_map.insert(*idx, on_spawn.take());
			}
		}
	}
	// println!("Instance entity: {:?}", instance_root);
	// println!("Static entity: {:?}", static_root);
	// println!("Here, loc:{:?}", expressions.iter().collect::<Vec<_>>());
	// println!("Here, loc:{:?}", instance_expr_map);


	commands
		.entity(instance_root)
		.despawn_related::<Children>()
		.despawn_related::<TemplateRoot>()
		.despawn_related::<Attributes>()
		.retain::<(
			SnippetRoot,
			InstanceRoot,
			HtmlDocument,
			HtmlFragment,
			ChildOf,
			TemplateOf,
		)>();

	// apply the snippet tree
	commands
		.entity(static_root)
		.clone_with(instance_root, |builder| {
			builder
				.deny::<(SnippetRoot, StaticRoot)>()
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
		apply_template_locations.pipe(maybe_panic),
		(snippet_root.clone(), instance_root, instance_expr_map),
	);
	Ok(())
}

/// A system queued after [`apply_rsx_snippets`],
fn apply_template_locations(
	In((snippet_root, entity, mut instance_exprs)): In<(
		SnippetRoot,
		Entity,
		HashMap<ExprIdx, OnSpawnDeferred>,
	)>,
	mut commands: Commands,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	exprs: Query<&ExprIdx>,
	templates: Query<&TemplateNode>,
) -> Result {
	let instance_keys = instance_exprs.keys().cloned().collect::<Vec<_>>();
	let mut consumed_keys = Vec::new();
	let mut get_on_spawn = |idx: &ExprIdx| -> Result<OnSpawnDeferred> {
		let out = instance_exprs.remove(idx).ok_or_else(|| {
			bevyhow!(
				"
Error resolving static root for snippet at {snippet_root}
The instance root is missing an ExprIdx found in the static root.
Instance idxs: 	{instance_keys:?}
Consumed idxs: 	{consumed_keys:?}
Expected idx: 	{idx}
				"
			)
		})?;
		consumed_keys.push(idx.clone());
		Ok(out)
	};

	for child in children.iter_descendants_inclusive(entity) {
		if let Ok(expr) = exprs.get(child) {
			commands.entity(child).insert(get_on_spawn(expr)?);
		}
		// an instance template node does not have attributes,
		// they are instead applied as props.
		// but a snippet template node might, we can ignore them
		let is_template = templates.get(child).is_ok();
		if !is_template {
			for attr in attributes.iter_direct_descendants(child) {
				if let Ok(expr_idx) = exprs.get(attr) {
					commands.entity(attr).insert(get_on_spawn(expr_idx)?);
				}
			}
		}
	}
	if !instance_exprs.is_empty() {
		bevybail!(
			"
Error resolving static root for snippet at {snippet_root}
Not all ExprIdx were applied.
The static root is missing idxs found in the instance root:
Instance idxs: 	{instance_keys:?}
Consumed idxs: 	{consumed_keys:?}
Remaining idxs: {:?}
",
			instance_exprs.keys().map(|idx| idx.0).collect::<Vec<_>>()
		);
	}
	Ok(())
}


/// Add this system for [`OnSpawnDeferred`] behavior.
/// It must be called after *apply_slots* as it doesnt recurse into [`TemplateOf`]
fn flush_on_spawn_deferred_recursive(
	In(root): In<Entity>,
	mut commands: Commands,
	children: Query<&Children>,
	attributes: Query<&Attributes>,
	mut query: Query<(Entity, &mut OnSpawnDeferred)>,
) -> Result {
	for node_entity in children.iter_descendants_inclusive(root) {
		if let Ok((entity, mut on_spawn)) = query.get_mut(node_entity) {
			commands.entity(entity).remove::<OnSpawnDeferred>();
			commands.queue(on_spawn.take().into_command(entity));
		}
		// only elements, not templates, will have attributes
		for attr in attributes.iter_direct_descendants(node_entity) {
			if let Ok((attr_entity, mut on_spawn)) = query.get_mut(attr) {
				commands.entity(attr_entity).remove::<OnSpawnDeferred>();
				commands.queue(on_spawn.take().into_command(attr_entity));
			}
		}
	}
	Ok(())
}


/// more tests in static_scene_roundtrip.rs
#[cfg(test)]
mod test {
	use super::*;
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use bevy::ecs::system::RunSystemOnce;
	use sweet::prelude::*;

	fn world() -> World {
		let world = World::new();

		// world
		// 	.register_component_hooks::<InstanceRoot>()
		// 	.on_add(apply_rsx_snippets_hook);
		world
	}

	#[test]
	fn retains_parent() {
		let mut world = world();

		let child = world
			.spawn(rsx! {<div/>})
			.insert(SnippetRoot::default())
			.id();
		let parent = world.spawn(rsx! {<main></main>}).id();
		let main = world.entity(parent).get::<Children>().unwrap()[0];
		world.entity_mut(main).add_child(child);

		let _snippet = world
			.spawn((rsx! {<span/>}, StaticRoot))
			.remove::<InstanceRoot>()
			.insert(SnippetRoot::default())
			.id();
		world
			.run_system_cached(apply_rsx_snippets)
			.unwrap()
			.unwrap();

		world
			.run_system_cached_with(render_fragment, parent)
			.unwrap()
			.xpect()
			.to_be("<main><span/></main>");
	}

	fn parse(instance: impl Bundle, rsx_snippet: impl Bundle) -> String {
		let mut world = world();
		// convert an instance to a snippet with the same SnippetRoot
		let _snippet = world
			.spawn((StaticRoot, rsx_snippet))
			.remove::<InstanceRoot>()
			.insert(SnippetRoot::default())
			.id();
		let instance =
			world.spawn(instance).insert(SnippetRoot::default()).id();

		world.run_system_once(apply_rsx_snippets).unwrap().unwrap();
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

		let idx1 = SnippetRoot::new_file_line_col(file!(), line!(), column!());
		let idx2 = SnippetRoot::new_file_line_col(file!(), line!(), column!());

		let mut world = world();
		let child = world.spawn(bundle.into_bundle()).insert(idx2.clone()).id();
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
			.spawn((rsx! {<span>{}</span>}, StaticRoot))
			.insert(idx1)
			.id();
		let _tree2 = world
			.spawn((rsx! {<span>{}</span>}, StaticRoot))
			.insert(idx2)
			.id();

		world.run_system_once(apply_rsx_snippets).unwrap().unwrap();
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
			.xpect()
			.to_be("<main><span/></main>");
	}
}


#[cfg(test)]
mod test_flush_recursive {
	use super::*;
	use crate::as_beet::*;
	use crate::templating::apply_slots;
	use bevy::ecs::system::RunSystemOnce;
	use sweet::prelude::*;

	#[test]
	fn bfs_order() {
		let (val, set_val) = signal::<Vec<u32>>(Vec::new());

		let mut world = World::new();
		let entity = world
			.spawn((
				InstanceRoot,
				OnSpawnDeferred::new(move |_| {
					set_val.update(|v| v.push(0));
					Ok(())
				}),
				children![
					(
						OnSpawnDeferred::new(move |_| {
							set_val.update(|v| v.push(1));
							Ok(())
						}),
						children![
							OnSpawnDeferred::new(move |_| {
								set_val.update(|v| v.push(3));
								Ok(())
							}),
							related! {Attributes[
								OnSpawnDeferred::new(move |_| {
									set_val.update(|v| v.push(4));
									Ok(())
								})
							]}
						],
					),
					// sibling, bfs!
					OnSpawnDeferred::new(move |_| {
						set_val.update(|v| v.push(2));
						Ok(())
					}),
				],
			))
			.id();
		world
			.run_system_cached_with(flush_on_spawn_deferred_recursive, entity)
			.unwrap()
			.unwrap();
		expect(val()).to_be(vec![0, 1, 2, 3, 4]);
	}


	fn parse(instance: impl Bundle) -> String {
		let mut world = World::new();
		let instance = world.spawn(instance).id();

		world.run_system_once(apply_rsx_snippets).unwrap().unwrap();
		world.run_system_once(apply_slots).ok(); // no matching entities ok
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
	}

	#[test]
	fn templates() {
		#[template]
		fn MyTemplate() -> impl Bundle {
			rsx! {<div/>}
		}

		parse(rsx! {
			<MyTemplate/>
		})
		.xpect()
		.to_be_str("<div/>");
	}
	#[test]
	fn attribute_blocks() {
		#[derive(Default, Buildable, AttributeBlock)]
		struct MyAttributeBlock {
			class: String,
		}

		#[template]
		fn MyTemplate(
			#[field(flatten)] attrs: MyAttributeBlock,
		) -> impl Bundle {
			rsx! {<div {attrs}/>}
		}

		parse(rsx! {
			<MyTemplate class="foo"/>
		})
		.xpect()
		.to_be_str("<div class=\"foo\"/>");
	}
}
