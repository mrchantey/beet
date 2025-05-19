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
		Request::new("https://api.openai.com/v1/realtime/sessions")
			.method(HttpMethod::Post)
			.auth_bearer(&OpenAiKey::get()?)
			.body(req)?
			.send()
			.await?
			.into_result()?
			.body::<RealtimeSessionCreateResponse>()
			.await?
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
