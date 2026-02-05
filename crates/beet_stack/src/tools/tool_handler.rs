use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::Typed;


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


impl<Func, In, Out> IntoToolHandler<(FunctionIntoToolHandlerMarker, In, Out)>
	for Func
where
	Func: 'static + Send + Sync + Clone + Fn(In) -> Out,
	In: Typed + 'static + Send + Sync,
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
				let output = self.clone()(payload);
				on_out.call(commands, tool, output)?;
				Ok(())
			},
		)
	}
}


/// Marker component for function tool handlers.
pub struct AsyncFunctionIntoToolHandlerMarker;


impl<Func, In, Fut, Out>
	IntoToolHandler<(AsyncFunctionIntoToolHandlerMarker, In, Out)> for Func
where
	Func: 'static + Send + Sync + Clone + Fn(In) -> Fut,
	In: Typed + 'static + Send + Sync,
	Fut: 'static + Send + Future<Output = Out>,
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
				let this = self.clone();
				commands.run(async move |world| -> Result {
					let output = this(payload).await;
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

		AsyncPlugin::world()
			.spawn(add_tool_async())
			.send_blocking::<(i32, i32), i32>((2, 2))
			.unwrap()
			.xpect_eq(4);
	}
}
