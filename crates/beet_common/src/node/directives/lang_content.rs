use crate::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;


/// The fs loaded and deduplicated [`LangContent`], existing seperately from the
/// originating tree(s).
#[derive(Debug, Clone, PartialEq, Hash, Deref, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangSnippet(pub String);

impl LangSnippet {
	/// Create a new [`LangSnippet`] from a `String`.
	pub fn new(content: impl Into<String>) -> Self { Self(content.into()) }
}

/// The replacement for [`LangContent`] after the lang snippet has been
/// extracted, referencing the path to the snippet scene file.
#[derive(Debug, Clone, PartialEq, Hash, Deref, Component, Reflect)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[reflect(Component)]
// #[component(immutable)]
pub struct LangSnippetPath(pub WsPathBuf);



/// The content of a script or style template, either as inner text or a file path.
/// Attributes and children are removed.
/// File paths are resolved lazily in beet_build.
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum LangContent {
	/// The content is the inner text of a `<style>` or `<script>` tag.
	InnerText(String),
	/// The content is a file path to a `<style src="...">` or `<script src="...">`.
	File(WsPathBuf),
}
impl LangContent {
	pub fn file(src: &str, span: &FileSpan) -> Self {
		let path = span
			.file()
			.parent()
			.map(|parent| parent.join(src))
			.unwrap_or(PathBuf::from(src));
		Self::File(WsPathBuf::new(path))
	}

	/// create a hash ignoring whitespace in the case of [`Self::InnerText`]
	pub fn hash_no_whitespace(&self, hasher: &mut impl Hasher) {
		match self {
			Self::InnerText(text) => {
				let text = text.replace(char::is_whitespace, "");
				text.hash(hasher);
			}
			Self::File(path) => {
				path.to_string_lossy().hash(hasher);
			}
		}
	}
}

/// For script and style tags, replace the [`ElementNode`] with a [`LangContent`]
pub(crate) fn extract_lang_content(
	mut commands: Commands,
	text_nodes: Query<&TextNode>,
	attr_lits: Query<(Entity, &AttributeKey, Option<&AttributeLit>)>,
	query: Populated<
		(
			Entity,
			&NodeTag,
			Option<&Attributes>,
			Option<&Children>,
			&FileSpanOf<ElementNode>,
		),
		Added<NodeTag>,
	>,
) {
	'iter_elements: for (entity, tag, attributes, children, span) in
		query.iter()
	{
		if !["style", "script"].contains(&tag.as_str()) {
			continue;
		}
		// 1. Check for file src attribute
		for (attr_entity, key, value) in attributes
			.iter()
			.flat_map(|a| a.iter())
			.filter_map(|a| attr_lits.get(a).ok())
		{
			match (key.as_str(), value) {
				("is:inline", _) => {
					commands.entity(attr_entity).despawn();
					// skip inline templates
					continue 'iter_elements;
				}
				("src", Some(AttributeLit::String(value)))
					if value.starts_with(".") =>
				{
					commands.entity(attr_entity).despawn();
					commands
						.entity(entity)
						.insert(LangContent::file(value, span));
					continue 'iter_elements;
				}
				_ => {}
			}
		}
		// 2. Check for inner text
		for child in children.iter().flat_map(|c| c.iter()) {
			if let Ok(text_node) = text_nodes.get(child) {
				commands.entity(entity).insert(LangContent::InnerText(
					text_node.text().to_string(),
				));
				commands.entity(child).despawn();
				continue 'iter_elements;
			}
		}
		commands.entity(entity).despawn_related::<Children>();
		// 3. ignore empty tag with no workspace src
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::dir;
	use beet_utils::prelude::WsPathBuf;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn extracts_inline() {
		let mut world = World::new();
		let entity = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::default()),
				children![TextNode::new("div { color: red; }")],
			))
			.id();
		world.run_system_once(super::extract_lang_content).unwrap();
		let entity = world.entity(entity);
		entity
			.get::<LangContent>()
			.unwrap()
			.xpect()
			.to_be(&LangContent::InnerText("div { color: red; }".to_string()));
		entity.contains::<Children>().xpect().to_be(false);
	}
	#[test]
	fn extracts_src() {
		let mut world = World::new();
		let entity = world
			.spawn((
				NodeTag::new("style"),
				FileSpanOf::<ElementNode>::new(FileSpan::new(
					file!(),
					default(),
					default(),
				)),
				related!(
					Attributes[(
						AttributeKey::new("src"),
						AttributeLit::String("./style.css".to_string())
					)]
				),
			))
			.id();
		world.run_system_once(super::extract_lang_content).unwrap();
		let entity = world.entity(entity);
		entity
			.get::<LangContent>()
			.unwrap()
			.xpect()
			.to_be(&LangContent::File(
				WsPathBuf::new(dir!()).join("style.css"),
			));
		entity.contains::<Attributes>().xpect().to_be(false);
	}
}
