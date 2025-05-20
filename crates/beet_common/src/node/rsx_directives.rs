use crate::as_beet::*;
use bevy::prelude::*;



pub fn rsx_directives_plugin(app: &mut App) {
	app.add_plugins(directive_plugin::<SlotDirective>);
}
/// Directive for which slot to render the node in.
#[derive(Debug, Default, Component, Deref)]
pub struct SlotDirective(String);

impl TemplateDirective for SlotDirective {
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self> {
		match (key, value) {
			("slot", Some(value)) => Some(Self(value.to_string())),
			_ => None,
		}
	}
}
