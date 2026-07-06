//! The wgpu render body for the perceive-act demo (v2).
//!
//! [`WgpuBody`] is the render swap for v1's logging [`MockBody`](super::MockBody): the
//! same socket client on its own root (route-tree isolation), but serving `drive`
//! by driving an on-screen 3d fox instead of logging. It hosts the 3d scene (a `Scene3d`
//! + `<Foxie>` + `CharacterDrive`) alongside the client, so `<WgpuBody/>` is the one
//! change from `main-v1` to `main-v2`. [`DriveFox`] feeds each agent-chosen
//! [`DifferentialDrive`] into the fox through the agnostic `SetDrive` leaf, so the
//! identical command walks this fox and (v3) the real robot.
use super::*;
use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::sockets::*;
use beet_router::prelude::*;

/// The wgpu body's `drive`: drive the on-screen fox at the commanded velocity for the
/// commanded duration, then stop.
///
/// Authors the agent-chosen [`DriveForDuration`] as an action of the fox, which brings the
/// canonical [`DriveForDurationAction`] — the `SetDrive` + `EndInDuration` + `SetDrive(0, 0)`
/// step that drives this fox and (v3) the real robot off the one command. Shares the `drive`
/// route + [`DriveForDuration`] input with the mock [`RecordDrive`](super::RecordDrive); this
/// is "the mock plus the actual effect".
#[action(route = "drive")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn DriveFox(cx: ActionContext<DriveForDuration>) -> Result<()> {
	let command = cx.input;
	info!(
		"driving: lin={} ang={} for {:.2}s",
		command.drive.linear.as_mm_per_sec(),
		command.drive.angular.as_deg_per_sec(),
		command.duration.as_secs_f32()
	);
	// the single fox this body renders; skip the step if the glb has not finished
	// loading yet (the model trails several round-trips, so in practice it is up).
	let world = cx.world();
	let Some(fox) = world
		.with_state::<Query<Entity, With<CharacterDrive>>, _>(|foxes| {
			foxes.iter().next()
		})
		.await
	else {
		warn!("drive: no fox body loaded yet, skipping the drive step");
		return Ok(());
	};
	// author the command as an action of the fox: `DriveForDuration` brings the canonical
	// `DriveForDurationAction`, which drives the fox for the duration then stops. Despawned
	// after so steps never stack.
	let step = world.spawn((ActionOf(fox), command)).await;
	step.call::<(), Outcome>(()).await?;
	step.despawn().await?;
	Ok(())
}

/// The in-process wgpu body: a socket client that connects to the agent, announces the
/// `body` role, and drives an on-screen 3d fox off `drive` ([`DriveFox`]). The render
/// swap for v1's [`MockBody`](super::MockBody) - the only change from v1 to v2.
///
/// Spawns two roots: its own detached socket root (so the `body` capability stays
/// isolated from the agent's identically-named routes) and the visible 3d scene the fox
/// drives in (mirroring `examples/action/wgpu-action.bsx`).
#[template(system)]
pub fn WgpuBody(
	/// The agent's socket url, eg `ws://127.0.0.1:8338`.
	#[prop(into)]
	url: String,
	mut commands: Commands,
) -> impl Bundle {
	// the socket client: its own root, connecting with a bounded retry and serving
	// `drive` via `DriveFox`.
	commands.spawn((
		PersistentSocket::new(url),
		ExchangeSocket::json(),
		Router,
		ClientRole(SmolStr::new("body")),
		children![WhoAmI, DriveFox],
	));
	// the visible scene the fox drives in: a look-at camera over a ground plane.
	rsx! {
		<Scene3d>
			<AppWindow/>
			<Camera3dLookAt x=0. y=42. z=60./>
			<Lighting3d/>
			<Ground3d/>
			<Foxie scale=0.1 {CharacterDrive::default()}/>
		</Scene3d>
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Calling `drive` on the body's [`DriveFox`] drives the fox: it feeds the
	/// [`DifferentialDrive`] into a [`SetDrive`] step, which `CharacterDrive` integrates
	/// into the fox's `Transform`, so the fox leaves the origin. The socket forward that
	/// delivers the call is covered by `binds_role_and_forwards_call`; this covers the effect.
	#[beet_core::test]
	async fn drives_the_fox_off_a_command() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin, ActionPlugin))
			.add_plugins(character_drive_plugin);
		// a bare render body: `CharacterDrive` requires `DifferentialDrive`, the command
		// `SetDrive` writes and `drive_character` integrates each frame.
		let fox = app
			.world_mut()
			.spawn((CharacterDrive::default(), Transform::default()))
			.id();
		let body = app.world_mut().spawn(DriveFox).id();
		app.world_mut().flush();

		// fire the `drive` call the socket would deliver, then tick the app.
		app.world_mut().entity_mut(body).run_async_local(|body| async move {
			body.call::<DriveForDuration, ()>(DriveForDuration {
				drive: DifferentialDrive::new(40., 0.),
				duration: Duration::from_secs(1),
			})
			.await?;
			Ok(())
		});

		// drive until the fox has stepped off the origin along its forward axis.
		app_ext::update_until(&mut app, |world| {
			world
				.get::<Transform>(fox)
				.map(|transform| transform.translation.length() > 1.0)
				.unwrap_or(false)
		})
		.await
		.xpect_true();
	}
}
