use crate::prelude::*;
use anyhow::Result;
use flume::Receiver;
use forky_core::ResultTEExt;
use forky_web::HtmlEventListener;
use forky_web::ResultTJsValueExt;
use js_sys::ArrayBuffer;
use js_sys::JsString;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::BinaryType;
use web_sys::MessageEvent;
use web_sys::WebSocket;


/// Can receive binary or json messages, sends as binary.
pub struct WebWsClient {
	ws: WebSocket,
	recv: Receiver<Vec<Message>>,
	#[allow(unused)] // dropping this deregisters the listener
	listener: HtmlEventListener<MessageEvent>,
}
impl WebWsClient {
	pub fn new(url: &str) -> Self {
		let ws = WebSocket::new(url).anyhow().unwrap();
		ws.set_binary_type(BinaryType::Arraybuffer);

		let (send, recv) = flume::unbounded();

		let listener = HtmlEventListener::new_with_target(
			"message",
			move |e: MessageEvent| {
				if let Some(messages) = js_value_to_messages(&e.data())
					.ok_or(|e| log::error!("{e}"))
				{
					send.send(messages).ok_or(|e| log::error!("{e}"));
				}
			},
			ws.clone(),
		);
		Self { ws, recv, listener }
	}
}

impl Transport for WebWsClient {
	fn send(&mut self, messages: &Vec<Message>) -> Result<()> {
		let bytes = Message::vec_into_bytes(messages)?;
		self.ws.send_with_u8_array(&bytes).anyhow()
	}

	fn recv(&mut self) -> Result<Vec<Message>> { self.recv.try_recv_all_flat() }
}

impl Drop for WebWsClient {
	fn drop(&mut self) {
		self.ws
			.close_with_code_and_reason(3000, "Client dropped")
			.anyhow()
			.ok_or(|e| log::error!("{e}"));
	}
}

/// Converts the [`MessageEvent::data`] field into a vec of bytes.
/// If the data is a string, it will be converted to bytes using `serde_json`.
pub fn js_value_to_messages(data: &JsValue) -> Result<Vec<Message>> {
	if let Some(array_buffer) = data.dyn_ref::<ArrayBuffer>() {
		let array = Uint8Array::new(&array_buffer);
		let bytes = array.to_vec();
		let messages = Message::vec_from_bytes(&bytes)?;
		Ok(messages)
	} else if let Some(str) = data.dyn_ref::<JsString>() {
		// #[allow(unused_variables)]
		#[cfg(feature = "serde_json")]
		return Ok(Message::vec_from_json(&str.as_string().unwrap())?);
		#[cfg(not(feature = "serde_json"))]
		anyhow::bail!(
			"received string but `serde_json` feature is not enabled\n{str}"
		)
	} else {
		anyhow::bail!(
			"received unknown message type: {}",
			data.js_typeof()
				.as_string()
				.unwrap_or("no type".to_string())
		)
	}
}
