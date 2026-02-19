use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;

/// Context passed to async tool handlers containing an [`AsyncEntity`]
/// handle and the input payload.
pub struct AsyncToolIn<In = ()> {
	/// The async entity handle for non-blocking ECS access.
	pub tool: AsyncEntity,
	/// The input payload for this tool call.
	pub input: In,
}

impl<In> std::ops::Deref for AsyncToolIn<In> {
	type Target = In;
	fn deref(&self) -> &Self::Target { &self.input }
}

impl<In> std::ops::DerefMut for AsyncToolIn<In> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.input }
}

/// Create a [`ToolHandler`] from an async closure that receives
/// [`AsyncToolIn`] and returns [`Result<Out>`].
///
/// Unlike [`func_tool`](crate::func_tool), async tools can perform
/// non-blocking work such as network requests or streaming without
/// stalling the ECS.
///
/// ## Examples
///
/// ```rust,no_run
/// # use beet_tool::prelude::*;
/// # use beet_core::prelude::*;
/// let handler = async_tool(|input: AsyncToolIn<u32>| async move {
///     Ok(*input * 2)
/// });
/// ```
pub fn async_tool<Func, Input, Out, Fut>(func: Func) -> ToolHandler<Input, Out>
where
	Func: 'static + Send + Sync + Clone + Fn(AsyncToolIn<Input>) -> Fut,
	Input: 'static + Send + Sync,
	Fut: 'static + Send + Future<Output = Result<Out>>,
	Out: 'static + Send + Sync,
{
	ToolHandler::new(
		TypeMeta::of::<Func>(),
		move |ToolCall {
		          mut commands,
		          tool,
		          input,
		          out_handler,
		      }| {
			let async_entity = commands.world().entity(tool);
			let arg = AsyncToolIn {
				tool: async_entity,
				input,
			};
			let func = func.clone();
			commands.run(async move |world: AsyncWorld| -> Result {
				let output = func(arg).await?;

				// Obtain fresh AsyncCommands via SystemState so
				// the out_handler (and any downstream pipe
				// handlers) can queue further work.
				world
					.with_then(move |world: &mut World| -> Result {
						let result = {
							let mut state =
								SystemState::<AsyncCommands>::new(world);
							let async_commands = state.get_mut(world);
							let result =
								out_handler.call(async_commands, output);
							state.apply(world);
							result
						};
						world.flush();
						result
					})
					.await
			});
			Ok(())
		},
	)
}

/// Marker for the async tool [`IntoToolHandler`] impl accepting
/// `Fn(AsyncToolIn<I>) -> Future<Output = Result<O>>`.
pub struct AsyncToolMarker;

impl<Func, Input, Out, Fut> IntoToolHandler<(AsyncToolMarker, Input, Out)>
	for Func
where
	Func: 'static + Send + Sync + Clone + Fn(AsyncToolIn<Input>) -> Fut,
	Input: 'static + Send + Sync,
	Fut: 'static + Send + Future<Output = Result<Out>>,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> {
		async_tool(self)
	}
}

/// Marker for the typed async tool [`IntoToolHandler`] impl accepting
/// `Fn(I) -> Future<Output = O>` where `I` is a plain input type.
pub struct TypedAsyncToolMarker;

impl<Func, Input, Out, Fut> IntoToolHandler<(TypedAsyncToolMarker, Input, Out)>
	for Func
where
	Func: 'static + Send + Sync + Clone + Fn(Input) -> Fut,
	Input: 'static + Send + Sync + bevy::reflect::Typed,
	Fut: 'static + Send + Future<Output = Out>,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> {
		async_tool(move |input: AsyncToolIn<Input>| {
			let fut = self(input.input);
			async move { fut.await.xok() }
		})
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		AsyncPlugin::world()
			.spawn(async_tool(async |input: AsyncToolIn<(i32, i32)>| {
				Ok(input.0 + input.1)
			}))
			.call_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(7);
	}

	#[test]
	fn negate() {
		AsyncPlugin::world()
			.spawn(async_tool(async |input: AsyncToolIn<i32>| Ok(-*input)))
			.call_blocking::<i32, i32>(42)
			.unwrap()
			.xpect_eq(-42);
	}

	#[test]
	fn returns_tool_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(async_tool(async |cx: AsyncToolIn<()>| Ok(cx.tool.id())))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}

	#[test]
	fn string_processing() {
		AsyncPlugin::world()
			.spawn(async_tool(async |input: AsyncToolIn<String>| {
				Ok(format!("hello {}", *input))
			}))
			.call_blocking::<String, String>("world".to_string())
			.unwrap()
			.xpect_eq("hello world".to_string());
	}

	#[test]
	fn typed_async_pure() {
		AsyncPlugin::world()
			.spawn((async |val: i32| -> i32 { -val }).into_tool_handler())
			.call_blocking::<i32, i32>(42)
			.unwrap()
			.xpect_eq(-42);
	}

	#[test]
	fn typed_async_add() {
		AsyncPlugin::world()
			.spawn(
				(async |(a, b): (i32, i32)| -> i32 { a + b })
					.into_tool_handler(),
			)
			.call_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(7);
	}
}
