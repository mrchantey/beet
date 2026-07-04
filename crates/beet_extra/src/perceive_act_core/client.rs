//! The socket-client primitives every perceive-act client shares.
//!
//! A client (the mock head/body, the wgpu body, the wasm browser head) connects to the
//! agent via a [`PersistentSocket`] (dial with backoff, reconnect on drop), announces
//! its role via `whoami`, and serves its capabilities. The role marker and the `whoami`
//! answer are identical across all of them and ride only the socket core, so they live
//! here rather than in the `thread`-gated agent module.
use beet_core::prelude::*;

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

