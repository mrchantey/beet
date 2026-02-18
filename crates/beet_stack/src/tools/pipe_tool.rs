use crate::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

/// Allows chaining two [`ToolHandler`] together, feeding the output of
/// the first into the input of the second.
pub trait IntoPipeTool<In, Out, M>
where
	Self: Sized + IntoToolHandler2<M, In = In, Out = Out>,
	Out: 'static,
{
	fn pipe<T2, M2>(self, other: T2) -> ToolHandler<In, T2::Out>
	where
		T2: IntoToolHandler2<M2>,
		T2::In: 'static + From<Out>,
	{
		let mut handler1 = self.into_tool_handler();
		let handler2 = Arc::new(Mutex::new(other.into_tool_handler()));

		ToolHandler::new(
			move |ToolCall {
			          commands,
			          tool,
			          input: in_a,
			          out_handler,
			      }: ToolCall<In, T2::Out>| {
				let handler2 = Arc::clone(&handler2);
				handler1.call(ToolCall {
					commands,
					tool,
					input: in_a,
					out_handler: OutHandler::new(
						move |commands, out_a: Out| {
							handler2.lock().unwrap().call(ToolCall::<
								T2::In,
								T2::Out,
							> {
								commands,
								tool,
								input: out_a.into(),
								out_handler,
							})
						},
					),
				})
			},
		)
	}
}


impl<In, Out, M, T> IntoPipeTool<In, Out, M> for T
where
	T: IntoToolHandler2<M, In = In, Out = Out>,
	Out: 'static,
{
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn add((a, b): (i32, i32)) -> i32 { a + b }
	fn negate(a: i32) -> i32 { -a }
	fn multiply(a: i32) -> i32 { a * a }

	#[test]
	fn pipe_two() {
		AsyncPlugin::world()
			.spawn(tool2(add.pipe(negate)))
			.call2_blocking::<(i32, i32), i32>((5, 2))
			.unwrap()
			.xpect_eq(-7);
	}
	#[test]
	fn pipe_three() {
		AsyncPlugin::world()
			.spawn(tool2(add.pipe(multiply).pipe(negate)))
			.call2_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(-64);
	}
}
