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
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use wasm_bindgen_futures::spawn_local;
// use wasm_bindgen::JsCast;
use web_sys::Document;
use web_sys::HtmlDivElement;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsciiObject {
	pub text: String,
	pub position: Vec2,
}


// TODO parent window choice
pub fn run() -> Result<()> {
	console_error_panic_hook::set_once();
	console_log::init_with_level(log::Level::Info).ok();

	let container = create_container();


	let relay = Relay::default();
	let mut post_message =
		PostMessageRelay::new_with_current_window(relay.clone());


	let rx = relay.add_subscriber::<AsciiObject>(Topic::new(
		"entity",
		TopicScheme::PubSub,
		TopicMethod::Create,
	));


	spawn_local(async {
		let current_frame = AtomicU64::new(0);

		let _frame = AnimationFrame::new(move || {
			let frame = current_frame.fetch_add(1, Ordering::SeqCst);
			let div = Document::x_create_div();
			div.set_inner_text(&format!("frame {frame}"));
			// HtmlElement::x_query_selector();
			container.append_child(&div).unwrap();

			log::info!("Hello from frame {frame}");
			post_message.send_all().ok_or(|e| log::error!("{e}"));
		});

		loop {
			wait_for_16_millis().await;
		}
	});




	Ok(())
}

fn create_container() -> HtmlDivElement {
	let container = Document::x_create_div();
	container.set_class_name("container");
	Document::x_append_child(&container);
	container
}
