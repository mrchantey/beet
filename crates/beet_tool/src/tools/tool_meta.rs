use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;

#[derive(Copy, Clone, Debug, Component)]
#[component(on_add=try_add_reflect_tool_meta)]
pub struct ToolMeta {
	/// Type metadata for the tool handler,
	/// this is the type of the actual function.
	pub(super) handler: TypeMeta,
	/// Type metadata for the tool input.
	pub(super) input: TypeMeta,
	/// Type metadata for the tool output.
	pub(super) output: TypeMeta,
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
	pub(super) tool_meta: ToolMeta,
	pub(super) input_info: &'static TypeInfo,
	pub(super) output_info: &'static TypeInfo,
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

#[derive(Debug, Clone, Get, Deref, Component)]
pub struct ToolDescription {
	description: String,
}
impl ToolDescription {
	pub fn new(description: impl Into<String>) -> Self {
		Self {
			description: description.into(),
		}
	}
	pub fn of<T: Typed>() -> Self {
		let type_info = T::type_info();
		cfg_if! {
			if #[cfg(feature="reflect")]{
			let docs = type_info
				.docs()
				.unwrap_or("No Description Available".into());
			} else {
				let docs = "No Description Available";
			}
		};
		Self::new(docs)
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
