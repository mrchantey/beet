use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;


pub struct ApplySnippetsPlugin;


#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ApplySnippetsSet;


/// This schedule recursively resolves each newly added [`InstanceRoot`]
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct ApplySnippets;

impl Plugin for ApplySnippetsPlugin {
	#[rustfmt::skip]
	fn build(&self, app: &mut App) {
		app.init_plugin(schedule_order_plugin).add_systems(
			ApplySnippets,
			(
				apply_static_and_flush,
				apply_template_children,
				apply_slots,
			)
				.chain().in_set(ApplySnippetsSet),
		);
	}
}

/// When a [`SnippetRoot`] is added to an entity,
/// recusively apply each [`StaticRoot`]
fn apply_static_and_flush(world: &mut World) -> Result {
	let mut query = world
		.query_filtered::<Entity, (With<InstanceRoot>, Without<ResolvedRoot>)>(
		);
	while let Some(entity) = query.iter(world).next() {
		// println!("Applying static rsx for {entity}");
		world.entity_mut(entity).insert(ResolvedRoot);
		world.run_system_cached_with(apply_static_rsx, entity)??;
		world.run_system_cached_with(
			flush_on_spawn_deferred_recursive,
			entity,
		)??;
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
	let mut instance_expr_map = HashMap::new();

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

	// we've taken all expressions, now we can clear the instance root
	// for repoplulating with the static root.
	commands
		.entity(instance_root)
		.despawn_related::<Children>()
		.despawn_related::<TemplateRoot>()
		.despawn_related::<Attributes>()
		// we've already taken all expressions
		.remove::<ExprIdx>();

	// commands.run_system_cached_with(log_component_names, instance_root);

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
	// we need to reapply it. cant use builder.deny because thats recursive
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
				"Apply Snippets Error:
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
			"Apply Snippets Error:
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


pub(super) fn flush_on_spawn_deferred_recursive(
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

/// Add this system for [`OnSpawnDeferred`] behavior.
/// It must be called after *apply_slots* as it doesnt recurse into [`TemplateOf`]

/// more tests in static_scene_roundtrip.rs
#[cfg(test)]
mod test {
	use super::*;
	use crate::as_beet::*;
	use bevy::ecs::system::RunSystemOnce;
	use sweet::prelude::*;

	fn world() -> World {
		let mut app = App::new();
		app.add_plugins(ApplySnippetsPlugin);
		std::mem::take(app.world_mut())
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

		world.run_schedule(ApplySnippets);
		// Safely runs multiple times
		world.run_schedule(ApplySnippets);

		world
			.run_system_once(crate::apply_snippets::apply_slots)
			.ok(); // no matching entities ok
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
	}

	#[test]
	fn retains_parent() {
		let mut world = world();

		let child = world
			.spawn(rsx! { <div /> })
			.insert(SnippetRoot::default())
			.id();
		let parent = world.spawn(rsx! { <main></main> }).id();
		let main = world.entity(parent).get::<Children>().unwrap()[0];
		world.entity_mut(main).add_child(child);

		let _snippet = world
			.spawn((rsx! { <span /> }, StaticRoot))
			.remove::<InstanceRoot>()
			.insert(SnippetRoot::default())
			.id();
		world.run_schedule(ApplySnippets);

		world
			.run_system_cached_with(render_fragment, parent)
			.unwrap()
			.xpect_eq("<main><span/></main>");
	}

	#[test]
	#[should_panic = "Not all ExprIdx were applied.."]
	fn rsx_snippet_missing_idx() {
		parse(rsx! { <div>{7}</div> }, rsx! {
			<div>
				<br />
			</div>
		});
	}
	#[test]
	#[should_panic = "The instance is missing an ExprIdx.."]
	fn instance_missing_idx() {
		parse(
			rsx! {
				<div>
					<br />
				</div>
			},
			rsx! { <div>{7}</div> },
		);
	}


	#[test]
	fn block_nodes() {
		parse(
			rsx! { <main>{7}</main> },
			// because ExprIdx matches, this should be replace with 7
			rsx! {
				<div>
					<span>{()}</span>
					<br />
				</div>
			},
		)
		.xpect_eq("<div><span>7</span><br/></div>");
	}
	#[test]
	fn combinator_block_nodes() {
		parse(
			rsx_combinator! {"<main>{7}</main>"},
			// because ExprIdx matches, this should be replace with 7
			rsx_combinator! {"<div><span>{()}</span><br/></div>"},
		)
		.xpect_eq("<div><span>7</span><br/></div>");
	}
	#[test]
	fn iterators() {
		parse(
			rsx! { <main>{vec!["a", "b", "c"]}</main> },
			// because ExprIdx matches, this should be replace with 7
			rsx! {
				<div>
					<span>{()}</span>
					<br />
				</div>
			},
		)
		.xpect_eq("<div><span>abc</span><br/></div>");
	}
	#[test]
	fn attribute_values() {
		let val1 = 1;
		let val2 = 7;
		parse(rsx! { <main key=val2 /> }, rsx! {
			<div>
				<span key=val1></span>
				<br />
			</div>
		})
		.xpect_eq("<div><span key=\"7\"></span><br/></div>");
	}
	#[test]
	fn events() {
		// didnt panic
		parse(
			rsx! { <main onclick=|| {} /> },
			rsx! { <main oninput=|| {} /> },
		)
		.xpect_eq("<main oninput/>");
	}

	#[test]
	fn attribute_blocks() {
		#[derive(Buildable, AttributeBlock)]
		struct Foo {
			key: u32,
		}
		parse(rsx! { <main {Foo { key: 9 }} /> }, rsx! {
			<div>
				<span {()}></span>
				<br />
			</div>
		})
		.xpect_eq("<div><span key=\"9\"></span><br/></div>");
	}
	#[test]
	fn root() {
		parse(rsx! { {7} }, rsx! {
			hello
			{()}
		})
		.xpect_eq("hello7");
	}

	#[template]
	fn MyTemplate(initial: u32) -> impl Bundle {
		rsx! { {initial} }
	}
	#[template]
	fn SomeOtherName() -> impl Bundle { () }

	#[test]
	fn template_simple() {
		parse(
			rsx! { <MyTemplate initial=3 /> },
			// the name doesnt matter, a <SomeTitleCase/> is treated the same as
			// any other block {}
			rsx! {
				<span>
					<SomeOtherName />
				</span>
			},
		)
		.xpect_eq("<span>3</span>");
	}
	#[test]
	fn template_expr_attr() {
		let val = 5;
		parse(
			// attributes are resolved here, there is only one ExprIdx
			// in this tree
			rsx! { <MyTemplate initial=val /> },
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
		.xpect_eq("5");
	}


	#[test]
	fn child_already_resolved() {
		let mut world = world();
		let child_idx =
			SnippetRoot::new_file_line_col(file!(), line!(), column!());
		let parent_idx =
			SnippetRoot::new_file_line_col(file!(), line!(), column!());

		let child_instance =
			rsx! { <div>pasta is <MyTemplate initial=3 /></div> };
		let child_static =
			rsx! { <div>pizza is <MyTemplate initial=4 /></div> };


		let child = world.spawn(child_instance).insert(child_idx.clone()).id();
		world
			.spawn(child_static)
			.remove::<InstanceRoot>()
			.insert((StaticRoot, child_idx));

		world.run_schedule(ApplySnippets);
		world
			.run_system_once_with(render_fragment, child)
			.unwrap()
			.xpect_eq("<div>pizza is 3</div>");

		let parent_instance = rsx! {
			<article>
				<h1>all about pasta</h1>
				{child}
			</article>
		};
		let parent_static = rsx! {
			<article>
				<h1>all about pizza</h1>
				{child}
			</article>
		};
		let parent =
			world.spawn(parent_instance).insert(parent_idx.clone()).id();
		world
			.spawn(parent_static)
			.remove::<InstanceRoot>()
			.insert((StaticRoot, parent_idx));

		world.run_schedule(ApplySnippets);
		world
			.run_system_once_with(render_fragment, parent)
			.unwrap()
			.xpect_eq(
				"<article><h1>all about pizza</h1><div>pizza is 3</div></article>",
			);
	}


	#[test]
	fn flush_on_spawn_bfs_order() {
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
		val().xpect_eq(vec![0, 1, 2, 3, 4]);
	}

	fn parse_instance(instance: impl Bundle) -> String {
		let mut world = world();
		let instance = world.spawn(instance).id();

		world.run_schedule(ApplySnippets);
		world
			.run_system_once(crate::apply_snippets::apply_slots)
			.ok(); // no matching entities ok
		world
			.run_system_once_with(render_fragment, instance)
			.unwrap()
	}

	#[test]
	fn flush_on_spawn_templates() {
		#[template]
		fn MyTemplate() -> impl Bundle {
			rsx! { <div /> }
		}

		parse_instance(rsx! { <MyTemplate /> }).xpect_str("<div/>");
	}
	#[test]
	fn flush_on_spawn_attribute_blocks() {
		#[derive(Default, Buildable, AttributeBlock)]
		struct MyAttributeBlock {
			class: String,
		}

		#[template]
		fn MyTemplate(
			#[field(flatten)] attrs: MyAttributeBlock,
		) -> impl Bundle {
			rsx! { <div {attrs} /> }
		}

		parse_instance(rsx! { <MyTemplate class="foo" /> })
			.xpect_str("<div class=\"foo\"/>");
	}
}
