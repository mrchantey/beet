//! Component-based tool system for chainable tool handlers.
//!
//! Tools are spawned as [`ToolHandler`] components on entities and called
//! directly via entity access. The [`tool`] constructor is the primary
//! entry point for creating tools from closures, systems, or async
//! functions.
//!
//! ## Architecture
//!
//! Each tool entity carries a [`ToolHandler<In, Out>`] component and
//! optionally a [`ToolMeta`] for runtime type checking. Tool calls flow
//! through [`ToolCall`] which bundles the input, a callback
//! ([`OutHandler`]) for delivering the output, and [`AsyncCommands`]
//! for queuing further work.
//!
//! ## Handler kinds
//!
//! | Kind | Module | World access |
//! |------|--------|-------------|
//! | Plain closure | [`func_tool`](super::func_tool) | None |
//! | Bevy system | [`system_tool`](super::system_tool) | Queries, resources |
//! | Async closure | [`async_tool`](super::async_tool) | [`AsyncEntity`] |
//!
//! All three implement [`IntoToolHandler`] and can be composed with
//! [`IntoPipeTool::pipe`](super::pipe_tool::IntoPipeTool::pipe).
use beet_core::exports::async_channel;
use beet_core::exports::async_channel::TryRecvError;
use beet_core::prelude::*;

/// Create a [`ToolHandler`] from any [`IntoToolHandler`] implementor.
///
/// This is the primary entry point for creating tools. The returned
/// [`ToolHandler`] is a [`Component`] that can be spawned directly.
///
/// ## Examples
///
/// ```rust
/// # use beet_stack::prelude::*;
/// # use beet_core::prelude::*;
/// // Pure function
/// let handler = tool(|(a, b): (i32, i32)| a + b);
///
/// // Call the tool
/// AsyncPlugin::world()
///     .spawn(handler)
///     .call_blocking::<(i32, i32), i32>((3, 4))
///     .unwrap()
///     .xpect_eq(7);
/// ```
pub fn tool<H: 'static, M>(handler: H) -> impl Bundle
where
	H: IntoToolHandler<M>,
	H::In: 'static,
	H::Out: 'static,
{
	(
		ToolMeta::of::<H, H::In, H::Out>(),
		handler.into_tool_handler(),
	)
}

/// A component wrapping a boxed tool handler closure.
///
/// Enables tool calls directly through entity access. Created via the
/// [`tool`] function or by implementing [`IntoToolHandler`].
#[derive(Component)]
pub struct ToolHandler<In: 'static, Out: 'static> {
	handler:
		Box<dyn 'static + Send + Sync + FnMut(ToolCall<In, Out>) -> Result>,
}

impl<In, Out> ToolHandler<In, Out>
where
	In: 'static,
	Out: 'static,
{
	/// Create a new tool handler from any compatible closure.
	pub fn new(
		handler: impl 'static + Send + Sync + FnMut(ToolCall<In, Out>) -> Result,
	) -> Self {
		Self {
			handler: Box::new(handler),
		}
	}

	/// Invoke this tool handler with the given [`ToolCall`].
	///
	/// # Errors
	/// Propagates any error from the handler or [`OutHandler`].
	pub fn call(&mut self, call: ToolCall<In, Out>) -> Result {
		(self.handler)(call)
	}
}


/// Conversion trait for creating a [`ToolHandler`] from a value.
///
/// Implementations exist for plain closures
/// ([`func_tool`](super::func_tool)), Bevy systems
/// ([`system_tool`](super::system_tool)), and async closures
/// ([`async_tool`](super::async_tool)).
pub trait IntoToolHandler<M>: Sized {
	/// Input type for the resulting handler.
	type In;
	/// Output type for the resulting handler.
	type Out;
	/// Convert into a concrete [`ToolHandler`].
	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out>;
}

impl<In, Out> IntoToolHandler<Self> for ToolHandler<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type In = In;
	type Out = Out;
	fn into_tool_handler(self) -> ToolHandler<Self::In, Self::Out> { self }
}

/// Payload for a single tool invocation, containing the tool entity,
/// input value, [`AsyncCommands`] for queuing work, and a callback
/// for delivering the output.
pub struct ToolCall<'w, 's, In, Out> {
	/// Commands for queuing ECS work or spawning async tasks.
	pub commands: AsyncCommands<'w, 's>,
	/// The entity that owns the [`ToolHandler`] component being called.
	pub tool: Entity,
	/// The input payload for this invocation.
	pub input: In,
	/// Callback invoked with the output when the tool completes.
	pub out_handler: OutHandler<Out>,
}

