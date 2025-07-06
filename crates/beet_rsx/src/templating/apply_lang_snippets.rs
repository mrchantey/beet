use crate::prelude::*;
use beet_bevy::prelude::HierarchyQueryExtExt;
use beet_common::prelude::*;
use beet_utils::prelude::ReadFile;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;


/// For trees with [`PortalTo<LangSnippet>`], insert a single element at the top
/// of the tree, to be hoisted to the head.
pub fn apply_lang_snippets(
	mut commands: Commands,
	snippets: Query<(Entity, &LangSnippet, &LangSnippetPath)>,
	parents: Query<&ChildOf>,
	roots: Query<(), With<HtmlDocument>>,
	query: Populated<
		(Entity, &LangSnippetPath),
		(Added<LangSnippetPath>, Without<LangSnippet>),
	>,
) -> Result {
	let mut root_content = HashMap::<Entity, HashMap<Entity, String>>::new();

	for (entity, instance_path) in query.iter() {
		let snippet = match snippets
			.iter()
			.find(|(_, _, snippet_path)| *snippet_path == instance_path)
		{
			Some(snippet) => snippet,
			None => {
				let _file = ReadFile::to_string(instance_path.0.clone())?;
				todo!("load snippet scene, needs world?");
			}
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
			.insert(snippet.0, snippet.1.0.clone());
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
						.deny::<(LangSnippet, LangSnippetPath)>();
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
	use beet_utils::prelude::WsPathBuf;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	// emulate the beet_build::extract_lang_partials
	#[test]
	fn global_style() {
		let mut world = World::new();
		let path = LangSnippetPath(WsPathBuf::new("some-path.ron"));
		world.spawn((
			NodeTag::new("style"),
			LangSnippet::new("body { color: red; }"),
			StyleScope::Global,
			path.clone(),
			HtmlHoistDirective::Body,
		));
		let tree = world.spawn((HtmlDocument, path)).id();
		world
			.run_system_once(super::apply_lang_snippets)
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
		let path = LangSnippetPath(WsPathBuf::new("some-path.ron"));

		world.spawn((
			NodeTag::new("style"),
			LangSnippet::new("body { color: red; }"),
			path.clone(),
		));
		let tree = world
			.spawn((HtmlDocument, children![
				path.clone(),
				(InstanceRoot, path)
			]))
			.id();
		world
			.run_system_once(super::apply_lang_snippets)
			.unwrap()
			.unwrap();
		world
			.run_system_once_with(render_fragment, tree)
			.unwrap()
			.xpect()
			.to_be_str("<style>body { color: red; }</style>");
	}
}
