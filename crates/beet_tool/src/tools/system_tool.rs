use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::IsFunctionSystem;
use bevy::ecs::system::SystemParamFunction;

/// Context passed to system tool handlers containing the tool entity
/// and input payload.
///
/// This mirrors [`FuncToolIn`] and [`AsyncToolIn`], giving
/// system-based tools access to the entity that owns the
/// [`Tool`] component.
pub struct SystemToolIn<In = ()> {
	/// The entity that owns the [`Tool`] being called.
	pub tool: Entity,
	/// The input payload for this tool call.
	pub input: In,
}

impl<In> std::ops::Deref for SystemToolIn<In> {
	type Target = In;
	fn deref(&self) -> &Self::Target { &self.input }
}

impl<In> std::ops::DerefMut for SystemToolIn<In> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.input }
}

impl<In> SystemToolIn<In> {
	/// Consume the context and return the inner input payload.
	pub fn take(self) -> In { self.input }
}

/// Create a [`Tool`] from a Bevy system that returns [`Result<Out>`].
///
/// Unlike [`func_tool`](crate::func_tool), system tools have access to
/// ECS queries, resources, and other system parameters.
///
/// The system's first argument must be `In<SystemToolIn<Input>>` (the
/// tool's input payload plus entity context), followed by any number
/// of regular system parameters.
///
/// ## Examples
///
/// ```rust
/// # use beet_tool::prelude::*;
/// # use beet_core::prelude::*;
/// let handler = system_tool(|In(input): In<SystemToolIn<()>>, time: Res<Time>| -> Result<f32> {
///     Ok(time.elapsed_secs())
/// });
/// ```
pub fn system_tool<Func, Input, Out, FnMarker>(func: Func) -> Tool<Input, Out>
where
	Func: 'static + Send + Sync + Clone,
	FnMarker: 'static,
	Func: SystemParamFunction<FnMarker, Out = Result<Out>>,
	Func: IntoSystem<
			In<SystemToolIn<Input>>,
			Result<Out>,
			(IsFunctionSystem, FnMarker),
		>,
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	Tool::new(
		TypeMeta::of::<Func>(),
		move |ToolCall {
		          mut commands,
		          tool,
		          input,
		          out_handler,
		      }| {
			let func = func.clone();
			let sys_input = SystemToolIn { tool, input };
			commands.commands.queue(move |world: &mut World| -> Result {
				let output: Result<Out> =
					world.run_system_cached_with(func, sys_input)?;
				let output = output?;
				out_handler.call_world(world, output)
			});
			Ok(())
		},
	)
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
			In<SystemToolIn<Input>>,
			Result<Out>,
			(IsFunctionSystem, FnMarker),
		>,
	Input: 'static + Send + Sync,
	Out: 'static + Send + Sync,
{
	type In = Input;
	type Out = Out;

	fn into_tool(self) -> Tool<Self::In, Self::Out> { system_tool(self) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	fn with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(system_tool(
				|In(input): In<SystemToolIn<()>>,
				 time: Res<Time>|
				 -> Result<f32> {
					let _ = input.tool;
					Ok(time.elapsed_secs())
				},
			))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), f32>(())
			.unwrap()
			.xpect_eq(0.0);
	}

	#[test]
	fn with_input() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(system_tool(
				|In(input): In<SystemToolIn<i32>>,
				 _time: Res<Time>|
				 -> Result<i32> { Ok(*input * 2) },
			))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<i32, i32>(21)
			.unwrap()
			.xpect_eq(42);
	}

	#[test]
	fn unit_in_unit_out() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(system_tool(|_: In<SystemToolIn<()>>| -> Result { Ok(()) }))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), ()>(())
			.unwrap();
	}

	#[test]
	fn access_tool_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world
			.spawn(system_tool(
				|In(input): In<SystemToolIn<()>>| -> Result<Entity> {
					Ok(input.tool)
				},
			))
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}

	#[test]
	fn pipe_with_system() {
		#[tool]
		fn negate(val: i32) -> i32 { -val }

		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world
			.spawn(
				system_tool(
					|In(input): In<SystemToolIn<i32>>,
					 _time: Res<Time>|
					 -> Result<i32> { Ok(*input * 2) },
				)
				.chain(negate),
			)
			.id();
		world
			.entity_mut(entity)
			.call_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(-10);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — system tools
	// -----------------------------------------------------------------------

	#[tool]
	fn sys_double(val: In<i32>) -> i32 { val.0 * 2 }

	#[test]
	fn tool_macro_system_basic() {
		AsyncPlugin::world()
			.spawn(sys_double.into_tool())
			.call_blocking::<i32, i32>(5)
			.unwrap()
			.xpect_eq(10);
	}

	#[tool]
	fn sys_with_resource(val: In<i32>, time: Res<Time>) -> f32 {
		val.0 as f32 + time.elapsed_secs()
	}

	#[test]
	fn tool_macro_system_with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world.spawn(sys_with_resource.into_tool()).id();
		world
			.entity_mut(entity)
			.call_blocking::<i32, f32>(10)
			.unwrap()
			.xpect_eq(10.0);
	}

	#[tool]
	fn sys_unit(_val: In<()>) {}

	#[test]
	fn tool_macro_system_unit() {
		AsyncPlugin::world()
			.spawn(sys_unit.into_tool())
			.call_blocking::<(), ()>(())
			.unwrap();
	}

	/// Verify that the macro re-wraps the input in `In()` so the
	/// user's type annotation is not a lie.
	#[tool]
	fn sys_in_rewrap(val: In<i32>) -> i32 {
		let val: In<i32> = val;
		val.0 * 2
	}

	#[test]
	fn tool_macro_system_in_rewrap() {
		AsyncPlugin::world()
			.spawn(sys_in_rewrap.into_tool())
			.call_blocking::<i32, i32>(5)
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

	#[test]
	fn tool_macro_system_result_ok() {
		AsyncPlugin::world()
			.spawn(sys_fallible.into_tool())
			.call_blocking::<i32, i32>(4)
			.unwrap()
			.xpect_eq(12);
	}

	// -----------------------------------------------------------------------
	// #[tool] macro — system passthrough
	// -----------------------------------------------------------------------

	#[tool]
	fn sys_passthrough(cx: In<SystemToolIn<()>>) -> Entity { cx.tool }

	#[test]
	fn tool_macro_system_passthrough_entity() {
		let mut world = AsyncPlugin::world();
		let entity = world.spawn(sys_passthrough.into_tool()).id();
		world
			.entity_mut(entity)
			.call_blocking::<(), Entity>(())
			.unwrap()
			.xpect_eq(entity);
	}

	#[tool]
	fn sys_passthrough_with_res(
		cx: In<SystemToolIn<i32>>,
		time: Res<Time>,
	) -> f32 {
		cx.take() as f32 + time.elapsed_secs()
	}

	#[test]
	fn tool_macro_system_passthrough_with_resource() {
		let mut world = AsyncPlugin::world();
		world.init_resource::<Time>();
		let entity = world.spawn(sys_passthrough_with_res.into_tool()).id();
		world
			.entity_mut(entity)
			.call_blocking::<i32, f32>(7)
			.unwrap()
			.xpect_eq(7.0);
	}
}
