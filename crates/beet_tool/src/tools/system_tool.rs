use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::IsFunctionSystem;

impl<In, Out> Tool<In, Out>
where
	In: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	/// Create a [`Tool`] from a Bevy system returning a value convertible
	/// to `Result<Out>` via [`IntoResult`].
	///
	/// The system's first argument must be `In<ToolContext<Input>>` (the
	/// tool's input payload plus entity context), followed by any number
	/// of regular system parameters.
	///
	/// Accepts systems returning either `Out` or `Result<Out>`.
	pub fn new_system<Func, FnMarker, RawOut>(func: Func) -> Self
	where
		Func: 'static + Send + Sync + Clone,
		FnMarker: 'static,
		Func: SystemParamFunction<FnMarker, Out = RawOut>,
		Func: IntoSystem<
				bevy::ecs::system::In<ToolContext<In>>,
				RawOut,
				(IsFunctionSystem, FnMarker),
			>,
		RawOut: 'static + Send + Sync + IntoResult<Out>,
	{
		Tool::new(
			TypeMeta::of::<Func>(),
			move |ToolCall {
			          mut commands,
			          caller,
			          input,
			          out_handler,
			      }| {
				let func = func.clone();
				let async_entity = commands.world().entity(caller);
				let sys_input = ToolContext {
					caller: async_entity,
					input,
				};
				commands.commands.queue(move |world: &mut World| -> Result {
					let raw: RawOut =
						world.run_system_cached_with(func, sys_input)?;
					let result: Result<Out> = raw.into_result();
					out_handler.call_world(world, result)
				});
				Ok(())
			},
		)
	}
}

/// Marker for the system tool [`IntoTool`] impl.
pub struct SystemToolMarker;

impl<Func, Input, Out, FnMarker>
	IntoTool<(SystemToolMarker, Input, Out, FnMarker)> for Func
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = Result<Out>>,
	Func: IntoSystem<
			In<ToolContext<Input>>,
			Result<Out>,
			(IsFunctionSystem, FnMarker),
		>,
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool(self) -> Tool<Self::In, Self::Out> { Tool::new_system(self) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(Tool::<(), f32>::new_system(
				|In(input): In<ToolContext>, time: Res<Time>| -> Result<f32> {
					let _ = input.caller;
					Ok(time.elapsed_secs())
				},
			))
			.id();
		world
			.entity_mut(entity)
			.call::<(), f32>(())
			.await
			.unwrap()
			.xpect_eq(0.0);
	}

	#[beet_core::test]
	async fn with_input() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(Tool::<i32, i32>::new_system(
				|In(input): In<ToolContext<i32>>,
				 _time: Res<Time>|
				 -> Result<i32> { Ok(*input * 2) },
			))
			.id();
		world
			.entity_mut(entity)
			.call::<i32, i32>(21)
			.await
			.unwrap()
			.xpect_eq(42);
	}

	#[beet_core::test]
	async fn unit_in_unit_out() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(Tool::<(), ()>::new_system(|_: In<ToolContext>| -> Result {
				Ok(())
			}))
			.id();
		world.entity_mut(entity).call::<(), ()>(()).await.unwrap();
	}

	#[beet_core::test]
	async fn access_tool_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(Tool::<(), Entity>::new_system(
				|In(input): In<ToolContext>| -> Result<Entity> {
					Ok(input.caller.id())
				},
			))
			.id();
		world
			.entity_mut(entity)
			.call::<(), Entity>(())
			.await
			.unwrap()
			.xpect_eq(entity);
	}

	#[beet_core::test]
	async fn pipe_with_system() {
		#[tool(pure)]
		fn negate(val: i32) -> i32 { -val }

		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(
				Tool::<i32, i32>::new_system(
					|In(input): In<ToolContext<i32>>,
					 _time: Res<Time>|
					 -> Result<i32> { Ok(*input * 2) },
				)
				.chain(negate),
			)
			.id();
		world
			.entity_mut(entity)
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(-10);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — system tools
	// -----------------------------------------------------------------------

	#[tool]
	fn sys_double(val: In<i32>) -> i32 { val.0 * 2 }

	#[beet_core::test]
	async fn tool_macro_system_basic() {
		AsyncPlugin::world()
			.spawn(sys_double.into_tool())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(10);
	}

	#[tool]
	fn sys_with_resource(val: In<i32>, time: Res<Time>) -> f32 {
		val.0 as f32 + time.elapsed_secs()
	}

	#[beet_core::test]
	async fn tool_macro_system_with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world.spawn(sys_with_resource.into_tool()).id();
		world
			.entity_mut(entity)
			.call::<i32, f32>(10)
			.await
			.unwrap()
			.xpect_eq(10.0);
	}

	#[tool]
	fn sys_unit(_val: In<()>) {}

	#[beet_core::test]
	async fn tool_macro_system_unit() {
		AsyncPlugin::world()
			.spawn(sys_unit.into_tool())
			.call::<(), ()>(())
			.await
			.unwrap();
	}

	/// Verify that the macro re-wraps the input in `In()` so the
	/// user's type annotation is not a lie.
	#[tool]
	fn sys_in_rewrap(val: In<i32>) -> i32 {
		let val: In<i32> = val;
		val.0 * 2
	}

	#[beet_core::test]
	async fn tool_macro_system_in_rewrap() {
		AsyncPlugin::world()
			.spawn(sys_in_rewrap.into_tool())
			.call::<i32, i32>(5)
			.await
			.unwrap()
			.xpect_eq(10);
	}

	#[tool]
	fn sys_fallible(val: In<i32>) -> Result<i32> {
		if val.0 == 0 {
			bevybail!("zero not allowed");
		}
		Ok(val.0 * 3)
	}

	#[beet_core::test]
	async fn tool_macro_system_result_ok() {
		AsyncPlugin::world()
			.spawn(sys_fallible.into_tool())
			.call::<i32, i32>(4)
			.await
			.unwrap()
			.xpect_eq(12);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — system passthrough
	// -----------------------------------------------------------------------

	#[tool]
	fn sys_passthrough(cx: In<ToolContext>) -> Entity { cx.id() }

	#[beet_core::test]
	async fn tool_macro_system_passthrough_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(sys_passthrough.into_tool()).id();
		world
			.entity_mut(entity)
			.call::<(), Entity>(())
			.await
			.unwrap()
			.xpect_eq(entity);
	}

	#[tool]
	fn sys_passthrough_with_res(
		cx: In<ToolContext<i32>>,
		time: Res<Time>,
	) -> f32 {
		*cx as f32 + time.elapsed_secs()
	}

	#[beet_core::test]
	async fn tool_macro_system_passthrough_with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world.spawn(sys_passthrough_with_res.into_tool()).id();
		world
			.entity_mut(entity)
			.call::<i32, f32>(7)
			.await
			.unwrap()
			.xpect_eq(7.0);
	}
}
