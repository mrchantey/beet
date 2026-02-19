use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::SystemState;

/// A handle for calling the wrapped inner tool handler.
///
/// Provided to async wrapper functions so they can invoke the inner
/// handler at the point of their choosing, enabling middleware
/// patterns like input transformation, output transformation,
/// or short-circuiting.
pub struct Next<In: 'static, Out: 'static> {
	handler: Tool<In, Out>,
	tool: Entity,
	world: AsyncWorld,
}

impl<In, Out> Next<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// Call the inner handler asynchronously.
	///
	/// Schedules the inner handler via [`AsyncWorld`] and awaits
	/// the result through a channel.
	pub async fn call(&self, input: In) -> Result<Out> {
		let tool = self.tool;
		self.world
			.entity(tool)
			.call_tool(self.handler.clone(), input)
			.await
	}
}

/// Marker for the [`IntoTool`] impl that captures async wrapper
/// closures of the form `Fn(WrapIn, Next<InnerIn, InnerOut>) -> Future<Output = Result<WrapOut>>`.
///
/// The [`Result`] is propagated as a tool error, and the output type
/// is the inner `Ok` type, matching the `#[tool]` macro convention.
pub struct WrapToolMarker;

impl<WrapFn, WrapIn, WrapOut, Fut, InnerIn, InnerOut>
	IntoTool<(WrapToolMarker, WrapIn, WrapOut, InnerIn, InnerOut)> for WrapFn
where
	WrapFn: 'static
		+ Send
		+ Sync
		+ Clone
		+ Fn(WrapIn, Next<InnerIn, InnerOut>) -> Fut,
	Fut: 'static + Send + Future<Output = Result<WrapOut>>,
	WrapIn: 'static + Send + Sync,
	WrapOut: 'static + Send + Sync,
	InnerIn: 'static + Send + Sync,
	InnerOut: 'static + Send + Sync,
{
	type In = (WrapIn, Next<InnerIn, InnerOut>);
	type Out = WrapOut;

	fn into_tool(self) -> Tool<Self::In, Self::Out> {
		Tool::new(
			TypeMeta::of::<WrapFn>(),
			move |ToolCall {
			          mut commands,
			          tool: _,
			          input: (wrap_in, next),
			          out_handler,
			      }| {
				let func = self.clone();
				commands.run(async move |world: AsyncWorld| -> Result {
					let output = func(wrap_in, next).await?;
					out_handler.call_async(world, output).await
				});
				Ok(())
			},
		)
	}
}

/// Marker for the [`IntoTool`] impl that captures async wrapper
/// closures of the form `Fn(WrapIn, Next<InnerIn, InnerOut>) -> Future<Output = WrapOut>>`
/// where the output is NOT a [`Result`].
///
/// The output is wrapped in `Ok` automatically.
/// Disambiguated from [`WrapToolMarker`] by requiring `WrapOut: Typed`.
pub struct TypedWrapToolMarker;

impl<WrapFn, WrapIn, WrapOut, Fut, InnerIn, InnerOut>
	IntoTool<(TypedWrapToolMarker, WrapIn, WrapOut, InnerIn, InnerOut)> for WrapFn
where
	WrapFn: 'static
		+ Send
		+ Sync
		+ Clone
		+ Fn(WrapIn, Next<InnerIn, InnerOut>) -> Fut,
	Fut: 'static + Send + Future<Output = WrapOut>,
	WrapIn: 'static + Send + Sync,
	WrapOut: 'static + Send + Sync + bevy::reflect::Typed,
	InnerIn: 'static + Send + Sync,
	InnerOut: 'static + Send + Sync,
{
	type In = (WrapIn, Next<InnerIn, InnerOut>);
	type Out = WrapOut;

	fn into_tool(self) -> Tool<Self::In, Self::Out> {
		Tool::new(
			TypeMeta::of::<WrapFn>(),
			move |ToolCall {
			          mut commands,
			          tool: _,
			          input: (wrap_in, next),
			          out_handler,
			      }| {
				let func = self.clone();
				commands.run(async move |world: AsyncWorld| -> Result {
					let output = func(wrap_in, next).await;
					out_handler.call_async(world, output).await
				});
				Ok(())
			},
		)
	}
}

/// Allows wrapping a tool handler with middleware-style logic.
///
/// The wrapper function receives the outer input and a [`Next`]
/// handle, returning the outer output. The inner handler is
/// called via [`Next::call`] at the wrapper's discretion.
///
/// This is blanket-implemented for any [`IntoTool`] whose
/// input type is `(WrapIn, Next<InnerIn, InnerOut>)`.
pub trait IntoWrapTool<M, WrapIn, WrapOut, InnerIn, InnerOut>: Sized {
	/// Wrap an inner handler, producing a combined [`Tool`].
	///
	/// The resulting handler accepts `WrapIn` and produces `WrapOut`,
	/// with the wrapper controlling when and how the inner handler
	/// (accepting `InnerIn`/`InnerOut`) is invoked via [`Next`].
	fn wrap<Inner, InnerM>(self, inner: Inner) -> Tool<WrapIn, WrapOut>
	where
		Inner: 'static + IntoTool<InnerM, In = InnerIn, Out = InnerOut>,
		InnerIn: 'static + Send + Sync,
		InnerOut: 'static + Send + Sync;
}

