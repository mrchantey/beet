//! `RespondMultiModalAction`: the single tool the perceive-act agent calls each cycle,
//! fanning out to the three capability routes (`set-emotion`, `speak-text`, `drive`) and
//! awaiting them all, so one model call per photo is the whole turn and the next photo
//! waits for the body to finish moving.
use super::*;
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// React to the photo you just saw: set your face, say a line out loud and drive off,
/// in one model call.
///
/// Dispatches the three capability routes through the agent's own router, so whichever
/// client serves each capability (a bound head/body, or the local handler) does its
/// work; the call resolves when the last finishes. By default the face + speech finish
/// before the drive starts, so the robot never moves while it is still talking; the
/// input's `parallel` flag fans all three out at once instead. Face and speech failures
/// are logged and tolerated (a missing client must not stop the robot's little life);
/// the drive result is reported to the model so it knows whether it actually moved.
///
/// Reads a [`RespondMultiModal`] config off its own caller each call: its
/// `max_drive_duration` clamps how long a single response may drive.
#[action(route = "respond-multi-modal")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn RespondMultiModalAction(
	cx: ActionContext<RespondMultiModalInput>,
) -> Result<String> {
	let RespondMultiModalInput {
		emotion,
		say,
		mut drive,
		parallel,
	} = cx.input;
	// clamp the commanded drive to this action's configured ceiling, if any.
	if let Some(max) = cx
		.caller
		.get_cloned::<RespondMultiModal>()
		.await
		.ok()
		.and_then(|config| config.max_drive_duration)
	{
		drive.duration = drive.duration.min(max);
	}
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
		"cycle {cycle}: {emotion:?} | \"{say}\" | drive lin={} ang={} for {:.2}s (model {model_secs:.2}s)",
		drive.drive.linear.as_mm_per_sec(),
		drive.drive.angular.as_deg_per_sec(),
		drive.duration.as_secs_f32()
	);
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
	// the clamped, agent-chosen drive command sent to the `drive` capability route.
	let drive = ("drive", serde_json::to_string(&drive)?);
	let outcomes = if parallel {
		// fan all three out at once: the robot may move while it is still talking.
		async_ext::join_all([
			run(face.0, face.1),
			run(speak.0, speak.1),
			run(drive.0, drive.1),
		])
		.await
	} else {
		// face + speech first: `call_capability` waits each reply's settle, so the
		// spoken line has ended; only then the drive step and its settle.
		let mut outcomes =
			async_ext::join_all([run(face.0, face.1), run(speak.0, speak.1)])
				.await;
		outcomes.push(run(drive.0, drive.1).await);
		outcomes
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

	// tolerate face/speech failures, report the drive result to the model
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

/// Per-response config read off the [`RespondMultiModalAction`]'s own caller each call
/// (like the esp body reads its drive-step config): a ceiling clamping how long the
/// agent may drive in one response, so a demo never sends the fox or robot careening.
/// `None` means no cap. Spawn it beside the action, eg
/// `<RespondMultiModalAction {RespondMultiModal{max_drive_duration:"2s"}}/>`.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RespondMultiModal {
	/// The longest a single response may drive, clamping `drive.duration`;
	/// `None` for no cap.
	pub max_drive_duration: Option<Duration>,
}

/// Everything the robot does with one photo: the face to wear, the line to say
/// and how to drive next.
#[derive(Reflect, serde::Deserialize, serde::Serialize)]
pub struct RespondMultiModalInput {
	/// The expression to show on the face, matching how you feel about what
	/// you see.
	pub emotion: Emotion,
	/// One short line to say out loud, in character.
	pub say: String,
	/// How to drive next: a `drive` velocity (`linear` mm/s forward, `angular`
	/// deg/s turn, positive = left) held for `duration` seconds. Prefer driving
	/// forward; turn only to avoid an obstacle.
	pub drive: DriveForDuration,
	/// Whether to act all at once instead of finishing your spoken line before you
	/// start to drive. Usually keep this false: say your line, then drive, so you
	/// never move while still talking. Set true only when there is both a lot to say
	/// and a long way to drive, and doing both at once is the better expression.
	#[serde(default)]
	pub parallel: bool,
}

