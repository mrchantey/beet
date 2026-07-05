//! `RespondMultiModal`: the single tool the perceive-act agent calls each cycle, fanning out
//! to the three capability routes (`set-emotion`, `speak-text`, `apply-heading`)
//! concurrently and awaiting them all, so one model call per photo is the whole
//! turn and the next photo waits for the body to finish moving.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// React to the photo you just saw: set your face, say a line out loud and
/// choose where to drive, all at once.
///
/// Dispatches the three capability routes concurrently through the agent's own
/// router, so whichever client serves each capability (a bound head/body, or
/// the local handler) does its work in parallel; the call resolves when the
/// slowest finishes. Face and speech failures are logged and tolerated (a
/// missing client must not stop the robot's little life); the heading result
/// is reported to the model so it knows whether it actually moved.
#[action(route = "respond-multi-modal")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn RespondMultiModal(cx: ActionContext<RespondMultiModalInput>) -> Result<String> {
	let RespondMultiModalInput {
		emotion,
		say,
		heading,
	} = cx.input;
	// the model latency: time since the cycle's photo landed in the window
	let (cycle, model_secs) = cx
		.caller
		.world()
		.with(|world| {
			world
				.get_resource::<CycleClock>()
				.map(|clock| {
					(clock.cycle, clock.photo_at.elapsed().as_secs_f32())
				})
				.unwrap_or((0, 0.0))
		})
		.await;
	info!(
		"cycle {cycle}: {emotion:?} | \"{say}\" | {heading:?} (model {model_secs:.2}s)"
	);
	// opt-in via [`SequentialResponse`] on this action: express the face and finish
	// the spoken line before driving, so the robot never moves while it is still
	// talking; otherwise the three fan out at once.
	let sequential = cx.caller.get_cloned::<SequentialResponse>().await.is_ok();
	let started = Instant::now();
	// each capability call, timed for the summary log below.
	let run = |path: &'static str, body: String| {
		let caller = cx.caller.clone();
		async move {
			let started = Instant::now();
			let result = call_capability(&caller, path, body).await;
			(path, started.elapsed(), result)
		}
	};
	let face =
		("set-emotion", serde_json::to_string(&SetEmotionInput { emotion })?);
	let speak =
		("speak-text", serde_json::to_string(&SpeakTextInput { text: say })?);
	let drive =
		("apply-heading", serde_json::to_string(&ApplyHeadingInput { heading })?);
	let outcomes = if sequential {
		// face + speech first: `call_capability` waits each reply's settle, so the
		// spoken line has ended; only then the drive step and its settle.
		let mut outcomes =
			async_ext::join_all([run(face.0, face.1), run(speak.0, speak.1)])
				.await;
		outcomes.push(run(drive.0, drive.1).await);
		outcomes
	} else {
		async_ext::join_all([
			run(face.0, face.1),
			run(speak.0, speak.1),
			run(drive.0, drive.1),
		])
		.await
	};

	let timings = outcomes
		.iter()
		.map(|(path, elapsed, _)| format!("{path} {:.2}s", elapsed.as_secs_f32()))
		.collect::<Vec<_>>()
		.join(" | ");
	info!(
		"cycle {cycle}: acted in {:.2}s ({timings})",
		started.elapsed().as_secs_f32()
	);

	// tolerate face/speech failures, report the heading result to the model
	let mut failures = Vec::new();
	for (path, _, result) in outcomes {
		if let Err(err) = result {
			warn!("{path} failed: {err}");
			failures.push(format!("{path} failed: {err}"));
		}
	}
	match failures.is_empty() {
		true => "done".to_string(),
		false => failures.join(", "),
	}
	.xok()
}

/// Ceiling on a single capability call: long enough for a full spoken line or
/// drive step, short enough that a wedged client (eg a half-open socket) cannot
/// stall the robot's life.
const CAPABILITY_TIMEOUT: Duration = Duration::from_secs(30);

