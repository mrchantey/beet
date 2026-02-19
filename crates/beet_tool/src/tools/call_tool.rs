use crate::prelude::*;
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::Receiver;
use beet_core::exports::async_channel::TryRecvError;
use beet_core::prelude::*;

/// Dispatches a tool call through a cached system, then flushes the world.
fn call_world<Input, Out>(
	entity: Entity,
	world: &mut World,
	input: Input,
	out_handler: OutHandler<Out>,
) -> Result
where
	Input: 'static,
	Out: 'static + Send + Sync,
{
	world.run_system_cached_with::<_, Result, _, _>(
		call_tool_system::<Input, Out>,
		(entity, input, out_handler),
	)??;
	world.flush();
	Ok(())
}

/// Wires a channel-based [`OutHandler`] and calls [`call_world`].
///
/// Returns the receiving end of the channel so the caller can await the result.
fn call_with_channel<Input, Out>(
	entity: Entity,
	world: &mut World,
	input: Input,
) -> Result<Receiver<Out>>
where
	Input: 'static,
	Out: 'static + Send + Sync,
{
	let (send, recv) = async_channel::bounded(1);
	let out_handler = OutHandler::new(move |_commands, output: Out| {
		send.try_send(output).map_err(|err| {
			bevyhow!("Failed to send tool output through channel: {err:?}")
		})
	});
	call_world(entity, world, input, out_handler)?;
	Ok(recv)
}

/// Drives a tool call to completion from an [`EntityWorldMut`] context,
/// polling the world as needed while waiting for the result.
async fn call_inner<Input, Out>(
	entity: EntityWorldMut<'_>,
	input: Input,
) -> Result<Out>
where
	Input: 'static,
	Out: 'static + Send + Sync,
{
	let id = entity.id();
	let world = entity.into_world_mut();
	let recv = call_with_channel::<Input, Out>(id, world, input)?;
	match recv.try_recv() {
		Ok(output) => output.xok(),
		Err(TryRecvError::Empty) => {
			AsyncRunner::poll_and_update(|| world.update_local(), recv)
				.await
				.xok()
		}
		Err(TryRecvError::Closed) => {
			bevybail!("Tool call response channel closed unexpectedly.")
		}
	}
}

fn call_tool_system<Input, Out>(
	In((tool, input, out_handler)): In<(Entity, Input, OutHandler<Out>)>,
	commands: AsyncCommands,
	tools: Query<&Tool<Input, Out>>,
) -> Result {
	tools.get(tool)?.call(ToolCall {
		commands,
		tool,
		input,
		out_handler,
	})?;
	Ok(())
}

/// Extension trait for calling [`Tool`] components on
/// [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutToolExt)]
pub impl EntityWorldMut<'_> {
	/// Call a tool and block until the result is ready.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Tool`] component
	/// or the tool call fails.
	fn call_blocking<Input: 'static, Out: 'static + Send + Sync>(
		self,
		input: Input,
	) -> Result<Out> {
		async_ext::block_on(call_inner(self, input))
	}

	/// Call a tool asynchronously, polling the world until completion.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Tool`] component
	/// or the tool call fails.
	fn call<Input: 'static, Out: 'static + Send + Sync>(
		self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		call_inner(self, input)
	}
}

fn call_with_channel_for_value<Input, Out>(
	entity: EntityWorldMut,
	tool: Tool<Input, Out>,
	input: Input,
) -> Result<Receiver<Out>>
where
	Input: 'static,
	Out: 'static + Send + Sync,
{
	let (send, recv) = async_channel::bounded(1);
	let out_handler = OutHandler::new(move |_, output: Out| {
		send.try_send(output).map_err(|err| {
			bevyhow!("Failed to send tool output through channel: {err:?}")
		})
	});
	tool.call_world(entity, input, out_handler)?;
	Ok(recv)
}

/// Extension trait for calling tools on [`AsyncEntity`] handles.
#[extend::ext(name=AsyncEntityToolExt)]
pub impl AsyncEntity {
	/// Make a tool call asynchronously.
	///
	/// The world's normal update loop drives any async work inside the tool;
	/// this side just awaits the channel result.
	///
	/// # Errors
	/// Errors if the entity has no matching [`Tool`] or the
	/// tool call fails.
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		let entity_id = self.id();
		let world = self.world().clone();
		async move {
			let recv = world
				.with_then(move |w: &mut World| {
					call_with_channel::<Input, Out>(entity_id, w, input)
				})
				.await?;
			recv.recv().await.map_err(|_| {
				bevyhow!("Tool call response channel closed unexpectedly.")
			})
		}
	}

	/// Call a [`Tool`] value directly, without it being attached to an entity.
	///
	/// Uses `self` as the entity context passed to the tool handler. The
	/// handler may use or ignore this entity depending on its implementation.
	///
	/// # Errors
	/// Errors if the tool handler fails or the response channel closes.
	fn call_tool<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&self,
		tool: Tool<Input, Out>,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		let entity_id = self.id();
		let world = self.world().clone();
		async move {
			let recv = world
				.with_then(move |world: &mut World| {
					call_with_channel_for_value(
						world.entity_mut(entity_id),
						tool,
						input,
					)
				})
				.await?;
			recv.recv().await.map_err(|_| {
				bevyhow!("Tool call response channel closed unexpectedly.")
			})
		}
	}
}

/// Extension trait for queuing tool calls via [`EntityCommands`].
#[extend::ext(name=EntityCommandsToolExt)]
pub impl EntityCommands<'_> {
	/// Queue a tool call with the provided input and output handler.
	///
	/// The call will be executed when commands are applied.
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&mut self,
		input: Input,
		out_handler: OutHandler<Out>,
	) {
		self.queue(move |entity: EntityWorldMut| -> Result {
			let id = entity.id();
			let world = entity.into_world_mut();
			call_world::<Input, Out>(id, world, input, out_handler)?;
			Ok(())
		});
	}
}
