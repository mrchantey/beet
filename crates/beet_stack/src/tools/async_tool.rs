//! Async function tool handler.
//!
//! [`AsyncTool`] wraps an async function, scheduling it via the
//! [`AsyncChannel`] so it runs on the task pool. The output is
//! delivered through the [`OutHandler`] callback when the future completes.

use crate::prelude::*;
use beet_core::prelude::*;

/// A tool handler backed by an async function.
///
/// The wrapped async closure is spawned as a task via the world's
/// [`AsyncChannel`]. Input is converted from the [`ToolContext`] via
/// [`FromAsyncToolContext`], and output is converted via [`IntoToolOutput`].
#[derive(Any)]
pub struct AsyncTool<In: 'static, Out: 'static> {
	runner: Box<
		dyn 'static
			+ Send
			+ Sync
			+ FnMut(&mut World, ToolCall<In, Out>) -> Result,
	>,
}

impl<In: 'static, Out: 'static> AsyncTool<In, Out> {
	/// Create a new [`AsyncTool`] from a runner closure.
	///
	/// Prefer the [`IntoToolHandler2`] blanket impl over calling this directly.
	pub fn new<F>(runner: F) -> Self
	where
		F: 'static
			+ Send
			+ Sync
			+ FnMut(&mut World, ToolCall<In, Out>) -> Result,
	{
		Self {
			runner: Box::new(runner),
		}
	}
}

impl<In: 'static, Out: 'static> ToolHandler for AsyncTool<In, Out> {
	type In = In;
	type Out = Out;

	fn call(
		&mut self,
		world: &mut World,
		call: ToolCall<Self::In, Self::Out>,
	) -> Result {
		(self.runner)(world, call)
	}
}

/// Blanket [`IntoToolHandler2`] impl for async closures.
///
/// The async function is spawned on the IO task pool. Its output
/// is delivered to the [`OutHandler`] when the future resolves.
impl<Func, In, Fut, Arg, Out, IntoOut, IntoOutM, ArgM>
	IntoToolHandler2<(
		AsyncTool<In, Out>,
		In,
		Arg,
		Out,
		IntoOut,
		IntoOutM,
		ArgM,
	)> for Func
where
	Func: 'static + Send + Sync + Clone + Fn(Arg) -> Fut,
	Arg: 'static + Send + Sync + FromAsyncToolContext<In, ArgM>,
	In: 'static + Send + Sync,
	Fut: 'static + Send + Future<Output = IntoOut>,
	IntoOut: 'static + IntoToolOutput<Out, IntoOutM>,
	Out: 'static + Send + Sync,
{
	type In = In;
	type Out = Out;

	fn into_tool_handler(
		self,
	) -> impl ToolHandler<In = Self::In, Out = Self::Out> {
		let func = self;
		AsyncTool::new(move |world: &mut World, call: ToolCall<In, Out>| {
			let async_world = world.resource::<AsyncChannel>().world();
			let arg = Arg::from_async_tool_context(AsyncToolContext::new(
				async_world.entity(call.tool),
				call.input,
			));
			let this = func.clone();
			let out_handler = call.out_handler;
			world.run_async(move |_| async move {
				let output = this(arg).await.into_tool_output()?;
				out_handler.call(output)?;
				Ok(())
			});
			Ok(())
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn async_add() {
		let handler = (async |input: (i32, i32)| -> i32 { input.0 + input.1 })
			.into_tool_handler();
		AsyncPlugin::world()
			.spawn(Tool::new(handler))
			.call2_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(7);
	}

	#[test]
	fn async_with_context() {
		let handler =
			(async |cx: AsyncToolContext<i32>| -> i32 { cx.input * 3 })
				.into_tool_handler();
		AsyncPlugin::world()
			.spawn(Tool::new(handler))
			.call2_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(15);
	}

	#[test]
	fn async_returns_result() {
		let handler = (async |input: i32| -> Result<i32> { (input + 1).xok() })
			.into_tool_handler();
		AsyncPlugin::world()
			.spawn(Tool::new(handler))
			.call2_blocking::<i32, i32>(9)
			.unwrap()
			.xpect_eq(10);
	}
}
