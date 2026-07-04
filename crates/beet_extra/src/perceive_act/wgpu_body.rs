//! The wgpu render body for the perceive-act demo (v2).
//!
//! [`WgpuBody`] is the render swap for v1's logging [`MockBody`](super::MockBody): the
//! same socket client on its own root (route-tree isolation), but serving `apply-heading`
//! by driving an on-screen 3d fox instead of logging. It hosts the 3d scene (a `Scene3d`
//! + `<Foxie>` + `CharacterDrive`) alongside the client, so `<WgpuBody/>` is the one
//! change from `main-v1` to `main-v2`. [`DriveFox`] maps each [`Heading`] onto the fox's
//! `DifferentialDrive` through the agnostic `SetDrive` leaf, so the identical command
//! walks this fox and (v3) the real robot.
use super::*;
use crate::beet::prelude::*;
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::sockets::*;
use beet_router::prelude::*;

/// The forward speed of a `Forward` step, mm/s (one mm renders as one world unit).
const STEP_SPEED: f32 = 40.;
/// The turn rate of a `Left`/`Right` step, deg/s (positive = left).
const TURN_RATE: f32 = 90.;
/// How long a step drives before the fox stops, matching the reference patrol
/// (`examples/action/wgpu-action.bsx`).
const STEP_DURATION: Duration = Duration::from_secs(1);

/// The wgpu body's `apply-heading`: drive the on-screen fox one step, then stop.
///
/// Maps the [`Heading`] onto the fox's `DifferentialDrive` through the agnostic
/// [`SetDrive`] leaf (`Forward` drives straight, `Left`/`Right` turn in place), held for
/// [`STEP_DURATION`] by an [`EndInDuration`] then zeroed - the same `SetDrive` +
/// `EndInDuration` sequence the reference patrol uses, so the one command drives this fox
/// and the v3 robot. Shares the `apply-heading` route + [`ApplyHeadingInput`] with the
/// mock [`ApplyHeading`](super::ApplyHeading); this is "the mock plus the actual effect".
#[action(route = "apply-heading")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn DriveFox(cx: ActionContext<ApplyHeadingInput>) -> Result<()> {
	let heading = cx.input.heading;
	info!("driving: {heading:?}");
	let (linear, angular) = match heading {
		Heading::Forward => (STEP_SPEED, 0.),
		Heading::Left => (0., TURN_RATE),
		Heading::Right => (0., -TURN_RATE),
	};
	// the single fox this body renders; skip the step if the glb has not finished
	// loading yet (the model trails several round-trips, so in practice it is up).
	let world = cx.world();
	let Some(fox) = world
		.with_state::<Query<Entity, With<CharacterDrive>>, _>(|foxes| {
			foxes.iter().next()
		})
		.await
	else {
		warn!("apply-heading: no fox body loaded yet, skipping the drive step");
		return Ok(());
	};
	// spawn the drive step as an action of the fox, run it (which drives the fox for
	// the dwell then stops it), then despawn so steps never stack.
	let step = world
		.spawn((ActionOf(fox), Sequence::<(), ()>::default(), children![
			SetDrive::new(linear, angular),
			EndInDuration::pass(STEP_DURATION),
			SetDrive::new(0., 0.),
		]))
		.await;
	step.call::<(), Outcome>(()).await?;
	step.despawn().await?;
	Ok(())
}

/// The in-process wgpu body: a socket client that connects to the agent, announces the
/// `body` role, and drives an on-screen 3d fox off `apply-heading` ([`DriveFox`]). The
/// render swap for v1's [`MockBody`](super::MockBody) - the only change from v1 to v2.
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
	// `apply-heading` via `DriveFox`.
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

	/// Calling `apply-heading` on the body's [`DriveFox`] drives the fox: it maps the
	/// [`Heading`] onto a [`SetDrive`] step, which `CharacterDrive` integrates into the
	/// fox's `Transform`, so the fox leaves the origin. The socket forward that delivers
	/// the call is covered by `binds_role_and_forwards_call`; this covers the effect.
	#[beet_core::test]
	async fn drives_the_fox_off_a_heading() {
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

		// fire the `apply-heading` call the socket would deliver, then tick the app.
		app.world_mut().entity_mut(body).run_async_local(|body| async move {
			body.call::<ApplyHeadingInput, ()>(ApplyHeadingInput {
				heading: Heading::Forward,
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
