use super::*;
#[cfg(feature = "tokens")]
use crate::as_beet::*;
use bevy::prelude::*;

/// plugin containing all web directive extraction
pub fn extract_web_directives_plugin(app: &mut App) {
	app.add_plugins((
		extract_directive_plugin::<HtmlInsertDirective>,
		extract_directive_plugin::<ClientLoadDirective>,
		extract_directive_plugin::<ClientOnlyDirective>,
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

/// Render the node statically then hydrate it on the client
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ClientLoadDirective;

impl TemplateDirective for ClientLoadDirective {
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self> {
		match (key, value) {
			("client:load", _) => Some(Self),
			_ => None,
		}
	}
}

/// aka Client Side Rendering, do not create a server-side version of this node,
/// and instead mount it directly on the client.
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct ClientOnlyDirective;

impl TemplateDirective for ClientOnlyDirective {
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self> {
		match (key, value) {
			("client:only", _) => Some(Self),
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
