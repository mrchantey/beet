use crate::as_beet::*;
use bevy::prelude::*;
use std::hash::Hash;

/// Define on a component to indicate that template scoped styles
/// in the outer template should cascade into it.
///
/// `rsx!{
/// 	<MyComponent style:cascade>
/// 	<style>
/// 		div { color: blue; }
/// 	</style>
/// }`
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Component, Reflect,
)]
#[reflect(Component)]
#[component(immutable)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct StyleCascade;


impl TemplateDirective for StyleCascade {
	fn try_from_attribute(
		key: &str,
		value: Option<&AttributeValueStr>,
	) -> Option<Self> {
		match (key, value) {
			("style:cascade", _) => Some(Self),
			_ => None,
		}
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



impl TemplateDirective for StyleScope {
	fn try_from_attribute(
		key: &str,
		value: Option<&AttributeValueStr>,
	) -> Option<Self> {
		match (key, value) {
			("scope:local", _) => Some(Self::Local),
			("scope:global", _) => Some(Self::Global),
			_ => None,
		}
	}
}
