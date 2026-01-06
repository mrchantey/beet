use crate::realtime::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// REST API endpoint to generate ephemeral session tokens for use in client-side applications.
pub struct RealtimeApi;

impl RealtimeApi {
	/// Create an ephemeral API token on the server for use in client-side applications with the Realtime API. Can be configured with the same session parameters as the session.update client event.
	/// It responds with a session object, plus a client_secret key which contains a usable ephemeral API token that can be used to authenticate browser clients for the Realtime API.
	/// https://platform.openai.com/docs/api-reference/realtime-sessions
	pub async fn create_session(
		req: RealtimeSessionCreateRequest,
	) -> Result<RealtimeSessionCreateResponse> {
		Request::new(
			HttpMethod::Post,
			"https://api.openai.com/v1/realtime/sessions",
		)
		.with_auth_bearer(&env_ext::var("OPENAI_API_KEY")?)
		.with_json_body(&req)
		.unwrap()
		.send()
		.await?
		.into_result()
		.await?
		.json::<RealtimeSessionCreateResponse>()
		.await?
		.xok()
	}

	/// Connect to the realtime api client side
	// TODO bevy integration
	#[cfg(target_arch = "wasm32")]
	pub async fn connect_webrtc(ephemeral_key: String) -> Result<()> {
		// async_ext::spawn_local(async move {
		connect_webrtc(ephemeral_key).await.map_jserr()
		// });
		// Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::realtime::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		use crate::realtime::types::RealtimeSessionCreateRequest;

		RealtimeApi::create_session(RealtimeSessionCreateRequest {
			voice: Some(Box::new(types::VoiceIdsShared::Ash)),
			model: Some(types::realtime_session_create_request::Model::Gpt4oRealtimePreview),
			..Default::default()
		})
		.await
		.unwrap().xmap(|res|res.voice).xpect_eq(Some(Box::new(types::VoiceIdsShared::Ash)));

		// println!("{:#?}", res);
	}
}
