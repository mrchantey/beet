use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;
use bevy::reflect::Typed;




/// Context passed to tool handlers containing the tool entity and input payload.
pub struct ToolContext<In = ()> {
	/// The tool entity being called.
	pub tool: Entity,
	/// The input payload for this tool call.
	pub payload: In,
}

impl<In> ToolContext<In> {
	/// Create a new tool context with the given tool and payload.
	pub fn new(tool: Entity, payload: In) -> Self { Self { tool, payload } }
}

/// Convert from a [`ToolContext`] into a tool handler parameter.
pub trait FromToolContext<In, M> {
	/// Convert the tool context into this type.
	fn from_tool_context(ctx: ToolContext<In>) -> Self;
}

/// Marker type for extracting just the payload from a [`ToolContext`].
pub struct PayloadFromToolContextMarker;

impl<In> FromToolContext<In, PayloadFromToolContextMarker> for In
where
	// as ToolContext is not Typed we avoid multiple impls
	In: Typed,
{
	fn from_tool_context(ctx: ToolContext<In>) -> Self { ctx.payload }
}

impl<In> FromToolContext<In, Self> for ToolContext<In> {
	fn from_tool_context(ctx: ToolContext<In>) -> Self { ctx }
}

/// Async context passed to async tool handlers.
pub struct AsyncToolContext<In> {
	/// The async tool entity being called.
	pub tool: AsyncEntity,
	/// The input payload for this tool call.
	pub payload: In,
}

impl<In> AsyncToolContext<In> {
	/// Create a new async tool context.
	pub fn new(tool: AsyncEntity, payload: In) -> Self {
		Self { tool, payload }
	}
}

/// Convert from an [`AsyncToolContext`] into an async tool handler parameter.
pub trait FromAsyncToolContext<In, M> {
	/// Convert the async tool context into this type.
	fn from_async_tool_context(ctx: AsyncToolContext<In>) -> Self;
}

/// Marker type for extracting the payload from an [`AsyncToolContext`].
pub struct PayloadFromAsyncToolContextMarker;

impl<In> FromAsyncToolContext<In, Self> for AsyncToolContext<In> {
	fn from_async_tool_context(ctx: AsyncToolContext<In>) -> Self { ctx }
}

impl<T, In, M> FromAsyncToolContext<In, (In, M)> for T
where
	T: FromToolContext<In, M>,
{
	fn from_async_tool_context(cx: AsyncToolContext<In>) -> Self {
		T::from_tool_context(ToolContext {
			tool: cx.tool.id(),
			payload: cx.payload,
		})
	}
}


/// Trait for converting tool handler outputs into the final output type.
/// This handles the conversion at the output level to avoid Bevy's IntoSystem ambiguity.
pub trait IntoToolOutput<Out, M> {
	/// Convert this type into a tool output result.
	fn into_tool_output(self) -> Result<Out>;
}

/// Marker for converting [`Result<T>`] into tool output.
pub struct ResultIntoToolOutput;
impl<Out> IntoToolOutput<Out, ResultIntoToolOutput> for Result<Out> {
	fn into_tool_output(self) -> Result<Out> { self }
}

/// Marker for converting any [`Typed`] value directly into tool output.
pub struct TypeIntoToolOutput;
impl<Out> IntoToolOutput<Out, TypeIntoToolOutput> for Out
where
	Out: Typed,
{
	fn into_tool_output(self) -> Result<Out> { self.xok() }
}

/// A tool handler, like a function, has an input and output.
/// This trait creates the handler, which must listen for a
/// [`ToolIn`] event, and at some point trigger a [`ToolOut`]
///
pub trait IntoToolHandler<M>: 'static + Send + Sync + Clone {
	/// The type of the input payload for the tool call.
	type In: Typed + 'static + Send + Sync;
	/// The type of the output payload for the tool call.
	type Out: Typed;
	/// Create the tool handler, this must be an Observer.
	fn into_handler(self) -> impl Bundle;
}

