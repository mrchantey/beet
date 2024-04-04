use crate::prelude::*;
use bevy::app::App;
use forky_web::DocumentExt;
use forky_web::HtmlEventListener;
use parking_lot::RwLock;
use std::sync::Arc;
use web_sys::Document;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;
use web_sys::KeyboardEvent;


pub fn init_test_app(app: Arc<RwLock<App>>) {
	setup_ui(app.clone());
	let container: &HtmlElement = &render_container();
	app.write()
		.insert_non_send_resource(DomRenderer::new(container.clone()));
	// test_container_listener(app.clone());
}

pub fn test_container_listener(app: Arc<RwLock<App>>) {
	HtmlEventListener::new("keydown", move |_event: KeyboardEvent| {
		if let Some(el) =
			Document::x_query_selector::<HtmlDivElement>(".dom-sim-container")
		{
			el.remove();
			remove_renderer(&mut app.write().world_mut());
		} else {
			let root =
				Document::x_query_selector::<HtmlDivElement>(".container")
					.unwrap();
			let container = Document::x_create_div();
			container.set_class_name("dom-sim-container");
			root.prepend_with_node_1(&container).unwrap();

			add_renderer(&mut app.write().world_mut(), &container);
		}
	})
	.forget();
}
