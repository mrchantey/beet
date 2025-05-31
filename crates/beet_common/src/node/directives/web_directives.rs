use std::path::PathBuf;
use crate::as_beet::*;
use bevy::prelude::*;
use sweet::prelude::WorkspacePathBuf;

define_token_collector!(
	CollectWebDirectiveTokens,
	html_insert: HtmlInsertDirective,
	client_island: ClientIslandDirective,
);

/// plugin containing all web directive extraction
pub fn web_directives_plugin(app: &mut App) {
	app.add_plugins((
		directive_plugin::<HtmlInsertDirective>,
		directive_plugin::<ClientIslandDirective>,
	))
	.add_systems(Update, extract_lang_content.in_set(ExtractDirectivesSet));
}

/// Directive to indicate that the node should be inserted directly under some part of the
/// body, regardless of where it is in the template.
/// [`HtmlInsertDirective::Head`] is usually automatically added to non-layout elements
/// like script and style.
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum HtmlInsertDirective {
	/// Insert the node in the head of the document.
	#[default]
	Head,
	/// Insert the node in the body of the document.
	Body,
}

impl TemplateDirective for HtmlInsertDirective {
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self> {
		match (key, value) {
			("insert:head", _) => Some(Self::Head),
			("insert:body", _) => Some(Self::Body),
			_ => None,
		}
	}
}

/// Directive for how the node should be rendered and loaded on the client.
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum ClientIslandDirective {
	/// Render the node statically then hydrate it on the client
	#[default]
	Load,
	/// aka Client Side Rendering, do not render the node statically, only render on the client
	Only,
}

impl TemplateDirective for ClientIslandDirective {
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self> {
		match (key, value) {
			("client:only", _) => Some(ClientIslandDirective::Only),
			("client:load", _) => Some(ClientIslandDirective::Load),
			_ => None,
		}
	}
}

/// Serialized version of this [`TemplateNode`]
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct TemplateSerde {
	type_name: String,
	ron: String,
}

#[cfg(feature = "serde")]
impl TemplateSerde {
	/// Create a new [`TemplateSerde`] from a value that can be serialized to RON.
	/// ## Panics
	/// Panics if the serialization failed.
	pub fn new<T: serde::ser::Serialize>(val: &T) -> Self {
		Self {
			type_name: std::any::type_name::<T>().to_string(),
			ron: ron::ser::to_string(val)
				.expect("Failed to serialize template"),
		}
	}
	pub fn parse<T>(&self) -> Result<T, ron::de::SpannedError>
	where
		T: serde::de::DeserializeOwned,
	{
		ron::de::from_str(&self.ron)
	}
}

/// The content of a style template, either as inner text or a file path. The
/// content of file paths is resolved lazily by the
#[derive(Debug, Clone, Component, Reflect)]
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


fn extract_lang_content(
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
				&& let Some(value) = &lit.value && value.starts_with(".")
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



/// Template directives related to web rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WebDirective {
	StyleId { id: u64 },
}


#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for WebDirective {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			WebDirective::StyleId { id } => {
				quote::quote! {WebDirective::StyleId{ id: #id }}
			}
		}
	}
}


#[derive(Debug, PartialEq, thiserror::Error)]
pub enum ParseDirectiveError {
	#[error("Failed To Parse Directive: {key}\n{message}")]
	InvalidValue { key: String, message: String },
}

pub type ParseDirectiveResult<T> = std::result::Result<T, ParseDirectiveError>;

impl WebDirective {
	pub fn try_from_attr(
		key: &str,
		value: Option<&str>,
	) -> ParseDirectiveResult<Option<Self>> {
		match (key, value) {
			("style:id", Some(val)) => {
				let id = val.parse::<u64>().map_err(|_| {
					ParseDirectiveError::InvalidValue {
						key: key.to_string(),
						message: format!(
							"Failed to parse style:id value: {}",
							val
						),
					}
				})?;
				Ok(Some(WebDirective::StyleId { id }))
			}
			_ => Ok(None),
		}
	}
}


pub trait WebDirectiveExt {
	fn find_map_web_directive<T>(
		&self,
		func: impl Fn(&WebDirective) -> Option<&T>,
	) -> Option<&T>;
	fn style_id(&self) -> Option<u64> {
		self.find_map_web_directive(|d| match d {
			WebDirective::StyleId { id } => Some(id),
		})
		.copied()
	}
}
impl WebDirectiveExt for Vec<WebDirective> {
	fn find_map_web_directive<T>(
		&self,
		func: impl Fn(&WebDirective) -> Option<&T>,
	) -> Option<&T> {
		self.iter().find_map(|d| func(d))
	}
}
impl WebDirectiveExt for WebDirective {
	fn find_map_web_directive<T>(
		&self,
		func: impl Fn(&WebDirective) -> Option<&T>,
	) -> Option<&T> {
		func(self)
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		WebDirective::try_from_attr("style:id", Some("123"))
			.xpect()
			.to_be(Ok(Some(WebDirective::StyleId { id: 123 })));
		WebDirective::try_from_attr("style:id", Some("foobar"))
			.xpect()
			.to_be_err();
	}
}
