use anyhow::Result;
use forky_web::wait_for_16_millis;
use forky_web::AnimationFrame;
use forky_web::DocumentExt;
use forky_web::HtmlElementExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
// use wasm_bindgen::JsCast;
use web_sys::window;
use web_sys::Document;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;




pub fn run() -> Result<()> {
	console_error_panic_hook::set_once();
	console_log::init_with_level(log::Level::Info).unwrap();

	let container = create_container();

	let relay = Relay::new();


	spawn_local(async {
		let current_frame = AtomicU64::new(0);

		let _frame = AnimationFrame::new(move || {
			let frame = current_frame.fetch_add(1, Ordering::SeqCst);
			let div = Document::x_create_div();
			div.set_inner_text(&format!("frame {frame}"));
			// HtmlElement::x_query_selector();
			container.append_child(&div).unwrap();

			log::info!("Hello from frame {frame}");
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
