use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Receiver;
use beet_core::exports::async_channel::TryRecvError;
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

fn call_action_system<Input: Send + Sync, Out: Send + Sync>(
	In((caller, input, out_handler)): In<(Entity, Input, OutHandler<Out>)>,
	commands: AsyncCommands,
	actions: Query<&Action<Input, Out>>,
	metas: Query<&ActionMeta>,
) -> Result {
	let action = match actions.get(caller) {
		Ok(action) => action,
		Err(_) => {
			// provide a detailed mismatch diagnostic when ActionMeta is present
			if let Ok(meta) = metas.get(caller) {
				meta.assert_match::<Input, Out>()?;
			}
			bevybail!(
				"No Action<{}, {}> on entity {caller:?}",
				std::any::type_name::<Input>(),
				std::any::type_name::<Out>()
			);
		}
	};

	action.call(ActionCall {
		commands,
		caller,
		input,
		out_handler,
	})?;
	Ok(())
}

/// Wires a channel-based [`OutHandler`] and calls [`call_world`].
///
/// Returns the receiving end of the channel so the caller can await the result.
/// The channel carries `Result<Out>` so that async action errors propagate
/// back to the caller instead of silently closing the channel.
#[track_caller]
fn call_with_channel<Input, Out>(
	entity: &mut EntityWorldMut,
	input: Input,
) -> Result<Receiver<Result<Out>>>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let (send, recv) = async_channel::bounded::<Result<Out>>(1);
	let out_handler = OutHandler::new(move |_commands, result: Result<Out>| {
		send.try_send(result).map_err(|err| {
			bevyhow!("Failed to send action output through channel: {err:?}")
		})
	});
	call_world(entity, input, out_handler)?;
	Ok(recv)
}

/// Unwraps a `Result<Out>` received from an action-call channel,
/// providing a clear error when the channel closes unexpectedly.
fn unwrap_channel_result<Out>(
	result: std::result::Result<Result<Out>, async_channel::RecvError>,
) -> Result<Out> {
	match result {
		Ok(inner) => inner,
		Err(_) => {
			bevybail!("Action call response channel closed unexpectedly.")
		}
	}
}

/// Drives an action call to completion from an [`EntityWorldMut`] context,
/// polling the world as needed while waiting for the result.
async fn call_polling<Input, Out>(
	mut entity: EntityWorldMut<'_>,
	input: Input,
) -> Result<Out>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let recv = call_with_channel::<Input, Out>(&mut entity, input)?;
	let world = entity.into_world_mut();
	match recv.try_recv() {
		Ok(result) => result,
		Err(TryRecvError::Empty) => {
			AsyncRunner::poll_and_update(|| world.update_local(), recv).await?
		}
		Err(TryRecvError::Closed) => {
			bevybail!("Action call response channel closed unexpectedly.")
		}
	}
}


/// Extension trait for calling [`Action`] components on
/// [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutActionExt)]
pub impl EntityWorldMut<'_> {
	/// Call an action and block until the result is ready.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Action`] component
	/// or the action call fails.
	fn call_blocking<
		Input: 'static + Send + Sync,
		Out: 'static + Send + Sync,
	>(
		self,
		input: Input,
	) -> Result<Out> {
		async_ext::block_on(call_polling(self, input))
	}

	/// Call an action asynchronously, polling the world until completion.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Action`] component
	/// or the action call fails.
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		call_polling(self, input)
	}
	fn call_with<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&mut self,
		input: Input,
		out_handler: OutHandler<Out>,
	) -> Result {
		call_world::<Input, Out>(self, input, out_handler)
	}
}

fn call_with_channel_for_value<Input, Out>(
	entity: EntityWorldMut,
	action: Action<Input, Out>,
	input: Input,
) -> Result<Receiver<Result<Out>>>
where
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	let (send, recv) = async_channel::bounded::<Result<Out>>(1);
	let out_handler = OutHandler::new(move |_, result: Result<Out>| {
		send.try_send(result).map_err(|err| {
			bevyhow!("Failed to send action output through channel: {err:?}")
		})
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
	/// this side just awaits the channel result.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Action`] or the
	/// action call fails.
	#[track_caller]
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let recv = self
				.with_then(
					// #[track_caller]
					move |mut entity| {
						call_with_channel::<Input, Out>(&mut entity, input)
					},
				)
				.await?;
			unwrap_channel_result(recv.recv().await)
		}
	}

	/// Call an [`Action`] value directly, without it being attached to an entity.
	///
	/// Uses `self` as the entity context passed to the action handler. The
	/// handler may use or ignore this entity depending on its implementation.
	///
	/// # Errors
	/// Errors if the action handler fails or the response channel closes.
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
				.with_then(move |world: &mut World| {
					call_with_channel_for_value(
						world.entity_mut(entity_id),
						action,
						input,
					)
				})
				.await?;
			unwrap_channel_result(recv.recv().await)
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
