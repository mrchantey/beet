use super::post_message_relay::PostMessageRelay;
use anyhow::Result;
use beet::exports::Deserialize;
use beet::exports::Serialize;
use beet::prelude::*;
use bevy_math::prelude::*;
use forky_core::ResultTEExt;
use forky_web::wait_for_16_millis;
use forky_web::AnimationFrame;
use forky_web::DocumentExt;
use forky_web::HtmlEventListener;
use wasm_bindgen_futures::spawn_local;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlButtonElement;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsciiObject {
	pub text: String,
	pub position: Vec2,
}

pub async fn start() -> Result<()> {
	let container = create_container();
	let play_button = play_button(&container);

	append_style_to_head();

	let relay = Relay::default();
	let mut post_message =
		PostMessageRelay::new_with_current_window(relay.clone());

	let _play_listener =
		spawn_entity_requester(&relay, play_button.clone()).await?;
	spawn_entity_responder(&relay, container.clone()).await?;
	// let current_frame = IdIncr::new();

	let _frame = AnimationFrame::new(move || {
		post_message.send_all().ok_or(|e| log::error!("{e}"));
	});
	loop {
		wait_for_16_millis().await;
	}
}


// TODO parent window choice
pub fn run() -> Result<()> {
	spawn_local(async move {
		start().await.ok_or(|e| log::error!("{e}"));
	});

	Ok(())
}

fn create_container() -> HtmlDivElement {
	let container = Document::x_create_div();
	container.set_class_name("container");
	Document::x_append_child(&container);
	container
}

fn play_button(container: &HtmlDivElement) -> HtmlButtonElement {
	let button = Document::x_create_button();
	button.set_class_name("play");
	button.set_inner_text("▶");
	container.append_child(&button).unwrap();
	// Document::x_append_child(&button);
	button
}




async fn spawn_entity_responder(
	relay: &Relay,
	container: HtmlDivElement,
) -> Result<()> {
	let id = IdIncr::new();

	let mut rx = relay
		.add_responder::<AsciiObject, u64>("entity", TopicMethod::Create)
		.await?;

	spawn_local(async move {
		rx.handle_requests_forever(|obj| {
			let id = id.next();
			let div = Document::x_create_div();
			div.set_inner_text(&obj.text);
			div.set_class_name("entity");
			set_position(&*div, obj.position, &container);

			container.append_child(&div).unwrap();
			id
		})
		.await
		.ok_or(|e| log::error!("{e}"));
	});

	Ok(())
}

#[must_use]
async fn spawn_entity_requester(
	relay: &Relay,
	button: HtmlButtonElement,
) -> Result<HtmlEventListener<Event>> {
	let tx = relay
		.add_requester::<AsciiObject, u64>("entity", TopicMethod::Create)
		.await?;

	let event_listener = HtmlEventListener::new_with_target(
		"click",
		move |_| {
			let obj = AsciiObject {
				text: "🐉".to_string(),
				position: Vec2::new(0.0, 0.0),
			};
			let tx = tx.clone();
			spawn_local(async move {
				tx.clone().request(&obj).await.ok_or(|e| log::error!("{e}"));
			});
		},
		button.into(),
	);

	Ok(event_listener)
}


fn set_position<'a>(
	el: &HtmlElement,
	position: Vec2,
	container: &HtmlDivElement,
) {
	let container_width = container.client_width() as f32;
	let container_height = container.client_height() as f32;
	let child_width = el.client_width() as f32;
	let child_height = el.client_height() as f32;


	let left = (container_width / 2.0) + (position.x * (container_width / 2.0))
		- (child_width / 2.0);
	let top = (container_height / 2.0)
		+ (position.y * (container_height / 2.0))
		- (child_height / 2.0);

	el.set_attribute("style", &format!("left: {}px; top: {}px;", left, top))
		.unwrap();
}

fn append_style_to_head() {
	let style_content = include_str!("style.css");
	let style_element = Document::x_create_element("style");
	style_element.set_inner_html(style_content);
	Document::x_head().append_child(&style_element).unwrap();
}
