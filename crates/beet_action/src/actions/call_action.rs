use crate::prelude::*;
use beet_core::prelude::*;

/// Dispatches an action call through a cached system, then flushes the world.
fn call_world<Input, Out>(
	entity: &mut EntityWorldMut,
	input: Input,
	out_handler: OutHandler<Out>,
) -> Result
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let id = entity.id();
	entity.world_scope(move |world| -> Result {
		world.run_system_cached_with::<_, Result, _, _>(
			call_action_system::<Input, Out>,
			(id, input, out_handler),
		)??;
		world.flush();
		Ok(())
	})
}

/// Resolves and calls the caller's own action: its canonical
/// `Action<Input, Out>`, else an [`ActionOverload<Input, Out>`] adapting it.
///
/// Resolution is self-only. [`ActionOf`]/[`Actions`] mean agent targeting, never
/// action attachment, so calling an agent never dispatches into a behaviour
/// pointed at it.
fn call_action_system<Input, Out>(
	In((caller, input, out_handler)): In<(Entity, Input, OutHandler<Out>)>,
	commands: AsyncCommands,
	actions: Query<&Action<Input, Out>>,
	overloads: Query<&ActionOverload<Input, Out>>,
	metas: Query<&ActionMeta>,
) -> Result
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	// the canonical action is preferred, so a coinciding overload never runs
	let action = actions
		.get(caller)
		.ok()
		.or_else(|| overloads.get(caller).ok().map(ActionOverload::action));
	match action {
		Some(action) => action.call(ActionCall {
			commands,
			caller,
			input,
			out_handler,
		}),
		None => bevybail!(
			"No Action<{}, {}> on {caller}, which matches {}",
			TypeName::of::<Input>(),
			TypeName::of::<Out>(),
			match metas.get(caller) {
				Ok(meta) => meta.signatures(),
				Err(_) => "no action".into(),
			}
		),
	}
}

/// Wires a [`CallRx`]-backed [`OutHandler`] and calls [`call_world`].
///
/// Returns the receiving half so the caller can await the result. The value
/// carries `Result<Out>` so async action errors propagate back to the caller
/// instead of silently dropping the handler.
#[track_caller]
fn call_with_oneshot<Input, Out>(
	entity: &mut EntityWorldMut,
	input: Input,
) -> Result<CallRx<Out>>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let (out_handler, recv) = oneshot_out_handler();
	call_world::<Input, Out>(entity, input, out_handler)?;
	Ok(recv)
}

/// A oneshot-backed [`OutHandler`] and the [`CallRx`] awaiting it.
///
/// A handler dropped unresolved fails the receiver with
/// [`ControlFlowError::Interrupted`] rather than parking it forever: the call
/// was stopped (its entity despawned, an [`InterruptRun`] above it) or the
/// [`Running`] parking it was removed.
fn oneshot_out_handler<Out>() -> (OutHandler<Out>, CallRx<Out>)
where
	Out: 'static + Send + Sync,
{
	let (send, recv) = OnceValue::<Result<Out>>::oneshot();
	let (guard, caller_gone) = CallerGuard::new();
	let mut send = InterruptOnDrop(Some(send));
	let out_handler = OutHandler::new(move |_commands, result: Result<Out>| {
		send.signal(result);
		Ok(())
	})
	.with_caller_gone(caller_gone);
	(out_handler, CallRx { recv, guard })
}

/// The caller's half of a call: awaiting it yields the output, and dropping it
/// unresolved stops the call, its [`CallerGuard`] resolving the
/// [`CallerGone`] the callee races against.
struct CallRx<Out> {
	recv: OnceValueRx<Result<Out>>,
	guard: CallerGuard,
}

impl<Out> CallRx<Out> {
	async fn wait(self) -> Result<Out> {
		let Self { recv, guard } = self;
		let output = recv.wait().await;
		// released once the output has landed, when there is nothing left to stop
		drop(guard);
		output
	}
}

/// The sending half of a call's oneshot, resolving the receiver as interrupted
/// if dropped unsignalled.
struct InterruptOnDrop<Out>(Option<OnceValue<Result<Out>>>);

