//! Turns, the words defined in the [`table`](crate::table) module docs: a
//! person's turn ([`UserInput`]), a wait on the records, and an agent's turn
//! cut short ([`InterruptTurn`], [`InterruptOnPost`]).
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;

/// Cuts short the running turn of the actor whose behavior entity it
/// targets, beet's own interrupt and the same for every provider.
///
/// A streamer's turn honours it: the model's stream is dropped, the post in
/// progress closed as [`PostStatus::Interrupted`], the turn's tool calls in
/// flight cancelled, each call without an output given one saying it was
/// interrupted, and the turn passes. Aimed at an actor not taking a turn, or
/// whose turn ignores it, ie a person's [`UserInput`], it does nothing.
#[derive(Debug, Clone, EntityEvent)]
pub struct InterruptTurn {
	/// The actor's behavior entity.
	pub entity: Entity,
}

impl InterruptTurn {
	/// Interrupt the turn of the behavior entity `entity`.
	pub fn new(entity: Entity) -> Self { Self { entity } }
}

/// An agent's manners: any post by another actor during its turn, that is not
/// a log, interrupts it ([`InterruptTurn`]), whatever shape the thread has.
///
/// On a `Sequence` it makes the thread semi duplex; under a `Parallel` it is
/// how an agent chooses to yield when another speaks.
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub struct InterruptOnPost;

/// A person's turn: a wait on the records, passing once the last word
/// ([`ThreadRecords::last_word`]) is its actor's.
///
/// It reads no composer: any post by its actor passes it, from whichever
/// window, so a post made while another actor works passes it the moment it
/// comes round. Spread it onto a User actor,
/// `<CreateActor name=".." kind="User" {UserInput}/>`.
#[action]
#[derive(Debug, Default, Clone, Copy, Component, Reflect)]
#[reflect(Component, Default)]
pub async fn UserInput(cx: ActionContext) -> Result<Outcome> {
	let (signal, posted) = OnceValue::oneshot();
	cx.caller.insert(PersonTurn(Some(signal))).await?;
	posted.wait().await;
	Ok(Pass(()))
}

/// A [`UserInput`] turn waiting, signalled and removed by [`pass_person_turns`]
/// once the last word is its actor's.
#[derive(Component)]
pub(crate) struct PersonTurn(Option<OnceValue<()>>);

/// An interruptible turn in progress on an agent's behavior entity, its
/// interrupt fired by the first [`InterruptTurn`].
#[derive(Component)]
pub(crate) struct AgentTurn {
	/// The newest post when the turn began: a post after it was made during
	/// the turn.
	since: Option<PostId>,
	/// Fired once, on the first [`InterruptTurn`].
	interrupt: Option<OnceValue<()>>,
}

/// Holds an agent's turn open for the action taking it.
///
/// Closed by [`close`](Self::close) when the action ends on its own. Dropped
/// unclosed, ie its call cancelled by a `Race` or a failing `Parallel`, it
/// closes the turn as interrupted in a task of its own, the drop guard pattern
/// of `beet_action`'s README.
pub(crate) struct TurnGuard {
	caller: AsyncEntity,
	spawner: AsyncSpawner,
	closed: bool,
}

impl TurnGuard {
	/// Begin the turn of the behavior entity `caller`, answering its guard
	/// and the future resolving once it is interrupted.
	pub(crate) async fn begin(
		caller: AsyncEntity,
	) -> Result<(Self, impl Future<Output = ()>)> {
		let (interrupt, interrupted) = OnceValue::oneshot();
		let spawner = caller
			.with_world(move |world, entity| -> Result<AsyncSpawner> {
				let since = world.with_state::<ThreadRecordsQuery, _>(
					|query| -> Result<Option<PostId>> {
						query.records(entity)?.last_post().map(Post::id).xok()
					},
				)?;
				world.entity_mut(entity).insert(AgentTurn {
					since,
					interrupt: Some(interrupt),
				});
				world.resource::<AsyncSpawner>().clone().xok()
			})
			.await??;
		let guard = Self {
			caller,
			spawner,
			closed: false,
		};
		(guard, interrupted.wait()).xok()
	}

	/// End the turn, closing what it left open when `interrupted`.
	pub(crate) async fn close(mut self, interrupted: bool) -> Result {
		self.closed = true;
		self.caller
			.with_world(move |world, entity| {
				Self::close_turn(world, entity, interrupted)
			})
			.await?
	}

