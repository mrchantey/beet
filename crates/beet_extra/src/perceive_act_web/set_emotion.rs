//! `SetEmotion`: the browser head's face, `In = SetEmotionInput`, `Out = ()`.
//!
//! Serves the same `set-emotion` route the desktop head does. Unlike the mock (which
//! only records the [`Emotion`]), the web head renders it: the handler records the
//! emotion on the caller and [`render_face`] observes that change and points the page's
//! `<img id="face">` at the matching sprite. Driven straight from ECS, not the `data-bx`
//! reactive layer (which binds document fields, not a wasm ECS component).
use super::*;
use beet_core::prelude::*;
use web_sys::HtmlImageElement;

/// The directory the eight robot-eyes sprites are served from, joined with an
/// [`Emotion::sprite_stem`] + `.png` to form the `<img>` src. The head page mounts the
/// workspace `assets` tree here (`<AssetsDir prefix="assets">`).
const FACE_SPRITE_DIR: &str = "assets/extra/robot-eyes";

/// Set the head's facial expression, recording the [`Emotion`] on the caller. The
/// [`render_face`] observer then updates the rendered face.
#[action(route = "set-emotion")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn SetEmotion(cx: ActionContext<SetEmotionInput>) -> Result<()> {
	let emotion = cx.input.emotion;
	info!("SetEmotion: {emotion:?}");
	cx.caller.insert(emotion).await?;
	Ok(())
}

/// Point the page's `<img id="face">` at the sprite for a freshly-set [`Emotion`].
///
/// A plain ECS observer, not a `data-bx` binding: the emotion is a wasm-side component
/// (component-source binding is server-only), so the head drives the DOM directly. Runs
/// on both insert and change, so every `set-emotion` call re-renders the face.
pub fn render_face(ev: On<Insert, Emotion>, emotions: Query<&Emotion>) {
	let Ok(emotion) = emotions.get(ev.entity) else {
		return;
	};
	let Some(face) = document_ext::query_selector::<HtmlImageElement>("#face")
	else {
		// no face element on the page (eg a headless test root): nothing to render.
		return;
	};
	face.set_src(&format!("{FACE_SPRITE_DIR}/{}.png", emotion.sprite_stem()));
}
