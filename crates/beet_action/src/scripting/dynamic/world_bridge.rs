//! Serving a running script's world calls, one at a time, against the live
//! world.
use crate::prelude::*;
use beet_core::prelude::*;

/// The seat a bridged script's calls are served from: an [`AsyncWorld`] handle
/// and the [`ScriptExposure`] every call is checked against.
///
/// Held by an out-of-process backend for the duration of a run, because there
/// the script and the world are in different realms and each call is a round
/// trip. The embedded engine has no use for it: being synchronous, it evaluates
/// inside one exclusive world access and reaches the `&mut World` directly.
#[derive(Clone)]
pub struct WorldBridge {
	world: AsyncWorld,
	exposure: ScriptExposure,
}

impl WorldBridge {
	/// A bridge serving `exposure`'s reach against `world`.
	pub fn new(world: AsyncWorld, exposure: ScriptExposure) -> Self {
		Self { world, exposure }
	}

	/// Perform one call and produce its reply.
	///
	/// Infallible: a refusal or a bad identifier is a [`WorldReply::Err`] the
	/// script can catch, not a failure of the run.
	pub async fn serve(&self, call: WorldCall) -> WorldReply {
		let exposure = self.exposure.clone();
		self.world
			.with(move |world| call.execute(world, &exposure))
			.await
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

	/// The bridge is the async face of the same executor, so a call served
	/// through it lands on the world it was built from.
	#[beet_core::test]
	async fn serves_a_call_against_the_live_world() {
		let mut world = world();
		// spawned as a task rather than polled inline: the bridge resolves at a
		// sync point, which drives the executor's tasks, not this one.
		world
			.run_async_then(|world| async move {
				WorldBridge::new(world, ScriptExposure::default())
					.serve_line(WorldCall {
						id: 0,
						op: WorldOp::Spawn {
							components: serde_json::json!({ "Name": "ada" })
								.as_object()
								.unwrap()
								.clone(),
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
