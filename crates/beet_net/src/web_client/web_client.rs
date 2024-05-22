use crate::prelude::*;
use anyhow::Result;
use flume::Receiver;
use forky_core::ResultTEExt;
use forky_web::HtmlEventListener;
use forky_web::ResultTJsValueExt;
use js_sys::ArrayBuffer;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use web_sys::BinaryType;
use web_sys::MessageEvent;
use web_sys::WebSocket;

// TODO move to coora_core.

// #[derive(Clone)]
// cannot be clone because it disconnects on drop which is cool
pub struct WebClient {
	ws: WebSocket,
	recv: Receiver<Vec<u8>>,
	/// dropping this deregisters the listener
	#[allow(unused)]
	listener: HtmlEventListener<MessageEvent>,
}
impl WebClient {
	pub fn new(url: &str) -> Result<Self> {
		let ws = WebSocket::new(url).anyhow()?;
		ws.set_binary_type(BinaryType::Arraybuffer);

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
			ws.clone(),
		);
		Ok(Self { ws, recv, listener })
	}
}

impl Transport for WebClient {
	async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
		self.ws.send_with_u8_array(&bytes).anyhow()
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>> {
		self.recv.try_recv_all()
	}
}

impl Drop for WebClient {
	fn drop(&mut self) {
		self.ws
			.close_with_code_and_reason(3000, "Client dropped")
			.anyhow()
			.ok_or(|e| log::error!("{e}"));
	}
}


fn message_event_to_bytes(event: MessageEvent) -> Result<Vec<u8>> {
	let data = event.data();
	let array_buffer = data.dyn_into::<ArrayBuffer>().anyhow()?;
	let array = Uint8Array::new(&array_buffer);
	let bytes = array.to_vec();
	Ok(bytes)
}
