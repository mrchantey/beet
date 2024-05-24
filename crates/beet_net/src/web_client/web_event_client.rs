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
/// - listens for `"js-message"`
/// - emits `"wasm-message"`
pub struct WebEventClient {
	target: EventTarget,
	recv: Receiver<Vec<u8>>,
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
				if let Some(bytes) =
					js_value_to_bytes(&e.detail()).ok_or(|e| log::error!("{e}"))
				{
					send.send(bytes).ok_or(|e| log::error!("{e}"));
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
	async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
		let json = Message::bytes_to_json(&bytes)?;
		let mut init = CustomEventInit::new();
		init.detail(&JsValue::from_str(&json));
		let event =
			CustomEvent::new_with_event_init_dict("wasm-message", &init)
				.anyhow()?;
		self.target.dispatch_event(&event).anyhow()?;
		Ok(())
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>> {
		self.recv.try_recv_all()
	}
}
