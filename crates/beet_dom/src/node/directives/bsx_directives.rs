use crate::prelude::*;
use beet_core::prelude::*;


// pub type BsxDirectives = BsxStructDefault;


/// Annotate the created struct with a ..default()
#[derive(Debug, Default, Copy, Clone, Component, Reflect)]
#[reflect(Default, Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
#[require(RequiresDomIdx)]
pub struct BsxStructDefault;

impl TemplateDirective for BsxStructDefault {
	fn try_from_attribute(key: &str, value: Option<&TextNode>) -> Option<Self> {
		match (key, value) {
			("default", _) => Some(Self),
			_ => None,
		}
	}
}
