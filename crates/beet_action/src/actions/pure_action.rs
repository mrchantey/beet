use crate::prelude::*;
use beet_core::prelude::*;

impl<In, Out> Action<In, Out>
where
	In: 'static,
	Out: 'static,
{
	/// Create an [`Action`] from a pure closure that receives [`ActionContext`]
	/// and returns a value convertible to `Result<Out>` via [`IntoResult`].
	///
	/// Accepts closures returning either `Out` or `Result<Out>`.
	pub fn new_pure<Func, RawOut>(func: Func) -> Self
	where
		Func: 'static + Send + Sync + Clone + FnOnce(ActionContext<In>) -> RawOut,
		RawOut: IntoResult<Out>,
	{
		Action::new(
			TypeMeta::of::<Func>(),
			move |ActionCall {
			          commands,
			          caller,
			          input,
			          out_handler,
			      }| {
				let async_entity = commands.world().entity(caller);
				let cx = ActionContext {
					caller: async_entity,
					input,
				};
				let result: Result<Out> = func.clone()(cx).into_result();
				out_handler.call(commands, result)
			},
		)
	}
}



pub struct FuncActionMarker;

impl<F, I, O> IntoAction<(FuncActionMarker, I, O)> for F
where
	F: 'static + Send + Sync + Clone + FnOnce(ActionContext<I>) -> Result<O>,
{
	type In = I;
	type Out = O;

	fn into_action(self) -> Action<Self::In, Self::Out> { Action::new_pure(self) }
}

pub struct TypedFuncActionMarker;

impl<F, I, O> IntoAction<(TypedFuncActionMarker, I, O)> for F
where
	F: 'static + Send + Sync + Clone + FnOnce(I) -> O,
	O: bevy::reflect::Typed,
{
	type In = I;
	type Out = O;

	fn into_action(self) -> Action<Self::In, Self::Out> {
		Action::new_pure(move |input: ActionContext<I>| {
			self(input.input).xok::<BevyError>()
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
			.spawn(Action::<(i32, i32), i32>::new_pure(
				|input: ActionContext<(i32, i32)>| -> Result<i32> {
					Ok(input.0 + input.1)
				},
			))
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[action(pure)]
	fn no_args_action() {}

	#[beet_core::test]
	async fn action_macro_no_args() {
		AsyncPlugin::world()
			.spawn(no_args_action.into_action())
			.call::<(), ()>(())
			.await
			.unwrap();
	}

	#[action(pure)]
	fn add_action((a, b): (i32, i32)) -> i32 { a + b }

	#[beet_core::test]
	async fn action_macro_with_args() {
		AsyncPlugin::world()
			.spawn(add_action.into_action())
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[action(pure)]
	fn single_arg_action(val: i32) -> i32 { val * 3 }

	#[beet_core::test]
	async fn action_macro_single_arg() {
		AsyncPlugin::world()
			.spawn(single_arg_action.into_action())
			.call::<i32, i32>(7)
			.await
			.unwrap()
			.xpect_eq(21);
	}

	#[action(pure)]
	fn fallible_action((a, b): (i32, i32)) -> Result<i32> {
		if b == 0 {
			bevybail!("cannot be zero");
		}
		Ok(a + b)
	}

	#[beet_core::test]
	async fn action_macro_result_ok() {
		AsyncPlugin::world()
			.spawn(fallible_action.into_action())
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[beet_core::test]
	async fn action_macro_result_err() {
		AsyncPlugin::world()
			.spawn(fallible_action.into_action())
			.call::<(i32, i32), i32>((5, 0))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be zero");
	}

	#[action(pure, result_out)]
	fn result_out_action(val: i32) -> Result<i32> { Ok(val * 2) }

	#[beet_core::test]
	async fn action_macro_result_out() {
		AsyncPlugin::world()
			.spawn(result_out_action.into_action())
			.call::<i32, Result<i32>>(4)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(8);
	}

	// -----------------------------------------------------------------------
	// #[action] macro — func passthrough
	// -----------------------------------------------------------------------

	#[action(pure)]
	fn func_passthrough_action(cx: ActionContext<i32>) -> i32 { *cx * 3 }

	#[beet_core::test]
	async fn action_macro_func_passthrough() {
		AsyncPlugin::world()
			.spawn(func_passthrough_action.into_action())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(15);
	}

	#[action(pure)]
	fn func_passthrough_entity(cx: ActionContext) -> Entity { cx.id() }

	#[beet_core::test]
	async fn action_macro_func_passthrough_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(func_passthrough_entity.into_action()).id();
		world
			.entity_mut(entity)
			.call::<(), Entity>(())
			.await
			.unwrap()
			.xpect_eq(entity);
	}
}
