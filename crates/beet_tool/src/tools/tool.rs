use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;
use std::sync::Arc;

#[derive(Component)]
#[component(on_add=on_add::<In, Out>)]
pub struct Tool<In: 'static, Out: 'static> {
	/// The full type name of the handler, for display and debugging.
	handler_meta: TypeMeta,
	handler: Arc<dyn 'static + Send + Sync + Fn(ToolCall<In, Out>) -> Result>,
}

impl<In: 'static, Out: 'static> std::fmt::Debug for Tool<In, Out> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Tool")
			.field("handler_meta", &self.handler_meta)
			.finish()
	}
}

impl<In: 'static, Out: 'static> Clone for Tool<In, Out> {
	fn clone(&self) -> Self {
		Self {
			handler_meta: self.handler_meta,
			handler: Arc::clone(&self.handler),
		}
	}
}

/// Fallback hook that inserts a basic [`ToolMeta`] when a `Tool` is
/// spawned without one. Tools created via the `#[tool]` macro provide
/// a richer [`ToolMeta`] through `#[require]`, which is already present
/// by the time this hook runs, so the check short-circuits.
// couldnt this just also be a require or would that clobber?
fn on_add<In: 'static, Out: 'static>(
	mut world: DeferredWorld,
	cx: HookContext,
) {
	if world.entity(cx.entity).contains::<ToolMeta>() {
		return;
	}
	world
		.commands()
		.entity(cx.entity)
		.insert(ToolMeta::of::<(), In, Out>());
}

impl<In, Out> Tool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	pub fn new(
		handler_meta: TypeMeta,
		handler: impl 'static + Send + Sync + Fn(ToolCall<In, Out>) -> Result,
	) -> Self {
		Self {
			handler_meta,
			handler: Arc::new(handler),
		}
	}

	pub fn handler_meta(&self) -> TypeMeta { self.handler_meta }


	/// Invoke this tool handler with the given [`ToolCall`].
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call(&self, call: ToolCall<In, Out>) -> Result {
		(self.handler)(call)
	}

	/// Invoke this tool handler, constructing the [`ToolCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call_with(
		&self,
		entity: Entity,
		input: In,
		commands: AsyncCommands,
		out_handler: OutHandler<Out>,
	) -> Result {
		let call = ToolCall {
			commands,
			caller: entity,
			input,
			out_handler,
		};
		self.call(call)
	}

	/// Invoke this tool handler from a [`World`], constructing the [`ToolCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call_world(
		&self,
		entity: EntityWorldMut,
		input: In,
		out_handler: OutHandler<Out>,
	) -> Result {
		let id = entity.id();
		let world = entity.into_world_mut();
		let mut state = SystemState::<AsyncCommands>::new(world);
		let commands = state.get_mut(world);
		let result = self.call_with(id, input, commands, out_handler);
		state.apply(world);
		world.flush();
		result
	}

	/// Invoke this tool handler asynchronously, constructing the [`ToolCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub async fn call_async(
		&self,
		entity: AsyncEntity,
		input: In,
		out_handler: OutHandler<Out>,
	) -> Result
	where
		In: 'static + Send,
		Out: 'static + Send,
	{
		let this = self.clone();
		entity
			.with_then(move |entity| {
				this.call_world(entity, input, out_handler)
			})
			.await
	}
}

/// Payload for a single tool invocation, containing the caller entity,
/// input value, [`AsyncCommands`] for queuing work, and a callback
/// for delivering the output.
pub struct ToolCall<'w, 's, In, Out> {
	/// Commands for queuing ECS work or spawning async tasks.
	pub commands: AsyncCommands<'w, 's>,
	/// The entity that initiated or owns this tool call.
	pub caller: Entity,
	/// The input payload for this invocation.
	pub input: In,
	/// Callback invoked with the output when the tool completes.
	pub out_handler: OutHandler<Out>,
}

impl<'w, 's, In, Out> ToolCall<'w, 's, In, Out> {}

/// Delivers a tool's output or error back to the caller.
///
/// Wraps a closure so that different delivery mechanisms (channels,
/// pipe chains, etc.) share a uniform interface.
pub struct OutHandler<Out = ()> {
	func: Box<
		dyn 'static
			+ Send
			+ Sync
			+ FnOnce(AsyncCommands, Result<Out>) -> Result,
	>,
}

impl<Out> Default for OutHandler<Out> {
	fn default() -> Self {
		Self {
			// by default discard the out value, propagating the error
			func: Box::new(|_, result| result.map(|_| ())),
		}
	}
}

impl<Out> OutHandler<Out> {
	/// Exit with [`AppExit::Success`] once the tool call is complete,
	/// discarding the [`Out`] value.
	pub fn exit() -> Self {
		OutHandler {
			func: Box::new(|mut commands, result| {
				result?;
				commands.run(async |world| {
					world.write_message(AppExit::Success);
				});
				Ok(())
			}),
		}
	}
}


impl<Out> OutHandler<Out> {
	/// Create an [`OutHandler`] from any compatible closure.
	pub fn new<F>(func: F) -> Self
	where
		F: 'static + Send + Sync + FnOnce(AsyncCommands, Result<Out>) -> Result,
	{
		Self {
			func: Box::new(func),
		}
	}

	/// Deliver the result, consuming this handler.
	///
	/// # Errors
	/// Returns whatever error the inner callback produces.
	pub fn call(self, commands: AsyncCommands, result: Result<Out>) -> Result {
		(self.func)(commands, result)
	}

	pub fn call_world(self, world: &mut World, result: Result<Out>) -> Result {
		let mut state = SystemState::<AsyncCommands>::new(world);
		let commands = state.get_mut(world);
		let result = (self.func)(commands, result);
		state.apply(world);
		world.flush();
		result
	}

	pub async fn call_async(
		self,
		world: AsyncWorld,
		result: Result<Out>,
	) -> Result
	where
		Out: 'static + Send,
	{
		world
			.with_then(move |world| self.call_world(world, result))
			.await
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	#[should_panic = "No Tool"]
	async fn missing_tool_component() {
		AsyncPlugin::world()
			.spawn_empty()
			.call::<(), ()>(())
			.await
			.unwrap();
	}

	#[tool(pure)]
	#[derive(Reflect)]
	fn add((a, b): (u32, u32)) -> u32 { a + b }

	#[test]
	fn bare_tool_auto_inserts_basic_meta() {
		let mut world = World::new();
		let entity = world.spawn(add.into_tool());
		let meta = entity.get::<ToolMeta>().unwrap();
		// basic fallback meta has no type_info
		meta.type_info().xpect_none();
	}

	#[test]
	fn macro_meta_takes_priority() {
		let mut world = World::new();
		let meta = ToolMeta::of_handler::<add, add>();
		let entity = world.spawn((add.into_tool(), meta));
		// the richer meta from of_handler is preserved
		entity.get::<ToolMeta>().unwrap().type_info().xpect_some();
	}

	#[test]
	fn reflect_meta_has_type_info() {
		let meta = ToolMeta::of_reflect::<add, add>();
		meta.type_info().xpect_some();
		meta.input_info().xpect_some();
		meta.output_info().xpect_some();
	}
}
