//! File upload and download utilities for browser environments.
//!
//! This module provides async helpers for triggering native file pickers
//! and programmatic file downloads using Blob URLs.

use crate::web_utils::HtmlEventListener;
use crate::web_utils::document_ext;
use js_sys::Array;
use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Blob;
use web_sys::BlobPropertyBag;
use web_sys::File;
use web_sys::HtmlInputElement;
use web_sys::Url;


/// Downloads binary data as a file with the given filename.
///
/// Creates a Blob with MIME type `application/octet-stream` and triggers
/// a browser download.
///
/// # Errors
///
/// Returns a [`JsValue`] error if Blob creation or download fails.
//https://developer.mozilla.org/en-US/docs/Web/HTTP/Basics_of_HTTP/MIME_types/Common_types
pub fn download_binary(bytes: &[u8], filename: &str) -> Result<(), JsValue> {
	let bytes: JsValue = Uint8Array::from(bytes).into();
	let opts = BlobPropertyBag::new();
	opts.set_type("application/octet-stream");
	let blob = Blob::new_with_u8_array_sequence_and_options(&bytes, &opts)?;
	download_blob(blob, filename)
}

/// Downloads text content as a file with the given filename.
///
/// Creates a Blob with MIME type `text/plain` and triggers a browser download.
///
/// # Errors
///
/// Returns a [`JsValue`] error if Blob creation or download fails.
pub fn download_text(text: &str, filename: &str) -> Result<(), JsValue> {
	let arr = Array::new();
	arr.push(&JsValue::from_str(text));
	let opts = BlobPropertyBag::new();
	opts.set_type("text/plain");
	let blob = Blob::new_with_str_sequence_and_options(&arr, &opts)?;
	download_blob(blob, filename)
}

/// Create a temporary object URL for `blob` and trigger a browser download with `filename`.
///
/// This function creates an `<a>` element, clicks it programmatically, and then
/// revokes the object URL to avoid leaks.
pub fn download_blob(blob: Blob, filename: &str) -> Result<(), JsValue> {
	let url = Url::create_object_url_with_blob(&blob)?;
	let anchor = document_ext::create_anchor();
	anchor.set_attribute("href", &url)?;
	anchor.set_attribute("download", filename)?;
	document_ext::append_child(&anchor);
	anchor.click();
	anchor.remove();
	Url::revoke_object_url(&url)?;
	Ok(())
}

/// Open a native file picker and resolve with the selected File.
///
/// - `accept`: Optional accept string (e.g. "image/*,.png"). Defaults to "*".
/// - Returns: The first selected `File`.
pub async fn upload_file(accept: Option<&str>) -> Result<File, JsValue> {
	let doc = document_ext::document();
	let el = doc
		.create_element("input")?
		.dyn_into::<HtmlInputElement>()?;
	el.set_type("file");
	el.set_accept(accept.unwrap_or("*"));

	doc.body().unwrap().append_child(&el)?;
	el.click();

	// Wait for the "change" event before detaching the input.
	let mut changes = HtmlEventListener::<web_sys::Event>::new_with_target(
		"change",
		el.clone().unchecked_into::<web_sys::EventTarget>(),
	);
	changes.next_event().await;

	doc.body().unwrap().remove_child(&el)?;

	let file = el.files().ok_or("no files")?.get(0).ok_or("no file")?;
	Ok(file)
}

/// Opens a file picker and returns the selected file's text content.
///
/// # Errors
///
/// Returns a [`JsValue`] error if no file is selected or reading fails.
pub async fn upload_text(accept: Option<&str>) -> Result<String, JsValue> {
	let file = upload_file(accept).await?;
	let text = JsFuture::from(file.text()).await?;
	Ok(text.as_string().unwrap())
}

/// Opens a file picker and returns the selected file's binary content.
///
/// # Errors
///
/// Returns a [`JsValue`] error if no file is selected or reading fails.
pub async fn upload_bytes(accept: Option<&str>) -> Result<Vec<u8>, JsValue> {
	let file = upload_file(accept).await?;
	let bytes = JsFuture::from(file.array_buffer()).await?;
	// let bytes: ArrayBuffer = bytes.dyn_into()?;
	let bytes = Uint8Array::new(&bytes);
	let mut vec = Vec::with_capacity(bytes.length() as usize);
	bytes.copy_to(&mut vec);
	Ok(vec)
}
