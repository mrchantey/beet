//! The socket-client primitives every perceive-act client shares.
//!
//! A client (the mock head/body, the wgpu body, the wasm browser head) connects to the
//! agent, announces its role via `whoami`, and serves its capabilities. These pieces -
//! the connect-with-retry, the role marker, and the `whoami` answer - are identical
//! across all of them and ride only the socket core (`Socket::connect` routes to the
//! web-sys transport on wasm and tungstenite natively), so they live here rather than
//! in the `thread`-gated agent module.
use beet_core::prelude::*;
use beet_net::sockets::*;

/// The role a client serves ("head"/"body"), on the client's socket root. Read by
/// [`WhoAmI`] to answer the agent's handshake.
#[derive(Debug, Clone, Component, Reflect, Deref)]
#[reflect(Component)]
pub struct ClientRole(pub SmolStr);

/// Announce which role this client serves, so the agent binds the matching
/// capabilities to the connection. Reads the [`ClientRole`] on the client's root.
#[action(route = "whoami")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn WhoAmI(cx: ActionContext<()>) -> Result<String> {
	cx.caller
		.with_state::<AncestorQuery<&ClientRole>, _>(|entity, roles| {
			roles.get(entity).map(|role| role.0.to_string())
		})
		.await?
}

/// Connect to `url`, retrying until the server's listener is up (an in-process client
/// races the scene that spawns the server; a browser or device retries until the agent
/// is reachable), then insert the [`Socket`].
pub fn connect_with_retry(url: impl Into<String>) -> OnSpawn {
	let url = url.into();
	OnSpawn::new_async_local(async move |entity| -> Result {
		for attempt in 0..50u32 {
			match Socket::connect(&url).await {
				Ok(socket) => {
					entity.insert(socket).await?;
					return Ok(());
				}
				// keep retrying until the last attempt, then surface the error.
				Err(err) if attempt == 49 => return Err(err),
				Err(_) => time_ext::sleep_millis(100).await,
			}
		}
		Ok(())
	})
}