impl<Out> InterruptOnDrop<Out> {
	fn signal(&mut self, result: Result<Out>) {
		if let Some(send) = self.0.take() {
			send.signal(result);
		}
	}
}

impl<Out> Drop for InterruptOnDrop<Out> {
	fn drop(&mut self) {
		self.signal(Err(ControlFlowError::Interrupted.into()));
	}
}

/// Drives an action call to completion from an owned [`EntityWorldMut`],
/// polling the world via [`AsyncRunner`] while waiting for the result.
///
/// std-only: it owns and drives the world itself. The bridge-based
/// [`AsyncEntityActionExt`] paths instead rely on the running app's update
/// loop and resume via the [`CallRx`] waker, so they are no_std-clean.
#[cfg(feature = "std")]
async fn call_polling<Input, Out>(
	mut entity: EntityWorldMut<'_>,
	input: Input,
) -> Result<Out>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let recv = call_with_oneshot::<Input, Out>(&mut entity, input)?;
	let world = entity.into_world_mut();
	let spawner = world.resource::<AsyncSpawner>().clone();
	AsyncRunner::poll_and_update(spawner, || world.update_local(), recv.wait())
		.await
}

/// Extension trait for calling [`Action`] components on
/// [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutActionExt)]
pub impl EntityWorldMut<'_> {
	/// Call an action and block until the result is ready.
	///
	/// # Errors
	/// Errors if the entity serves no matching signature or the call fails.
	#[cfg(feature = "std")]
	fn call_blocking<
		Input: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	>(
		self,
		input: Input,
	) -> Result<Out> {
		async_ext::block_on(call_polling::<Input, Out>(self, input))
	}

	/// Call an action asynchronously, polling the world until completion.
	/// Dropping the future stops the call, as [`AsyncEntity::call`] does.
	///
	/// # Errors
	/// Errors if the entity serves no matching signature or the call fails.
	#[cfg(feature = "std")]
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		call_polling::<Input, Out>(self, input)
	}

	fn call_with<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&mut self,
		input: Input,
		out_handler: OutHandler<Out>,
	) -> Result {
		call_world::<Input, Out>(self, input, out_handler)
	}
}

fn call_with_oneshot_for_value<Input, Out>(
	entity: EntityWorldMut,
	action: Action<Input, Out>,
	input: Input,
) -> Result<CallRx<Out>>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let (out_handler, recv) = oneshot_out_handler();
	action.call_world(entity, input, out_handler)?;
	Ok(recv)
}

/// Extension trait for calling actions on [`AsyncEntity`] handles.
#[extend::ext(name=AsyncEntityActionExt)]
pub impl AsyncEntity {
	/// Make an action call asynchronously.
	///
	/// The world's normal update loop drives any async work inside the action;
	/// this side just awaits the result. The future owns the work it asked
	/// for: dropped before it resolves, the callee's task for this call ends at
	/// its next await, and so do the calls that task was awaiting.
	///
	/// # Errors
	/// Errors if the entity serves no matching signature or the call fails.
	#[track_caller]
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		let async_entity = self.clone();
		async move {
			let recv = async_entity
				.with(move |mut entity_mut| {
					call_with_oneshot::<Input, Out>(&mut entity_mut, input)
				})
				.await
				.flatten()?;
			recv.wait().await
		}
	}

	/// Call an [`Action`] value directly, without it being attached to an entity.
	///
	/// Uses `self` as the entity context passed to the action handler. The
	/// handler may use or ignore this entity depending on its implementation.
	///
	/// # Errors
	/// Errors if the action handler fails.
	fn call_detached<
		Input: 'static + Send + Sync,
		Out: 'static + Send + Sync,
		M,
	>(
		&self,
		action: impl IntoAction<M, In = Input, Out = Out>,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		let entity_id = self.id();
		let world = self.world().clone();
		let action = action.into_action();
		async move {
			let recv = world
				.with(move |world: &mut World| {
					call_with_oneshot_for_value(
						world.entity_mut(entity_id),
						action,
						input,
					)
				})
				.await?;
			recv.wait().await
		}
	}
}

