use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;

pub struct ToolContext<In = ()> {
	pub tool: Entity,
	pub payload: In,
}

impl<In> ToolContext<In> {
	pub fn new(tool: Entity, payload: In) -> Self { Self { tool, payload } }
}

pub trait FromToolContext<In, M> {
	fn from_tool_context(ctx: ToolContext<In>) -> Self;
}

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

pub struct AsyncToolContext<In> {
	pub tool: AsyncEntity,
	pub payload: In,
}

impl<In> AsyncToolContext<In> {
	pub fn new(tool: AsyncEntity, payload: In) -> Self {
		Self { tool, payload }
	}
}

pub trait FromAsyncToolContext<In, M> {
	fn from_async_tool_context(ctx: AsyncToolContext<In>) -> Self;
}

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

/// Helper to allow both T and Result<T> as tool outputs.
trait IntoResult<Out, M> {
	fn into_result(self) -> Result<Out>;
}
impl<Out> IntoResult<Out, Self> for Out
where
	Out: Typed,
{
	fn into_result(self) -> Result<Out> { self.xok() }
}
impl<Out> IntoResult<Out, Self> for Result<Out>
where
	Out: Typed,
{
	fn into_result(self) -> Result<Out> { self }
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
	IntoOut: IntoResult<Out, IntoOutM>,
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
				let output = self.clone()(arg).into_result()?;
				on_out.call(commands, tool, output)?;
				Ok(())
			},
		)
	}
}

/// Marker component for function tool handlers.
pub struct SystemIntoToolHandlerMarker;

impl<Func, In, Arg, ArgM, Out, IntoOut, IntoOutM, SysM>
	IntoToolHandler<(
		SystemIntoToolHandlerMarker,
		In,
		Arg,
		ArgM,
		Out,
		IntoOut,
		IntoOutM,
		SysM,
	)> for Func
where
	Func: 'static + Send + Sync + Clone + IntoSystem<Arg, IntoOut, SysM>,
	Arg: 'static + SystemInput,
	for<'a> Arg::Inner<'a>: FromToolContext<In, ArgM>,
	In: Typed + 'static + Send + Sync,
	IntoOut: 'static + IntoResult<Out, IntoOutM>,
	Out: Typed + 'static + Send + Sync,
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
					let output = world.run_system_cached_with(this, arg)?.into_result()?;
					on_out.call(world.commands(), tool, output)?;
					world.flush();
					Ok(())
				});
				Ok(())
			},
		)
	}
}

/// Marker component for function tool handlers.
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
	IntoOut: 'static + IntoResult<Out, IntoOutM>,
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
					let output = this(arg).await.into_result()?;
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

		// compile checks

		// --- System ---
		let _ = tool(|| {});
		let _ = tool(|_: In<ToolContext<()>>, _: Res<Time>| {});
		let _ = tool(|_: Res<Time>| {});
		let _ = tool(|_: Res<Time>| -> Result { Ok(()) });
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
	#[test]
	fn async_function() {
		AsyncPlugin::world()
			.spawn(add_tool_async())
			.send_blocking::<(i32, i32), i32>((2, 2))
			.unwrap()
			.xpect_eq(4);
	}
}
