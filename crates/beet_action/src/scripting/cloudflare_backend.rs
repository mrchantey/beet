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
/// This is the sanctioned exception to the no-panic convention (Pete's call):
/// the state is a build misconfiguration, not a runtime condition, and there is
/// no sensible value to hand an error handler.
pub(crate) async fn run_cloudflare<Sink>(
	_request: ScriptRequest,
	_sink: Sink,
) -> Result<Option<JsonValue>>
where
	Sink: FnMut(ConsoleStream, &str),
{
	unimplemented!(
		"`Script` has no host backend on Cloudflare Workers: a Worker cannot \
spawn an attenuated child isolate. Enable the `quickjs` feature — the embedded \
engine compiles into the Worker itself, with no ambient authority and enforced \
limits, which is the right answer for roughly every situation."
	)
}
