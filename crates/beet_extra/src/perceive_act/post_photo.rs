//! `PostPhoto`: the camera turn of the perceive-act loop. Each cycle it captures
//! a photo through the `take-photo` route (a bound head client serves it, else
//! the local fixtures handler) and pushes it straight into the thread window as
//! this actor's image post, so the agent's single model call sees the photo
//! directly, with no separate vision round-trip.
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// The camera actor's behavior: capture a photo, prune the window, post the
/// photo as this actor's turn.
///
/// Spread onto a `User`-kind `<CreateActor>` before the agent actor, so each
/// `Sequence` iteration begins with a fresh photo in the window as a user-role
/// image message. Pruning keeps the endless loop's request bounded: the seed
/// prompt always survives, older photos drop out first ([`ThreadWindow::prune`]).
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Action<(), Outcome> = Action::new_async(post_photo_action))]
pub struct PostPhoto {
	/// How many recent non-seed posts to keep in the window.
	pub keep_posts: usize,
	/// How many recent photos to keep in the window.
	pub keep_media: usize,
}

impl Default for PostPhoto {
	fn default() -> Self {
		Self {
			// ~8 cycles of call/output pairs plus the current + previous photo
			// (photos dominate request bytes; two are enough to see change)
			keep_posts: 24,
			keep_media: 2,
		}
	}
}

/// Marks the start of each perceive-act cycle, for the per-stage latency logs:
/// [`PostPhoto`] stamps it, `Act` reads it to report the model latency.
#[derive(Debug, Resource)]
pub struct CycleClock {
	/// The current cycle, counting from 1.
	pub cycle: u64,
	/// When the current cycle's photo landed in the window.
	pub photo_at: Instant,
}

async fn post_photo_action(cx: ActionContext) -> Result<Outcome> {
	let config = cx.caller.get_cloned::<PostPhoto>().await?;
	let started = Instant::now();
	// capture through the router: a bound head serves it, else the local handler.
	// The loop's heartbeat: a failed capture (eg no head connected yet, so no
	// photo source) retries rather than failing the thread, so the robot
	// patiently waits for its eyes instead of dying.
	let mut attempt = 0u32;
	let media = loop {
		match capture_via_router(&cx.caller).await {
			Ok(media) => break media,
			Err(err) => {
				if attempt % 10 == 0 {
					warn!("take-photo failed, retrying: {err}");
				}
				attempt += 1;
				time_ext::sleep_millis(500).await;
			}
		}
	};
	let capture_secs = started.elapsed().as_secs_f32();
	let size_kb = media.bytes().len() / 1024;

	// prune old posts, then push the photo as this actor's turn
	cx.caller
		.with_state::<ThreadWindowQuery, _>(move |entity, mut windows| -> Result {
			let actor_id = windows.actor_id(entity)?;
			let thread_id = windows.thread_id(entity)?;
			let mut window = windows.window_mut(entity)?;
			window.prune(config.keep_posts, config.keep_media);
			window.upsert_post(AgentPost::new_bytes(
				actor_id,
				thread_id,
				media.media_type().clone(),
				media.bytes().to_vec(),
				None,
				PostStatus::Completed,
			));
			Ok(())
		})
		.await??;

	// stamp the cycle clock so `Act` can report the model latency
	let (cycle, previous_photo_at) = cx
		.caller
		.world()
		.with(|world| {
			let previous = world
				.get_resource::<CycleClock>()
				.map(|clock| (clock.cycle, clock.photo_at));
			let cycle = previous.map(|(cycle, _)| cycle + 1).unwrap_or(1);
			world.insert_resource(CycleClock {
				cycle,
				photo_at: Instant::now(),
			});
			(cycle, previous.map(|(_, photo_at)| photo_at))
		})
		.await;
	// the previous cycle's total is the demo's headline number
	match previous_photo_at {
		Some(photo_at) => info!(
			"cycle {cycle}: photo captured ({size_kb}KB in {capture_secs:.2}s, previous cycle {:.2}s)",
			photo_at.elapsed().as_secs_f32() - capture_secs,
		),
		None => info!(
			"cycle {cycle}: photo captured ({size_kb}KB in {capture_secs:.2}s)"
		),
	}
	Ok(Pass(()))
}

/// Ceiling on one capture attempt, so a wedged head (eg a half-open socket)
/// fails into the retry loop instead of stalling the cycle forever.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(20);

/// One capture attempt through the agent's own `take-photo` route.
async fn capture_via_router(caller: &AsyncEntity) -> Result<MediaBytes> {
	async_ext::timeout(
		CAPTURE_TIMEOUT,
		caller.call_detached(route_action(), Request::get("take-photo")),
	)
	.await
	.map_err(|_| bevyhow!("timed out after {CAPTURE_TIMEOUT:?}"))??
	.into_result()
	.await?
	.into_media_bytes()
	.await
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;

	/// Each camera turn captures through `take-photo` and lands exactly one new
	/// image post in the window, authored by the camera actor.
	#[beet_core::test]
	async fn posts_a_photo_each_turn() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ThreadPlugin::default()))
			.init_resource::<PhotoStream>();
		let store = BlobStore::temp();
		for index in 0..2u8 {
			store
				.insert(
					&SmolPath::from(format!("assets/floor-photos/{index}.jpg")),
					vec![index],
				)
				.await
				.unwrap();
		}
		let root = app.world_mut().spawn((Router, store)).id();
		app.world_mut().spawn((TakePhoto, ChildOf(root)));
		let thread = app
			.world_mut()
			.spawn((Thread::default(), ChildOf(root)))
			.id();
		let camera = app
			.world_mut()
			.spawn((Actor::user(), PostPhoto::default(), ChildOf(thread)))
			.id();
		app.world_mut().flush();
		ThreadWindow::reduce_now(app.world_mut());

		app.world_mut()
			.entity_mut(camera)
			.run_async_local(|camera| async move {
				camera.call::<(), Outcome>(()).await?;
				Ok(())
			});
		app_ext::update_until(&mut app, move |world| {
			world
				.get::<ThreadWindow>(thread)
				.is_some_and(|window| !window.is_empty())
		})
		.await
		.xpect_true();

		let window = app.world().get::<ThreadWindow>(thread).unwrap();
		window.posts().len().xpect_eq(1);
		window.posts()[0].media_type().is_image().xpect_true();
	}
}
