//! Bevy system tool handler.
//!
//! [`SystemTool`] wraps a Bevy system function, running it via
//! `run_system_cached_with` with exclusive world access. This allows
//! tool handlers to use standard Bevy system parameters like
//! [`Query`], [`Res`], [`Commands`], etc.

use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;

/// A tool handler backed by a Bevy system.
///
/// The wrapped system is executed via `run_system_cached_with`,
/// giving it access to standard system parameters. Input is
/// converted from the [`ToolContext`] via [`FromToolContext`],
/// and output is converted via [`IntoToolOutput`].
#[derive(Any)]
pub struct SystemTool<In: 'static, Out: 'static> {
	runner: Box<
		dyn 'static
			+ Send
			+ Sync
			+ FnMut(&mut World, ToolContext<In>) -> Result<Out>,
	>,
}

impl<In: 'static, Out: 'static> SystemTool<In, Out> {
	/// Create a new [`SystemTool`] from a runner closure.
	///
	/// Prefer the [`IntoToolHandler2`] blanket impl over calling this directly.
	pub fn new<F>(runner: F) -> Self
	where
		F: 'static
			+ Send
			+ Sync
			+ FnMut(&mut World, ToolContext<In>) -> Result<Out>,
	{
		Self {
			runner: Box::new(runner),
		}
	}
}

impl<In: 'static, Out: 'static> ToolHandler for SystemTool<In, Out> {
	type In = In;
	type Out = Out;

	fn call(
		&mut self,
		world: &mut World,
		ToolCall {
			tool,
			input,
			out_handler,
		}: ToolCall<Self::In, Self::Out>,
	) -> Result {
		let cx = ToolContext::new(tool, input);
		let output = (self.runner)(world, cx)?;
		out_handler.call(output)
	}
}

/// Blanket [`IntoToolHandler2`] impl for Bevy system functions.
///
/// Uses `SystemParamFunction::Out` to bind the **actual** return type
/// of the function, bypassing Bevy's `IntoResult` ambiguity.
/// See the docs on `SystemIntoToolHandlerMarker` in `tool_handler.rs`
/// for a detailed explanation.
impl<Func, In, Arg, ArgM, Out, IntoOut, IntoOutM, FnMarker>
	IntoToolHandler2<(
		SystemTool<In, Out>,
		In,
		Arg,
		ArgM,
		Out,
		IntoOut,
		IntoOutM,
		FnMarker,
	)> for Func
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = IntoOut>,
	Func: IntoSystem<Arg, IntoOut, (IsFunctionSystem, FnMarker)>,
	Arg: 'static + SystemInput,
	for<'a> Arg::Inner<'a>: FromToolContext<In, ArgM>,
	In: 'static + Send + Sync,
	IntoOut: 'static + Send + Sync + IntoToolOutput<Out, IntoOutM>,
	Out: 'static + Send + Sync,
{
	type In = In;
	type Out = Out;

	fn into_tool_handler(
		self,
	) -> impl ToolHandler<In = Self::In, Out = Self::Out> {
		let func = self;
		SystemTool::new(move |world: &mut World, cx: ToolContext<In>| {
			let arg =
				<Arg::Inner<'_> as FromToolContext<In, ArgM>>::from_tool_context(
					cx,
				);
			let raw_output: IntoOut = world
				.run_system_cached_with::<_, IntoOut, _, _>(
					func.clone(),
					arg,
				)?;
			raw_output.into_tool_output()
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[derive(Resource, Default)]
	struct Counter(i32);

	#[test]
	fn reads_resource() {
		let mut world = World::new();
		world.insert_resource(Counter(42));
		let handler =
			(|counter: Res<Counter>| -> i32 { counter.0 }).into_tool_handler();
		world
			.spawn(Tool::new(handler))
			.call2_blocking::<(), i32>(())
			.unwrap()
			.xpect_eq(42);
	}

	#[test]
	fn with_input() {
		let mut world = World::new();
		world.insert_resource(Counter(10));
		let handler = (|input: In<i32>, counter: Res<Counter>| -> i32 {
			*input + counter.0
		})
		.into_tool_handler();
		world
			.spawn(Tool::new(handler))
			.call2_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(15);
	}

	#[test]
	fn mutates_resource() {
		let mut world = World::new();
		world.insert_resource(Counter(0));
		let handler = (|input: In<i32>, mut counter: ResMut<Counter>| {
			counter.0 += *input;
		})
		.into_tool_handler();
		let entity = world.spawn(Tool::new(handler)).id();

		world
			.entity_mut(entity)
			.call2_blocking::<i32, ()>(7)
			.unwrap();
		world.resource::<Counter>().0.xpect_eq(7);

		world
			.entity_mut(entity)
			.call2_blocking::<i32, ()>(3)
			.unwrap();
		world.resource::<Counter>().0.xpect_eq(10);
	}

	#[test]
	fn returns_result() {
		let mut world = World::new();
		world.insert_resource(Counter(0));
		let handler =
			(|counter: Res<Counter>| -> Result<i32> { counter.0.xok() })
				.into_tool_handler();
		world
			.spawn(Tool::new(handler))
			.call2_blocking::<(), i32>(())
			.unwrap()
			.xpect_eq(0);
	}

	#[test]
	fn tool_context_input() {
		let mut world = World::new();
		world.insert_resource(Counter(100));
		let handler =
			(|input: In<ToolContext<()>>, counter: Res<Counter>| -> i32 {
				let _ = input.tool;
				counter.0
			})
			.into_tool_handler();
		world
			.spawn(Tool::new(handler))
			.call2_blocking::<(), i32>(())
			.unwrap()
			.xpect_eq(100);
	}
}
