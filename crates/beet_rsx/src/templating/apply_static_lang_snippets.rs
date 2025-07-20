use crate::prelude::*;
use beet_core::prelude::HierarchyQueryExtExt;
use beet_core::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;


/// For trees with [`PortalTo<LangSnippet>`], insert a single element at the top
/// of the tree, to be hoisted to the head.
pub fn apply_static_lang_snippets(
	mut commands: Commands,
	snippets: Query<(Entity, &LangSnippetHash), With<StaticLangNode>>,
	parents: Query<&ChildOf>,
	roots: Query<(), With<HtmlDocument>>,
	query: Populated<
		(Entity, &LangSnippetHash),
		(Added<LangSnippetHash>, Without<StaticLangNode>),
	>,
) -> Result {
	// hashmap where key is root ancestor and value is list of required LangSnippets
	let mut root_snippets = HashMap::<Entity, HashSet<Entity>>::new();

	for (instance_ent, instance_hash) in query.iter() {
		let (static_ent, _) =
			match snippets.iter().find(|(_, hash)| *hash == instance_hash) {
				Some(snippet) => snippet,
				None => {
					// static snippet not found, lazy load?
					continue;
				}
			};

		let Some(root_ancestor) = parents
			.iter_ancestors_inclusive(instance_ent)
			.find(|e| roots.contains(*e))
		else {
			// if no root HtmlDocument this must be a StaticNode?
			continue;
		};

		// despawn to ensure we don't have duplicates
		commands.entity(instance_ent).despawn();

		root_snippets
			.entry(root_ancestor)
			.or_default()
			.insert(static_ent);
	}

	for (root, snippets) in root_snippets.into_iter() {
		for snippet_entity in snippets.into_iter() {
			commands
				.entity(snippet_entity)
				.clone_and_spawn_with(|builder| {
					builder.deny::<StaticLangNode>();
				})
				// insert as direct child of root to be hoisted
				// to correct position later
				.insert(ChildOf(root));
		}
	}

	Ok(())
}



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;


	fn parse(
		// emulate the beet_build::extract_lang_partials
		static_bundle: impl Bundle,
		instance_bundle: impl Bundle,
	) -> String {
		let mut app = App::new();
		app.add_plugins(TemplatePlugin)
			.insert_resource(TemplateFlags::None);
		let _static_ent = app.world_mut().spawn(static_bundle).id();
		let instance_ent = app
			.world_mut()
			.spawn(HtmlDocument)
			.with_child(instance_bundle)
			.id();
		app.update();
		app.world_mut()
			.run_system_cached_with(render_fragment, instance_ent)
			.unwrap()
	}

	#[test]
	fn removes_instance() {
		parse(
			(
				NodeTag::new("style"),
				StyleElement,
				ElementNode::open(),
				InnerText::new("body { color: red; }"),
				StyleScope::Global,
				StaticLangNode,
				LangSnippetHash::new(0),
			),
			(
				NodeTag::new("style"),
				StyleElement,
				InnerText::new("body { color: red; }"),
				StyleScope::Global,
				LangSnippetHash::new(0),
			),
		)
		.xpect()
		.to_be_str("<!DOCTYPE html><html><head><style>body { color: red; }</style></head><body></body></html>");
	}
	#[test]
	fn global_style() {
		parse(
			(
				NodeTag::new("style"),
				StyleElement,
				ElementNode::open(),
				InnerText::new("body { color: red; }"),
				StyleScope::Global,
				StaticLangNode,
				LangSnippetHash::new(0),
				HtmlHoistDirective::Body,
			),
			LangSnippetHash::new(0),
		)
		.xpect()
		.to_be_str("<!DOCTYPE html><html><head></head><body><style>body { color: red; }</style></body></html>");
	}
	#[test]
	fn deduplicates_nested_roots() {
		parse(
			(
				NodeTag::new("style"),
				StyleElement,
				ElementNode::open(),
				StaticLangNode,
				InnerText::new("body { color: red; }"),
				LangSnippetHash::new(0),
			),
			children![
				LangSnippetHash::new(0),
				LangSnippetHash::new(0),
				(InstanceRoot, LangSnippetHash::new(0)),
				(InstanceRoot, LangSnippetHash::new(0)),
				(InstanceRoot, LangSnippetHash::new(0)),
			],
		)
		.xpect()
		.to_be_str("<!DOCTYPE html><html><head><style>body { color: red; }</style></head><body></body></html>");
	}
}
