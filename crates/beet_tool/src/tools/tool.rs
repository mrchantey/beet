use beet_core::prelude::*;
use bevy::ecs::system::SystemState;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;
use std::sync::Arc;

#[derive(Component)]
#[component(on_add=on_add::<In, Out>)]
pub struct Tool<In: 'static, Out: 'static> {
	/// The full type name of the handler, for display and debugging.
	handler_meta: TypeMeta,
	handler: Arc<dyn 'static + Send + Sync + Fn(ToolCall<In, Out>) -> Result>,
}

impl<In: 'static, Out: 'static> std::fmt::Debug for Tool<In, Out> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Tool")
			.field("handler_meta", &self.handler_meta)
			.finish()
	}
}

impl<In: 'static, Out: 'static> Clone for Tool<In, Out> {
	fn clone(&self) -> Self {
		Self {
			handler_meta: self.handler_meta,
			handler: Arc::clone(&self.handler),
		}
	}
}

fn on_add<In: 'static, Out: 'static>(
	mut world: DeferredWorld,
	cx: HookContext,
) {
	let handler = world
		.entity(cx.entity)
		.get::<Tool<In, Out>>()
		.unwrap()
		.handler_meta
		.clone();
	let meta = ToolMeta {
		handler,
		input: TypeMeta::of::<In>(),
		output: TypeMeta::of::<Out>(),
	};

	world.commands().entity(cx.entity).insert(meta);
}

impl<In, Out> Tool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	pub fn new(
		handler_meta: TypeMeta,
		handler: impl 'static + Send + Sync + Fn(ToolCall<In, Out>) -> Result,
	) -> Self {
		Self {
			handler_meta,
			handler: Arc::new(handler),
		}
	}

	pub fn handler_meta(&self) -> TypeMeta { self.handler_meta }


	/// Invoke this tool handler with the given [`ToolCall`].
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call(&self, call: ToolCall<In, Out>) -> Result {
		(self.handler)(call)
	}

	/// Invoke this tool handler, constructing the [`ToolCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call_with(
		&self,
		entity: Entity,
		input: In,
		commands: AsyncCommands,
		out_handler: OutHandler<Out>,
	) -> Result {
		let call = ToolCall {
			commands,
			caller: entity,
			input,
			out_handler,
		};
		self.call(call)
	}

	/// Invoke this tool handler from a [`World`], constructing the [`ToolCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call_world(
		&self,
		entity: EntityWorldMut,
		input: In,
		out_handler: OutHandler<Out>,
	) -> Result {
		let id = entity.id();
		let world = entity.into_world_mut();
		let mut state = SystemState::<AsyncCommands>::new(world);
		let commands = state.get_mut(world);
		let result = self.call_with(id, input, commands, out_handler);
		state.apply(world);
		world.flush();
		result
	}

	/// Invoke this tool handler asynchronously, constructing the [`ToolCall`] internally.
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub async fn call_async(
		&self,
		entity: AsyncEntity,
		input: In,
		out_handler: OutHandler<Out>,
	) -> Result
	where
		In: 'static + Send,
		Out: 'static + Send,
	{
		let this = self.clone();
		entity
			.with_then(move |entity| {
				this.call_world(entity, input, out_handler)
			})
			.await
	}
}

#[derive(Copy, Clone, Debug, Component)]
#[component(on_add=try_add_reflect_tool_meta)]
pub struct ToolMeta {
	/// Type metadata for the tool handler.
	handler: TypeMeta,
	/// Type metadata for the tool input.
	input: TypeMeta,
	/// Type metadata for the tool output.
	output: TypeMeta,
}

impl ToolMeta {
	/// Create a [`ToolMeta`] from handler, input and output type parameters.
	pub fn of<H: 'static, In: 'static, Out: 'static>() -> Self {
		Self {
			handler: TypeMeta::of::<H>(),
			input: TypeMeta::of::<In>(),
			output: TypeMeta::of::<Out>(),
		}
	}

	/// Get the handler type metadata for this tool.
	pub fn handler(&self) -> TypeMeta { self.handler }
	/// The full type name of the handler function or type.
	pub fn name(&self) -> &'static str { self.handler.type_name }
	/// Get the input type metadata for this tool.
	pub fn input(&self) -> TypeMeta { self.input }
	/// Get the output type metadata for this tool.
	pub fn output(&self) -> TypeMeta { self.output }

	/// Assert that the provided types match this tool's input/output types.
	///
	/// # Errors
	/// Returns an error if types don't match.
	pub fn assert_match<In: 'static, Out: 'static>(&self) -> Result {
		let expected_input = self.input();
		let expected_output = self.output();
		let received_input = TypeMeta::of::<In>();
		let received_output = TypeMeta::of::<Out>();
		if expected_input != received_input {
			bevybail!(
				"Tool Call Input mismatch.\nExpected: {}\nReceived: {}.",
				expected_input,
				received_input,
			);
		} else if expected_output != received_output {
			bevybail!(
				"Tool Call Output mismatch.\nExpected: {}\nReceived: {}.",
				expected_output,
				received_output,
			);
		} else {
			Ok(())
		}
	}
}

