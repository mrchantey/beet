use crate::as_beet::*;
use bevy::prelude::*;
use std::path::PathBuf;
use sweet::prelude::WorkspacePathBuf;



/// The content of a style template, either as inner text or a file path. The
/// content of file paths is resolved lazily by the
#[derive(Debug, Clone, PartialEq, Hash, Component, Reflect)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum LangContent {
	InnerText(String),
	File(WorkspacePathBuf),
}
impl LangContent {
	pub fn file(src: &str, span: &FileSpan) -> Self {
		let path = span
			.file()
			.parent()
			.map(|parent| parent.join(src))
			.unwrap_or(PathBuf::from(src));
		Self::File(WorkspacePathBuf::new(path))
	}
}


pub(super) fn extract_lang_content(
	mut commands: Commands,
	text_nodes: Query<&TextNode>,
	attr_lits: Query<&AttributeLit>,
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
		for lit in attributes
			.iter()
			.flat_map(|a| a.iter())
			.filter_map(|a| attr_lits.get(a).ok())
		{
			if lit.key == "is:inline" {
				// skip inline templates
				continue 'iter_elements;
			} else if lit.key == "src"
				&& let Some(value) = &lit.value
				&& value.starts_with(".")
			{
				commands
					.entity(entity)
					.insert(LangContent::file(value, span));
				// found a LangContent::File
				continue 'iter_elements;
			}
		}
		for child in children.iter().flat_map(|c| c.iter()) {
			if let Ok(text_node) = text_nodes.get(child) {
				commands.entity(entity).insert(LangContent::InnerText(
					text_node.text().to_string(),
				));
				commands.entity(child).despawn();
				// found a LangContent::InnerText
				continue 'iter_elements;
			}
		}
		// ignore empty tag with no workspace src
	}
}


/// Define the scope of a style tag, set by using the `scope` template directive
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum StyleScope {
	/// The default scope for a style tag, its styles will only be applied to
	/// elements within the component, each selector will be preprended with
	/// an attribute selector for the component, eg `[data-styleid-1]`.
	/// ## Example
	/// Remember `scope:local` is the default so this directive can be ommitted.
	/// ```rust ignore
	/// <style scope:local>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	#[default]
	Local,
	/// Global scope for a style tag, its styles will not have an attribute
	/// selector prepended to them, so will apply to all elements in the document.
	/// The style tag will still be extracted and deduplicated.
	/// ## Example
	/// ```rust ignore
	/// <style scope:global>
	/// 	div { color: blue; }
	/// </style>
	/// ```
	Global,
}


impl StyleScope {}
#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for StyleScope {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			Self::Local => quote::quote! { StyleScope::Local },
			Self::Global => quote::quote! { StyleScope::Global },
		}
	}
}