	/// Remove the [`AgentTurn`] of `entity` and, when `interrupted`, close
	/// each of its actor's posts in progress as interrupted and answer each
	/// call of the turn left without an output.
	fn close_turn(
		world: &mut World,
		entity: Entity,
		interrupted: bool,
	) -> Result {
		let Some(turn) = world.entity_mut(entity).take::<AgentTurn>() else {
			return Ok(());
		};
		if !interrupted {
			return Ok(());
		}
		world.with_state::<ThreadRecordsQuery, _>(|mut query| -> Result {
			let actor = query.actor_id(entity)?;
			let thread = query.thread_id(entity)?;
			query
				.records_mut(entity)?
				.close_interrupted(actor, thread, turn.since);
			Ok(())
		})
	}
}

impl Drop for TurnGuard {
	fn drop(&mut self) {
		if self.closed {
			return;
		}
		let caller = self.caller.clone();
		self.spawner
			.spawn(async move {
				// a despawned entity has no turn left to close
				let _ = caller
					.with_world(|world, entity| {
						if let Err(err) = Self::close_turn(world, entity, true)
						{
							world.handle_command_error::<TurnGuard>(err);
						}
					})
					.await;
			})
			.detach();
	}
}

impl ThreadRecords {
	/// Close the turn of `actor` cut short: each of its posts in progress
	/// closed as [`PostStatus::Interrupted`], and each of its function calls
	/// after `since` without an output given one saying it was interrupted,
	/// itself interrupted, so a provider still reads every call answered.
	fn close_interrupted(
		&mut self,
		actor: ActorId,
		thread: ThreadId,
		since: Option<PostId>,
	) {
		let in_progress = self
			.posts()
			.iter()
			.filter(|post| post.author() == actor && post.in_progress())
			.cloned()
			.collect::<Vec<_>>();
		for mut post in in_progress {
			post.set_status(PostStatus::Interrupted);
			self.upsert_post(post);
		}
		let answered = self
			.posts()
			.iter()
			.filter_map(|post| match AgentPost::new(post)? {
				AgentPost::FunctionCallOutput(output) => {
					output.call_id().to_string().xsome()
				}
				_ => None,
			})
			.collect::<HashSet<_>>();
		let unanswered = self
			.posts()
			.iter()
			.filter(|post| {
				post.author() == actor
					&& since.is_none_or(|since| post.id() > since)
			})
			.filter_map(|post| match AgentPost::new(post)? {
				AgentPost::FunctionCall(call)
					if !answered.contains(call.call_id()) =>
				{
					(call.call_id().to_string(), call.name().to_string())
						.xsome()
				}
				_ => None,
			})
			.collect::<Vec<_>>();
		for (call_id, name) in unanswered {
			self.upsert_post(AgentPost::new_function_call_output(
				actor,
				thread,
				call_id,
				format!(
					"Function call interrupted '{name}': the turn was cut short before it answered"
				),
				name.xsome(),
				PostStatus::Interrupted,
			));
		}
	}

	/// Whether an actor other than `actor` made a post that is not a log
	/// after `since`.
	fn posted_by_other_since(
		&self,
		actor: ActorId,
		since: Option<PostId>,
	) -> bool {
		self.posts()
			.iter()
			.rev()
			.take_while(|post| since.is_none_or(|since| post.id() > since))
			.any(|post| {
				post.author() != actor && post.intent() != PostIntent::Log
			})
	}
}

/// Fire the interrupt of the turn [`InterruptTurn`] targets, if it is taking
/// one.
pub(crate) fn interrupt_turn(
	ev: On<InterruptTurn>,
	mut turns: Query<&mut AgentTurn>,
) {
	if let Ok(mut turn) = turns.get_mut(ev.entity)
		&& let Some(interrupt) = turn.interrupt.take()
	{
		interrupt.signal(());
	}
}

/// Interrupt each [`InterruptOnPost`] agent taking a turn once another actor
/// posted during it, a post window sync received included.
pub(crate) fn interrupt_on_post(
	turns: Populated<(Entity, &ActorRef, &AgentTurn), With<InterruptOnPost>>,
	query: ThreadRecordsQuery,
	mut commands: Commands,
) {
	for (entity, actor, turn) in turns.iter() {
		if turn.interrupt.is_some()
			&& query.records(entity).is_ok_and(|records| {
				records.posted_by_other_since(**actor, turn.since)
			}) {
			commands.trigger(InterruptTurn::new(entity));
		}
	}
}

