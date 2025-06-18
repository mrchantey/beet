use crate::as_beet::*;
use beet_utils::prelude::*;
use bevy::prelude::*;
use std::hash::Hash;
use std::hash::Hasher;
use std::path::PathBuf;



/// The content of a style template, either as inner text or a file path. The
/// content of file paths is resolved lazily by the
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum LangContent {
	InnerText(String),
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
pub(super) fn extract_lang_content(
	mut commands: Commands,
	text_nodes: Query<&TextNode>,
	attr_lits: Query<(&AttributeKey, Option<&AttributeLit>)>,
	query: Populated<
		(
			Entity,
			&NodeTag,
			Option<&Attributes>,
			Option<&Children>,
			&ItemOf<ElementNode, FileSpan>,
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
		for (key, value) in attributes
			.iter()
			.flat_map(|a| a.iter())
			.filter_map(|a| attr_lits.get(a).ok())
		{
			match (key.as_str(), value) {
				("is:inline", _) => {
					// skip inline templates
					continue 'iter_elements;
				}
				("src", Some(AttributeLit::String(value)))
					if value.starts_with(".") =>
				{
					commands
						.entity(entity)
						// .remove::<ElementNode>()
						.insert(LangContent::file(value, span));

					// TODO load content as child text node?

					// found a LangContent::File
					continue 'iter_elements;
				}
				_ => {}
			}
		}
		for child in children.iter().flat_map(|c| c.iter()) {
			if let Ok(text_node) = text_nodes.get(child) {
				commands.entity(entity).insert(LangContent::InnerText(
					text_node.text().to_string(),
				));
				// found a LangContent::InnerText
				continue 'iter_elements;
			}
		}
		// ignore empty tag with no workspace src
	}
}