fn try_add_reflect_tool_meta(mut world: DeferredWorld, cx: HookContext) {
	let entity = world.entity(cx.entity);
	if entity.contains::<ReflectToolMeta>() {
		// already added, ususally by `into_reflect_tool`
		return;
	}
	let Some(registry) = world.get_resource::<AppTypeRegistry>() else {
		// no registry, can't add ReflectToolMeta
		return;
	};
	let tool_meta = entity.get::<ToolMeta>().unwrap().clone();
	let registry = registry.read();

	let input = registry
		.get(tool_meta.input().type_id())
		.map(|info| info.type_info());
	let output = registry
		.get(tool_meta.output().type_id())
		.map(|info| info.type_info());

	drop(registry);

	if let (Some(input), Some(output)) = (input, output) {
		// both input and output types are registered in the AppTypeRegistry
		// so we can add ReflectToolMeta
		world.commands().entity(cx.entity).insert(ReflectToolMeta {
			tool_meta,
			input_info: input,
			output_info: output,
		});
	}
}

/// Superset of ToolMeta, added in one of two ways:
/// 1. When a tool is added via `world.spawn(my_tool.into_reflect_tool())`
/// 2. Alternatively by a `ToolMeta` on_add hook
/// if both the input and output are registered in the [`AppTypeRegistry`]
#[derive(Debug, Clone, Copy, Component)]
pub struct ReflectToolMeta {
	tool_meta: ToolMeta,
	input_info: &'static TypeInfo,
	output_info: &'static TypeInfo,
}
impl std::ops::Deref for ReflectToolMeta {
	type Target = ToolMeta;
	fn deref(&self) -> &Self::Target { &self.tool_meta }
}

impl ReflectToolMeta {
	pub fn input_info(&self) -> &'static TypeInfo { self.input_info }
	pub fn output_info(&self) -> &'static TypeInfo { self.output_info }

	#[cfg(feature = "json")]
	pub fn input_json_schema(&self) -> serde_json::Value {
		reflect_ext::type_info_to_json_schema(self.input_info)
	}
	#[cfg(feature = "json")]
	pub fn output_json_schema(&self) -> serde_json::Value {
		reflect_ext::type_info_to_json_schema(self.output_info)
	}
}

/// Lightweight type metadata using [`TypeId`](std::any::TypeId) for
/// comparison and [`type_name`](std::any::type_name) for display.
#[derive(Debug, Copy, Clone)]
pub struct TypeMeta {
	type_name: &'static str,
	type_id: std::any::TypeId,
}

impl TypeMeta {
	/// Create a [`TypeMeta`] for the given type.
	pub fn of<T: 'static>() -> Self {
		Self {
			type_name: std::any::type_name::<T>(),
			type_id: std::any::TypeId::of::<T>(),
		}
	}
	pub fn of_val<T: 'static>(_: &T) -> Self { Self::of::<T>() }

	/// The full type name, ie `core::option::Option<i32>`.
	pub fn type_name(&self) -> &'static str { self.type_name }
	/// The [`TypeId`](std::any::TypeId) for this type.
	pub fn type_id(&self) -> std::any::TypeId { self.type_id }
}

impl std::fmt::Display for TypeMeta {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.type_name)
	}
}

impl PartialEq for TypeMeta {
	fn eq(&self, other: &Self) -> bool { self.type_id == other.type_id }
}

/// Payload for a single tool invocation, containing the caller entity,
/// input value, [`AsyncCommands`] for queuing work, and a callback
/// for delivering the output.
pub struct ToolCall<'w, 's, In, Out> {
	/// Commands for queuing ECS work or spawning async tasks.
	pub commands: AsyncCommands<'w, 's>,
	/// The entity that initiated or owns this tool call.
	pub caller: Entity,
	/// The input payload for this invocation.
	pub input: In,
	/// Callback invoked with the output when the tool completes.
	pub out_handler: OutHandler<Out>,
}

impl<'w, 's, In, Out> ToolCall<'w, 's, In, Out> {}

