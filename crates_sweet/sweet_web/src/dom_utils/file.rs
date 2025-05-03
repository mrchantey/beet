use crate::prelude::*;
use crate::DocumentExt;
use js_sys::Array;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Blob;
use web_sys::BlobPropertyBag;
use web_sys::Document;
use web_sys::File;
use web_sys::HtmlInputElement;
use web_sys::Url;


//https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
pub fn download_binary(bytes: &[u8], filename: &str) -> Result<(), JsValue> {
	let bytes: JsValue = Uint8Array::from(bytes).into();
	let opts = BlobPropertyBag::new();
	opts.set_type("application/octet-stream");
	let blob = Blob::new_with_u8_array_sequence_and_options(&bytes, &opts)?;
	download_blob(blob, filename)
}
pub fn download_text(text: &str, filename: &str) -> Result<(), JsValue> {
	let arr = Array::new();
	arr.push(&JsValue::from_str(text));
	let opts = BlobPropertyBag::new();
	opts.set_type("text/plain");
	let blob = Blob::new_with_str_sequence_and_options(&arr, &opts)?;
	download_blob(blob, filename)
}

pub fn download_blob(blob: Blob, filename: &str) -> Result<(), JsValue> {
	let url = Url::create_object_url_with_blob(&blob)?;
	let anchor = Document::x_create_anchor();
	anchor.set_attribute("href", &url)?;
	anchor.set_attribute("download", filename)?;
	Document::x_append_child(&anchor);
	anchor.click();
	anchor.remove();
	Url::revoke_object_url(&url)?;
	Ok(())
}

pub async fn upload_file(accept: Option<&str>) -> Result<File, JsValue> {
	let document = Document::get();
	let el = document
		.create_element("input")?
		.dyn_into::<HtmlInputElement>()?;
	el.set_type("file");
	el.set_accept(&accept.unwrap_or("*"));

	document.body().unwrap().append_child(&el)?;
	el.click();
	document.body().unwrap().remove_child(&el)?;
	HtmlEventWaiter::new_with_target("change", el.clone())
		.wait()
		.await?;

	let file = el.files().ok_or("no files")?.get(0).ok_or("no file")?;

	Ok(file)
}

pub async fn upload_text(accept: Option<&str>) -> Result<String, JsValue> {
	let file = upload_file(accept).await?;
	let text = JsFuture::from(file.text()).await?;
	Ok(text.as_string().expect("blob.text() must be string"))
}
pub async fn upload_bytes(accept: Option<&str>) -> Result<Vec<u8>, JsValue> {
	let file = upload_file(accept).await?;
	let bytes = JsFuture::from(file.array_buffer()).await?;
	// let bytes: ArrayBuffer = bytes.dyn_into()?;
	let bytes = Uint8Array::new(&bytes);
	let mut vec = Vec::with_capacity(bytes.length() as usize);
	bytes.copy_to(&mut vec);
	Ok(vec)
}


pub async fn fetch_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
	let res = fetch(url).await?;
	let bytes = JsFuture::from(res.array_buffer().anyhow()?)
		.await
		.anyhow()?;
	let bytes = Uint8Array::new(&bytes);
	let mut vec = Vec::with_capacity(bytes.length() as usize);
	bytes.copy_to(&mut vec);
	Ok(vec)
}

pub async fn fetch_text(url: &str) -> anyhow::Result<String> {
	let res = fetch(url).await?;
	let text = JsFuture::from(res.text().anyhow()?).await.anyhow()?;
	Ok(text.as_string().expect("response.text() must be string"))
}
