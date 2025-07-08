use beet_bevy::prelude::*;
use beet_common::prelude::*;
use beet_rsx::prelude::*;
use beet_utils::prelude::*;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use rapidhash::RapidHasher;
use std::hash::Hash;
use std::hash::Hasher;


/// Ensure every [`LangSnippet`] has a unique id, even across
/// partial changes. This is also used as the [`StyleId`]
#[derive(Debug, Default, Clone, Resource)]
pub(super) struct LangSnippetId(u64);
impl LangSnippetId {
	/// Get the next unique style id.
	pub fn next(&mut self) -> u64 {
		let id = self.0;
		self.0 += 1;
		id
	}
}

/// Deduplicate `script` and `style` tags by replacing the
/// [`LangContent`] directive with a [`LangSnippetPath`].
pub(super) fn extract_lang_snippets(
	mut commands: Commands,
	mut id_counter: Local<LangSnippetId>,
	config: Res<WorkspaceConfig>,
	idxs: Query<&MacroIdx>,
	parents: Query<&ChildOf>,
	query: Populated<
		(
			Entity,
			&NodeTag,
			&LangContent,
			Option<&StyleScope>,
			Option<&HtmlHoistDirective>,
		),
		Added<LangContent>,
	>,
) -> Result {
	let mut groups = HashMap::<u64, Vec<Entity>>::new();
	// create a unique hash for each LangSnippet to deduplicate,
	// including the tag, content, scope, and hoist directive.
	for (entity, tag, lang_content, scope, hoist) in query.iter() {
		let mut hasher = RapidHasher::default();
		tag.hash(&mut hasher);
		lang_content.hash_no_whitespace(&mut hasher);
		scope.map(|s| s.hash(&mut hasher));
		hoist.map(|h| h.hash(&mut hasher));
		let hash = hasher.finish();
		groups.entry(hash).or_default().push(entity);
	}
	for rsx_entities in groups.into_values() {
		// we just take the first entity as the representative,
		// they are identical
		let (_, tag, content, scope, hoist) = query
			.get(rsx_entities[0])
			// must exist
			.unwrap();

		let snippet_source = match content {
			LangContent::InnerText(_) => {
				let idx = parents
					.iter_ancestors_inclusive(rsx_entities[0])
					.find_map(|e| idxs.get(e).ok())
					.ok_or_else(|| {
						bevyhow!(
							"LangContent without parent MacroIdx: {:?}",
							rsx_entities[0]
						)
					})?;
				&idx.file
			}
			LangContent::File(path) => path,
		};
		let index = id_counter.next();
		let snippet_path =
			LangSnippetPath(config.lang_snippet_path(snippet_source, index));

		let content = match content {
			LangContent::InnerText(text) => text.to_string(),
			LangContent::File(path) => {
				let file = ReadFile::to_string(path.into_abs())?;
				file.to_string()
			}
		};


		// only local style tags
		let style_id = if tag.as_str() == "style"
			&& scope.map_or(true, |s| s == &StyleScope::Local)
		{
			Some(StyleId::new(index))
		} else {
			None
		};

		// this should be a clone instead
		let mut lang_entity = commands.spawn((
			tag.clone(),
			LangSnippet(content),
			snippet_path.clone(),
		));
		style_id.map(|id| {
			lang_entity.insert(id);
		});
		scope.map(|scope| {
			lang_entity.insert(scope.clone());
		});
		hoist.map(|hoist| {
			lang_entity.insert(hoist.clone());
		});
		let lang_entity = lang_entity.id();

		for rsx_entity in rsx_entities.iter() {
			// these entities are now just pointers to the shared content
			let mut entity = commands.entity(*rsx_entity);
			entity
				.remove::<LangContent>()
				.remove::<NodeTag>()
				.remove::<ElementNode>()
				.remove::<Children>()
				.remove::<Attributes>()
				.insert(snippet_path.clone())
				// despawning the last of these entities will
				// remove the lang entity
				.with_child(GarbageCollectRef(lang_entity));
			style_id.map(|id| {
				entity.insert(id);
			});
		}
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use crate::as_beet::*;
	use beet_bevy::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::without_fs());

		let path = WorkspaceConfig::default()
			.lang_snippet_path(&WsPathBuf::new(file!()), 0);

		let entity = app
			.world_mut()
			.spawn(rsx! {<style>div{color:blue;}</style>})
			.get::<Children>()
			.unwrap()[0];
		app.update();

		app.world()
			.entity(entity)
			.get::<StyleId>()
			.unwrap()
			.xpect()
			.to_be(&StyleId::new(0));
		app.world()
			.entity(entity)
			.get::<LangSnippetPath>()
			.unwrap()
			.xpect()
			.to_be(&LangSnippetPath(path.clone()));


		let (snippet, snippet_path) = app
			.world_mut()
			.query_once::<(&LangSnippet, &LangSnippetPath)>()[0];
		#[cfg(feature = "css")]
		expect(&snippet.0)
			.to_be("div[data-beet-style-id-0] {\n  color: #00f;\n}\n");
		#[cfg(not(feature = "css"))]
		expect(&snippet.0).to_be("div{color:blue;}");
		expect(&snippet_path.0).to_be(&path);
	}
	#[test]
	fn global_no_styleid() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::without_fs());

		let entity = app
			.world_mut()
			.spawn(rsx! {<style scope:global>div{color:blue;}</style>})
			.get::<Children>()
			.unwrap()[0];
		app.update();
		app.world()
			.entity(entity)
			.get::<StyleId>()
			.xpect()
			.to_be_none();
	}
	#[test]
	fn file_src() {
		let mut app = App::new();
		app.add_plugins(BuildPlugin::without_fs());

		let entity = app
			.world_mut()
			.spawn((
				NodeTag(String::from("style")),
				ElementNode { self_closing: true },
				LangContent::File(WsPathBuf::new(
					"crates/beet_router/src/test_site/components/style.css",
				)),
			))
			.id();
		app.update();
		app.world()
			.entity(entity)
			.get::<StyleId>()
			.unwrap()
			.xpect()
			.to_be(&StyleId::new(0));
		app.world()
			.entity(entity)
			.contains::<ElementNode>()
			.xpect()
			.to_be_false();
	}
}