#[cfg(test)]
mod test {
	use super::*;

	/// Build an agent serving `set-emotion` + `drive` (but deliberately no `speak-text`,
	/// so a failed capability is exercised as tolerated), spawn a `RespondMultiModalAction`
	/// carrying `config`, run one `input` through it, and return the app plus the emotion
	/// and drive route entities so callers can assert what each recorded.
	async fn run_response(
		config: RespondMultiModal,
		input: RespondMultiModalInput,
	) -> (App, Entity, Entity) {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ThreadPlugin::default()));
		let agent = app.world_mut().spawn(Router).id();
		let emotion_entity =
			app.world_mut().spawn((SetEmotion, ChildOf(agent))).id();
		let drive_entity = app
			.world_mut()
			.spawn((DriveForDurationAction, ChildOf(agent)))
			.id();
		app.world_mut()
			.spawn((RespondMultiModalAction, config, ChildOf(agent)));
		app.world_mut().flush();

		let body = serde_json::to_string(&input).unwrap();
		app.world_mut()
			.entity_mut(agent)
			.run_async_local(move |agent| async move {
				agent
					.call_detached(
						route_action(),
						Request::get("respond-multi-modal")
							.with_body(body)
							.with_header::<header::ContentType>(MediaType::Json),
					)
					.await?
					.into_result()
					.await?;
				Ok(())
			});

		app_ext::update_until(&mut app, move |world| {
			world.get::<DriveForDuration>(drive_entity).is_some()
		})
		.await
		.xpect_true();
		(app, emotion_entity, drive_entity)
	}

	/// Drive one `respond-multi-modal` call, with `parallel` either fanning all three
	/// capabilities out or sequencing the drive after speech. Asserts both the emotion
	/// and the drive command are recorded on their route entities — the fan-out completes
	/// either way.
	async fn drive_and_assert(parallel: bool) {
		let (mut app, emotion_entity, drive_entity) = run_response(
			RespondMultiModal::default(),
			RespondMultiModalInput {
				emotion: Emotion::Joy,
				say: "onward!".into(),
				drive: DriveForDuration {
					drive: DifferentialDrive::new(40., 90.),
					duration: Duration::from_secs(1),
				},
				parallel,
			},
		)
		.await;
		app.world_mut()
			.get::<Emotion>(emotion_entity)
			.copied()
			.xpect_eq(Some(Emotion::Joy));
		app.world_mut()
			.get::<DriveForDuration>(drive_entity)
			.map(|command| command.drive)
			.xpect_eq(Some(DifferentialDrive::new(40., 90.)));
	}

	/// `parallel = true`: one call fans out to the capability routes at once, the
	/// local handlers recording the emotion and the drive command.
	#[beet_core::test]
	async fn fans_out_in_parallel() { drive_and_assert(true).await; }

	/// `parallel = false` (the default): the face + speech finish before the drive —
	/// but the fan-out still completes, recording both the emotion and the drive.
	#[beet_core::test]
	async fn sequences_speech_before_drive() { drive_and_assert(false).await; }

	/// `RespondMultiModal.max_drive_duration` clamps an over-long command so a demo body
	/// never careens: a 5s command under a 1s cap records 1s.
	#[beet_core::test]
	async fn clamps_drive_duration() {
		let (mut app, _emotion_entity, drive_entity) = run_response(
			RespondMultiModal {
				max_drive_duration: Some(Duration::from_secs(1)),
			},
			RespondMultiModalInput {
				emotion: Emotion::Joy,
				say: "onward!".into(),
				drive: DriveForDuration {
					drive: DifferentialDrive::new(40., 0.),
					duration: Duration::from_secs(5),
				},
				parallel: true,
			},
		)
		.await;
		app.world_mut()
			.get::<DriveForDuration>(drive_entity)
			.map(|command| command.duration)
			.xpect_eq(Some(Duration::from_secs(1)));
	}
}