/// Extension trait for queuing action calls via [`EntityCommands`].
#[extend::ext(name=EntityCommandsActionExt)]
pub impl EntityCommands<'_> {
	/// Queue an action call with the provided input and output handler.
	///
	/// The call will be executed when commands are applied. Nobody awaits it,
	/// so it runs until it resolves, its entity despawns or an [`InterruptRun`]
	/// above it stops it: the way to start work that outlives whoever started
	/// it, its output or error going to `out_handler` ([`OutHandler::default`]
	/// raises an error through the world's error handler).
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&mut self,
		input: Input,
		out_handler: OutHandler<Out>,
	) {
		self.queue(move |mut entity: EntityWorldMut| -> Result {
			call_world::<Input, Out>(&mut entity, input, out_handler)?;
			Ok(())
		});
	}
}

#[cfg(test)]
mod test {
	use crate::actions::test_support;
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Call `entity`, dropping the call after a few milliseconds, then settle
	/// long enough for anything still running to land.
	async fn drop_call(world: &mut World, entity: Entity) {
		world
			.run_async_then(move |world| async move {
				async_ext::timeout(
					Duration::from_millis(5),
					world.entity(entity).call::<(), Outcome>(()),
				)
				.await
				.xpect_err();
			})
			.await;
		test_support::settle(world, 80).await;
	}

	#[beet_core::test]
	async fn a_dropped_call_never_lands() {
		let mut world = AsyncPlugin::world();
		let landed = Store::<bool>::default();
		let entity = world
			.spawn(test_support::sleep_then_set(30, landed.clone()))
			.id();
		drop_call(&mut world, entity).await;
		landed.get().xpect_false();
		world.get::<ActionCalls>(entity).unwrap().len().xpect_eq(0);
	}

	/// The callee's dropped future drops the call it was awaiting in turn.
	#[beet_core::test]
	async fn a_dropped_call_stops_two_deep() {
		let mut world = AsyncPlugin::world();
		let landed = Store::<bool>::default();
		let inner = world
			.spawn(test_support::sleep_then_set(30, landed.clone()))
			.id();
		let outer = world
			.spawn(Action::<(), Outcome>::new_async(
				move |cx: ActionContext| async move {
					cx.world().entity(inner).call::<(), Outcome>(()).await
				},
			))
			.id();
		drop_call(&mut world, outer).await;
		landed.get().xpect_false();
		world.get::<ActionCalls>(inner).unwrap().len().xpect_eq(0);
	}

	/// A call made with a handler has no caller to leave, so it lands.
	#[beet_core::test]
	async fn a_call_with_a_handler_runs_unawaited() {
		let mut world = AsyncPlugin::world();
		let landed = Store::<bool>::default();
		let entity = world
			.spawn(test_support::sleep_then_set(30, landed.clone()))
			.id();
		world
			.commands()
			.entity(entity)
			.call((), OutHandler::<Outcome>::default());
		test_support::settle(&mut world, 80).await;
		landed.get().xpect_true();
	}

	fn double() -> Action<i32, i32> {
		Action::new_pure(|cx: ActionContext<i32>| (*cx * 2).xok())
	}

	#[beet_core::test]
	async fn calls_the_canonical_action() {
		AsyncPlugin::world()
			.spawn(double())
			.call::<i32, i32>(21)
			.await
			.unwrap()
			.xpect_eq(42);
	}

	/// Resolution is self-only: an action pointed at an agent via [`ActionOf`]
	/// is agent targeting, never a dispatch candidate for that agent.
	#[beet_core::test]
	async fn ignores_agent_targeting() {
		let mut world = AsyncPlugin::world();
		let agent = world.spawn_empty().id();
		world.spawn((double(), ActionOf(agent)));
		world
			.entity_mut(agent)
			.call::<i32, i32>(21)
			.await
			.xpect_err();
	}

	#[beet_core::test]
	async fn no_match_lists_signatures() {
		AsyncPlugin::world()
			.spawn(double())
			.call::<String, String>("nope".into())
			.await
			.unwrap_err()
			.to_string()
			.xref()
			.xpect_contains("i32 -> i32");
	}
}
