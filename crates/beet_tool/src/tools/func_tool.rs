use crate::prelude::*;
use beet_core::prelude::*;

pub fn func_tool<F, Input, Out>(func: F) -> Tool<Input, Out>
where
	F: 'static
		+ Send
		+ Sync
		+ Clone
		+ FnOnce(ToolContext<Input>) -> Result<Out>,
{
	Tool::<Input, Out>::new(
		TypeMeta::of::<F>(),
		move |ToolCall {
		          commands,
		          caller,
		          input,
		          out_handler,
		      }| {
			let async_entity = commands.world().entity(caller);
			let cx = ToolContext {
				caller: async_entity,
				input,
			};
			let result = func.clone()(cx);
			out_handler.call(commands, result)
		},
	)
}

pub struct FuncToolMarker;

impl<F, I, O> IntoTool<(FuncToolMarker, I, O)> for F
where
	F: 'static + Send + Sync + Clone + FnOnce(ToolContext<I>) -> Result<O>,
{
	type In = I;
	type Out = O;

	fn into_tool(self) -> Tool<Self::In, Self::Out> { func_tool(self) }
}

pub struct TypedFuncToolMarker;

impl<F, I, O> IntoTool<(TypedFuncToolMarker, I, O)> for F
where
	F: 'static + Send + Sync + Clone + FnOnce(I) -> O,
	O: bevy::reflect::Typed,
{
	type In = I;
	type Out = O;

	fn into_tool(self) -> Tool<Self::In, Self::Out> {
		func_tool(move |input| self(input.input).xok())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn works() {
		AsyncPlugin::world()
			.spawn(func_tool(|input: ToolContext<(i32, i32)>| {
				Ok(input.0 + input.1)
			}))
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[tool(pure)]
	fn no_args_tool() {}

	#[beet_core::test]
	async fn tool_macro_no_args() {
		AsyncPlugin::world()
			.spawn(no_args_tool.into_tool())
			.call::<(), ()>(())
			.await
			.unwrap();
	}

	#[tool(pure)]
	fn add_tool((a, b): (i32, i32)) -> i32 { a + b }

	#[beet_core::test]
	async fn tool_macro_with_args() {
		AsyncPlugin::world()
			.spawn(add_tool.into_tool())
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[tool(pure)]
	fn single_arg_tool(val: i32) -> i32 { val * 3 }

	#[beet_core::test]
	async fn tool_macro_single_arg() {
		AsyncPlugin::world()
			.spawn(single_arg_tool.into_tool())
			.call::<i32, i32>(7)
			.await
			.unwrap()
			.xpect_eq(21);
	}

	#[tool(pure)]
	fn fallible_tool((a, b): (i32, i32)) -> Result<i32> {
		if b == 0 {
			bevybail!("cannot be zero");
		}
		Ok(a + b)
	}

	#[beet_core::test]
	async fn tool_macro_result_ok() {
		AsyncPlugin::world()
			.spawn(fallible_tool.into_tool())
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[beet_core::test]
	async fn tool_macro_result_err() {
		AsyncPlugin::world()
			.spawn(fallible_tool.into_tool())
			.call::<(i32, i32), i32>((5, 0))
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be zero");
	}

	#[tool(pure, result_out)]
	fn result_out_tool(val: i32) -> Result<i32> { Ok(val * 2) }

	#[beet_core::test]
	async fn tool_macro_result_out() {
		AsyncPlugin::world()
			.spawn(result_out_tool.into_tool())
			.call::<i32, Result<i32>>(4)
			.await
			.unwrap()
			.unwrap()
			.xpect_eq(8);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — func passthrough
	// -----------------------------------------------------------------------

	#[tool(pure)]
	fn func_passthrough_tool(cx: ToolContext<i32>) -> i32 { *cx * 3 }

	#[beet_core::test]
	async fn tool_macro_func_passthrough() {
		AsyncPlugin::world()
			.spawn(func_passthrough_tool.into_tool())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(15);
	}

	#[tool(pure)]
	fn func_passthrough_entity(cx: ToolContext) -> Entity { cx.id() }

	#[beet_core::test]
	async fn tool_macro_func_passthrough_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(func_passthrough_entity.into_tool()).id();
		world
			.entity_mut(entity)
			.call::<(), Entity>(())
			.await
			.unwrap()
			.xpect_eq(entity);
	}
}
