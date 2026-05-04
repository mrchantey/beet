use crate::ssh::*;
use beet_core::prelude::*;

/// A connected SSH client session backed by async channels.
#[derive(BundleEffect)]
pub struct SshSession {
	pub(crate) to_server: async_channel::Sender<SshData>,
	pub(crate) from_server: async_channel::Receiver<SshData>,
}

impl SshSession {
	fn effect(self, entity: &mut EntityWorldMut) {
		let to_server = self.to_server.clone();
		let from_server = self.from_server;

		entity
			.observe_any(
				move |ev: On<SshDataSend>,
				      mut commands: AsyncCommands|
				      -> Result {
					let to_server = to_server.clone();
					let data = ev.event().clone();
					commands.run_local(async move |_| {
						// forward data to server channel
						to_server
							.send(data.take())
							.await
							.unwrap_or_else(|err| error!("{:?}", err));
					});
					Ok(())
				},
			)
			.run_async_local(async move |entity| {
				while let Ok(data) = from_server.recv().await {
					entity.trigger_target_then(SshDataRecv(data)).await;
				}
				// channel closed, session ended
			})
			.trigger_target(SshSessionReady);
	}

	/// Connects to an SSH server and returns a ready [`SshSession`].
	#[allow(unused_variables)]
	pub async fn connect(
		addr: impl AsRef<str>,
		user: &str,
		password: &str,
	) -> Result<SshSession> {
		cfg_if! {
			if #[cfg(all(feature = "russh_client", not(target_arch = "wasm32")))] {
				super::impl_russh_client::connect_russh(addr, user, password).await
			} else {
				panic!(
					"SSH client requires 'russh_client' feature on non-wasm32 targets"
				)
			}
		}
	}

	/// Returns an [`OnSpawn`] callback that connects and inserts the session.
	pub fn insert_on_connect(
		addr: impl AsRef<str>,
		user: &str,
		password: &str,
	) -> OnSpawn {
		let addr = addr.as_ref().to_owned();
		let user = user.to_owned();
		let password = password.to_owned();
		OnSpawn::new_async_local(async move |entity| -> Result {
			let session = SshSession::connect(addr, &user, &password).await?;
			entity.insert_then(session).await;
			Ok(())
		})
	}
}

/// Triggered after an [`SshSession`] is connected and ready.
#[derive(EntityTargetEvent)]
pub struct SshSessionReady;
