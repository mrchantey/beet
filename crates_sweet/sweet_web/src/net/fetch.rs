use crate::*;
use anyhow::Result;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;


pub async fn fetch(url: &str) -> Result<Response> {
	let window = web_sys::window().unwrap();
	let prom = window.fetch_with_str(url);

	let res = JsFuture::from(prom).await.anyhow()?;
	let res = res.dyn_into().anyhow()?;
	Ok(res)
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod test {
	use crate::prelude::*;
	use sweet_test::as_sweet::*;

	#[sweet_test::test]
	async fn works() {
		let res = fetch("https://example.com").await.unwrap();
		expect(res.status()).to_be(200);
	}
	#[sweet_test::test]
	async fn text() {
		let res = fetch("https://example.com").await.unwrap();
		let text = res.x_text().await.unwrap();
		expect(text.as_str())
			.to_contain("This domain is for use in illustrative examples");
		expect(res.status()).to_be(200);
	}
}
