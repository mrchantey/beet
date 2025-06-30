use crate::prelude::*;
use beet_bevy::bevybail;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;


/// For trees with [`PortalTo<LangPartial>`], insert a single element at the top
/// of the tree, to be hoisted to the head.
pub fn apply_lang_partials(
	mut commands: Commands,
	partials: Query<(Entity, &LangPartial)>,
	parents: Query<&ChildOf>,
	roots: Query<(), With<HtmlDocument>>,
	query: Populated<(Entity, &NodePortal), Added<PortalTo<LangPartial>>>,
	node_location: NodeLocation,
) -> Result {
	let mut root_content = HashMap::<Entity, HashMap<Entity, String>>::new();

	for (entity, portal) in query.iter() {
		let Ok(partial) = partials.get(**portal) else {
			bevybail!(
				"Failed to find a matching LangPartial for NodePortal: {}",
				node_location.stringify(**portal)
			);
		};

		let Some(root_ancestor) = parents
			.iter_ancestors_inclusive(entity)
			.find(|e| roots.contains(*e))
		else {
			// if no HtmlDocument this must be a StaticNode
			continue;
			// bevybail!(
			// 	"NodePortal is not a descendant of a HtmlDocument: {}",
			// 	node_location.stringify(**portal)
			// );
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
					builder
						.allow::<(NodeTag, StyleId)>()
						.deny::<(LangPartial, NodePortalTarget)>();
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
	use crate::prelude::*;
	use beet_common::prelude::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	// emulate the beet_build::extract_lang_partials
	#[test]
	fn global_style() {
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
			.spawn((
				HtmlDocument,
				NodePortal::new(partial),
				PortalTo::<LangPartial>::default(),
			))
			.id();
		world
			.run_system_once(super::apply_lang_partials)
			.unwrap()
			.unwrap();
		world
			.run_system_once_with(render_fragment, tree)
			.unwrap()
			.xpect()
			.to_be_str("<style>body { color: red; }</style>");
	}
	#[test]
	fn deduplicates_nested_roots() {
		let mut world = World::new();
		let partial = world
			.spawn((
				NodeTag::new("style"),
				LangPartial::new("body { color: red; }"),
			))
			.id();
		let tree = world
			.spawn((HtmlDocument, children![
				(NodePortal::new(partial), PortalTo::<LangPartial>::default()),
				(
					InstanceRoot,
					NodePortal::new(partial),
					PortalTo::<LangPartial>::default()
				)
			]))
			.id();
		world
			.run_system_once(super::apply_lang_partials)
			.unwrap()
			.unwrap();
		world
			.run_system_once_with(render_fragment, tree)
			.unwrap()
			.xpect()
			.to_be_str("<style>body { color: red; }</style>");
	}
}