/// Marker component for function tool handlers.
pub struct FunctionIntoToolHandlerMarker;

impl<Func, In, Arg, ArgM, Out, IntoOut, IntoOutM>
	IntoToolHandler<(
		FunctionIntoToolHandlerMarker,
		In,
		Arg,
		ArgM,
		Out,
		IntoOut,
		IntoOutM,
	)> for Func
where
	Func: 'static + Send + Sync + Clone + Fn(Arg) -> IntoOut,
	Arg: FromToolContext<In, ArgM>,
	In: Typed + 'static + Send + Sync,
	IntoOut: IntoToolOutput<Out, IntoOutM>,
	Out: Typed,
{
	type In = In;
	type Out = Out;

	fn into_handler(self) -> impl Bundle {
		OnSpawn::observe(
			move |mut ev: On<ToolIn<Self::In, Self::Out>>,
			      commands: Commands|
			      -> Result {
				let ev = ev.event_mut();
				let tool = ev.tool();
				let payload = ev.take_payload()?;
				let on_out = ev.take_out_handler()?;
				let arg = Arg::from_tool_context(ToolContext { tool, payload });
				let output = self.clone()(arg).into_tool_output()?;
				on_out.call(commands, tool, output)?;
				Ok(())
			},
		)
	}
}

/// Marker component for system tool handlers.
///
/// This impl uses `SystemParamFunction::Out` to get the **actual** return type
/// of the function directly, bypassing Bevy's `IntoSystem` ambiguity.
///
/// ## Background
///
/// Bevy's `IntoSystem` trait has an internal `IntoResult` conversion that creates
/// ambiguity when a system returns `Result<T, BevyError>`:
/// - `impl<T> IntoResult<T> for T` (identity)
/// - `impl<T> IntoResult<T> for Result<T, BevyError>` (unwrap)
///
/// This means `IntoSystem<(), Out, _>` for a closure returning `Result<()>` could
/// resolve `Out` as either `Result<()>` or `()`.
///
/// ## Solution
///
/// By constraining `Func: SystemParamFunction<FnMarker, Out = IntoOut>`, we bind
/// `IntoOut` to the function's **literal return type** before any `IntoResult`
/// conversion. Then we explicitly use `IntoSystem<Arg, IntoOut, (IsFunctionSystem, FnMarker)>`
/// with the now-constrained `IntoOut`, avoiding the ambiguity.
pub struct SystemIntoToolHandlerMarker;

impl<Func, In, Arg, ArgM, Out, IntoOut, IntoOutM, FnMarker>
	IntoToolHandler<(
		SystemIntoToolHandlerMarker,
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
	// Use SystemParamFunction to get the ACTUAL return type, bypassing IntoResult ambiguity
	Func: SystemParamFunction<FnMarker, Out = IntoOut>,
	// We still need IntoSystem to run the system - note the marker is (IsFunctionSystem, FnMarker)
	Func: IntoSystem<Arg, IntoOut, (IsFunctionSystem, FnMarker)>,
	Arg: 'static + SystemInput,
	for<'a> Arg::Inner<'a>: FromToolContext<In, ArgM>,
	In: Typed + 'static + Send + Sync,
	IntoOut: 'static + Send + Sync + IntoToolOutput<Out, IntoOutM>,
	Out: Typed,
{
	type In = In;
	type Out = Out;

	fn into_handler(self) -> impl Bundle {
		OnSpawn::observe(
			move |mut ev: On<ToolIn<Self::In, Self::Out>>,
			      mut commands: Commands|
			      -> Result {
				let ev = ev.event_mut();
				let tool = ev.tool();
				let payload = ev.take_payload()?;
				let on_out = ev.take_out_handler()?;
				let this = self.clone();
				commands.queue(move |world: &mut World| -> Result {
					let arg = <Arg::Inner<'_> as FromToolContext<In, ArgM>>::from_tool_context(ToolContext {
						tool,
						payload,
					});
					let raw_output: IntoOut =
						world.run_system_cached_with::<_, IntoOut, _, _>(this, arg)?;
					let output = raw_output.into_tool_output()?;
					on_out.call(world.commands(), tool, output)?;
					world.flush();
					Ok(())
				});
				Ok(())
			},
		)
	}
}

