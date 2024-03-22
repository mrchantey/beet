use crate::prelude::*;
use bevy::prelude::*;
use web_sys::HtmlElement;




pub fn update_positions(
	elements: NonSend<DomSimElements>,
	query: Query<
		(Entity, &Transform),
		(With<DomSimEntity>, Changed<Transform>),
	>,
) {
	for (entity, transform) in query.iter() {
		if let Some(element) = elements.get(&entity) {
			let position = transform.translation.xy();
			set_position(&element, position);
		}
	}
}

pub fn set_position(el: &HtmlElement, position: Vec2) {
	let parent = get_entities_container();

	let parent_width = parent.client_width() as f32;
	let parent_height = parent.client_height() as f32;
	let child_width = el.client_width() as f32;
	let child_height = el.client_height() as f32;


	let left = (parent_width / 2.0) + (position.x * (parent_width / 2.0))
		- (child_width / 2.0);
	let top = (parent_height / 2.0)
		+ (-1. * position.y * (parent_height / 2.0))// invert y
		- (child_height / 2.0);

	el.set_attribute("style", &format!("left: {}px; top: {}px;", left, top))
		.unwrap();
}