/// Call a capability route on the agent's router with a JSON body.
///
/// A body that cannot block on its own effect (the esp robot: no async-timer
/// waker in its handler task) replies with a [`SettleTime`] instead, and the
/// wait happens here, so the next photo still only follows a settled robot.
async fn call_capability(
	caller: &AsyncEntity,
	path: &str,
	body: String,
) -> Result {
	let response = async_ext::timeout(
		CAPABILITY_TIMEOUT,
		caller.call_detached(
			route_action(),
			Request::get(path)
				.with_body(body)
				.with_header::<header::ContentType>(MediaType::Json),
		),
	)
	.await
	.map_err(|_| bevyhow!("timed out after {CAPABILITY_TIMEOUT:?}"))??
	.into_result()
	.await?;
	if let Ok(settle) = response.json::<SettleTime>().await {
		let settle = settle.duration_or_zero();
		if !settle.is_zero() {
			time_ext::sleep(settle).await;
		}
	}
	Ok(())
}

/// Opt-in marker on a [`RespondMultiModal`] action: express the face and finish
/// speaking before driving (and await the movement), rather than fanning all
/// three out at once. Spawn it beside the action, eg
/// `<RespondMultiModal {SequentialResponse}/>`.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct SequentialResponse;

/// Everything the robot does with one photo: the face to wear, the line to say
/// and the direction to drive, applied simultaneously.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct RespondMultiModalInput {
	/// The expression to show on the face, matching how you feel about what
	/// you see.
	pub emotion: Emotion,
	/// One short line to say out loud, in character.
	pub say: String,
	/// The direction to drive next. Prefer `Forward`, turning `Left` or
	/// `Right` only to avoid an obstacle.
	pub heading: Heading,
}

#[cfg(test)]
mod test {
	use super::*;

	/// Drive one `respond-multi-modal` call through a router serving `set-emotion`
	/// and `apply-heading` (but deliberately no `speak-text`, so a failed
	/// capability is tolerated), optionally in `sequential` mode. Asserts both the
	/// emotion and the heading are recorded on their route entities — the fan-out
	/// completes either way.
	async fn drive_and_assert(sequential: bool) {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ThreadPlugin::default()));
		let agent = app.world_mut().spawn(Router).id();
		let emotion_entity =
			app.world_mut().spawn((SetEmotion, ChildOf(agent))).id();
		let heading_entity =
			app.world_mut().spawn((ApplyHeading, ChildOf(agent))).id();
		let action =
			app.world_mut().spawn((RespondMultiModal, ChildOf(agent))).id();
		if sequential {
			app.world_mut().entity_mut(action).insert(SequentialResponse);
		}
		app.world_mut().flush();

		app.world_mut()
			.entity_mut(agent)
			.run_async_local(|agent| async move {
				agent
					.call_detached(
						route_action(),
						Request::get("respond-multi-modal")
							.with_body(
								serde_json::to_string(&RespondMultiModalInput {
									emotion: Emotion::Joy,
									say: "onward!".into(),
									heading: Heading::Left,
								})
								.unwrap(),
							)
							.with_header::<header::ContentType>(MediaType::Json),
					)
					.await?
					.into_result()
					.await?;
				Ok(())
			});

		app_ext::update_until(&mut app, move |world| {
			world.get::<Heading>(heading_entity).is_some()
		})
		.await
		.xpect_true();

		app.world_mut()
			.get::<Emotion>(emotion_entity)
			.copied()
			.xpect_eq(Some(Emotion::Joy));
		app.world_mut()
			.get::<Heading>(heading_entity)
			.copied()
			.xpect_eq(Some(Heading::Left));
	}

	/// The default: one call fans out to the capability routes concurrently, the
	/// local handlers recording the emotion and heading.
	#[beet_core::test]
	async fn fans_out_to_capabilities() { drive_and_assert(false).await; }

	/// [`SequentialResponse`] expresses the face + speech first, then drives — but
	/// the fan-out still completes, recording both the emotion and the heading.
	#[beet_core::test]
	async fn sequential_still_records_all() { drive_and_assert(true).await; }
}
