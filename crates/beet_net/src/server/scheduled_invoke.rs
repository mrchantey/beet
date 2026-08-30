//! The payload a scheduled invoke delivers, the one shape both a deploy
//! declaration and a serverless adapter read.

use crate::prelude::*;
use beet_core::prelude::*;

/// The request a timer dispatches, rendered into the schedule at deploy time and
/// deserialized by the adapter the invoke arrives at.
///
/// A serverless function has no listener, so a schedule cannot send it an http
/// request: it delivers a json payload verbatim. This is that payload. Owning it
/// as one type is what keeps the two sides from drifting — the deploy never
/// hand-crafts a fake http event, and the adapter never guesses which of a
/// provider's event shapes it was handed.
///
/// The [`kind`](Self::kind) tag is load-bearing: a schedule renders its payload
/// at deploy time and keeps invoking with it until the next deploy, so a stale
/// schedule reaching a newer binary is ordinary rather than exceptional. An
/// unrecognized payload fails to deserialize and the invoke fails loudly, rather
/// than dispatching some default request.
#[derive(Debug, Clone, PartialEq, Get, SetWith, Serialize, Deserialize)]
pub struct ScheduledInvoke {
	/// The wire tag, so a payload that is not a beet invoke never dispatches.
	#[set_with(skip)]
	kind: ScheduledInvokeKind,
	/// The method the dispatched request carries.
	method: HttpMethod,
	/// The route this invoke dispatches, optionally carrying query params, ie
	/// `analytics/rollup?full=true`.
	#[set_with(into)]
	path: SmolStr,
}

impl ScheduledInvoke {
	/// A `POST` invoke of `path`, the shape a job takes: a scheduled dispatch
	/// acts, it does not fetch.
	pub fn new(path: impl Into<SmolStr>) -> Self {
		Self {
			kind: ScheduledInvokeKind::V1,
			method: HttpMethod::Post,
			path: path.into(),
		}
	}

	/// The request this invoke dispatches, the counterpart of the http request a
	/// listener would have parsed.
	pub fn into_request(self) -> Request {
		Request::new(self.method, self.path.as_str())
	}

	/// The invoke a delivered payload names, or a loud error naming what
	/// arrived: an adapter that guessed here would dispatch some default route
	/// on every event the function is ever sent.
	#[cfg(feature = "json")]
	pub fn from_payload(payload: &[u8]) -> Result<Self> {
		serde_json::from_slice(payload).map_err(|err| {
			bevyhow!(
				"a scheduled invoke delivered a payload this binary cannot dispatch: {err}\npayload: {}",
				String::from_utf8_lossy(payload)
			)
		})
	}
}

/// The [`ScheduledInvoke`] wire tag, whose serialized name is both the
/// discriminator and the version: a payload the adapter does not recognize is a
/// deserialize failure, never a silently different request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledInvokeKind {
	/// The current shape: a method and a route.
	#[serde(rename = "beet.scheduled_invoke.v1")]
	V1,
}

// the round trip is the payload's whole job, and both halves of it need json
#[cfg(all(test, feature = "json"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The round trip the two sides depend on: what a deploy renders is what an
	/// adapter dispatches, query params included.
	#[beet_core::test]
	fn round_trips_into_a_request() {
		let json = serde_json::to_string(&ScheduledInvoke::new(
			"analytics/rollup?full=true",
		))
		.unwrap();
		json.as_str().xpect_contains("beet.scheduled_invoke.v1");
		let request = ScheduledInvoke::from_payload(json.as_bytes())
			.unwrap()
			.into_request();
		request.method().xpect_eq(HttpMethod::Post);
		request.path().xpect_eq(&["analytics", "rollup"]);
		request.get_param("full").unwrap().xpect_eq("true");
	}

	/// A payload from some other event source never dispatches: the tag is
	/// required, so an unrecognized shape fails here rather than at the route it
	/// guessed.
	#[beet_core::test]
	fn an_untagged_payload_is_rejected() {
		ScheduledInvoke::from_payload(
			br#"{"method":"Post","path":"analytics/rollup"}"#,
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("cannot dispatch")
		.xpect_contains("kind");
	}
}
