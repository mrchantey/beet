//! [`IntoToolHandler`] implementation for plain closures.
//!
//! Any `FnMut(Arg) -> Out` where `Arg` implements [`FromToolContext`]
//! automatically becomes a tool handler. This is the simplest handler
//! kind — it runs synchronously with no world access.
//!
//! ## Extractors
//!
//! The closure's argument is created via [`FromToolContext`], which
//! allows extracting either the raw input payload or the full
//! [`ToolContext`] (payload + tool entity).
//!
//! ## Examples
//!
//! ```rust
//! # use beet_stack::prelude::*;
//! # use beet_core::prelude::*;
//! // Pure function — input extracted via `Reflect` blanket impl
//! let handler = tool(|(a, b): (i32, i32)| a + b);
//!
//! // Access the tool entity via `ToolContext`
//! let handler = tool(|cx: ToolContext<()>| -> Entity { cx.tool });
//! ```
use crate::prelude::*;

/// Blanket [`IntoToolHandler`] impl for closures that accept an argument
/// convertible from [`ToolContext`].
///
/// The [`IntoToolOutput`] bound on the return type ensures that async
/// closures (whose return type is a [`Future`], not [`Reflect`]) are
/// routed to the [`async_tool`](super::async_tool) impl instead.
impl<F, In, Arg, ArgM, IntoOut, Out, IntoOutM>
	IntoToolHandler<(In, Arg, ArgM, IntoOut, Out, IntoOutM)> for F
where
	F: 'static + Send + Sync + FnMut(Arg) -> IntoOut,
	Arg: FromToolContext<In, ArgM>,
	IntoOut: IntoToolOutput<Out, IntoOutM>,
{
	type In = In;
	type Out = Out;

	fn into_tool_handler(mut self) -> ToolHandler<Self::In, Self::Out> {
		ToolHandler::new(
			move |ToolCall {
			          commands,
			          tool,
			          input,
			          out_handler,
			      }| {
				let arg = Arg::from_tool_context(ToolContext { tool, input });
				let out = self(arg).into_tool_output()?;
				out_handler.call(commands, out)
			},
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn pure_add() {
		AsyncPlugin::world()
			.spawn(tool(|(a, b): (i32, i32)| a + b))
			.call_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(8);
	}

	#[test]
	fn returns_tool_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(tool(|cx: ToolContext<()>| -> Entity { cx.tool }))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}

	#[test]
	fn captures_mutable() {
		let mut val = 0;
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(tool(move |_cx: ToolContext| -> i32 {
				val += 1;
				val
			}))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), i32>(())
			.unwrap()
			.xpect_eq(1);
		world
			.entity_mut(entity)
			.call_blocking::<(), i32>(())
			.unwrap()
			.xpect_eq(2);
	}

	#[test]
	fn unit_output() {
		AsyncPlugin::world()
			.spawn(tool(|_: u32| {}))
			.call_blocking::<u32, ()>(0)
			.unwrap();
	}

	#[test]
	fn request_mirror() {
		let request = Request::get("/hello");
		AsyncPlugin::world()
			.spawn(tool(Request::mirror))
			.call_blocking::<Request, Response>(request)
			.unwrap()
			.status()
			.xpect_eq(StatusCode::Ok);
	}

	#[test]
	fn result_output() {
		AsyncPlugin::world()
			.spawn(tool(|_: u32| -> Result { Ok(()) }))
			.call_blocking::<u32, ()>(42)
			.unwrap();
	}

	#[test]
	fn deref_tool_context() {
		AsyncPlugin::world()
			.spawn(tool(|cx: ToolContext<i32>| -> i32 { *cx + 10 }))
			.call_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(15);
	}
}
