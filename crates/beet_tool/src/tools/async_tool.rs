use crate::prelude::*;
use beet_core::prelude::*;

impl<In, Out> Tool<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// Create a [`Tool`] from an async closure that receives [`ToolContext`]
	/// and returns a value convertible to `Result<Out>` via [`IntoResult`].
	///
	/// Accepts closures returning either `Out` or `Result<Out>`.
	pub fn new_async<Func, Fut, RawOut>(func: Func) -> Self
	where
		Func: 'static + Send + Sync + Clone + FnOnce(ToolContext<In>) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = RawOut>,
		RawOut: IntoResult<Out>,
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
				let arg = ToolContext {
					caller: async_entity,
					input,
				};
				let func = func.clone();
				commands.run(async move |world: AsyncWorld| -> Result {
					let result: Result<Out> = func(arg).await.into_result();
					out_handler.call_async(world, result).await
				});
				Ok(())
			},
		)
	}
}


/// Marker for the async tool [`IntoTool`] impl accepting
/// `Fn(ToolContext<I>) -> Future<Output = Result<O>>`.
pub struct AsyncToolMarker;

impl<Func, Input, Out, Fut> IntoTool<(AsyncToolMarker, Input, Out)> for Func
where
	Func: 'static + Send + Sync + Clone + Fn(ToolContext<Input>) -> Fut,
	Input: 'static + Send + Sync,
	Fut: 'static + MaybeSend + Future<Output = Result<Out>>,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool(self) -> Tool<Self::In, Self::Out> { Tool::new_async(self) }
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
		Tool::new_async(move |input: ToolContext<Input>| {
			let fut = self(input.input);
			async move { fut.await.xok::<BevyError>() }
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
			.spawn(Tool::<(i32, i32), i32>::new_async(
				async |input: ToolContext<(i32, i32)>| -> Result<i32> {
					Ok(input.0 + input.1)
				},
			))
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(7);
	}

	#[beet_core::test]
	async fn negate() {
		AsyncPlugin::world()
			.spawn(Tool::<i32, i32>::new_async(
				async |input: ToolContext<i32>| -> Result<i32> { Ok(-*input) },
			))
			.call::<i32, i32>(42)
			.await
			.unwrap()
			.xpect_eq(-42);
	}

	#[beet_core::test]
	async fn returns_tool_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(Tool::<(), Entity>::new_async(
				async |cx: ToolContext| -> Result<Entity> {
					Ok(cx.caller.id())
				},
			))
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
			.spawn(Tool::<String, String>::new_async(
				async |input: ToolContext<String>| -> Result<String> {
					Ok(format!("hello {}", *input))
				},
			))
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
	async fn async_add((a, b): (i32, i32)) -> i32 { a + b }

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
	async fn async_passthrough_tool(cx: ToolContext<i32>) -> i32 { *cx * 3 }

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
	async fn async_passthrough_entity(cx: ToolContext) -> Entity {
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
