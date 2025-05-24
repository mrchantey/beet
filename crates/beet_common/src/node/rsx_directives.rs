use crate::as_beet::*;
use bevy::prelude::*;

define_token_collector!(
	CollectRsxDirectiveTokens,
	slot: SlotDirective,
);

pub fn rsx_directives_plugin(app: &mut App) {
	app.add_plugins(directive_plugin::<SlotDirective>);
}

/// Directive indicating a node should be moved to the slot with the given name.
/// All nodes without this directive are moved to the default slot.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "tokens", derive(ToTokens))]
pub struct SlotDirective(String);

impl TemplateDirective for SlotDirective {
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self> {
		match (key, value) {
			("slot", Some(value)) => Some(Self(value.to_string())),
			_ => None,
		}
	}
}