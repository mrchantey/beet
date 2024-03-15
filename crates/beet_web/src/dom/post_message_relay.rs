use anyhow::Result;
use beet_net::prelude::*;
use forky_core::ResultTEExt;
use forky_web::AnimationFrame;
use forky_web::HtmlEventListener;
use forky_web::ResultTJsValueExt;
use js_sys::ArrayBuffer;
use js_sys::JsString;
use js_sys::Uint8Array;
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::spawn_local;
use web_sys::MessageEvent;
use web_sys::Window;

#[derive(Clone)]
pub struct PostMessageRelay {
	pub target: Window,
	pub relay: Relay,
	pub listener: HtmlEventListener<MessageEvent>,
}


impl PostMessageRelay {
	/// For loopback postmessage, ie send to self
	pub fn new_with_current_window(relay: Relay) -> Self {
		let window = web_sys::window().unwrap();
		Self::new(window, relay)
	}

	#[must_use]
	pub fn run(mut self) -> AnimationFrame {
		AnimationFrame::new(move || {
			self.send_all().ok_or(|e| log::error!("{e}"));
		})
	}


	/// For use within iframes
	pub fn new_with_parent_window(relay: Relay) -> Self {
		let parent = web_sys::window().unwrap().parent().unwrap().unwrap();
		Self::new(parent, relay)
	}

	pub fn new(target: Window, relay: Relay) -> Self {
		let listener = {
			let relay = relay.clone();
			HtmlEventListener::new_with_target(
				"message",
				move |e: MessageEvent| {
					let data = e.data();
					let mut relay = relay.clone();
					if let Some(messages) =
						from_js_value(&data).ok_or(|e| log::error!("{e}"))
					{
						spawn_local(async move {
							relay
								.try_send_all_messages(messages)
								.await
								.ok_or(|e| log::error!("{e}"));
						});
					}
				},
				target.clone().into(),
			)
		};
		Self {
			target,
			relay,
			listener,
		}
	}

	pub fn send_all(&mut self) -> Result<()> {
		let messages = self.relay.get_all_messages()?;
		let buff = into_array_buffer(messages)?;
		self.target.post_message(&buff.into(), "*").anyhow()?;
		Ok(())
	}

	#[cfg(feature = "json")]
	pub fn send_all_string(&mut self) -> Result<()> {
		let messages = self.relay.get_all_messages()?;
		let str = serde_json::to_string(messages);
		todo!("string to jsvalue");
		self.target.post_message(&str.into(), "*").anyhow()?;
		Ok(())
	}
}


fn from_js_value<T: DeserializeOwned>(value: &JsValue) -> Result<T> {
	if let Ok(abuf) = value.clone().dyn_into::<ArrayBuffer>() {
		let js_array = Uint8Array::new(&abuf);
		let bytes = js_array.to_vec();
		let val = bincode::deserialize(&bytes)?;
		return Ok(val);
	}
	#[allow(unused_variables)]
	if let Ok(str) = value.clone().dyn_into::<JsString>() {
		#[cfg(feature = "json")]
		return Ok(serde_json::from_str(&str.as_string().unwrap())?);
		#[cfg(not(feature = "json"))]
		anyhow::bail!(
			"received string but beet_net/json feature is not enabled"
		)
	}

	anyhow::bail!(
		"Unknown data type\n(websockets - was set_binary_type set correctly?)"
	)
}
/// Note: this seems to error if trying to pass in bytes, ie [`Vec<u8>`]
fn into_array_buffer<T: Serialize>(val: T) -> Result<ArrayBuffer> {
	let bytes = bincode::serialize(&val)?;
	let array = Uint8Array::from(bytes.as_slice());
	Ok(array.buffer())
}
