//! Serving a running script's world calls, one at a time, against the live
//! world.
use crate::prelude::*;
use beet_core::prelude::*;

/// The seat a bridged script's calls are served from: an [`AsyncWorld`] handle
/// and the [`ScriptConfig`] every call is checked against.
///
/// The only path a [`WorldCall`] is executed through, on every backend. An
/// out-of-process backend holds one for the duration of a run and answers each
/// round trip with it; the embedded engine, which evaluates in-process, holds
/// one too and awaits it between pumps. So there is one executor rather than a
/// synchronous shape and an asynchronous one that have to agree.
#[derive(Clone, Get)]
pub struct WorldBridge {
	/// The world every call is served against.
	world: AsyncWorld,
	/// The reach every call is checked against.
	config: ScriptConfig,
}

impl WorldBridge {
	/// A bridge serving `config`'s reach against `world`.
	pub fn new(world: AsyncWorld, config: ScriptConfig) -> Self {
		Self { world, config }
	}

	/// Perform one call and produce its reply.
	///
	/// Infallible: a refusal or a bad identifier is a [`WorldReply::Err`] the
	/// script can catch, not a failure of the run.
	pub async fn serve(&self, call: WorldCall) -> WorldReply {
		call.execute(self).await
	}

	/// Serve one call and encode its reply as a protocol line.
	///
	/// The transport-facing form: every out-of-process backend carries replies
	/// as JSON lines, so the encoding belongs here rather than once per backend.
	///
	/// # Errors
	/// Errors only when the reply cannot be encoded.
	pub async fn serve_line(&self, call: WorldCall) -> Result<String> {
		serde_json::to_string(&self.serve(call).await)
			.map_err(|err| bevyhow!("failed to encode world reply: {err}"))
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn world() -> World {
		let world = AsyncPlugin::world();
		world
			.resource::<AppTypeRegistry>()
			.write()
			.register::<Name>();
		world
	}

	/// The bridge is the only face of the executor, so a call served through it
	/// lands on the world it was built from.
	#[beet_core::test]
	async fn serves_a_call_against_the_live_world() {
		let mut world = world();
		// spawned as a task rather than polled inline: the bridge resolves at a
		// sync point, which drives the executor's tasks, not this one.
		world
			.run_async_then(|world| async move {
				WorldBridge::new(world, ScriptConfig::default())
					.serve_line(WorldCall {
						id: 0,
						op: WorldOp::Spawn {
							components: {
								let mut components = Map::default();
								components.insert("Name", "ada");
								components
							},
						},
					})
					.await
			})
			.await
			.unwrap()
			.xpect_contains(r#"{"status":"ok","id":0,"value":""#);
		world
			.query::<&Name>()
			.iter(&world)
			.next()
			.unwrap()
			.as_str()
			.xpect_eq("ada");
	}
}
