use crate::prelude::*;
use bevy::prelude::*;



impl TemplateApp {
	pub fn mount(bundle: impl Bundle) {
		let html = bundle_to_html(bundle);
		let document = web_sys::window().unwrap().document().unwrap();
		let body = document.body().unwrap();
		let current_html = body.inner_html();
		body.set_inner_html(&format!("{}{}", current_html, html));
	}
	pub fn mount_with_id(bundle: impl Bundle, id: &str) {
		let html = bundle_to_html(bundle);
		let document = web_sys::window().unwrap().document().unwrap();
		let element = document.get_element_by_id(id).unwrap();
		element.set_inner_html(&html);
	}
}
