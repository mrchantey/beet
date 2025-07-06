use crate::prelude::*;
use anyhow::Result;
use extend::ext;
// use js_sys::JSON;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;

#[ext]
pub impl Response {
	async fn x_text(&self) -> Result<String> {
		let text = JsFuture::from(self.text().anyhow()?).await.anyhow()?;
		if let Some(text) = text.as_string() {
			Ok(text)
		} else {
			Err(anyhow::anyhow!("Response text is null"))
		}
	}
	// async fn x_json<T>(&self) -> Result<T>
	// where
	// 	T: serde::de::DeserializeOwned,
	// {
	// 	let json = JsFuture::from(self.json().anyhow()?).await.anyhow()?;
	// 	let value = serde_json::from_str(&config).unwrap_throw();
	// 	Ok(value)
	// }
}
