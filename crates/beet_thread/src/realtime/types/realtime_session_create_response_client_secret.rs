use serde::Deserialize;
use serde::Serialize;

/// RealtimeSessionCreateResponseClientSecret : Ephemeral key returned by the API.
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RealtimeSessionCreateResponseClientSecret {
	/// Ephemeral key usable in client environments to authenticate connections to the Realtime API. Use this in client-side environments rather than a standard API token, which should only be used server-side.
	#[serde(rename = "value")]
	pub value: String,
	/// Timestamp for when the token expires. Currently, all tokens expire after one minute.
	#[serde(rename = "expires_at")]
	pub expires_at: i32,
}

impl RealtimeSessionCreateResponseClientSecret {
	/// Ephemeral key returned by the API.
	pub fn new(
		value: String,
		expires_at: i32,
	) -> RealtimeSessionCreateResponseClientSecret {
		RealtimeSessionCreateResponseClientSecret { value, expires_at }
	}
}
