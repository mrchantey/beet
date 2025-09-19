use crate::prelude::sockets::*;
use beet_core::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use futures::channel::mpsc;
use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::future::FutureExt;
use futures::future::ready;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::BinaryType;
use web_sys::CloseEvent;
use web_sys::Event;
use web_sys::MessageEvent;
use web_sys::WebSocket;

/// Connect to a WebSocket endpoint in the browser and return a cross-platform `Socket`.
///
/// This function:
/// - Creates a `WebSocket` using `web_sys`
/// - Configures binary frames to arrive as `ArrayBuffer`
/// - Hooks up event listeners to stream incoming messages as `Message`
/// - Awaits the `open` event before returning so the socket is ready to send
pub async fn connect_wasm(url: impl AsRef<str>) -> Result<Socket> {
	let ws = WebSocket::new(url.as_ref()).map_jserr()?;
	ws.set_binary_type(BinaryType::Arraybuffer);

	// Stream of inbound messages
	let (tx, rx) = mpsc::unbounded::<Result<Message>>();

	// onmessage: forward as Message::Text or Message::Binary
	let tx_msg = tx.clone();
	let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
		let data = e.data();
		let res = if let Some(s) = data.as_string() {
			Ok(Message::Text(s))
		} else if data.is_instance_of::<js_sys::ArrayBuffer>() {
			let buf: js_sys::ArrayBuffer =
				match data.dyn_into::<js_sys::ArrayBuffer>() {
					Ok(b) => b,
					Err(_) => {
						let _ = tx_msg.unbounded_send(Err(bevyhow!(
							"Failed to read ArrayBuffer message"
						)));
						return;
					}
				};
			let arr = js_sys::Uint8Array::new(&buf).to_vec();
			Ok(Message::Binary(Bytes::from(arr)))
		} else if data.is_instance_of::<js_sys::Uint8Array>() {
			let arr: js_sys::Uint8Array =
				match data.dyn_into::<js_sys::Uint8Array>() {
					Ok(a) => a,
					Err(_) => {
						let _ = tx_msg.unbounded_send(Err(bevyhow!(
							"Failed to read Uint8Array message"
						)));
						return;
					}
				};
			Ok(Message::Binary(Bytes::from(arr.to_vec())))
		} else {
			Err(bevyhow!(
				"Unsupported WebSocket message type: {:?}",
				data.js_typeof()
			))
		};
		let _ = tx_msg.unbounded_send(res);
	}) as Box<dyn FnMut(MessageEvent)>);
	ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

	// onerror: surface an error into the stream
	let tx_err = tx.clone();
	let on_error = Closure::wrap(Box::new(move |_e: Event| {
		let _ = tx_err.unbounded_send(Err(bevyhow!("WebSocket error event")));
	}) as Box<dyn FnMut(Event)>);
	ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));

	// onclose: translate to a Close frame and close the stream
	let tx_close = tx.clone();
	let on_close = Closure::wrap(Box::new(move |e: CloseEvent| {
		let _ = tx_close.unbounded_send(Ok(Message::Close(Some(CloseFrame {
			code: e.code(),
			reason: e.reason(),
		}))));
		// then mark channel as closed
		tx_close.close_channel();
	}) as Box<dyn FnMut(CloseEvent)>);
	ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

	// Wait until the socket is open before returning
	let (open_tx, open_rx) = oneshot::channel::<()>();
	let open_cell = std::cell::RefCell::new(Some(open_tx));
	let on_open = Closure::wrap(Box::new(move |_e: Event| {
		if let Some(tx) = open_cell.borrow_mut().take() {
			let _ = tx.send(());
		}
	}) as Box<dyn FnMut(Event)>);
	ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

	// Await open
	open_rx
		.await
		.map_err(|_| bevyhow!("Failed to await WebSocket open"))?;

	// We no longer need to retain the on_open closure; removing the handler avoids leaks
	ws.set_onopen(None);

	// Build writer that holds the WebSocket and the closures to keep them alive
	let writer = Box::new(WasmSocketWriter {
		ws,
		_on_message: on_message,
		_on_error: on_error,
		_on_close: on_close,
		_on_open: None, // already opened
	});

	Ok(Socket::new(rx, writer))
}

struct WasmSocketWriter {
	ws: WebSocket,
	// Keep closures alive while Socket is alive
	_on_message: Closure<dyn FnMut(MessageEvent)>,
	_on_error: Closure<dyn FnMut(Event)>,
	_on_close: Closure<dyn FnMut(CloseEvent)>,
	// Only used prior to open, cleared afterward
	_on_open: Option<Closure<dyn FnMut(Event)>>,
}

impl SocketWriter for WasmSocketWriter {
	fn send_boxed(&mut self, msg: Message) -> BoxFuture<'static, Result<()>> {
		let res = match msg {
			Message::Text(s) => self.ws.send_with_str(&s).map_jserr(),
			Message::Binary(b) => {
				self.ws.send_with_u8_array(b.as_ref()).map_jserr()
			}
			// Browsers do not expose app-level ping/pong; treat as no-op
			Message::Ping(_) | Message::Pong(_) => Ok(()),
			Message::Close(frame) => match frame {
				Some(CloseFrame { code, reason }) => self
					.ws
					.close_with_code_and_reason(code, &reason)
					.map_jserr(),
				None => self.ws.close().map_jserr(),
			},
		};
		ready(res).boxed()
	}
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> BoxFuture<'static, Result<()>> {
		let res = match close {
			Some(CloseFrame { code, reason }) => self
				.ws
				.close_with_code_and_reason(code, &reason)
				.map_jserr(),
			None => self.ws.close().map_jserr(),
		};
		ready(res).boxed()
	}
}
