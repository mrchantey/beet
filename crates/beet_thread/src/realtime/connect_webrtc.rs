use beet_core::prelude::*;
use beet_core::web_utils::document_ext;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlAudioElement;
use web_sys::MediaDevices;
use web_sys::MediaStreamConstraints;
use web_sys::RtcPeerConnection;
use web_sys::RtcSessionDescriptionInit;
use web_sys::window;

// TODO integrate with bevy app using async tasks
//
// Speaks beet's [`Result`]: each web-sys call converts its [`JsValue`] error via
// [`map_jserr`], so a bevy error (eg `document_ext::media_devices`) flows through
// with a plain `?` rather than being round-tripped back into a `JsValue`.
pub(super) async fn connect_webrtc(ephemeral_key: String) -> Result<()> {
	// Get voice param from URL
	let window = window().ok_or_else(|| bevyhow!("no browser window"))?;

	// Create RTCPeerConnection
	let pc = RtcPeerConnection::new().map_jserr()?;

	// Set up to play remote audio from the model
	let audio_el = HtmlAudioElement::new().map_jserr()?;
	audio_el.set_autoplay(true);

	// Add local audio track for microphone input in the browser
	{
		let audio_el = audio_el.clone();
		let ontrack =
			Closure::<dyn FnMut(_)>::new(move |e: web_sys::RtcTrackEvent| {
				audio_el
					.set_src_object(Some(e.streams().get(0).unchecked_ref()));
			});
		pc.set_ontrack(Some(ontrack.as_ref().unchecked_ref()));
		ontrack.forget();
	}

	// Get user media (microphone); `document_ext::media_devices` fails with
	// remedies on an insecure origin instead of a cryptic getUserMedia TypeError
	let media_devices: MediaDevices = document_ext::media_devices()?;
	let constraints = MediaStreamConstraints::new();
	constraints.set_audio(&JsValue::TRUE);
	let ms_promise = media_devices
		.get_user_media_with_constraints(&constraints)
		.map_jserr()?;
	let ms = JsFuture::from(ms_promise).await.map_jserr()?;
	let ms = ms.dyn_into::<web_sys::MediaStream>().map_jserr()?;
	pc.add_track_0(ms.get_tracks().get(0).unchecked_ref(), &ms);

	// Set up data channel for sending and receiving events
	let dc = pc.create_data_channel("oai-events");

	// Data channel message event
	// {
	// 	let dc = dc.clone();
	// 	let mut is_initialized = false;
	// 		// let data = e.data();
	// 		// let realtime_event: serde_json::Value =
	// 		// 	serde_wasm_bindgen::from_value(data).unwrap_or_default();
	// 		// if !is_initialized {
	// 		// 	if let Some(session) = realtime_event.get("session") {
	// 		// 		if session.get("instructions")
	// 		// 			== Some(&serde_json::Value::String(
	// 		// 				instructions.clone(),
	// 		// 			)) {
	// 		// 			cross_log!("Session Instructions received");
	// 		// 			// You can send response.create here if needed
	// 		// 			// dc.send_with_str(...);
	// 		// 			is_initialized = true;
	// 		// 		}
	// 		// 	}
	// 		// }
	// 		cross_log!("Realtime event: {:?}", e);
	// 	});
	// 	dc.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
	// 	onmessage.forget();
	// }

	// Create offer and set local description
	debug!("creating offer");
	let offer: RtcSessionDescriptionInit =
		JsFuture::from(pc.create_offer()).await.map_jserr()?.into();
	debug!("setting local description");
	JsFuture::from(pc.set_local_description(&offer))
		.await
		.map_jserr()?;
	debug!("local description set");

	// Fetch SDP answer
	let base_url = "https://api.openai.com/v1/realtime";
	let model = "gpt-4o-realtime-preview-2024-12-17";
	let url = format!("{}?model={}", base_url, model);

	let opts = web_sys::RequestInit::new();
	opts.set_method("POST");
	opts.set_body(&JsValue::from_str(&offer.get_sdp().unwrap_or_default()));
	let headers = web_sys::Headers::new().map_jserr()?;
	headers
		.append("Authorization", &format!("Bearer {}", ephemeral_key))
		.map_jserr()?;
	headers.append("Content-Type", "application/sdp").map_jserr()?;
	opts.set_headers(&headers);

	let request =
		web_sys::Request::new_with_str_and_init(&url, &opts).map_jserr()?;
	let resp_value = JsFuture::from(window.fetch_with_request(&request))
		.await
		.map_jserr()?;
	debug!("request made");
	let resp: web_sys::Response = resp_value.dyn_into().map_jserr()?;
	let sdp_text = JsFuture::from(resp.text().map_jserr()?).await.map_jserr()?;
	let sdp_str = sdp_text.as_string().unwrap_or_default();

	// Set remote description
	let answer =
		web_sys::RtcSessionDescriptionInit::new(web_sys::RtcSdpType::Answer);
	answer.set_sdp(&sdp_str);
	JsFuture::from(pc.set_remote_description(&answer))
		.await
		.map_jserr()?;
	debug!("answer is set");

	// Data channel open event
	{
		let dc = dc.clone();
		let onopen = Closure::<dyn FnMut()>::new(move || {
			debug!("Data channel is open");
			// web_sys::console::log_1(&"Data channel is open".into());
			// let event = serde_json::json!({
			// 	"type": "session.update",
			// 	"session": {
			// 		"instructions": instructions,
			// 		"voice": voice,
			// 	}
			// });
			// let event_str = serde_json::to_string(&event).unwrap();
			// dc.send_with_str(&event_str).unwrap();
		});
		dc.set_onopen(Some(onopen.as_ref().unchecked_ref()));
		onopen.forget();
	}

	Ok(())
}
