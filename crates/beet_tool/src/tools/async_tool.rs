use crate::prelude::*;
use beet_core::prelude::*;

/// Context passed to async tool handlers containing an [`AsyncEntity`]
/// handle and the input payload.
pub struct AsyncToolIn<In = ()> {
	/// The entity that initiated this tool call.
	pub caller: AsyncEntity,
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

impl<In> AsyncToolIn<In> {
	/// Map the input of this `AsyncToolIn` to a different type, keeping the same caller.
	pub fn map_input<NewIn>(self, input: NewIn) -> AsyncToolIn<NewIn> {
		AsyncToolIn {
			caller: self.caller,
			input,
		}
	}
}

/// Create a [`Tool`] from an async closure that receives
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
pub fn async_tool<Func, Input, Out, Fut>(func: Func) -> Tool<Input, Out>
where
	Func: 'static + Send + Sync + Clone + FnOnce(AsyncToolIn<Input>) -> Fut,
	Input: 'static + Send + Sync,
	Fut: 'static + MaybeSend + Future<Output = Result<Out>>,
	Out: 'static + Send + Sync,
{
	Tool::new(
		TypeMeta::of::<Func>(),
		move |ToolCall {
		          mut commands,
		          caller,
		          input,
		          out_handler,
		      }| {
			let async_entity = commands.world().entity(caller);
			let arg = AsyncToolIn {
				caller: async_entity,
				input,
			};
			let func = func.clone();
			commands.run(async move |world: AsyncWorld| -> Result {
				let output = func(arg).await?;
				out_handler.call_async(world, output).await
			});
			Ok(())
		},
	)
}

/// Marker for the async tool [`IntoTool`] impl accepting
/// `Fn(AsyncToolIn<I>) -> Future<Output = Result<O>>`.
pub struct AsyncToolMarker;

impl<Func, Input, Out, Fut> IntoTool<(AsyncToolMarker, Input, Out)> for Func
where
	Func: 'static + Send + Sync + Clone + Fn(AsyncToolIn<Input>) -> Fut,
	Input: 'static + Send + Sync,
	Fut: 'static + MaybeSend + Future<Output = Result<Out>>,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool(self) -> Tool<Self::In, Self::Out> { async_tool(self) }
}

/// Marker for the typed async tool [`IntoTool`] impl accepting
/// `Fn(I) -> Future<Output = O>` where `I` is a plain input type.
pub struct TypedAsyncToolMarker;

impl<Func, Input, Out, Fut> IntoTool<(TypedAsyncToolMarker, Input, Out)>
	for Func
where
	Func: 'static + Send + Sync + Clone + Fn(Input) -> Fut,
	Input: 'static + Send + Sync + bevy::reflect::Typed,
	Fut: 'static + MaybeSend + Future<Output = Out>,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool(self) -> Tool<Self::In, Self::Out> {
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

	#[beet_core::test]
	async fn works() {
		AsyncPlugin::world()
			.spawn(async_tool(async |input: AsyncToolIn<(i32, i32)>| {
				Ok(input.0 + input.1)
			}))
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(7);
	}

	#[beet_core::test]
	async fn negate() {
		AsyncPlugin::world()
			.spawn(async_tool(async |input: AsyncToolIn<i32>| Ok(-*input)))
			.call::<i32, i32>(42)
			.await
			.unwrap()
			.xpect_eq(-42);
	}

	#[beet_core::test]
	async fn returns_tool_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(async_tool(async |cx: AsyncToolIn<()>| Ok(cx.caller.id())))
			.id();
		world
			.entity_mut(entity)
			.call::<(), Entity>(())
			.await
			.unwrap()
			.xpect_eq(entity);
	}

	#[beet_core::test]
	async fn string_processing() {
		AsyncPlugin::world()
			.spawn(async_tool(async |input: AsyncToolIn<String>| {
				Ok(format!("hello {}", *input))
			}))
			.call::<String, String>("world".to_string())
			.await
			.unwrap()
			.xpect_eq("hello world".to_string());
	}

	#[beet_core::test]
	async fn typed_async_pure() {
		AsyncPlugin::world()
			.spawn((async |val: i32| -> i32 { -val }).into_tool())
			.call::<i32, i32>(42)
			.await
			.unwrap()
			.xpect_eq(-42);
	}

	#[beet_core::test]
	async fn typed_async_add() {
		AsyncPlugin::world()
			.spawn((async |(a, b): (i32, i32)| -> i32 { a + b }).into_tool())
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(7);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — async tools
	// -----------------------------------------------------------------------

	#[tool]
	async fn async_negate(val: i32) -> i32 { -val }

	#[beet_core::test]
	async fn tool_macro_async_single_arg() {
		AsyncPlugin::world()
			.spawn(async_negate.into_tool())
			.call::<i32, i32>(7)
			.await
			.unwrap()
			.xpect_eq(-7);
	}

	#[tool]
	async fn async_add(a: i32, b: i32) -> i32 { a + b }

	#[beet_core::test]
	async fn tool_macro_async_multi_arg() {
		AsyncPlugin::world()
			.spawn(async_add.into_tool())
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(7);
	}

	#[tool]
	async fn async_no_args() -> i32 { 42 }

	#[beet_core::test]
	async fn tool_macro_async_no_args() {
		AsyncPlugin::world()
			.spawn(async_no_args.into_tool())
			.call::<(), i32>(())
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[tool]
	async fn async_fallible(val: i32) -> Result<i32> {
		if val == 0 {
			bevybail!("zero");
		}
		Ok(val * 2)
	}

	#[beet_core::test]
	async fn tool_macro_async_result_ok() {
		AsyncPlugin::world()
			.spawn(async_fallible.into_tool())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(10);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — async passthrough
	// -----------------------------------------------------------------------

	#[tool]
	async fn async_passthrough_tool(cx: AsyncToolIn<i32>) -> i32 { *cx * 3 }

	#[beet_core::test]
	async fn tool_macro_async_passthrough() {
		AsyncPlugin::world()
			.spawn(async_passthrough_tool.into_tool())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(15);
	}

	#[tool]
	async fn async_passthrough_entity(cx: AsyncToolIn<()>) -> Entity {
		cx.caller.id()
	}

	#[beet_core::test]
	async fn tool_macro_async_passthrough_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(async_passthrough_entity.into_tool()).id();
		world
			.entity_mut(entity)
			.call::<(), Entity>(())
			.await
			.unwrap()
			.xpect_eq(entity);
	}
}
