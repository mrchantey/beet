use beet_core::prelude::*;

/// Triggered after an SSH client session is connected and ready.
#[derive(EntityTargetEvent)]
pub struct SshSessionReady;

/// Helpers for connecting to an SSH server.
pub struct SshSession;

impl SshSession {
	/// Returns an [`OnSpawn`] that connects with credentials and sets up data flow.
	pub fn insert_on_connect(
		addr: impl AsRef<str>,
		user: impl AsRef<str>,
		password: impl AsRef<str>,
	) -> OnSpawn {
		let addr = addr.as_ref().to_owned();
		let user = user.as_ref().to_owned();
		let password = password.as_ref().to_owned();
		OnSpawn::new_async_local(async move |_entity| -> Result {
			#[cfg(all(feature = "russh_client", not(target_arch = "wasm32")))]
			{
				super::impl_russh_client::connect_and_setup_entity(
					_entity,
					addr,
					Some(user),
					Some(password),
				)
				.await
			}
			#[cfg(not(all(
				feature = "russh_client",
				not(target_arch = "wasm32")
			)))]
			{
				let _ = (addr, user, password);
				Err(bevyhow!(
					"SSH client requires the 'russh_client' feature on non-wasm32 targets"
				))
			}
		})
	}

	/// Returns an [`OnSpawn`] that connects anonymously (no credentials).
	pub fn insert_anon(addr: impl AsRef<str>) -> OnSpawn {
		let addr = addr.as_ref().to_owned();
		OnSpawn::new_async_local(async move |_entity| -> Result {
			#[cfg(all(feature = "russh_client", not(target_arch = "wasm32")))]
			{
				super::impl_russh_client::connect_and_setup_entity(
					_entity, addr, None, None,
				)
				.await
			}
			#[cfg(not(all(
				feature = "russh_client",
				not(target_arch = "wasm32")
			)))]
			{
				let _ = addr;
				Err(bevyhow!(
					"SSH client requires the 'russh_client' feature on non-wasm32 targets"
				))
			}
		})
	}

	/// Connects to an SSH server, returning an error if authentication fails.
	///
	/// This is a low-level helper used in tests; prefer [`insert_on_connect`].
	#[cfg(all(feature = "russh_client", not(target_arch = "wasm32")))]
	pub async fn connect_raw(
		addr: impl AsRef<str>,
		user: Option<&str>,
		password: Option<&str>,
	) -> Result<()> {
		super::impl_russh_client::test_connect(addr, user, password).await
	}
}
