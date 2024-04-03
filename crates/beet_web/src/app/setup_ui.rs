use super::dom_renderer::clear_world_with_dom_renderer;
use super::scene_io::download_scene;
use super::scene_io::upload_scene;
use super::spawn::DomSimMessage;
use crate::prelude::BeetWebNode;
use bevy::prelude::*;
use flume::Sender;
use forky_core::ResultTEExt;
use forky_web::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlButtonElement;
use web_sys::HtmlDivElement;



pub fn render_container() -> HtmlDivElement {
	Document::x_query_selector::<HtmlDivElement>(".dom-sim-container").unwrap()
}

pub fn setup_ui(send: Sender<DomSimMessage>, app: Arc<RwLock<App>>) {
	message_button(
		send.clone(),
		"#create-bee",
		DomSimMessage::SpawnBeeFromFirstNode,
	);
	message_button(send.clone(), "#create-flower", DomSimMessage::SpawnFlower);
	message_button(send.clone(), "#clear-all", DomSimMessage::DespawnAll);
	download_button(app.clone());
	upload_button(app);

	ResizeListener::new(&render_container(), move |_e| {
		send.send(DomSimMessage::Resize).ok();
	})
	.forget();
}

fn download_button(app: Arc<RwLock<App>>) {
	let target =
		Document::x_query_selector::<HtmlButtonElement>("#download").unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			download_scene::<BeetWebNode>(&app.read().world())
				.ok_or(|e| log::error!("{e}"));
		},
		target,
	)
	.forget();
}
fn upload_button(app: Arc<RwLock<App>>) {
	let target =
		Document::x_query_selector::<HtmlButtonElement>("#upload").unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			let app = app.clone();
			spawn_local(async move {
				let Some(scene) = upload_scene::<BeetWebNode>()
					.await
					.ok_or(|e| log::error!("{e}",))
				else {
					return;
				};

				let mut app = app.write();
				let mut world = app.world_mut();
				clear_world_with_dom_renderer(&mut world);
				scene
					.scene
					.write_to_world(&mut world, &mut Default::default())
					.ok_or(|e| log::error!("{e}"));
			})
		},
		target,
	)
	.forget();
}

fn message_button(
	send: Sender<DomSimMessage>,
	selector: &str,
	message: DomSimMessage,
) {
	let target =
		Document::x_query_selector::<HtmlButtonElement>(selector).unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			send.send(message.clone()).ok();
		},
		target,
	)
	.forget();
}
