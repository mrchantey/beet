//! `PostPhoto`: the camera turn of the perceive-act loop. Each cycle it captures
//! a photo through the `take-photo` route (a bound head client serves it, else
//! the local fixtures handler) and pushes it straight into the thread records as
//! this actor's image post, so the agent's single model call sees the photo
//! directly, with no separate vision round-trip.
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// The camera actor's behavior: capture a photo and post it as this actor's
/// turn. Scene rotation is its own preceding step
/// ([`RotateScene`](super::RotateScene)).
///
/// Spread onto a `User`-kind `<CreateActor>` before the agent actor, so each
/// `Sequence` iteration begins with a fresh photo in the records as a user-role
/// image message. The records keep every photo: the agent's [`KeepBlobs`]
/// bounds how many it carries, listing the older ones, and with them what the
/// window holds in memory.
#[action]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn PostPhoto(cx: ActionContext) -> Result<Outcome> {
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

	// append the photo as this actor's turn
	cx.caller
		.with_state::<ThreadRecordsQuery, _>(
			move |entity, mut thread_records| -> Result {
				let actor_id = thread_records.actor_id(entity)?;
				let thread_id = thread_records.thread_id(entity)?;
				let (media_type, bytes) = media.take();
				thread_records.push_post(
					entity,
					AgentPost::new_media(
						actor_id,
						thread_id,
						media_type,
						bytes,
						None,
						PostStatus::Completed,
					),
				)
			},
		)
		.await??;

	// stamp the cycle clock so `RespondMultiModal` can report the model latency
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

/// Marks the start of each perceive-act cycle, for the per-stage latency logs:
/// [`PostPhoto`] stamps it, `RespondMultiModal` reads it to report the model
/// latency.
#[derive(Debug, Resource)]
pub struct CycleClock {
	/// The current cycle, counting from 1.
	pub cycle: u64,
	/// When the current cycle's photo landed in the records.
	pub photo_at: Instant,
}

/// Ceiling on one capture attempt, so a wedged head (eg a half-open socket)
/// fails into the retry loop instead of stalling the cycle forever.
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(20);

/// One capture attempt through the agent's own `take-photo` route.
async fn capture_via_router(caller: &AsyncEntity) -> Result<MediaBytes> {
	async_ext::timeout(
		CAPTURE_TIMEOUT,
		caller.call_detached(Router::action(), Request::get("take-photo")),
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
	/// image post in the records, authored by the camera actor.
	#[beet_core::test]
	async fn posts_a_photo_each_turn() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ThreadPlugin::default()))
			.init_resource::<PhotoStream>();
		let store = BlobStore::temp();
		for index in 0..2u8 {
			store
				.insert(
					&FwdPath::new(format!("assets/floor-photos/{index}.jpg")).unwrap(),
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
		ThreadRecords::reduce_now(app.world_mut());

		app.world_mut().entity_mut(camera).run_async_local(
			|camera| async move {
				camera.call::<(), Outcome>(()).await?;
				Ok(())
			},
		);
		app_ext::update_until(&mut app, move |world| {
			world
				.get::<ThreadRecords>(thread)
				.is_some_and(|records| !records.is_empty())
		})
		.await
		.xpect_true();

		let records = app.world().get::<ThreadRecords>(thread).unwrap();
		records.posts().len().xpect_eq(1);
		matches!(
			records.posts()[0].as_agent_post(),
			Some(AgentPost::Media(view)) if view.media_type().is_image()
		)
		.xpect_true();
	}
}
