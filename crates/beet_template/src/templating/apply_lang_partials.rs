use beet_bevy::bevybail;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;


/// For trees with [`PortalTo<LangPartial>`], insert a single element at the top
/// of the tree, to be hoisted to the head.
pub fn apply_lang_partials(
	mut commands: Commands,
	partials: Query<(Entity, &LangPartial)>,
	parents: Query<&ChildOf>,
	roots: Query<&BeetRoot>,
	query: Populated<(Entity, &NodePortal), With<PortalTo<LangPartial>>>,
) -> Result {
	let mut root_content = HashMap::<Entity, HashMap<Entity, String>>::new();

	for (entity, portal) in query.iter() {
		let Ok(partial) = partials.get(**portal) else {
			bevybail!(
				"NodePortal is missing a target LangPartial: {:?}",
				**portal
			);
		};

		let Some(root_ancestor) =
			parents.iter_ancestors(entity).find(|e| roots.contains(*e))
		else {
			bevybail!("NodePortal is not a child of a BeetRoot: {:?}", entity);
		};

		root_content
			.entry(root_ancestor)
			.or_default()
			.insert(partial.0, partial.1.0.clone());
	}

	for (root, partials) in root_content.into_iter() {
		for (partial_entity, contents) in partials.into_iter() {
			// insert as direct child of root
			commands
				.entity(partial_entity)
				// just cloning NodeTag and StyleId
				.clone_and_spawn_with(|builder| {
					builder.deny::<(LangPartial, NodePortalTarget)>();
				})
				.insert((ChildOf(root), ElementNode::open(), children![
					TextNode::new(contents)
				]));
		}
	}

	Ok(())
}



#[cfg(test)]
mod test {
	use beet_common::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;


	// emulate the beet_build::extract_lang_partials
	fn setup() -> (World, Entity) {
		let mut world = World::new();
		let partial = world
			.spawn((
				NodeTag::new("style"),
				LangPartial::new("body { color: red; }"),
				StyleScope::Global,
				HtmlHoistDirective::Body,
			))
			.id();
		let tree = world
			.spawn((InstanceRoot, NodeTag::new("html"), children![(
				NodePortal::new(partial),
				PortalTo::<LangPartial>::default()
			)]))
			.id();

		(world, tree)
	}


	#[test]
	fn works() {
		let (mut world, tree) = setup();
		world
			.run_system_once(super::apply_lang_partials)
			.unwrap()
			.unwrap();

		let children = world.entity(tree).get::<Children>().unwrap();
		expect(children.len()).to_be(2);

		let spawned = world.entity(children[1]);
		spawned.contains::<Children>().xpect().to_be_true();
		spawned
			.get::<NodeTag>()
			.unwrap()
			.xpect()
			.to_be(&NodeTag::new("style"));
		spawned
			.get::<HtmlHoistDirective>()
			.unwrap()
			.xpect()
			.to_be(&HtmlHoistDirective::Body);
		spawned
			.get::<StyleScope>()
			.unwrap()
			.xpect()
			.to_be(&StyleScope::Global);
	}
}
