use crate::prelude::*;
use beet_bevy::bevybail;
use beet_common::node::TextNode;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

/// The fs loaded and deduplicated [`LangContent`], existing seperately from the
/// originating tree(s).
#[derive(Debug, Clone, PartialEq, Hash, Deref, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangPartial(pub String);

impl LangPartial {
	/// Create a new [`LangPartial`] from a `String`.
	pub fn new(content: impl Into<String>) -> Self { Self(content.into()) }
}


/// For trees with [`PortalTo<LangPartial`], insert a single element at the top
/// of the tree, to be hoisted to the head.
pub fn resolve_lang_partials(
	mut commands: Commands,
	partials: Query<(Entity, &LangPartial)>,
	parents: Query<&ChildOf>,
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
		root_content
			.entry(parents.root_ancestor(entity))
			.or_default()
			.insert(partial.0, partial.1.0.clone());
	}

	for (root, partials) in root_content.into_iter() {
		for (partial_entity, contents) in partials.into_iter() {
			// insert as direct child of root
			commands
				.entity(partial_entity)
				.clone_and_spawn()
				.remove::<LangPartial>()
				.insert((ChildOf(root), children![TextNode::new(contents)]));
		}
	}

	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_common::node::HtmlHoistDirective;
	use beet_common::node::NodeTag;
	use beet_common::node::StyleScope;
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
			.spawn((NodeTag::new("html"), children![(
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
			.run_system_once(super::resolve_lang_partials)
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
