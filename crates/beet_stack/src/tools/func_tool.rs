//! Pure function tool handler.
//!
//! [`FuncTool`] wraps a closure that maps a [`ToolContext`] to an output
//! value without requiring any ECS world access.

use crate::prelude::*;
use beet_core::prelude::*;

/// A tool handler backed by a pure function.
///
/// The wrapped closure receives a [`ToolContext`] and returns an output
/// value synchronously. No world access is needed, making this the
/// simplest handler type.
#[derive(Any)]
pub struct FuncTool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	handler: Box<dyn 'static + Send + Sync + FnMut(ToolContext<In>) -> Out>,
}

impl<In, Out> FuncTool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	/// Create a new [`FuncTool`] from a closure.
	pub fn new<F>(func: F) -> Self
	where
		F: 'static + Send + Sync + FnMut(ToolContext<In>) -> Out,
	{
		Self {
			handler: Box::new(func),
		}
	}
}

impl<In, Out> ToolHandler for FuncTool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type In = In;
	type Out = Out;

	fn call(
		&mut self,
		_world: &mut World,
		ToolCall {
			tool,
			input,
			out_handler,
		}: ToolCall<Self::In, Self::Out>,
	) -> Result {
		let cx = ToolContext { tool, input };
		let output = (self.handler)(cx);
		out_handler.call(output)
	}
}

/// Blanket [`IntoToolHandler2`] impl for closures that accept an argument
/// convertible from [`ToolContext`].
impl<F, In, Arg, Out> IntoToolHandler2<(FuncTool<In, Out>, In, Arg, Out)> for F
where
	F: 'static + Send + Sync + FnMut(Arg) -> Out,
	Arg: From<ToolContext<In>>,
{
	type In = In;
	type Out = Out;

	fn into_tool_handler(
		mut self,
	) -> impl ToolHandler<In = Self::In, Out = Self::Out> {
		FuncTool::new(move |cx| {
			let arg = Arg::from(cx);
			self(arg)
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn pure_add() {
		let tool = FuncTool::new(|cx: ToolContext<(i32, i32)>| -> i32 {
			cx.input.0 + cx.input.1
		});
		World::new()
			.spawn(Tool::new(tool))
			.call2_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(8);
	}

	#[test]
	fn returns_tool_entity() {
		let mut world = World::new();
		let entity = world
			.spawn(Tool::new(FuncTool::new(|cx: ToolContext<()>| -> Entity {
				cx.tool
			})))
			.id();
		world
			.entity_mut(entity)
			.call2_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}

	#[test]
	fn into_handler_closure() {
		let handler = (|cx: ToolContext<i32>| -> i32 { cx.input * 2 })
			.into_tool_handler();
		World::new()
			.spawn(Tool::new(handler))
			.call2_blocking::<i32, i32>(7)
			.unwrap()
			.xpect_eq(14);
	}
}
