//! `ShowImage`: the browser head's face, `In = ShowImageInput`, `Out = ()`.
//!
//! Serves the same `show-image` route the desktop mock does, but renders it: the
//! handler records the [`DisplayedImage`] url on the caller and [`render_face`]
//! observes that change and points the page's `<img id="face">` at the url. Driven
//! straight from ECS, not the `data-bx` reactive layer (which binds document fields,
//! not a wasm ECS component).
use super::*;
use beet_core::prelude::*;
use web_sys::HtmlImageElement;

/// Display an image on the head's face, recording the [`DisplayedImage`] url on the
/// caller. The [`render_face`] observer then updates the rendered face.
#[action(route = "show-image")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn ShowImage(cx: ActionContext<ShowImageInput>) -> Result<()> {
	let src = cx.input.src;
	info!("ShowImage: {src}");
	cx.caller.insert(DisplayedImage(src.into())).await?;
	Ok(())
}

/// Point the page's `<img id="face">` at the url of a freshly-set [`DisplayedImage`].
///
/// A plain ECS observer, not a `data-bx` binding: the url is a wasm-side component
/// (component-source binding is server-only), so the head drives the DOM directly.
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
