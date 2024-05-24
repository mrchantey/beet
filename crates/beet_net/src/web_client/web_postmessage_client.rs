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
	recv: Receiver<Vec<u8>>,
	#[allow(unused)] // dropping this deregisters the listener
	listener: HtmlEventListener<MessageEvent>,
}
impl WebPostmessageClient {
	pub fn new() -> Result<Self> {
		Self::new_with_window(web_sys::window().unwrap())
	}
	pub fn new_with_window(target: Window) -> Result<Self> {
		let (send, recv) = flume::unbounded();

		let listener = HtmlEventListener::new_with_target(
			"message",
			move |e: MessageEvent| {
				if let Some(bytes) =
					message_event_to_bytes(e).ok_or(|e| log::error!("{e}"))
				{
					send.send(bytes).ok_or(|e| log::error!("{e}"));
				}
			},
			target.clone(),
		);
		Ok(Self {
			target,
			recv,
			listener,
		})
	}
}

impl Transport for WebPostmessageClient {
	async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
		let json = Message::bytes_to_json(&bytes)?;
		self.target
			.post_message(&JsValue::from_str(&json), "*")
			.anyhow()?;
		Ok(())
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>> {
		self.recv.try_recv_all()
	}
}
