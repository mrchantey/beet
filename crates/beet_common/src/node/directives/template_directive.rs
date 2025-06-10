use crate::prelude::*;
use bevy::prelude::*;


/// System set in which all template directives are extracted.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct ExtractDirectivesSet;


/// A helper for simple template directives that can be
/// extracted via [`try_extract_directive`]. Some directives
/// like [`SlotTarget`] require more complex logic so do not implement this.
pub trait TemplateDirective: 'static + Sized + Component {
	/// Try to parse from an attribute key-value pair
	fn try_from_attribute(key: &str, value: Option<&str>) -> Option<Self>;
}




/// Generic plugin for extracting and propagating directives to tokens.
/// ## Example
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_common::prelude::*;
/// App::new().add_plugins(extract_directive_plugin::<ClientIslandDirective>);
/// ```
pub fn extract_directive_plugin<T: TemplateDirective>(app: &mut App) {
	app.add_systems(
		Update,
		try_extract_directive::<T>.in_set(ExtractDirectivesSet),
	);
}

/// Generic system for extracting a [TemplateDirective] from attributes.
fn try_extract_directive<T: TemplateDirective>(
	mut commands: Commands,
	query: Populated<(Entity, &AttributeOf, &AttributeLit)>,
) {
	for (entity, parent, lit) in query.iter() {
		let (key, value) = lit.into_parts();
		if let Some(directive) = T::try_from_attribute(key, value) {
			commands.entity(**parent).insert(directive);
			commands.entity(entity).despawn();
		}
	}
}
