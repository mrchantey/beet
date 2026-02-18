//! Composable tool chaining via the pipe combinator.
//!
//! The [`IntoPipeTool`] trait lets you chain two [`IntoToolHandler`]
//! implementations together so the output of the first feeds into the
//! input of the second, producing a single [`ToolHandler`].
//!
//! ## Examples
//!
//! ```rust
//! # use beet_stack::prelude::*;
//! # use beet_core::prelude::*;
//! fn add((a, b): (i32, i32)) -> i32 { a + b }
//! fn negate(val: i32) -> i32 { -val }
//!
//! let handler = tool(add.pipe(negate));
//!
//! AsyncPlugin::world()
//!     .spawn(handler)
//!     .call_blocking::<(i32, i32), i32>((3, 4))
//!     .unwrap()
//!     .xpect_eq(-7);
//! ```
use crate::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

/// Allows chaining two [`ToolHandler`] implementations, feeding the
/// output of the first into the input of the second.
///
/// Both handlers are fused into a single [`ToolHandler`] whose input
/// type matches the first handler and whose output type matches the
/// second. The intermediate value is converted via [`From`].
pub trait IntoPipeTool<In, Out, M>
where
	Self: Sized + IntoToolHandler<M, In = In, Out = Out>,
	Out: 'static,
{
	/// Pipe `self` to `other`, producing a combined handler.
	fn pipe<T2, M2>(self, other: T2) -> ToolHandler<In, T2::Out>
	where
		T2: IntoToolHandler<M2>,
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
	T: IntoToolHandler<M, In = In, Out = Out>,
	Out: 'static,
{
}




#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn add((a, b): (i32, i32)) -> i32 { a + b }
	fn negate(val: i32) -> i32 { -val }
	fn multiply(val: i32) -> i32 { val * val }
	fn to_string(val: i32) -> String { val.to_string() }

	#[test]
	fn pipe_two() {
		AsyncPlugin::world()
			.spawn(tool(add.pipe(negate)))
			.call_blocking::<(i32, i32), i32>((5, 2))
			.unwrap()
			.xpect_eq(-7);
	}
	#[test]
	fn pipe_three() {
		AsyncPlugin::world()
			.spawn(tool(add.pipe(multiply).pipe(negate)))
			.call_blocking::<(i32, i32), i32>((5, 3))
			.unwrap()
			.xpect_eq(-64);
	}

	#[test]
	fn pipe_type_conversion() {
		AsyncPlugin::world()
			.spawn(tool(add.pipe(to_string)))
			.call_blocking::<(i32, i32), String>((3, 4))
			.unwrap()
			.xpect_eq("7".to_string());
	}

	#[test]
	fn pipe_with_closure() {
		AsyncPlugin::world()
			.spawn(tool(add.pipe(|val: i32| val * 2)))
			.call_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(14);
	}

	#[test]
	fn pipe_called_multiple_times() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(tool(add.pipe(negate))).id();
		world
			.entity_mut(entity)
			.call_blocking::<(i32, i32), i32>((1, 2))
			.unwrap()
			.xpect_eq(-3);
		world
			.entity_mut(entity)
			.call_blocking::<(i32, i32), i32>((10, 20))
			.unwrap()
			.xpect_eq(-30);
	}

	#[test]
	fn pipe_identity() {
		AsyncPlugin::world()
			.spawn(tool(add.pipe(|val: i32| val)))
			.call_blocking::<(i32, i32), i32>((5, 5))
			.unwrap()
			.xpect_eq(10);
	}
}