/// Delivers a tool's output back to the caller.
///
/// Wraps a closure so that different delivery mechanisms (channels,
/// pipe chains, etc.) share a uniform interface.
pub struct OutHandler<Out = ()> {
	func: Box<dyn 'static + Send + Sync + FnOnce(AsyncCommands, Out) -> Result>,
}

impl<Out> Default for OutHandler<Out> {
	fn default() -> Self {
		Self {
			func: Box::new(|_, _| Ok(())),
		}
	}
}

impl<Out> OutHandler<Out> {
	/// Create an [`OutHandler`] from any compatible closure.
	pub fn new<F>(func: F) -> Self
	where
		F: 'static + Send + Sync + FnOnce(AsyncCommands, Out) -> Result,
	{
		Self {
			func: Box::new(func),
		}
	}

	/// Deliver the output, consuming this handler.
	///
	/// # Errors
	/// Returns whatever error the inner callback produces.
	pub fn call(self, commands: AsyncCommands, output: Out) -> Result {
		(self.func)(commands, output)
	}

	pub fn call_world(self, world: &mut World, output: Out) -> Result {
		let mut state = SystemState::<AsyncCommands>::new(world);
		let commands = state.get_mut(world);
		let result = (self.func)(commands, output);
		state.apply(world);
		world.flush();
		result
	}

	pub async fn call_async(self, world: AsyncWorld, output: Out) -> Result
	where
		Out: 'static + Send,
	{
		world
			.with_then(move |world| self.call_world(world, output))
			.await
	}
}


/// Conversion trait for creating a [`Tool`] from a value.
///
/// Implementations exist for plain closures
/// ([`func_tool`](super::func_tool)), Bevy systems
/// ([`system_tool`](super::system_tool)), and async closures
/// ([`async_tool`](super::async_tool)).
pub trait IntoTool<M>: Sized {
	/// Input type for the resulting handler.
	type In;
	/// Output type for the resulting handler.
	type Out;
	/// Convert into a concrete [`Tool`].
	fn into_tool(self) -> Tool<Self::In, Self::Out>;
}

impl<In, Out> IntoTool<()> for Tool<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type In = In;
	type Out = Out;

	fn into_tool(self) -> Tool<Self::In, Self::Out> { self }
}

pub trait IntoReflectTool<M>: IntoTool<M>
where
	Self::In: Typed,
	Self::Out: Typed,
{
	fn reflect_meta() -> ReflectToolMeta;
	fn into_reflect_tool(self) -> (Tool<Self::In, Self::Out>, ReflectToolMeta);
}

impl<T, M> IntoReflectTool<M> for T
where
	T: 'static + IntoTool<M>,
	T::In: Typed,
	T::Out: Typed,
{
	fn into_reflect_tool(self) -> (Tool<Self::In, Self::Out>, ReflectToolMeta) {
		(self.into_tool(), Self::reflect_meta())
	}

	fn reflect_meta() -> ReflectToolMeta {
		{
			ReflectToolMeta {
				tool_meta: ToolMeta::of::<Self, Self::In, Self::Out>(),
				input_info: Self::In::type_info(),
				output_info: Self::Out::type_info(),
			}
		}
	}
}



#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship(relationship_target = Tools)]
pub struct ToolOf(Entity);

impl ToolOf {
	pub fn new(value: Entity) -> Self { Self(value) }
}

/// Component for storing the tools associated with an entity,
/// useful for defining available behaviors on a page, or a clanker tool set.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship_target(relationship = ToolOf,linked_spawn)]
pub struct Tools(Vec<Entity>);



#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	#[should_panic = "No Tool"]
	async fn missing_tool_component() {
		AsyncPlugin::world()
			.spawn_empty()
			.call::<(), ()>(())
			.await
			.unwrap();
	}

	#[tool]
	fn add(a: u32, b: u32) -> u32 { a + b }
	#[test]
	fn missing_reflect_tool() {
		// not registered
		let mut world = World::new();
		let entity = world.spawn(add.into_tool());
		entity.get::<ToolMeta>().xpect_some();
		entity.get::<ReflectToolMeta>().xpect_none();
	}
	#[test]
	fn register_reflect_tool() {
		let mut app = App::new();
		app.register_type::<u32>();
		app.register_type::<(u32, u32)>();
		let world = app.world_mut();
		let entity = world.spawn(add.into_tool());
		entity.get::<ReflectToolMeta>().xpect_some();
	}
	#[test]
	fn into_reflect_tool() {
		let mut world = World::new();
		let entity = world.spawn(add.into_reflect_tool());
		entity.get::<ReflectToolMeta>().xpect_some();
	}
}
