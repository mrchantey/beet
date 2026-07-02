//! `TakePhoto`: the browser head's webcam capture, `In = ()`, `Out = MediaBytes`.
//!
//! Serves the same `take-photo` route the desktop head does, but from the real
//! webcam: `getUserMedia({video:true})` -> draw a frame onto a canvas ->
//! `canvas.toDataURL("image/jpeg")` -> [`MediaBytes`]. The agent's `interpret-photo`
//! calls this over the socket, then one-shots the bytes to a vision model.
use beet_core::prelude::*;
use beet_net::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::CanvasRenderingContext2d;
use web_sys::HtmlVideoElement;
use web_sys::MediaStream;
use web_sys::MediaStreamConstraints;
use web_sys::MediaStreamTrack;

/// Capture a webcam frame and return its bytes (a jpeg, no description).
#[action(route = "take-photo")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn TakePhoto(_cx: ActionContext<()>) -> Result<MediaBytes> {
	capture_webcam().await
}

/// Grab one webcam frame as jpeg [`MediaBytes`].
///
/// Opens the camera, plays the stream into an offscreen `<video>`, waits for its first
/// frame so the dimensions are known, draws that frame onto a matching canvas, then
/// reads a jpeg `data:` URL and decodes it. The `MediaStream` tracks are stopped so
/// the camera light goes off between shots.
async fn capture_webcam() -> Result<MediaBytes> {
	let stream = open_camera().await?;
	let video = document_ext::create_element("video")
		.dyn_into::<HtmlVideoElement>()
		.map_err(|_| bevyhow!("not a video element"))?;
	// muted + inline so autoplay is allowed and the frame is ready to draw.
	video.set_muted(true);
	video.set_autoplay(true);
	video.set_src_object(Some(&stream));
	JsFuture::from(video.play().map_jserr()?).await.map_jserr()?;
	await_first_frame(&video).await;

	// draw the current frame onto a canvas sized to the video, then read it back as jpeg.
	let (width, height) = (video.video_width(), video.video_height());
	let canvas = document_ext::create_canvas();
	canvas.set_width(width);
	canvas.set_height(height);
	let ctx = canvas
		.get_context("2d")
		.map_jserr()?
		.ok_or_else(|| bevyhow!("no 2d canvas context"))?
		.dyn_into::<CanvasRenderingContext2d>()
		.map_err(|_| bevyhow!("not a 2d context"))?;
	ctx.draw_image_with_html_video_element_and_dw_and_dh(
		&video,
		0.0,
		0.0,
		width as f64,
		height as f64,
	)
	.map_jserr()?;

	// stop the tracks so the camera light goes off, then decode the jpeg data URL.
	stop_tracks(&stream);
	let data_url = canvas.to_data_url_with_type("image/jpeg").map_jserr()?;
	MediaBytes::from_url(&Url::parse(data_url))
}

/// Open the default camera, returning its [`MediaStream`]. Mirrors the audio
/// getUserMedia shape in `beet_thread`'s webrtc connect, but requesting video.
async fn open_camera() -> Result<MediaStream> {
	let media_devices = web_sys::window()
		.ok_or_else(|| bevyhow!("no window"))?
		.navigator()
		.media_devices()
		.map_jserr()?;
	let constraints = MediaStreamConstraints::new();
	constraints.set_video(&true.into());
	JsFuture::from(
		media_devices
			.get_user_media_with_constraints(&constraints)
			.map_jserr()?,
	)
	.await
	.map_jserr()?
	.dyn_into::<MediaStream>()
	.map_jserr()
}

/// Wait until the `<video>` has decoded its first frame (non-zero dimensions), so the
/// canvas is sized correctly and the draw is not blank. Polls a short interval rather
/// than wiring a `loadeddata` listener, keeping the capture a plain await.
async fn await_first_frame(video: &HtmlVideoElement) {
	for _ in 0..100u32 {
		if video.video_width() > 0 && video.ready_state() >= 2 {
			return;
		}
		time_ext::sleep_millis(20).await;
	}
}

/// Stop every track on the stream, releasing the camera.
fn stop_tracks(stream: &MediaStream) {
	let tracks = stream.get_tracks();
	for i in 0..tracks.length() {
		if let Ok(track) = tracks.get(i).dyn_into::<MediaStreamTrack>() {
			track.stop();
		}
	}
}
