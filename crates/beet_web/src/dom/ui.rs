use crate::prelude::AppOptions;
use crate::prelude::BeeNode;
use crate::prelude::CreateBeeHandler;
use crate::prelude::CreateFlowerHandler;
use anyhow::Result;
use base64::engine::general_purpose;
use base64::Engine;
use beet::prelude::*;
use bevy::prelude::*;
use forky_core::utility::random_signed;
use forky_core::utility::random_value;
use forky_core::ResultTEExt;
use forky_web::DocumentExt;
use forky_web::History;
use forky_web::HtmlEventListener;
use forky_web::Interval;
use js_sys::JSON;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlButtonElement;
use web_sys::HtmlDivElement;
use web_sys::HtmlTextAreaElement;


#[must_use]
pub fn setup_ui(relay: Relay, options: &AppOptions) -> Result<()> {
	let create_bee_button =
		Document::x_query_selector::<HtmlButtonElement>("#create-bee").unwrap();
	create_flower(relay.clone(), options);
	create_clear_all(relay.clone());
	create_toggle_json(options);

	let textarea = create_textarea(create_bee_button.clone(), options);

	create_bee(relay, create_bee_button, textarea, options);
	Ok(())
}




fn create_clear_all(relay: Relay) {
	let clear_all_button =
		Document::x_query_selector::<HtmlButtonElement>("#clear-all").unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			let mut relay = relay.clone();
			spawn_local(async move {
				DespawnEntityHandler::publisher(&mut relay)
					.unwrap()
					.push(&DespawnEntityPayload::all())
					.ok_or(|e| log::error!("{e}"));
			});
		},
		clear_all_button,
	)
	.forget();
}


fn create_bee(
	mut relay: Relay,
	button: HtmlButtonElement,
	textarea: HtmlTextAreaElement,
	options: &AppOptions,
) {
	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			let prefab: TypedBehaviorPrefab<BeeNode> =
				serde_json::from_str(&textarea.value()).unwrap(); // already validated

			CreateBeeHandler::publisher(&mut relay)
				.unwrap()
				.push(&prefab)
				.ok_or(|e| log::error!("{e}"));
		},
		button.clone(),
	)
	.forget();
	for _ in 0..options.bees {
		button.click();
	}
}

fn create_flower(mut relay: Relay, options: &AppOptions) {
	let button =
		Document::x_query_selector::<HtmlButtonElement>("#create-flower")
			.unwrap();

	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			let x = random_signed() * 0.9;
			let y = random_value() * -0.9;

			CreateFlowerHandler::publisher(&mut relay)
				.unwrap()
				.push(&Vec3::new(x, y, 0.))
				.ok_or(|e| log::error!("{e}"));
		},
		button.clone(),
	)
	.forget();

	for _ in 0..options.flowers {
		button.click();
	}

	if let Some(interval) = options.auto_flowers {
		Interval::new(interval as i32, move || {
			button.click();
		})
		.forget();
	}
}

fn create_toggle_json(options: &AppOptions) {
	let button =
		Document::x_query_selector::<HtmlButtonElement>("#toggle-json")
			.unwrap();
	let container =
		Document::x_query_selector::<HtmlDivElement>("#graph-json-container")
			.unwrap();
	let toggle_json_button2 = button.clone();
	HtmlEventListener::new_with_target(
		"click",
		move |_: Event| {
			if container.hidden() {
				container.set_hidden(false);
				toggle_json_button2.set_inner_text("Hide Graph");
			} else {
				container.set_hidden(true);
				toggle_json_button2.set_inner_text("Show Graph");
			}
		},
		button.clone(),
	)
	.forget();

	if options.hide_json {
		button.click();
	}
}

fn create_textarea(
	create_bee_button: HtmlButtonElement,
	options: &AppOptions,
) -> HtmlTextAreaElement {
	let warning_text =
		Document::x_query_selector::<HtmlDivElement>("#graph-json-error")
			.unwrap();
	let textarea =
		Document::x_query_selector::<HtmlTextAreaElement>("#graph-json-text")
			.unwrap();
	textarea.set_value(&prettify(&options.initial_prefab));

	let textarea2 = textarea.clone();
	HtmlEventListener::new_with_target(
		"input",
		move |_: Event| {
			let text = textarea2.value();
			match serde_json::from_str::<TypedBehaviorPrefab<BeeNode>>(&text) {
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
		textarea.clone(),
	)
	.forget();
	textarea
}

fn set_url(prefab: &TypedBehaviorPrefab<BeeNode>) {
	let val = bincode::serialize(prefab).unwrap();
	let val = general_purpose::STANDARD_NO_PAD.encode(val);
	// let url = serde_json::to_string(tre).unwrap();
	History::set_param("graph", &val);
}




fn prettify(prefab: &TypedBehaviorPrefab<BeeNode>) -> String {
	let tree = serde_json::to_string(&prefab).unwrap();
	let parsed = JSON::parse(&tree).unwrap();
	let pretty = JSON::stringify_with_replacer_and_space(
		&parsed,
		&JsValue::NULL,
		&JsValue::from_f64(4.),
	)
	.unwrap();
	pretty.as_string().unwrap()
}
