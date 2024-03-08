use crate::prelude::get_container;
use crate::prelude::BeeGame;
use anyhow::Result;
use beet::prelude::*;
use bevy_math::Vec3;
use forky_core::ResultTEExt;
use forky_web::DocumentExt;
use forky_web::HtmlEventListener;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlButtonElement;

#[must_use]
pub fn setup_ui(relay: Relay) -> Result<Vec<HtmlEventListener<Event>>> {
	let create_bee_button = create_button("Create Bee");
	let create_flower_button = create_button("Create Flower");
	let clear_all_button = create_button("Clear");

	let graph = BehaviorTree::new(Translate::new(Vec3::new(-0.1, 0., 0.)))
		.into_action_graph();

	let mut relay2 = relay.clone();
	let mut relay3 = relay.clone();
	let relay4 = relay.clone();
	let create_bee_listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			BeeGame::create_bee_pub(&mut relay2)
				.push(&graph)
				.ok_or(|e| log::error!("{e}"));
		},
		create_bee_button.into(),
	);
	let create_flower_listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			BeeGame::create_flower_pub(&mut relay3)
				.push(&())
				.ok_or(|e| log::error!("{e}"));
		},
		create_flower_button.into(),
	);
	let clear_all_listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			let mut relay4 = relay4.clone();
			spawn_local(async move {
				log::info!("clearing all");
				DespawnEntityHandler::publisher(&mut relay4)
					.push(&DespawnEntityPayload::all())
					.ok_or(|e| log::error!("{e}"));
			});
		},
		clear_all_button.into(),
	);

	Ok(vec![
		create_bee_listener,
		create_flower_listener,
		clear_all_listener,
	])
}

fn create_button(text: &str) -> HtmlButtonElement {
	let container = get_container();
	let button = Document::x_create_button();
	// button.set_class_name("button");
	button.set_inner_text(text);
	container.append_child(&button).unwrap();
	// Document::x_append_child(&button);
	button
}
