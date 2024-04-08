use crate::prelude::*;
use beet::core_module::RenderText;
use bevy::prelude::*;

pub fn update_dom_text(
	renderer: NonSend<DomRenderer>,
	query: Query<
		(Entity, &RenderText),
		(
			With<HasElement>,
			Or<(Changed<RenderText>, Added<HasElement>)>,
		),
	>,
) {
	for (entity, text) in query.iter() {
		if let Some(element) = renderer.elements.get(&entity) {
			element.set_inner_text(text);
		}
	}
}