/// Pass each [`UserInput`] turn whose actor had the last word.
pub(crate) fn pass_person_turns(
	mut turns: Populated<(Entity, &ActorRef, &mut PersonTurn)>,
	query: ThreadRecordsQuery,
	mut commands: Commands,
) {
	for (entity, actor, mut turn) in turns.iter_mut() {
		let passed = query.records(entity).is_ok_and(|records| {
			records
				.last_word()
				.is_some_and(|post| post.author() == **actor)
		});
		if passed && let Some(signal) = turn.0.take() {
			signal.signal(());
			commands.entity(entity).try_remove::<PersonTurn>();
		}
	}
}

#[cfg(all(test, feature = "agent"))]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

	/// A person, posting `seed` first if given, taking turns when `typing`.
	fn person(seed: Option<&str>, typing: bool) -> impl Bundle {
		let seed = seed.map(String::from);
		(
			Actor::new_with_id(
				ActorId::from_u128(1),
				"Person",
				ActorKind::User,
			),
			OnSpawn::new(move |entity| {
				if typing {
					entity.insert(UserInput);
				}
				if let Some(seed) = seed {
					entity.with_child(Post::spawn(seed));
				}
			}),
		)
	}

	/// An agent `id` playing `script`, held by `gate`.
	fn agent(
		id: u128,
		gate: &MockGate,
		script: impl IntoIterator<Item = MockTurn>,
	) -> impl Bundle {
		(
			Actor::new_with_id(
				ActorId::from_u128(id),
				"Agent",
				ActorKind::Agent,
			),
			MockPostStreamer::with_script(script).with_gate(gate.clone()),
		)
	}

	/// A tool route `wait` answering once `gate` is released, setting
	/// `landed` as it does.
	fn wait_tool(gate: &MockGate, landed: Store<bool>) -> impl Bundle {
		let gate = gate.clone();
		(Router, children![route::new(
			"wait",
			Action::<Request, Response>::new_async(
				move |_: ActionContext<Request>| {
					let gate = gate.clone();
					async move {
						gate.wait().await;
						landed.set(true);
						Response::ok().xok::<BevyError>()
					}
				}
			),
		)])
	}

	fn call_wait() -> MockTurn {
		MockTurn::call("wait", serde_json::json!({})).unwrap()
	}

	/// An app over `root`, its threads reduced and its behavior called,
	/// answering the root and whether the call has resolved.
	fn run(root: impl Bundle) -> (App, Entity, Store<bool>) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let root = app.world_mut().spawn(root).id();
		ThreadRecords::reduce_now(app.world_mut());
		let done = call(&mut app, root);
		(app, root, done)
	}

	/// Call `entity`'s behavior in a task, answering whether it resolved.
	fn call(app: &mut App, entity: Entity) -> Store<bool> {
		let done = Store::default();
		app.world_mut()
			.run_async(async move |world: AsyncWorld| -> Result {
				world.entity(entity).call::<(), Outcome>(()).await?;
				done.set(true);
				Ok(())
			});
		done
	}

	/// Update `app` until `holds`, naming `what` if it never does.
	fn pump_until(app: &mut App, what: &str, holds: impl Fn(&World) -> bool) {
		for _ in 0..500 {
			if holds(app.world()) {
				return;
			}
			app.update();
		}
		panic!("never {what}");
	}

	fn pump(app: &mut App) {
		for _ in 0..30 {
			app.update();
		}
	}

	/// The records of the one thread in `world`.
	fn records(world: &World) -> &ThreadRecords {
		world
			.iter_entities()
			.find_map(|entity| entity.get::<ThreadRecords>())
			.unwrap()
	}

	/// The thread's posts that are not logs, as `text` or `text (status)`
	/// when not completed, a call as `call` and its output as `output`.
	fn said(world: &World) -> Vec<String> {
		records(world)
			.content_posts()
			.map(|view| describe(view.post))
			.collect()
	}

	fn describe(post: &Post) -> String {
		let text = match AgentPost::new(post) {
			Some(AgentPost::FunctionCall(_)) => "call".to_string(),
			Some(AgentPost::FunctionCallOutput(_)) => "output".to_string(),
			_ => post.to_string(),
		};
		match post.status() {
			PostStatus::Completed => text,
			status => format!("{text} ({status:?})"),
		}
	}

	/// Whether the thread holds a post saying `text`, in any status.
	fn holds(world: &World, text: &str) -> bool {
		said(world).iter().any(|said| said.starts_with(text))
	}

	/// Push a completed post by the person into the window `thread`.
	fn post(app: &mut App, thread: Entity, text: &str) {
		let mut entity = app.world_mut().entity_mut(thread);
		let id = entity.get::<Thread>().unwrap().id();
		entity.get_mut::<ThreadRecords>().unwrap().upsert_post(
			AgentPost::new_text(
				ActorId::from_u128(1),
				id,
				text,
				PostStatus::Completed,
			),
		);
	}

	/// The window holding records.
	fn thread_entity(app: &mut App) -> Entity {
		app.world_mut()
			.query_filtered::<Entity, With<ThreadRecords>>()
			.single(app.world())
			.unwrap()
	}

	/// The behavior entity of the actor `id`.
	fn actor_entity(app: &mut App, id: u128) -> Entity {
		app.world_mut()
			.query::<(Entity, &ActorRef)>()
			.iter(app.world())
			.find(|(_, actor)| ***actor == ActorId::from_u128(id))
			.unwrap()
			.0
	}

	/// Turn based: a post made during a tool call stops nothing, and the next
	/// model call reads it after the call's output.
	#[beet_core::test]
	async fn a_post_mid_tool_call_is_read_after_its_output() {
		let gate = MockGate::default();
		let landed = Store::default();
		let (mut app, _, done) = run((
			RepeatWhileFunctionCallOutput,
			children![(Thread::default(), Sequence::new(), children![
				person(Some("go"), false),
				(agent(2, &gate, [call_wait()]), children![wait_tool(
					&gate, landed
				)]),
			])],
		));
		pump_until(&mut app, "called the tool", |world| holds(world, "call"));
		let thread = thread_entity(&mut app);
		post(&mut app, thread, "by the way");
		gate.release();
		pump_until(&mut app, "finished", |_| done.get());
		landed.get().xpect_true();
		said(app.world()).xpect_eq(
			["go", "call", "by the way", "output", "you said: by the way"]
				.map(String::from)
				.to_vec(),
		);
		records(app.world())
			.model_posts(None)
			.iter()
			.filter_map(ModelPost::post)
			.map(|view| describe(view.post))
			.collect::<Vec<_>>()
			.xpect_eq(
				["go", "call", "output", "by the way", "you said: by the way"]
					.map(String::from)
					.to_vec(),
			);
	}

	/// A chat looping a typing person's turn then `agent`'s, the person having
	/// said `go`.
	fn chat(agent: impl Bundle) -> impl Bundle {
		(Repeat::new(), children![(
			Thread::default(),
			Sequence::new(),
			children![person(Some("go"), true), agent]
		)])
	}

	/// A post made during the agent's last stream makes the person's turn
	/// pass at once, and the agent answers it.
	#[beet_core::test]
	async fn the_person_passes_at_once_after_posting_mid_stream() {
		let gate = MockGate::default();
		let (mut app, ..) =
			run(chat(agent(2, &gate, [MockTurn::Hold("thinking".into())])));
		pump_until(&mut app, "began thinking", |world| {
			holds(world, "thinking (InProgress)")
		});
		let thread = thread_entity(&mut app);
		post(&mut app, thread, "by the way");
		gate.release();
		pump_until(&mut app, "answered", |world| {
			holds(world, "you said: by the way")
		});
		pump(&mut app);
		said(app.world()).xpect_eq(
			["go", "thinking", "by the way", "you said: by the way"]
				.map(String::from)
				.to_vec(),
		);
	}

	/// Semi duplex: a post cuts the turn of an agent carrying
	/// [`InterruptOnPost`], its post closed as interrupted, and is answered
	/// next.
	#[beet_core::test]
	async fn semi_duplex_cuts_the_turn_and_answers_next() {
		let gate = MockGate::default();
		let (mut app, ..) = run(chat((
			agent(2, &gate, [
				MockTurn::Hold("thinking".into()),
				MockTurn::Text("answered".into()),
			]),
			InterruptOnPost,
		)));
		pump_until(&mut app, "began thinking", |world| {
			holds(world, "thinking (InProgress)")
		});
		let thread = thread_entity(&mut app);
		post(&mut app, thread, "by the way");
		pump_until(&mut app, "answered", |world| holds(world, "answered"));
		pump(&mut app);
		said(app.world()).xpect_eq(
			["go", "thinking (Interrupted)", "by the way", "answered"]
				.map(String::from)
				.to_vec(),
		);
	}

	/// An interrupted tool call's side effect never lands, its output says
	/// so, and the agent's tool loop ends with the turn.
	#[beet_core::test]
	async fn an_interrupted_tool_call_never_lands() {
		let gate = MockGate::default();
		let landed = Store::default();
		let (mut app, _, done) = run((
			RepeatWhileFunctionCallOutput,
			children![(Thread::default(), Sequence::new(), children![
				person(Some("go"), false),
				(agent(2, &gate, [call_wait()]), children![wait_tool(
					&gate, landed
				)]),
			])],
		));
		pump_until(&mut app, "called the tool", |world| holds(world, "call"));
		let agent = actor_entity(&mut app, 2);
		app.world_mut().trigger(InterruptTurn::new(agent));
		pump_until(&mut app, "finished", |_| done.get());
		gate.release();
		pump(&mut app);
		landed.get().xpect_false();
		said(app.world()).xpect_eq(
			["go", "call", "output (Interrupted)"]
				.map(String::from)
				.to_vec(),
		);
		records(app.world())
			.last_content_post()
			.unwrap()
			.to_string()
			.xpect_contains("Function call interrupted 'wait'");
	}

	/// A chat whose agent, carrying [`InterruptOnPost`] in a tool loop, plays
	/// `script` over a tool held by `gate`.
	fn tool_chat(
		gate: &MockGate,
		landed: Store<bool>,
		script: impl IntoIterator<Item = MockTurn>,
	) -> impl Bundle {
		chat((RepeatWhileFunctionCallOutput, children![(
			agent(2, gate, script),
			InterruptOnPost,
			children![wait_tool(gate, landed)]
		)]))
	}

	/// Semi duplex: a note cutting a tool call is the last word, though the
	/// output closing the call comes after it, so it is answered next.
	#[beet_core::test]
	async fn semi_duplex_cuts_a_tool_call_and_answers_next() {
		let gate = MockGate::default();
		let landed = Store::default();
		let (mut app, ..) = run(tool_chat(&gate, landed, [
			call_wait(),
			MockTurn::Text("answered".into()),
		]));
		pump_until(&mut app, "called the tool", |world| holds(world, "call"));
		let thread = thread_entity(&mut app);
		post(&mut app, thread, "by the way");
		pump_until(&mut app, "answered", |world| holds(world, "answered"));
		gate.release();
		pump(&mut app);
		landed.get().xpect_false();
		said(app.world()).xpect_eq(
			[
				"go",
				"call",
				"by the way",
				"output (Interrupted)",
				"answered",
			]
			.map(String::from)
			.to_vec(),
		);
	}

	/// A stop with no note leaves the cut call the last word, so the agent's
	/// loop ends and the person's turn waits.
	#[beet_core::test]
	async fn a_stopped_tool_call_waits_for_the_person() {
		let gate = MockGate::default();
		let landed = Store::default();
		let (mut app, ..) = run(tool_chat(&gate, landed, [
			call_wait(),
			MockTurn::Text("answered".into()),
		]));
		pump_until(&mut app, "called the tool", |world| holds(world, "call"));
		let agent = actor_entity(&mut app, 2);
		app.world_mut().trigger(InterruptTurn::new(agent));
		pump_until(&mut app, "cut the call", |world| {
			holds(world, "output (Interrupted)")
		});
		pump(&mut app);
		said(app.world()).xpect_eq(
			["go", "call", "output (Interrupted)"]
				.map(String::from)
				.to_vec(),
		);
	}

	/// Full duplex: two agents under a `Parallel` both stream into the
	/// records, neither cutting the other.
	#[beet_core::test]
	async fn full_duplex_streams_without_loss() {
		let (first, second) = (MockGate::default(), MockGate::default());
		let (mut app, _, done) =
			run((Thread::default(), Parallel::new(), children![
				person(Some("go"), false),
				agent(2, &first, [MockTurn::Hold("one".into())]),
				agent(3, &second, [MockTurn::Hold("two".into())]),
			]));
		pump_until(&mut app, "streamed both", |world| {
			holds(world, "one (InProgress)") && holds(world, "two (InProgress)")
		});
		first.release();
		second.release();
		pump_until(&mut app, "finished", |_| done.get());
		let mut said = said(app.world());
		said.sort();
		said.xpect_eq(["go", "one", "two"].map(String::from).to_vec());
	}

	/// Full duplex: a post by one agent cuts the other only where the other
	/// carries [`InterruptOnPost`]. The plain agent's post is its tool's
	/// output, landing once both are mid turn.
	#[beet_core::test]
	async fn full_duplex_cuts_only_who_yields() {
		let (first, second) = (MockGate::default(), MockGate::default());
		let landed = Store::default();
		let (mut app, _, done) =
			run((Thread::default(), Parallel::new(), children![
				person(Some("go"), false),
				(agent(2, &first, [call_wait()]), children![wait_tool(
					&first, landed
				)]),
				(
					agent(3, &second, [MockTurn::Hold("two".into())]),
					InterruptOnPost
				),
			]));
		pump_until(&mut app, "began both turns", |world| {
			holds(world, "call") && holds(world, "two (InProgress)")
		});
		first.release();
		// the yielding agent's gate is never released: only its cut ends it
		pump_until(&mut app, "finished", |_| done.get());
		landed.get().xpect_true();
		let mut said = said(app.world());
		said.sort();
		said.xpect_eq(
			["call", "go", "output", "two (Interrupted)"]
				.map(String::from)
				.to_vec(),
		);
	}

	/// A thread under a `Race` ends with its first actor, the other's post
	/// closed as interrupted.
	#[beet_core::test]
	async fn a_race_ends_with_its_first_actor() {
		let (first, second) = (MockGate::default(), MockGate::default());
		let (mut app, _, done) =
			run((Thread::default(), Race::new(), children![
				person(Some("go"), false),
				agent(2, &first, [MockTurn::Hold("one".into())]),
				agent(3, &second, [MockTurn::Hold("two".into())]),
			]));
		pump_until(&mut app, "streamed both", |world| {
			holds(world, "one (InProgress)") && holds(world, "two (InProgress)")
		});
		first.release();
		pump_until(&mut app, "closed the loser", |world| {
			done.get() && holds(world, "two (Interrupted)")
		});
		second.release();
		pump(&mut app);
		let mut said = said(app.world());
		said.sort();
		said.xpect_eq(
			["go", "one", "two (Interrupted)"]
				.map(String::from)
				.to_vec(),
		);
	}

	/// A post from a window with no behavior reaches the canonical window by
	/// window sync and interrupts its agent carrying [`InterruptOnPost`].
	#[beet_core::test]
	async fn a_view_window_post_interrupts_the_canonical_agent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.install_types::<ThreadPlugin>();
		let root = app
			.world_mut()
			.spawn(TableStore::new(InMemoryTableStore::new()))
			.id();
		let gate = MockGate::default();
		let mount = || MountThreadStore {
			path: RelPath::new("threads"),
			new: false,
		};
		let canonical = app
			.world_mut()
			.spawn((ChildOf(root), Thread::default(), mount(), children![
				person(None, false),
				(
					agent(2, &gate, [MockTurn::Hold("thinking".into())]),
					InterruptOnPost
				),
			]))
			.id();
		let view = app
			.world_mut()
			.spawn((ChildOf(root), Thread::default(), mount(), children![
				person(None, false)
			]))
			.id();
		for window in [canonical, view] {
			app.world_mut()
				.run_async_local_then(
					async move |world: AsyncWorld| -> Result {
						Thread::enter(
							world.clone(),
							window,
							ThreadEntry::parse(
								Request::get("chat").parts().clone(),
							)?,
						)
						.await
					},
				)
				.await
				.unwrap();
		}
		let agent = actor_entity(&mut app, 2);
		let done = call(&mut app, agent);
		let texts = move |world: &World, window: Entity| {
			world
				.get::<ThreadRecords>(window)
				.unwrap()
				.content_posts()
				.map(|view| describe(view.post))
				.collect::<Vec<_>>()
		};
		pump_until(&mut app, "streamed into the view", |world| {
			texts(world, view) == vec!["thinking (InProgress)".to_string()]
		});
		post(&mut app, view, "by the way");
		pump_until(&mut app, "interrupted the agent", |_| done.get());
		pump(&mut app);
		let expected = ["thinking (Interrupted)", "by the way"]
			.map(String::from)
			.to_vec();
		texts(app.world(), canonical).xpect_eq(expected.clone());
		texts(app.world(), view).xpect_eq(expected);
	}
}
