use crate::prelude::*;
use beet_core::prelude::*;



pub fn func_tool<F, Input, Out>(func: F) -> ToolHandler<Input, Out>
where
	F: 'static + Send + Sync + Fn(FuncToolIn<Input>) -> Result<Out>,
{
	ToolHandler::<Input, Out>::new(
		TypeMeta::of::<F>(),
		move |ToolCall {
		          commands,
		          tool,
		          input,
		          out_handler,
		      }| {
			let cx = FuncToolIn { tool, input };
			let out = func(cx)?;
			out_handler.call(commands, out)
		},
	)
}

/// Context passed to tool handlers containing the tool entity and input payload.
pub struct FuncToolIn<In = ()> {
	/// The async tool entity being called.
	pub tool: Entity,
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

pub struct FuncToolMarker;

impl<F, I, O> IntoToolHandler<(FuncToolMarker, I, O)> for F
where
	F: 'static + Send + Sync + Fn(FuncToolIn<I>) -> Result<O>,
{
	type In = I;
	type Out = O;

	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> {
		func_tool(self)
	}
}

pub struct TypedFuncToolMarker;

impl<F, I, O> IntoToolHandler<(TypedFuncToolMarker, I, O)> for F
where
	F: 'static + Send + Sync + Fn(I) -> O,
	O: bevy::reflect::Typed,
{
	type In = I;
	type Out = O;

	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> {
		func_tool(move |input| self(input.input).xok())
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn works() {
		AsyncPlugin::world()
			.spawn(func_tool(|input: FuncToolIn<(i32, i32)>| {
				Ok(input.0 + input.1)
			}))
			.call_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(8);
	}

	#[tool]
	fn no_args_tool() {}

	#[test]
	fn tool_macro_no_args() {
		AsyncPlugin::world()
			.spawn(no_args_tool.into_tool_handler())
			.call_blocking::<(), ()>(())
			.unwrap();
	}

	#[tool]
	fn add_tool(a: i32, b: i32) -> i32 { a + b }

	#[test]
	fn tool_macro_with_args() {
		AsyncPlugin::world()
			.spawn(add_tool.into_tool_handler())
			.call_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(8);
	}

	#[tool]
	fn single_arg_tool(val: i32) -> i32 { val * 3 }

	#[test]
	fn tool_macro_single_arg() {
		AsyncPlugin::world()
			.spawn(single_arg_tool.into_tool_handler())
			.call_blocking::<i32, i32>(7)
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

	#[test]
	fn tool_macro_result_ok() {
		AsyncPlugin::world()
			.spawn(fallible_tool.into_tool_handler())
			.call_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(8);
	}

	#[test]
	fn tool_macro_result_err() {
		AsyncPlugin::world()
			.spawn(fallible_tool.into_tool_handler())
			.call_blocking::<(i32, i32), i32>((5, 0))
			.unwrap_err()
			.to_string()
			.xpect_contains("cannot be zero");
	}

	#[tool(result_out)]
	fn result_out_tool(val: i32) -> Result<i32> { Ok(val * 2) }

	#[test]
	fn tool_macro_result_out() {
		AsyncPlugin::world()
			.spawn(result_out_tool.into_tool_handler())
			.call_blocking::<i32, Result<i32>>(4)
			.unwrap()
			.unwrap()
			.xpect_eq(8);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — func passthrough
	// -----------------------------------------------------------------------

	#[tool]
	fn func_passthrough_tool(cx: FuncToolIn<i32>) -> i32 { *cx * 3 }

	#[test]
	fn tool_macro_func_passthrough() {
		AsyncPlugin::world()
			.spawn(func_passthrough_tool.into_tool_handler())
			.call_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(15);
	}

	#[tool]
	fn func_passthrough_entity(cx: FuncToolIn<()>) -> Entity { cx.tool }

	#[test]
	fn tool_macro_func_passthrough_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(func_passthrough_entity.into_tool_handler())
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}
}
