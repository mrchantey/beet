use super::types::*;
use crate::openai::*;

/// REST API endpoint to generate ephemeral session tokens for use in client-side applications.
pub struct RealtimeApi;

impl RealtimeApi {
	/// Create an ephemeral API token for use in client-side applications with the Realtime API. Can be configured with the same session parameters as the session.update client event.
	/// It responds with a session object, plus a client_secret key which contains a usable ephemeral API token that can be used to authenticate browser clients for the Realtime API.
	/// https://platform.openai.com/docs/api-reference/realtime-sessions
	pub async fn create(
		req: RealtimeSessionCreateRequest,
	) -> OpenAiResult<RealtimeSessionCreateResponse> {
		ReqwestClient::client()
			.post("https://api.openai.com/v1/realtime/sessions")
			.header("Authorization", format!("Bearer {}", OpenAiKey::get()?))
			.header("Content-Type", "application/json")
			.body(
				serde_json::to_string(&req)
					.map_err(|e| OpenAiError::SerializationFailed(e))?,
			)
			.send()
			.await?
			.error_for_status()?
			.text()
			.await?
			.xmap(|s| {
				serde_json::from_str::<RealtimeSessionCreateResponse>(&s)
					.map_err(|e| OpenAiError::DeserializationFailed(e))
			})?
			.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::openai::realtime::*;
	use sweet::prelude::*;

	#[sweet::test]
	async fn works() {
		use crate::openai::realtime::types::RealtimeSessionCreateRequest;

		RealtimeApi::create(RealtimeSessionCreateRequest {
			voice: Some(Box::new(types::VoiceIdsShared::Ash)),
			model: Some(types::realtime_session_create_request::Model::Gpt4oRealtimePreview),
			..Default::default()
		})
		.await
		.unwrap().xmap(|res|res.voice).xmap(expect).to_be(Some(Box::new(types::VoiceIdsShared::Ash)));

		// println!("{:#?}", res);
	}
}
