use crate::prelude::*;
use beet_core::prelude::*;



pub fn func_tool<F, Input, Out>(func: F) -> Tool<Input, Out>
where
	F: 'static + Send + Sync + Clone + FnOnce(FuncToolIn<Input>) -> Result<Out>,
{
	Tool::<Input, Out>::new(
		TypeMeta::of::<F>(),
		move |ToolCall {
		          commands,
		          caller,
		          input,
		          out_handler,
		      }| {
			let cx = FuncToolIn { caller, input };
			let result = func.clone()(cx);
			out_handler.call(commands, result)
		},
	)
}

/// Context passed to tool handlers containing the caller entity and input payload.
pub struct FuncToolIn<In = ()> {
	/// The entity that initiated this tool call.
	pub caller: Entity,
	/// The input payload for this tool call.
	pub input: In,
}

impl<In> std::ops::Deref for FuncToolIn<In> {
	type Target = In;

	fn deref(&self) -> &Self::Target { &self.input }
}
impl<In> std::ops::DerefMut for FuncToolIn<In> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.input }
}

impl<In> FuncToolIn<In> {
	pub fn take(self) -> In { self.input }
}

pub struct FuncToolMarker;

impl<F, I, O> IntoTool<(FuncToolMarker, I, O)> for F
where
	F: 'static + Send + Sync + Clone + FnOnce(FuncToolIn<I>) -> Result<O>,
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
			.spawn(func_tool(|input: FuncToolIn<(i32, i32)>| {
				Ok(input.0 + input.1)
			}))
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[tool]
	fn no_args_tool() {}

	#[beet_core::test]
	async fn tool_macro_no_args() {
		AsyncPlugin::world()
			.spawn(no_args_tool.into_tool())
			.call::<(), ()>(())
			.await
			.unwrap();
	}

	#[tool]
	fn add_tool(a: i32, b: i32) -> i32 { a + b }

	#[beet_core::test]
	async fn tool_macro_with_args() {
		AsyncPlugin::world()
			.spawn(add_tool.into_tool())
			.call::<(i32, i32), i32>((5, 3))
			.await
			.unwrap()
			.xpect_eq(8);
	}

	#[tool]
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

	#[tool]
	fn fallible_tool(a: i32, b: i32) -> Result<i32> {
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

	#[tool(result_out)]
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

	#[tool]
	fn func_passthrough_tool(cx: FuncToolIn<i32>) -> i32 { *cx * 3 }

	#[beet_core::test]
	async fn tool_macro_func_passthrough() {
		AsyncPlugin::world()
			.spawn(func_passthrough_tool.into_tool())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(15);
	}

	#[tool]
	fn func_passthrough_entity(cx: FuncToolIn<()>) -> Entity { cx.caller }

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