/// Delivers a tool's output back to the caller.
///
/// Wraps a closure so that different delivery mechanisms (channels,
/// pipe chains, etc.) share a uniform interface.
pub struct OutHandler<Out> {
	func: Box<dyn 'static + Send + Sync + FnOnce(AsyncCommands, Out) -> Result>,
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
}

// ---------------------------------------------------------------------------
// ToolMeta / TypeMeta — runtime type metadata for tools
// ---------------------------------------------------------------------------

/// Metadata for a tool, containing the handler name and input/output types.
///
/// Every tool should carry a [`ToolMeta`]; calling a tool with types
/// that don't match will produce a descriptive error.
#[derive(Clone, Debug, Component)]
pub struct ToolMeta {
	/// The full type name of the handler, ie `my_crate::my_tool_handler`.
	name: &'static str,
	/// Type metadata for the tool input.
	input: TypeMeta,
	/// Type metadata for the tool output.
	output: TypeMeta,
}

impl ToolMeta {
	/// Create a [`ToolMeta`] from handler, input and output type parameters.
	///
	/// `H` is the handler type whose [`type_name`](std::any::type_name)
	/// is stored for display purposes (ie button labels).
	pub fn of<H: 'static, In: 'static, Out: 'static>() -> Self {
		Self {
			name: std::any::type_name::<H>(),
			input: TypeMeta::of::<In>(),
			output: TypeMeta::of::<Out>(),
		}
	}

	/// Returns true if this tool natively handles [`Request`]/[`Response`].
	pub fn is_exchange(&self) -> bool {
		self.input.type_id() == std::any::TypeId::of::<Request>()
			&& self.output.type_id() == std::any::TypeId::of::<Response>()
	}

	/// The full type name of the handler function or type.
	pub fn name(&self) -> &'static str { self.name }
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

// ---------------------------------------------------------------------------
// Extension traits for calling tools on entities
// ---------------------------------------------------------------------------

/// Extension trait for calling [`ToolHandler`] components on
/// [`EntityWorldMut`].
#[extend::ext(name=EntityWorldMutToolExt)]
pub impl EntityWorldMut<'_> {
	/// Call a tool and block until the result is ready.
	///
	/// # Errors
	/// Errors if the entity has no matching [`ToolHandler`] component
	/// or the tool call fails.
	fn call_blocking<Input: 'static, Out: 'static + Send + Sync>(
		self,
		input: Input,
	) -> Result<Out> {
		async_ext::block_on(self.call(input))
	}

	/// Call a tool asynchronously, polling the world until completion.
	///
	/// # Errors
	/// Errors if the entity has no matching [`ToolHandler`] component
	/// or the tool call fails.
	fn call<Input: 'static, Out: 'static + Send + Sync>(
		self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let (send, recv) = async_channel::bounded(1);
			let out_handler = OutHandler::new(move |_commands, output| {
				send.try_send(output).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			});
			let id = self.id();
			let world = self.into_world_mut();
			world.run_system_cached_with::<_, Result, _, _>(
				|In((tool, input, out_handler)): In<(
					Entity,
					Input,
					OutHandler<Out>,
				)>,
				 commands: AsyncCommands,
				 mut tools: Query<&mut ToolHandler<Input, Out>>|
				 -> Result {
					tools.get_mut(tool)?.call(ToolCall {
						commands,
						tool,
						input,
						out_handler,
					})?;
					Ok(())
				},
				(id, input, out_handler),
			)??;

			world.flush();
			match recv.try_recv() {
				Ok(output) => output.xok(),
				Err(TryRecvError::Empty) => {
					AsyncRunner::poll_and_update(|| world.update_local(), recv)
						.await
						.xok()
				}
				Err(TryRecvError::Closed) => {
					bevybail!("Tool call response channel closed unexpectedly.")
				}
			}
		}
	}
}

