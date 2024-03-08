use crate::prelude::get_container;
use crate::prelude::BeeGame;
use crate::prelude::BeeNode;
use anyhow::Result;
use beet::prelude::*;
use bevy_math::Vec3;
use forky_core::ResultTEExt;
use forky_web::DocumentExt;
use forky_web::HtmlEventListener;
use js_sys::JSON;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlButtonElement;
use web_sys::HtmlDivElement;
use web_sys::HtmlTextAreaElement;

#[must_use]
pub fn setup_ui(relay: Relay) -> Result<Vec<HtmlEventListener<Event>>> {
	let create_bee_button = create_button("Create Bee");
	let create_flower_listener = create_flower(relay.clone());
	let clear_all_listener = create_clear_all(relay.clone());

	let (textarea, text_changed_listener) =
		create_text(create_bee_button.clone());

	let mut relay = relay;
	let create_bee_listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			let tree: BehaviorTree<BeeNode> =
				serde_json::from_str(&textarea.value()).unwrap(); // already validated
			let graph = tree.into_action_graph();

			BeeGame::create_bee_pub(&mut relay)
				.push(&graph)
				.ok_or(|e| log::error!("{e}"));
		},
		create_bee_button.clone().into(),
	);

	Ok(vec![
		text_changed_listener,
		create_bee_listener,
		create_flower_listener,
		clear_all_listener,
	])
}

fn create_clear_all(relay: Relay) -> HtmlEventListener<Event> {
	let clear_all_button = create_button("Clear");
	HtmlEventListener::new_with_target(
		"click",
		move |_| {
			let mut relay = relay.clone();
			spawn_local(async move {
				log::info!("clearing all");
				DespawnEntityHandler::publisher(&mut relay)
					.push(&DespawnEntityPayload::all())
					.ok_or(|e| log::error!("{e}"));
			});
		},
		clear_all_button.into(),
	)
}
fn create_flower(mut relay: Relay) -> HtmlEventListener<Event> {
	let create_flower_button = create_button("Create Flower");
	HtmlEventListener::new_with_target(
		"click",
		move |_| {
			BeeGame::create_flower_pub(&mut relay)
				.push(&())
				.ok_or(|e| log::error!("{e}"));
		},
		create_flower_button.into(),
	)
}


fn create_text(
	create_bee_button: HtmlButtonElement,
) -> (HtmlTextAreaElement, HtmlEventListener<Event>) {
	let initial =
		BehaviorTree::<BeeNode>::new(Translate::new(Vec3::new(-0.1, 0., 0.)));

	let warning_text = create_warning_div();
	let textarea = create_textarea();
	textarea.set_value(&prettify(&initial));

	let textarea2 = textarea.clone();
	let text_changed_listener = HtmlEventListener::new_with_target(
		"change",
		move |_| {
			let text = textarea2.value();
			match serde_json::from_str::<BehaviorTree<BeeNode>>(&text) {
				Ok(tree) => {
					create_bee_button.set_disabled(false);
					warning_text.set_inner_text(LOOKING_GOOD);
					textarea2.set_value(&prettify(&tree));

				}
				Err(e) => {
					create_bee_button.set_disabled(true);
					warning_text.set_inner_text(&format!("Error: {}", e));
				}
			}
		},
		textarea.clone().into(),
	);
	(textarea, text_changed_listener)
}

fn create_textarea() -> HtmlTextAreaElement {
	let container = get_container();
	let textarea: HtmlTextAreaElement = Document::get()
		.create_element("textarea")
		.unwrap()
		.dyn_into()
		.unwrap();
	container.append_child(&textarea).unwrap();
	textarea
}
const LOOKING_GOOD: &str = "Looking good!";
fn create_warning_div() -> HtmlDivElement {
	let container = get_container();
	let el = Document::x_create_div();
	el.set_inner_text(LOOKING_GOOD);
	container.append_child(&el).unwrap();
	el
}

fn prettify(tree: &BehaviorTree<BeeNode>) -> String {
	let tree = serde_json::to_string(&tree).unwrap();
	let parsed = JSON::parse(&tree).unwrap();
	let pretty = JSON::stringify_with_replacer_and_space(
		&parsed,
		&JsValue::NULL,
		&JsValue::from_f64(2.),
	)
	.unwrap();
	pretty.as_string().unwrap()
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
