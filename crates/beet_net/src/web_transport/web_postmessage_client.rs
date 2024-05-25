use crate::prelude::*;
use anyhow::Result;
use flume::Receiver;
use forky_core::ResultTEExt;
use forky_web::HtmlEventListener;
use forky_web::ResultTJsValueExt;
use wasm_bindgen::JsValue;
use web_sys::MessageEvent;
use web_sys::Window;

pub struct WebPostmessageClient {
	target: Window,
	recv: Receiver<Vec<Message>>,
	#[allow(unused)] // dropping this deregisters the listener
	listener: HtmlEventListener<MessageEvent>,
}
impl WebPostmessageClient {
	pub fn new() -> Self { Self::new_with(web_sys::window().unwrap()) }
	pub fn new_with(target: Window) -> Self {
		let (send, recv) = flume::unbounded();

		let listener = HtmlEventListener::new_with_target(
			"message",
			move |e: MessageEvent| {
				if let Some(msg) = js_value_to_messages(&e.data())
					.ok_or(|e| log::error!("{e}"))
				{
					send.send(msg).ok_or(|e| log::error!("{e}"));
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

impl Transport for WebPostmessageClient {
	fn send(&mut self, messages: &Vec<Message>) -> Result<()> {
		let json = Message::vec_into_json(messages)?;
		self.target
			.post_message(&JsValue::from_str(&json), "*")
			.anyhow()?;
		Ok(())
	}

	fn recv(&mut self) -> Result<Vec<Message>> { self.recv.try_recv_all_flat() }
}
