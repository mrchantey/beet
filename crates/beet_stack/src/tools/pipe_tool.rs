//! Chainable tool handler that pipes the output of one handler into another.
//!
//! [`PipeTool`] composes two [`ToolHandler`] implementations sequentially:
//! the output of handler A is converted into the input of handler B.

use crate::prelude::*;
use beet_core::prelude::*;

/// A tool handler that chains two handlers: A then B.
///
/// The output of handler A is converted via `Into` to the input of handler B.
/// Both handlers are called synchronously with exclusive world access.
#[derive(Any)]
pub struct PipeTool<A, B>
where
	A: 'static,
	B: 'static,
{
	tool_a: A,
	tool_b: B,
}

impl<A, B> PipeTool<A, B> {
	/// Create a new pipe from two handlers.
	pub fn new(tool_a: A, tool_b: B) -> Self { Self { tool_a, tool_b } }
}

impl<A, B> ToolHandler for PipeTool<A, B>
where
	A: ToolHandler,
	A::Out: 'static + Send + Sync,
	B: ToolHandler,
	B::In: 'static + Send + Sync + From<A::Out>,
	B::Out: 'static + Send + Sync,
{
	type In = A::In;
	type Out = B::Out;

	fn call(
		&mut self,
		world: &mut World,
		ToolCall {
			tool,
			input,
			out_handler: out_handler_b,
		}: ToolCall<Self::In, Self::Out>,
	) -> Result {
		// Capture the intermediate output from handler A via a channel.
		let (send, recv) =
			beet_core::exports::async_channel::bounded::<A::Out>(1);
		let out_handler_a = OutHandler::new(move |output_a: A::Out| {
			send.try_send(output_a).map_err(|err| {
				bevyhow!("Pipe intermediate send failed: {err:?}")
			})
		});

		let call_a = ToolCall {
			tool,
			input,
			out_handler: out_handler_a,
		};
		self.tool_a.call(world, call_a)?;

		// For synchronous handlers the output is available immediately.
		let output_a = recv.try_recv().map_err(|err| {
			bevyhow!("Pipe intermediate recv failed: {err:?}")
		})?;

		let call_b = ToolCall {
			tool,
			input: output_a.into(),
			out_handler: out_handler_b,
		};
		self.tool_b.call(world, call_b)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn pipes_two_func_tools() {
		let add = FuncTool::new(|cx: ToolContext<(i32, i32)>| -> i32 {
			cx.input.0 + cx.input.1
		});
		let double =
			FuncTool::new(|cx: ToolContext<i32>| -> i32 { cx.input * 2 });
		let pipe = PipeTool::new(add, double);

		World::new()
			.spawn(Tool::new(pipe))
			.call2_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(14); // (3+4)*2
	}

	#[test]
	fn pipes_three_tools() {
		let add = FuncTool::new(|cx: ToolContext<(i32, i32)>| -> i32 {
			cx.input.0 + cx.input.1
		});
		let double =
			FuncTool::new(|cx: ToolContext<i32>| -> i32 { cx.input * 2 });
		let negate = FuncTool::new(|cx: ToolContext<i32>| -> i32 { -cx.input });
		let pipe = PipeTool::new(PipeTool::new(add, double), negate);

		World::new()
			.spawn(Tool::new(pipe))
			.call2_blocking::<(i32, i32), i32>((1, 2))
			.unwrap()
			.xpect_eq(-6); // -((1+2)*2)
	}
}
