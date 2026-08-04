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
			"No Action<{}, {}> on {caller}, which serves {}",
			core::any::type_name::<Input>(),
			core::any::type_name::<Out>(),
			match metas.get(caller) {
				Ok(meta) => meta.signatures(),
				Err(_) => "no action".into(),
			}
		),
	}
}

/// Wires a [`oneshot`]-backed [`OutHandler`] and calls [`call_world`].
///
/// Returns the receiving half so the caller can await the result. The value
/// carries `Result<Out>` so async action errors propagate back to the caller
/// instead of silently dropping the handler.
#[track_caller]
fn call_with_oneshot<Input, Out>(
	entity: &mut EntityWorldMut,
	input: Input,
) -> Result<OnceValueRx<Result<Out>>>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let (send, recv) = oneshot::<Result<Out>>();
	let out_handler = OutHandler::new(move |_commands, result: Result<Out>| {
		send.signal(result);
		Ok(())
	});
	call_world::<Input, Out>(entity, input, out_handler)?;
	Ok(recv)
}

/// Drives an action call to completion from an owned [`EntityWorldMut`],
/// polling the world via [`AsyncRunner`] while waiting for the result.
///
/// std-only: it owns and drives the world itself. The bridge-based
/// [`AsyncEntityActionExt`] paths instead rely on the running app's update
/// loop and resume via the [`oneshot`] waker, so they are no_std-clean.
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
	AsyncRunner::poll_and_update(|| world.update_local(), recv.wait()).await
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
) -> Result<OnceValueRx<Result<Out>>>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let (send, recv) = oneshot::<Result<Out>>();
	let out_handler = OutHandler::new(move |_, result: Result<Out>| {
		send.signal(result);
		Ok(())
	});
	action.call_world(entity, input, out_handler)?;
	Ok(recv)
}

/// Extension trait for calling actions on [`AsyncEntity`] handles.
#[extend::ext(name=AsyncEntityActionExt)]
pub impl AsyncEntity {
	/// Make an action call asynchronously.
	///
	/// The world's normal update loop drives any async work inside the action;
	/// this side just awaits the [`oneshot`] result.
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
	/// The call will be executed when commands are applied.
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
	use crate::prelude::*;
	use beet_core::prelude::*;

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
