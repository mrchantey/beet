use crate::prelude::*;
use anyhow::Result;
use flume::Receiver;
use forky_core::ResultTEExt;
use forky_web::HtmlEventListener;
use forky_web::ResultTJsValueExt;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::CustomEvent;
use web_sys::CustomEventInit;
use web_sys::EventTarget;

/// The [`WebEventClient`] can be used on any [`EventTarget`].
/// Can receive binary or json messages.
/// Sends json messages.
/// - listens for `"js-message"`
/// - emits `"wasm-message"`
pub struct WebEventClient {
	target: EventTarget,
	recv: Receiver<Vec<Message>>,
	#[allow(unused)] // dropping this deregisters the listener
	listener: HtmlEventListener<CustomEvent>,
}
impl WebEventClient {
	pub fn new_with_window() -> Self {
		Self::new(web_sys::window().unwrap().dyn_into().unwrap())
	}

	pub fn new(target: EventTarget) -> Self {
		let (send, recv) = flume::unbounded();

		let listener = HtmlEventListener::new_with_target(
			"js-message",
			move |e: CustomEvent| {
				if let Some(messags) = js_value_to_messages(&e.detail())
					.ok_or(|e| log::error!("{e}"))
				{
					send.send(messags).ok_or(|e| log::error!("{e}"));
				}
			},
			target.clone(),
		);
		Self {
			target,
			recv,
			listener,
		}
	}
}

impl Transport for WebEventClient {
	fn send(&mut self, messages: &Vec<Message>) -> Result<()> {
		let json = Message::vec_into_json(messages)?;
		let mut init = CustomEventInit::new();
		init.detail(&JsValue::from_str(&json));
		let event =
			CustomEvent::new_with_event_init_dict("wasm-message", &init)
				.anyhow()?;
		self.target.dispatch_event(&event).anyhow()?;
		Ok(())
	}

	fn recv(&mut self) -> Result<Vec<Message>> { self.recv.try_recv_all_flat() }
}
