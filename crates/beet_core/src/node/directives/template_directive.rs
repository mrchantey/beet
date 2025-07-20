use crate::prelude::*;
use bevy::prelude::*;


/// A helper for simple template directives that can be
/// extracted via [`try_extract_directive`]. Some directives
/// like [`SlotTarget`] require more complex logic so do not implement this.
pub trait TemplateDirective: 'static + Sized + Component {
	/// Try to parse from an attribute key-value pair
	fn try_from_attribute(
		key: &str,
		value: Option<&AttributeLit>,
	) -> Option<Self>;
}

/// Generic system for extracting a [TemplateDirective] from attributes.
/// ```rust
/// # use bevy::prelude::*;
/// # use beet_core::prelude::*;
/// App::new().add_systems(Update, try_extract_directive::<ClientLoadDirective>);
/// ```
pub fn try_extract_directive<T: TemplateDirective>(
	mut commands: Commands,
	query: Populated<(
		Entity,
		&AttributeOf,
		&AttributeKey,
		Option<&AttributeLit>,
	)>,
) {
	for (entity, parent, key, value) in query.iter() {
		if let Some(directive) = T::try_from_attribute(key, value) {
			commands.entity(**parent).insert(directive);
			commands.entity(entity).despawn();
		}
	}
}
