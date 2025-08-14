use super::*;
use crate::as_beet::*;
use bevy::prelude::*;

/// Specify types for variadic functions like TokenizeComponent
pub type WebDirectives =
	(HtmlHoistDirective, ClientLoadDirective, ClientOnlyDirective);


/// Directive to indicate that the node should be inserted directly under some part of the
/// body, regardless of where it is in the template.
/// Note that by default elements matching the [`HtmlConstants::hoist_to_head_tags`]
/// will be hoisted to the head of the document, to disable this behavior
/// use the `hoist:none` directive on the element.
#[derive(
	Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub enum HtmlHoistDirective {
	/// Insert the node in the head of the document.
	#[default]
	Head,
	/// Insert the node in the body of the document.
	Body,
	/// Do not hoist this element, even if it matches a [`HtmlConstants::hoist_to_head_tags`].
	None,
}

impl TemplateDirective for HtmlHoistDirective {
	fn try_from_attribute(key: &str, value: Option<&TextNode>) -> Option<Self> {
		match (key, value) {
			("hoist:head", _) => Some(Self::Head),
			("hoist:body", _) => Some(Self::Body),
			("hoist:none", _) => Some(Self::None),
			_ => None,
		}
	}
}



/// Render the node statically then hydrate it on the client
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[require(RequiresDomIdx)]
pub struct ClientLoadDirective;

impl TemplateDirective for ClientLoadDirective {
	fn try_from_attribute(key: &str, value: Option<&TextNode>) -> Option<Self> {
		match (key, value) {
			("client:load", _) => Some(Self),
			_ => None,
		}
	}
}

/// aka Client Side Rendering, do not create a server-side version of this node,
/// and instead mount it directly on the client.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[require(RequiresDomIdx)]
pub struct ClientOnlyDirective {
	/// Optionally specify the element to mount to, defaulting to the body
	pub root_id: Option<String>,
}

impl TemplateDirective for ClientOnlyDirective {
	fn try_from_attribute(key: &str, value: Option<&TextNode>) -> Option<Self> {
		match (key, value) {
			("client:only", None) => Some(Self::default()),
			("client:only", Some(text)) => Some(Self {
				root_id: Some(text.0.clone()),
			}),
			_ => None,
		}
	}
}
