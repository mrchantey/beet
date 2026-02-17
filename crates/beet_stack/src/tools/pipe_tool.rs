use crate::prelude::*;
use beet_core::prelude::*;


pub struct PipeTool<A, B> {
	tool_a: A,
	tool_b: B,
}


impl<A, B> ToolHandler for PipeTool<A, B>
where
	A: ToolHandler,
	B: 'static + Send + Sync + Clone + ToolHandler,
	B::In: 'static + Send + Sync + From<A::Out>,
	B::Out: 'static,
{
	type In = A::In;
	type Out = B::Out;

	fn call(
		&mut self,
		commands: Commands,
		async_commands: AsyncCommands,
		ToolCall {
			tool,
			input,
			out_handler: out_handler_b,
		}: ToolCall<Self::In, Self::Out>,
	) -> Result {
		// let cx = ToolContext { tool, input };
		let tool_b = self.tool_b.clone();
		let world = async_commands.world();
		let out_handler_a = OutHandler::new(move |output_a: A::Out| {
			let call2 = ToolCall {
				tool,
				input: output_a.into(),
				out_handler: out_handler_b,
			};
			let _ = world.run_async(async move |world| {
				world
					.run_system_cached_with::<_, Result, _, _>(
						|In((mut tool_b, call)): In<(
							B,
							ToolCall<B::In, B::Out>,
						)>,
						 commands: Commands,
						 async_commands: AsyncCommands|
						 -> Result {
							tool_b.call(commands, async_commands, call)
						},
						(tool_b, call2),
					)
					.await?
			});

			Ok(())
		});

		let tool_call_a = ToolCall {
			tool,
			input,
			out_handler: out_handler_a,
		};

		self.tool_a.call(commands, async_commands, tool_call_a)?;
		Ok(())
	}
}
