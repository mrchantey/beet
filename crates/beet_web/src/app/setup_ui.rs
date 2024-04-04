use crate::prelude::*;
use beet::prelude::*;
use bevy::prelude::*;
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

pub fn setup_ui(app: Arc<RwLock<App>>) {
	let send_beet = app.read().world().resource::<BeetMessageSend>().0.clone();
	let send_dom_sim =
		app.read().world().resource::<DomSimMessageSend>().0.clone();

	let send_dom_sim2 = send_dom_sim.clone();
	let despawn_all = move || {
		send_dom_sim2.send(DomSimMessage::DespawnAll).ok();
	};

	let send_beet2 = send_beet.clone();
	message_button("#create-bee", move || {
		send_beet2
			.send(
				BeetMessage::spawn_bundle::<CoreModule>(bee_bundle()).unwrap(),
			)
			.ok();
	});
	message_button("#create-flower", move || {
		send_beet
			.send(
				BeetMessage::spawn_bundle::<CoreModule>(flower_bundle())
					.unwrap(),
			)
			.ok();
	});
	message_button("#clear-all", despawn_all);
	download_button(app.clone());
	upload_button(app);

	ResizeListener::new(&render_container(), move |_e| {
		send_dom_sim.send(DomSimMessage::Resize).ok();
	})
	.forget();
}

fn download_button(app: Arc<RwLock<App>>) {
	let target =
		Document::x_query_selector::<HtmlButtonElement>("#download").unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			download_scene::<CoreModule>(&app.read().world())
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
				let Some(scene) = upload_scene::<CoreModule>()
					.await
					.ok_or(|e| log::error!("{e}",))
				else {
					return;
				};

				let mut app = app.write();
				let mut world = app.world_mut();
				clear_world_with_dom_renderer(&mut world);
				scene.write(&mut world).ok_or(|e| log::error!("{e}"));
			})
		},
		target,
	)
	.forget();
}

fn message_button(selector: &str, func: impl 'static + Fn()) {
	let target =
		Document::x_query_selector::<HtmlButtonElement>(selector).unwrap();

	HtmlEventListener::new_with_target("click", move |_: Event| func(), target)
		.forget();
}