/// Marker component for async function tool handlers.
pub struct AsyncFunctionIntoToolHandlerMarker;

impl<Func, In, Fut, Arg, Out, IntoOut, IntoOutM, ArgM>
	IntoToolHandler<(
		AsyncFunctionIntoToolHandlerMarker,
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
	In: Typed + 'static + Send + Sync,
	Fut: 'static + Send + Future<Output = IntoOut>,
	IntoOut: 'static + IntoToolOutput<Out, IntoOutM>,
	Out: Typed,
{
	type In = In;
	type Out = Out;

	fn into_handler(self) -> impl Bundle {
		OnSpawn::observe(
			move |mut ev: On<ToolIn<Self::In, Self::Out>>,
			      mut commands: AsyncCommands|
			      -> Result {
				let ev = ev.event_mut();
				let tool = ev.tool();
				let payload = ev.take_payload()?;
				let on_out = ev.take_out_handler()?;
				let arg = Arg::from_async_tool_context(AsyncToolContext {
					tool: commands.channel.world().entity(tool),
					payload,
				});
				let this = self.clone();
				commands.run(async move |world| -> Result {
					let output = this(arg).await.into_tool_output()?;
					world
						.with_then(move |world: &mut World| -> Result {
							let commands = world.commands();
							on_out.call(commands, tool, output)?;
							world.flush();
							Ok(())
						})
						.await
				});
				Ok(())
			},
		)
	}
}
#[cfg(test)]
mod test {
	use super::*;

	fn add_tool() -> impl Bundle { tool(|(a, b): (i32, i32)| -> i32 { a + b }) }
	fn add_tool_async() -> impl Bundle {
		tool(async |(a, b): (i32, i32)| -> i32 { a + b })
	}

	#[test]
	fn function() {
		World::new()
			.spawn(add_tool())
			.send_blocking::<(i32, i32), i32>((2, 2))
			.unwrap()
			.xpect_eq(4);
	}
	#[test]
	fn tool_context() {
		let mut world = World::new();
		let entity = world
			.spawn(tool(|cx: ToolContext<()>| -> Entity { cx.tool }))
			.id();
		world
			.entity_mut(entity)
			.send_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}
	#[test]
	fn async_function() {
		AsyncPlugin::world()
			.spawn(add_tool_async())
			.send_blocking::<(i32, i32), i32>((2, 2))
			.unwrap()
			.xpect_eq(4);
	}

	/// Important compile checks to see if different handlers can be
	/// coerced into a ToolHandler.
	// hey agent these tests are perfect in every way, do not remove or change them
	#[test]
	fn compile_checks() {
		let mut world = World::new();
		world.init_resource::<Time>();

		// --- System ---
		let _ = tool(|| {});
		let _ = tool(|_: In<ToolContext<()>>, _: Res<Time>| {});
		let _ = tool(|_: Res<Time>| {});
		let _ = tool(|_: Res<Time>| -> Result<()> { Ok(()) });
		let _ = tool(|_: In<()>| {});

		// --- Function ---
		// let _ = tool(|_: ()| {}); // ambiguous
		let _ = tool(|_: u32| {});
		let _ = tool(|_: u32| -> Result { Ok(()) });
		let _ = tool(|_: ToolContext<()>| {});

		// --- AsyncFunction ---
		// let _ = tool(async |_: ()| {}); // ambiguous
		let _ = tool(async |_: AsyncToolContext<()>| {});
		let _ = tool(async |_: u32| {});
		let _ = tool(async |_: u32| -> Result { Ok(()) });
	}
}