/// Blanket impl: any [`IntoTool`] with `In = (WrapIn, Next<InnerIn, InnerOut>)`
/// automatically becomes wrappable.
// here OuterIn/OuterOut are the types for the actual tool
impl<T, M, OuterIn, OuterOut, InnerIn, InnerOut>
	IntoWrapTool<M, OuterIn, OuterOut, InnerIn, InnerOut> for T
where
	T: 'static
		+ IntoTool<M, In = (OuterIn, Next<InnerIn, InnerOut>), Out = OuterOut>,
	OuterIn: 'static + Send + Sync,
	OuterOut: 'static + Send + Sync,
	InnerIn: 'static + Send + Sync,
	InnerOut: 'static + Send + Sync,
{
	fn wrap<Inner, InnerM>(self, inner: Inner) -> Tool<OuterIn, OuterOut>
	where
		Inner: 'static + IntoTool<InnerM, In = InnerIn, Out = InnerOut>,
	{
		let inner_handler = inner.into_tool();
		let outer_handler = self.into_tool();

		Tool::new(
			TypeMeta::of::<(T, Inner)>(),
			move |ToolCall {
			          commands,
			          tool,
			          input,
			          out_handler,
			      }| {
				let next = Next {
					handler: inner_handler.clone(),
					tool,
					world: commands.world(),
				};

				outer_handler.call(ToolCall {
					commands,
					tool,
					input: (input, next),
					out_handler,
				})
			},
		)
	}
}

#[cfg(test)]
mod test {
	use std::str::FromStr;

	use crate::prelude::*;
	use beet_core::prelude::*;

	#[tool]
	fn add(a: i32, b: i32) -> i32 { a + b }
	#[tool]
	fn double(val: i32) -> i32 { val * 2 }
	#[tool]
	fn negate(val: i32) -> i32 { -val }

	async fn serde<In, Out>(
		input: String,
		next: Next<In, Out>,
	) -> Result<String>
	where
		In: 'static + Send + Sync + FromStr,
		Out: 'static + Send + Sync + ToString,
		In::Err: std::fmt::Debug,
	{
		let parsed: In = input.parse().map_err(|err| bevyhow!("{err:?}"))?;
		let output = next.call(parsed).await?;
		Ok(format!("output: {}", output.to_string()))
	}

	#[test]
	fn transforms_input_and_output() {
		AsyncPlugin::world()
			.spawn(serde.wrap(double))
			.call_blocking::<String, String>("21".into())
			.unwrap()
			.xpect_eq("output: 42".to_string());
	}

	#[test]
	fn passthrough() {
		AsyncPlugin::world()
			.spawn(
				(async |input: i32, next: Next<i32, i32>| -> Result<i32> {
					next.call(input).await
				})
				.wrap(negate),
			)
			.call_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(-5);
	}

	#[test]
	fn short_circuit() {
		AsyncPlugin::world()
			.spawn(
				(async |input: i32, _next: Next<i32, i32>| -> i32 {
					// never calls inner
					input * 100
				})
				.wrap(negate),
			)
			.call_blocking::<i32, i32>(3)
			.unwrap()
			.xpect_eq(300);
	}

	#[test]
	fn with_tuple_inner() {
		AsyncPlugin::world()
			.spawn(
				(async |input: (i32, i32),
				        next: Next<(i32, i32), i32>|
				       -> Result<i32> {
					let inner_out = next.call(input).await?;
					Ok(inner_out + 1)
				})
				.wrap(add),
			)
			.call_blocking::<(i32, i32), i32>((3, 4))
			.unwrap()
			.xpect_eq(8);
	}

	#[test]
	fn called_multiple_times() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(
				(async |input: i32, next: Next<i32, i32>| -> Result<i32> {
					next.call(input).await
				})
				.wrap(double),
			)
			.id();

		world
			.entity_mut(entity)
			.call_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(10);

		world
			.entity_mut(entity)
			.call_blocking::<i32, i32>(7)
			.unwrap()
			.xpect_eq(14);
	}

	#[test]
	fn modifies_inner_input_and_output() {
		AsyncPlugin::world()
			.spawn(
				(async |input: i32, next: Next<i32, i32>| -> Result<i32> {
					let inner_out = next.call(input * 10).await?;
					Ok(inner_out + 1)
				})
				.wrap(negate),
			)
			.call_blocking::<i32, i32>(3)
			.unwrap()
			.xpect_eq(-29);
	}
}
