use crate::prelude::BeeGame;
use crate::prelude::BeeNode;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use beet::prelude::*;
use bevy_math::Vec3;
use forky_core::utility::random_signed;
use forky_core::utility::random_value;
use forky_core::ResultTEExt;
use forky_web::DocumentExt;
use forky_web::History;
use forky_web::HtmlEventListener;
use forky_web::SearchParams;
use js_sys::JSON;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlButtonElement;
use web_sys::HtmlDivElement;
use web_sys::HtmlTextAreaElement;

#[must_use]
pub fn setup_ui(relay: Relay) -> Result<Vec<HtmlEventListener<Event>>> {
	let create_bee_button =
		Document::x_query_selector::<HtmlButtonElement>("#create-bee").unwrap();
	let create_flower_listener = create_flower(relay.clone());
	let clear_all_listener = create_clear_all(relay.clone());
	let toggle_json = create_toggle_json();

	let (textarea, text_changed_listener) =
		create_text(create_bee_button.clone());

	let create_bee_listener = create_bee(relay, create_bee_button, textarea);
	Ok(vec![
		toggle_json,
		text_changed_listener,
		create_bee_listener,
		create_flower_listener,
		clear_all_listener,
	])
}




fn create_clear_all(relay: Relay) -> HtmlEventListener<Event> {
	let clear_all_button =
		Document::x_query_selector::<HtmlButtonElement>("#clear-all").unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_| {
			let mut relay = relay.clone();
			spawn_local(async move {
				DespawnEntityHandler::publisher(&mut relay)
					.push(&DespawnEntityPayload::all())
					.ok_or(|e| log::error!("{e}"));
			});
		},
		clear_all_button.into(),
	)
}


fn create_bee(
	mut relay: Relay,
	button: HtmlButtonElement,
	textarea: HtmlTextAreaElement,
) -> HtmlEventListener<Event> {
	let listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			let tree: BehaviorTree<BeeNode> =
				serde_json::from_str(&textarea.value()).unwrap(); // already validated

			BeeGame::create_bee_pub(&mut relay)
				.push(&tree.into_behavior_graph())
				.ok_or(|e| log::error!("{e}"));
		},
		button.clone().into(),
	);
	if SearchParams::get_flag("spawn-bee") {
		button.click();
	}

	listener
}

fn create_flower(mut relay: Relay) -> HtmlEventListener<Event> {
	let button =
		Document::x_query_selector::<HtmlButtonElement>("#create-flower")
			.unwrap();

	let listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			let x = random_signed() * 0.9;
			let y = random_value() * -0.9;

			BeeGame::create_flower_pub(&mut relay)
				.push(&Vec3::new(x, y, 0.))
				.ok_or(|e| log::error!("{e}"));
		},
		button.clone().into(),
	);

	if SearchParams::get_flag("spawn-flower") {
		button.click();
	}

	listener
}



fn create_toggle_json() -> HtmlEventListener<Event> {
	let button =
		Document::x_query_selector::<HtmlButtonElement>("#toggle-json")
			.unwrap();
	let container =
		Document::x_query_selector::<HtmlDivElement>("#graph-json-container")
			.unwrap();
	let toggle_json_button2 = button.clone();
	let listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			if container.hidden() {
				container.set_hidden(false);
				toggle_json_button2.set_inner_text("Hide Graph");
			} else {
				container.set_hidden(true);
				toggle_json_button2.set_inner_text("Show Graph");
			}
		},
		button.clone().into(),
	);

	if SearchParams::get_flag("hide-graph") {
		button.click();
	}
	listener
}

fn create_text(
	create_bee_button: HtmlButtonElement,
) -> (HtmlTextAreaElement, HtmlEventListener<Event>) {
	let initial = get_tree_url_param().unwrap_or_else(|_| {
		BehaviorTree::<BeeNode>::new(Translate::new(Vec3::new(-0.1, 0., 0.)))
	});

	let warning_text =
		Document::x_query_selector::<HtmlDivElement>("#graph-json-error")
			.unwrap();
	let textarea =
		Document::x_query_selector::<HtmlTextAreaElement>("#graph-json-text")
			.unwrap();
	textarea.set_value(&prettify(&initial));

	let textarea2 = textarea.clone();
	let text_changed_listener = HtmlEventListener::new_with_target(
		"input",
		move |_| {
			let text = textarea2.value();
			match serde_json::from_str::<BehaviorTree<BeeNode>>(&text) {
				Ok(tree) => {
					create_bee_button.set_disabled(false);
					warning_text.set_inner_html("&nbsp;");
					// textarea2.set_value(&prettify(&tree));
					set_url(&tree);
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

fn set_url(tre: &BehaviorTree<BeeNode>) {
	let val = bincode::serialize(tre).unwrap();
	let val = general_purpose::STANDARD_NO_PAD.encode(val);
	// let url = serde_json::to_string(tre).unwrap();
	History::set_param("graph", &val);
}

fn get_tree_url_param() -> Result<BehaviorTree<BeeNode>> {
	if let Some(tree) = SearchParams::get("graph") {
		let bytes = general_purpose::STANDARD_NO_PAD.decode(tree.as_bytes())?;
		let tree = bincode::deserialize(&bytes)?;
		Ok(tree)
	} else {
		anyhow::bail!("no tree param found");
	}
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
