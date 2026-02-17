use crate::prelude::*;
use beet_core::prelude::*;

pub struct FuncTool<In, Out> {
	handler: Box<dyn FnMut(ToolContext<In>) -> Out>,
}

impl<In, Out> FuncTool<In, Out> {
	pub fn new<F>(f: F) -> Self
	where
		F: FnMut(ToolContext<In>) -> Out + 'static,
	{
		Self {
			handler: Box::new(f),
		}
	}
}
impl<In, Out> ToolHandler for FuncTool<In, Out> {
	type In = In;
	type Out = Out;

	fn call(
		&mut self,
		// pure functions need no world access
		_commands: Commands,
		_async_commands: AsyncCommands,
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

impl<F, In, Arg, Out> IntoToolHandler2<(FuncTool<In, Out>, In, Arg, Out)> for F
where
	F: 'static + FnMut(Arg) -> Out,
	Arg: From<ToolContext<In>>,
{
	type In = In;
	type Out = Out;

	fn into_handler(
		mut self,
	) -> impl ToolHandler<In = Self::In, Out = Self::Out> {
		FuncTool::new(move |cx| {
			let arg = Arg::from(cx);
			self(arg)
		})
	}
}
