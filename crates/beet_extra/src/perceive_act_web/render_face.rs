//! `render_face`: the browser head's rendering of the shared `show-image` capability.
//!
//! The [`ShowImage`] action (shared with the native mock, from `perceive_act_core`)
//! records the chosen [`DisplayedImage`] url on the caller; this observer points the
//! page's `<img id="face">` at it. Driven straight from ECS, not the `data-bx` reactive
//! layer (which binds document fields, not a wasm ECS component).
use super::*;
use beet_core::prelude::*;
use web_sys::HtmlImageElement;

/// Point the page's `<img id="face">` at the url of a freshly-set [`DisplayedImage`].
///
/// Runs on insert, so every `show-image` call re-renders the face.
pub fn render_face(
	ev: On<Insert, DisplayedImage>,
	images: Query<&DisplayedImage>,
) {
	let Ok(image) = images.get(ev.entity) else {
		return;
	};
	let Some(face) = document_ext::query_selector::<HtmlImageElement>("#face")
	else {
		// no face element on the page (eg a headless test root): nothing to render.
		return;
	};
	face.set_src(image.0.as_str());
}
