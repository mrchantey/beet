//! The Cloudflare Workers position: not implemented, and deliberately so.
//!
//! A Worker cannot spawn an attenuated child isolate. Worker Loaders (the
//! dynamic-isolate API that would make this possible) are young enough that
//! building on them now would mean maintaining a moving target for a case the
//! embedded engine already covers.
//!
//! And it does cover it. `quickjs` compiles to wasm and runs inside the Worker's
//! own module with zero ambient authority and enforced time, memory and stack
//! limits — a stricter isolate than a nested Worker would give, with nothing
//! required of the host. Enabling the feature is the answer here, and for very
//! nearly every other situation: beet ships the runtime everything it needs.

use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value as JsonValue;

/// Refuses to evaluate, naming the one supported path.
///
/// An error rather than a panic: this runs inside a live Worker serving other
/// requests, and a panicking wasm module takes the whole isolate down with it.
/// One request failing is the correct blast radius for a build that reached a
/// backend it does not have.
///
/// The error carries no HTTP status of its own — `HttpError` lives in beet_net,
/// which depends on this crate — so a router surfaces it as a 500 through
/// `HttpError::from_opaque`. `501 Not Implemented` would be the truer status; it
/// needs a status-carrying error type further down the stack than beet_net to
/// express, which is a larger call than this backend should make.
pub(crate) async fn run_cloudflare<Sink>(
	_request: ScriptRequest,
	_sink: Sink,
	_bridge: Option<&WorldBridge>,
) -> Result<Option<JsonValue>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	bevybail!(
		"`Script` has no host backend on Cloudflare Workers: a Worker cannot \
spawn an attenuated child isolate. Enable the `quickjs` feature — the embedded \
engine compiles into the Worker itself, with no ambient authority and enforced \
limits, which is the right answer for roughly every situation."
	)
}
