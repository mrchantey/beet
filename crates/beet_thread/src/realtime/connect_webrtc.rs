//! The WebRTC transport for a realtime session: microphone audio up, model
//! audio down, and the `oai-events` data channel carrying both directions of
//! realtime events as JSON text.
//!
//! Speaks beet's [`Result`]: each web-sys call converts its [`JsValue`] error
//! via [`map_jserr`], so a bevy error (eg `document_ext::media_devices`) flows
//! through with a plain `?` rather than being round-tripped back into a
//! `JsValue`.
use crate::realtime::realtime_session_create_request::Model;
use crate::realtime::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlAudioElement;
use web_sys::MediaDevices;
use web_sys::MediaStreamConstraints;
use web_sys::MessageEvent;
use web_sys::RtcDataChannel;
use web_sys::RtcPeerConnection;
use web_sys::RtcSessionDescriptionInit;
use web_sys::RtcTrackEvent;

/// A live realtime session over WebRTC.
///
/// Owns every browser-side resource: the peer connection, the data channel,
/// the detached `<audio>` sink and the `track` handler wiring remote audio
/// into it. Dropping the connection closes the peer connection and clears the
/// handler, so nothing leaks and no callback outlives its rust state (the
/// `web_utils` lifetime rule; never `Closure::forget`).
pub struct RealtimeConnection {
	pc: RtcPeerConnection,
	dc: RtcDataChannel,
	/// remote model audio plays through this detached element; it works
	/// unattached, or mount it via [`Self::audio`].
	audio: HtmlAudioElement,
	/// the `track` property handler: applied synchronously by the browser, so
	/// remote audio attaches even while no rust future is polling. Held here
	/// for exactly the connection's lifetime.
	_on_track: Closure<dyn FnMut(RtcTrackEvent)>,
	/// server events, a stream of `message` events on the data channel.
	messages: HtmlEventListener<MessageEvent>,
}

impl RealtimeConnection {
	/// The `<audio>` element remote model audio plays through, for callers that
	/// want to mount or control it.
	pub fn audio(&self) -> &HtmlAudioElement { &self.audio }

	/// Send a client event onto the data channel as JSON text.
	pub fn send(&self, event: &Value) -> Result {
		let text = serde_json::to_string(event)?;
		self.dc.send_with_str(&text).map_jserr()
	}

	/// The next server event, parsed from the data channel's JSON text into a
	/// [`Value`]. `None` once the channel closes.
	pub async fn recv(&mut self) -> Option<Result<Value>> {
		let ev = self.messages.next_event().await?;
		Some(Self::parse_event(ev))
	}

	fn parse_event(ev: MessageEvent) -> Result<Value> {
		let Some(text) = ev.data().as_string() else {
			bevybail!("expected a text data channel message");
		};
		serde_json::from_str::<Value>(&text)?.xok()
	}
}

impl Drop for RealtimeConnection {
	fn drop(&mut self) {
		// closed first, so the cleared handler can never miss a live event
		self.pc.close();
		self.pc.set_ontrack(None);
	}
}

/// Establish the WebRTC session: microphone capture, the remote audio sink,
/// SDP negotiation against the realtime endpoint, and the opened `oai-events`
/// data channel, then a `session.update` applying `request`'s session settings.
pub(super) async fn connect_webrtc(
	ephemeral_key: String,
	request: RealtimeSessionCreateRequest,
) -> Result<RealtimeConnection> {
	let pc = RtcPeerConnection::new().map_jserr()?;

	// remote model audio plays through a detached element; the `track` property
	// handler applies synchronously in the browser, so it wires up even though
	// no rust future is polling. Owned by the returned connection.
	let audio = HtmlAudioElement::new().map_jserr()?;
	audio.set_autoplay(true);
	let on_track = {
		let audio = audio.clone();
		Closure::from_func(move |ev: RtcTrackEvent| {
			audio.set_src_object(Some(ev.streams().get(0).unchecked_ref()));
		})
	};
	pc.set_ontrack(Some(on_track.as_ref().unchecked_ref()));

	// microphone input; `document_ext::media_devices` fails with remedies on an
	// insecure origin instead of a cryptic getUserMedia TypeError
	let media_devices: MediaDevices = document_ext::media_devices()?;
	let constraints = MediaStreamConstraints::new();
	constraints.set_audio(&JsValue::TRUE);
	let stream_promise = media_devices
		.get_user_media_with_constraints(&constraints)
		.map_jserr()?;
	let stream = JsFuture::from(stream_promise)
		.await
		.map_jserr()?
		.dyn_into::<web_sys::MediaStream>()
		.map_jserr()?;
	pc.add_track_0(stream.get_tracks().get(0).unchecked_ref(), &stream);

	// the event channel; listeners registered before negotiation so no event
	// can slip past unobserved.
	let dc = pc.create_data_channel("oai-events");
	let mut opened = HtmlEventListener::<web_sys::Event>::new_with_target(
		"open",
		dc.clone(),
	);
	let messages = HtmlEventListener::<MessageEvent>::new_with_target(
		"message",
		dc.clone(),
	);

	// SDP offer -> answer against the realtime endpoint, through beet's
	// `Request` (wasm-capable), never a raw fetch.
	debug!("creating sdp offer");
	let offer: RtcSessionDescriptionInit =
		JsFuture::from(pc.create_offer()).await.map_jserr()?.into();
	JsFuture::from(pc.set_local_description(&offer))
		.await
		.map_jserr()?;
	let model = model_slug(request.model.unwrap_or_default())?;
	let answer_sdp = Request::new(
		HttpMethod::Post,
		format!("https://api.openai.com/v1/realtime?model={model}"),
	)
	.with_auth_bearer(&ephemeral_key)
	.with_content_type(MediaType::Other("application/sdp".into()))
	.with_body(offer.get_sdp().unwrap_or_default())
	.send()
	.await?
	.into_result()
	.await?
	.text()
	.await?;
	let answer = RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
	answer.set_sdp(&answer_sdp);
	JsFuture::from(pc.set_remote_description(&answer))
		.await
		.map_jserr()?;
	debug!("remote description set");

	// the channel opens once the connection settles; sending before that throws
	opened
		.next_event()
		.await
		.ok_or_else(|| bevyhow!("data channel closed before opening"))?;
	debug!("data channel open");

	let connection = RealtimeConnection {
		pc,
		dc,
		audio,
		_on_track: on_track,
		messages,
	};
	// align the live session with the requested settings (instructions, voice,
	// ..): the session was created server-side, but an updated request or a
	// reconnect re-applies them here.
	connection.send(&session_update(&request)?)?;
	Ok(connection)
}

/// The `session.update` client event applying `request`'s session settings.
fn session_update(request: &RealtimeSessionCreateRequest) -> Result<Value> {
	Value::from_serde(serde_json::json!({
		"type": "session.update",
		"session": request,
	}))
}

/// The wire name a [`Model`] serializes to, ie `gpt-4o-realtime-preview`.
fn model_slug(model: Model) -> Result<String> {
	serde_json::to_value(model)?
		.as_str()
		.map(str::to_string)
		.ok_or_else(|| bevyhow!("model must serialize to a string"))
}
