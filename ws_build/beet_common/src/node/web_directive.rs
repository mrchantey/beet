use super::TemplateDirective;
use crate::prelude::*;
use bevy::prelude::*;

/// plugin containing all web directive extraction
pub fn web_directives_plugin(app: &mut App) {
	app.add_plugins((
		directive_plugin::<HeadDirective>,
		directive_plugin::<ClientIslandDirective>,
	));
}

/// Directive to indicate that the node should be hoisted to the head of the document
#[derive(Component)]
pub struct HeadDirective;

impl TemplateDirective for HeadDirective {
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self> {
		match (key, value) {
			("hoist:head", _) => Some(Self),
			_ => None,
		}
	}
}

#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for HeadDirective {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		quote::quote! {HeadDirective}
	}
}

/// Directive for how the node should be rendered and loaded on the client.
#[derive(Debug, Default, Component)]
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
#[cfg(feature = "tokens")]
impl crate::prelude::RustTokens for ClientIslandDirective {
	fn into_rust_tokens(&self) -> proc_macro2::TokenStream {
		match self {
			ClientIslandDirective::Load => {
				quote::quote! {ClientIslandDirective::Load}
			}
			ClientIslandDirective::Only => {
				quote::quote! {ClientIslandDirective::Only}
			}
		}
	}
}



/// Template directives related to web rendering.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum WebDirective {
	StyleId { id: u64 },
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
