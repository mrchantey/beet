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

pub struct WebWsClient {
	ws: WebSocket,
	recv: Receiver<Vec<u8>>,
	#[allow(unused)] // dropping this deregisters the listener
	listener: HtmlEventListener<MessageEvent>,
}
impl WebWsClient {
	pub fn new(url: &str) -> Result<Self> {
		let ws = WebSocket::new(url).anyhow()?;
		ws.set_binary_type(BinaryType::Arraybuffer);

		let (send, recv) = flume::unbounded();

		let listener = HtmlEventListener::new_with_target(
			"message",
			move |e: MessageEvent| {
				if let Some(bytes) =
					js_value_to_bytes(&e.data()).ok_or(|e| log::error!("{e}"))
				{
					send.send(bytes).ok_or(|e| log::error!("{e}"));
				}
			},
			ws.clone(),
		);
		Ok(Self { ws, recv, listener })
	}
}

impl Transport for WebWsClient {
	async fn send_bytes(&mut self, bytes: Vec<u8>) -> Result<()> {
		self.ws.send_with_u8_array(&bytes).anyhow()
	}

	fn recv_bytes(&mut self) -> Result<Vec<Vec<u8>>> {
		self.recv.try_recv_all()
	}
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
pub fn js_value_to_bytes(data: &JsValue) -> Result<Vec<u8>> {
	if let Some(array_buffer) = data.dyn_ref::<ArrayBuffer>() {
		let array = Uint8Array::new(&array_buffer);
		let bytes = array.to_vec();
		Ok(bytes)
	} else if let Some(str) = data.dyn_ref::<JsString>() {
		// #[allow(unused_variables)]
		#[cfg(feature = "serde_json")]
		return Ok(Message::json_to_bytes(&str.as_string().unwrap())?);
		#[cfg(not(feature = "serde_json"))]
		anyhow::bail!(
			"received string but `serde_json` feature is not enabled\n{str}"
		)
	} else {
		anyhow::bail!("received unknown message type")
	}
}
