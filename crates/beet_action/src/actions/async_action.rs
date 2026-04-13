use crate::prelude::*;
use beet_core::prelude::*;

impl<In, Out> Action<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// Create an [`Action`] from an async closure that receives [`ActionContext`]
	/// and returns a value convertible to `Result<Out>` via [`IntoResult`].
	///
	/// Accepts closures returning either `Out` or `Result<Out>`.
	pub fn new_async<Func, Fut, RawOut>(func: Func) -> Self
	where
		Func: 'static + Send + Sync + Clone + FnOnce(ActionContext<In>) -> Fut,
		Fut: 'static + MaybeSend + Future<Output = RawOut>,
		RawOut: IntoResult<Out>,
	{
		Action::new(
			TypeMeta::of::<Func>(),
			move |ActionCall {
			          mut commands,
			          caller,
			          input,
			          out_handler,
			      }| {
				let async_entity = commands.world().entity(caller);
				let arg = ActionContext {
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


/// Marker for the async action [`IntoAction`] impl accepting
/// `Fn(ActionContext<I>) -> Future<Output = Result<O>>`.
pub struct AsyncActionMarker;

impl<Func, Input, Out, Fut> IntoAction<(AsyncActionMarker, Input, Out)> for Func
where
	Func: 'static + Send + Sync + Clone + Fn(ActionContext<Input>) -> Fut,
	Input: 'static + Send + Sync,
	Fut: 'static + MaybeSend + Future<Output = Result<Out>>,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_action(self) -> Action<Self::In, Self::Out> { Action::new_async(self) }
}

/// Marker for the typed async action [`IntoAction`] impl accepting
/// `Fn(I) -> Future<Output = O>` where `I` is a plain input type.
pub struct TypedAsyncActionMarker;

impl<Func, Input, Out, Fut> IntoAction<(TypedAsyncActionMarker, Input, Out)>
	for Func
where
	Func: 'static + Send + Sync + Clone + Fn(Input) -> Fut,
	Input: 'static + Send + Sync + bevy::reflect::Typed,
	Fut: 'static + MaybeSend + Future<Output = Out>,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_action(self) -> Action<Self::In, Self::Out> {
		Action::new_async(move |input: ActionContext<Input>| {
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
			.spawn(Action::<(i32, i32), i32>::new_async(
				async |input: ActionContext<(i32, i32)>| -> Result<i32> {
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
			.spawn(Action::<i32, i32>::new_async(
				async |input: ActionContext<i32>| -> Result<i32> { Ok(-*input) },
			))
			.call::<i32, i32>(42)
			.await
			.unwrap()
			.xpect_eq(-42);
	}

	#[beet_core::test]
	async fn returns_action_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(Action::<(), Entity>::new_async(
				async |cx: ActionContext| -> Result<Entity> {
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
			.spawn(Action::<String, String>::new_async(
				async |input: ActionContext<String>| -> Result<String> {
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
			.spawn((async |val: i32| -> i32 { -val }).into_action())
			.call::<i32, i32>(42)
			.await
			.unwrap()
			.xpect_eq(-42);
	}

	#[beet_core::test]
	async fn typed_async_add() {
		AsyncPlugin::world()
			.spawn((async |(a, b): (i32, i32)| -> i32 { a + b }).into_action())
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(7);
	}

	// -----------------------------------------------------------------------
	// #[action] macro — async actions
	// -----------------------------------------------------------------------

	#[action]
	async fn async_negate(val: i32) -> i32 { -val }

	#[beet_core::test]
	async fn action_macro_async_single_arg() {
		AsyncPlugin::world()
			.spawn(async_negate.into_action())
			.call::<i32, i32>(7)
			.await
			.unwrap()
			.xpect_eq(-7);
	}

	#[action]
	async fn async_add((a, b): (i32, i32)) -> i32 { a + b }

	#[beet_core::test]
	async fn action_macro_async_multi_arg() {
		AsyncPlugin::world()
			.spawn(async_add.into_action())
			.call::<(i32, i32), i32>((3, 4))
			.await
			.unwrap()
			.xpect_eq(7);
	}

	#[action]
	async fn async_no_args() -> i32 { 42 }

	#[beet_core::test]
	async fn action_macro_async_no_args() {
		AsyncPlugin::world()
			.spawn(async_no_args.into_action())
			.call::<(), i32>(())
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[action]
	async fn async_fallible(val: i32) -> Result<i32> {
		if val == 0 {
			bevybail!("zero");
		}
		Ok(val * 2)
	}

	#[beet_core::test]
	async fn action_macro_async_result_ok() {
		AsyncPlugin::world()
			.spawn(async_fallible.into_action())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(10);
	}

	// -----------------------------------------------------------------------
	// #[action] macro — async passthrough
	// -----------------------------------------------------------------------

	#[action]
	async fn async_passthrough_action(cx: ActionContext<i32>) -> i32 { *cx * 3 }

	#[beet_core::test]
	async fn action_macro_async_passthrough() {
		AsyncPlugin::world()
			.spawn(async_passthrough_action.into_action())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(15);
	}

	#[action]
	async fn async_passthrough_entity(cx: ActionContext) -> Entity {
		cx.caller.id()
	}

	#[beet_core::test]
	async fn action_macro_async_passthrough_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(async_passthrough_entity.into_action()).id();
		world
			.entity_mut(entity)
			.call::<(), Entity>(())
			.await
			.unwrap()
			.xpect_eq(entity);
	}
}