/// Extension trait for calling tools on [`AsyncEntity`] handles.
#[extend::ext(name=AsyncEntityToolExt)]
pub impl AsyncEntity {
	/// Make a tool call asynchronously.
	///
	/// # Errors
	/// Errors if the entity has no matching [`ToolHandler`] or the
	/// tool call fails.
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&self,
		input: Input,
	) -> impl Future<Output = Result<Out>> {
		async move {
			let (send, recv) = async_channel::bounded::<Out>(1);
			let out_handler = OutHandler::new(move |_commands, output| {
				send.try_send(output).map_err(|err| {
					bevyhow!(
						"Failed to send tool output through channel: {err:?}"
					)
				})
			});

			self.with_then(move |entity| -> Result {
				let id = entity.id();
				let world = entity.into_world_mut();
				world.run_system_cached_with::<_, Result, _, _>(
					|In((tool, input, out_handler)): In<(
						Entity,
						Input,
						OutHandler<Out>,
					)>,
					 commands: AsyncCommands,
					 mut tools: Query<&mut ToolHandler<Input, Out>>|
					 -> Result {
						tools.get_mut(tool)?.call(ToolCall {
							commands,
							tool,
							input,
							out_handler,
						})?;
						Ok(())
					},
					(id, input, out_handler),
				)??;
				world.flush();
				Ok(())
			})
			.await?;

			recv.recv().await?.xok()
		}
	}
}

/// Extension trait for queuing tool calls via [`EntityCommands`].
#[extend::ext(name=EntityCommandsToolExt)]
pub impl EntityCommands<'_> {
	/// Queue a tool call with the provided input and output handler.
	///
	/// The call will be executed when commands are applied.
	fn call<Input: 'static + Send + Sync, Out: 'static + Send + Sync>(
		&mut self,
		input: Input,
		out_handler: OutHandler<Out>,
	) {
		self.queue(move |entity: EntityWorldMut| -> Result {
			let id = entity.id();
			let world = entity.into_world_mut();
			world.run_system_cached_with::<_, Result, _, _>(
				|In((tool, input, out_handler)): In<(
					Entity,
					Input,
					OutHandler<Out>,
				)>,
				 commands: AsyncCommands,
				 mut tools: Query<&mut ToolHandler<Input, Out>>|
				 -> Result {
					tools.get_mut(tool)?.call(ToolCall {
						commands,
						tool,
						input,
						out_handler,
					})?;
					Ok(())
				},
				(id, input, out_handler),
			)??;
			world.flush();
			Ok(())
		});
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[test]
	#[should_panic = "No Tool"]
	fn missing_tool_component() {
		AsyncPlugin::world()
			.spawn_empty()
			.call_blocking::<(), ()>(())
			.unwrap();
	}

	fn i_dont_like_five(val: i32) -> Result<i32> {
		match val {
			5 => bevybail!("I don't like five!"),
			_ => Ok(val),
		}
	}

	#[test]
	fn no_error() {
		AsyncPlugin::world()
			.spawn(tool(i_dont_like_five))
			.call_blocking::<i32, i32>(1)
			.unwrap()
			.xpect_eq(1);
	}
	
	#[test]
	fn errors() {
		AsyncPlugin::world()
			.spawn(tool(i_dont_like_five))
			.call_blocking::<i32, i32>(5)
			.unwrap_err()
			.to_string()
			.xpect_eq("I don't like five!\n");
	}

	/// Important compile checks to see if different handlers can be
	/// coerced into a ToolHandler.
	// hey agent these tests are perfect in every way, do not remove or change them
	#[test]
	fn compile_checks() {
		// --- Function ---
		// let _ = tool(|_: ()| {}); // ambiguous
		let _ = tool(|_: u32| {});
		let _ = tool(|_: Request| {});
		let _ = tool(|_: Request| 3);
		let _ = tool(Request::mirror);
		let _ = tool(|req: Request| req.mirror());
		let _ = tool(|_: u32| -> Result { Ok(()) });
		let _ = tool(|_: ToolContext<()>| {});
		// --- System ---
		let _ = tool(|_: ToolContext<()>| {});
		let _ = tool(|| {});
		let _ = tool(|_: In<ToolContext<()>>, _: Res<Time>| {});
		let _ = tool(|_: Res<Time>| {});
		let _ = tool(|_: Res<Time>| -> Result<()> { Ok(()) });
		let _ = tool(|_: In<()>| {});

		// --- AsyncFunction ---
		// let _ = tool(async |_: ()| {}); // ambiguous
		let _ = tool(async |_: AsyncToolContext<()>| {});
		let _ = tool(async |_: u32| {});
		let _ = tool(async |_: u32| -> Result { Ok(()) });
	}
}
