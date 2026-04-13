use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::Typed;

/// Unified metadata for a tool, combining handler/input/output type
/// information with optional reflection data and description.
///
/// Created via [`ToolMeta::of`], [`ToolMeta::of_tool`],
/// [`ToolMeta::of_handler`], or [`ToolMeta::of_reflect`].
#[derive(Copy, Clone, Debug, Component)]
pub struct ToolMeta {
	/// Type metadata for the tool handler.
	handler: TypeMeta,
	/// Type metadata for the tool input.
	input: TypeMeta,
	/// Type metadata for the tool output.
	output: TypeMeta,
	/// Reflection data, present when the handler type implements [`Typed`].
	/// Input/output [`TypeInfo`] is optionally available when those types
	/// also implement [`Typed`].
	type_info: Option<ToolTypeInfo>,
}

impl ToolMeta {
	/// Create a [`ToolMeta`] from explicit handler, input and output type parameters.
	pub fn of<H: 'static, In: 'static, Out: 'static>() -> Self {
		Self {
			handler: TypeMeta::of::<H>(),
			input: TypeMeta::of::<In>(),
			output: TypeMeta::of::<Out>(),
			type_info: None,
		}
	}

	/// Create a [`ToolMeta`] from a type implementing [`IntoTool`](crate::prelude::IntoTool).
	pub fn of_tool<T, M>() -> Self
	where
		T: 'static + crate::prelude::IntoTool<M>,
		T::In: 'static,
		T::Out: 'static,
	{
		Self {
			handler: TypeMeta::of::<T>(),
			input: TypeMeta::of::<T::In>(),
			output: TypeMeta::of::<T::Out>(),
			type_info: None,
		}
	}

	/// Create a [`ToolMeta`] with handler reflection data. Provides
	/// description from doc comments but no JSON schemas for input/output.
	/// Requires only the handler to implement [`Typed`].
	pub fn of_handler<T, M>() -> Self
	where
		T: 'static + Typed + crate::prelude::IntoTool<M>,
		T::In: 'static,
		T::Out: 'static,
	{
		Self {
			handler: TypeMeta::of::<T>(),
			input: TypeMeta::of::<T::In>(),
			output: TypeMeta::of::<T::Out>(),
			type_info: Some(ToolTypeInfo::of_handler::<T>()),
		}
	}

	/// Create a [`ToolMeta`] with full reflection data from a type
	/// implementing both [`Typed`] and [`IntoTool`](crate::prelude::IntoTool).
	/// Provides description and JSON schemas for input/output.
	pub fn of_reflect<T, M>() -> Self
	where
		T: 'static + Typed + crate::prelude::IntoTool<M>,
		T::In: 'static + Typed,
		T::Out: 'static + Typed,
	{
		Self {
			handler: TypeMeta::of::<T>(),
			input: TypeMeta::of::<T::In>(),
			output: TypeMeta::of::<T::Out>(),
			type_info: Some(ToolTypeInfo::of_full::<T, M>()),
		}
	}

	/// The handler type metadata.
	pub fn handler(&self) -> TypeMeta { self.handler }
	/// The full type name of the handler function or type.
	pub fn name(&self) -> &'static str { self.handler.type_name() }
	/// The input type metadata.
	pub fn input(&self) -> TypeMeta { self.input }
	/// The output type metadata.
	pub fn output(&self) -> TypeMeta { self.output }
	/// The reflection data, if available.
	pub fn type_info(&self) -> Option<&ToolTypeInfo> { self.type_info.as_ref() }

	/// Returns true if the output type matches `T`.
	pub fn output_is<T: 'static>(&self) -> bool {
		self.output.type_id() == std::any::TypeId::of::<T>()
	}

	/// The handler [`TypeInfo`], if reflection data is available.
	pub fn handler_info(&self) -> Option<&'static TypeInfo> {
		self.type_info.map(|info| info.handler_info)
	}

	/// The input [`TypeInfo`], if full reflection data is available.
	pub fn input_info(&self) -> Option<&'static TypeInfo> {
		self.type_info.and_then(|info| info.input_info)
	}

	/// The output [`TypeInfo`], if full reflection data is available.
	pub fn output_info(&self) -> Option<&'static TypeInfo> {
		self.type_info.and_then(|info| info.output_info)
	}

	/// A description from doc comments, if reflection data is available.
	pub fn description(&self) -> Option<&str> {
		self.type_info.as_ref().and_then(|info| info.description())
	}

	/// JSON schema for the input type, if full reflection data is available.
	#[cfg(feature = "json")]
	pub fn input_json_schema(&self) -> Option<serde_json::Value> {
		self.type_info
			.and_then(|info| info.input_info)
			.map(|info| reflect_ext::type_info_to_json_schema(info))
	}

	/// JSON schema for the output type, if full reflection data is available.
	#[cfg(feature = "json")]
	pub fn output_json_schema(&self) -> Option<serde_json::Value> {
		self.type_info
			.and_then(|info| info.output_info)
			.map(|info| reflect_ext::type_info_to_json_schema(info))
	}

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


/// Reflection metadata for a tool. Always includes the handler
/// [`TypeInfo`]; input and output [`TypeInfo`] are optional and
/// present only when created via [`ToolTypeInfo::of_full`].
#[derive(Debug, Copy, Clone)]
pub struct ToolTypeInfo {
	/// The handler [`TypeInfo`].
	handler_info: &'static TypeInfo,
	/// The input [`TypeInfo`], if available.
	input_info: Option<&'static TypeInfo>,
	/// The output [`TypeInfo`], if available.
	output_info: Option<&'static TypeInfo>,
}

impl ToolTypeInfo {
	/// Create [`ToolTypeInfo`] with only handler reflection data.
	/// Provides description but no JSON schemas.
	pub fn of_handler<T: Typed>() -> Self {
		Self {
			handler_info: T::type_info(),
			input_info: None,
			output_info: None,
		}
	}

	/// Create [`ToolTypeInfo`] with full reflection data including
	/// input and output types.
	pub fn of_full<T, M>() -> Self
	where
		T: Typed + crate::prelude::IntoTool<M>,
		T::In: Typed,
		T::Out: Typed,
	{
		Self {
			handler_info: T::type_info(),
			input_info: Some(T::In::type_info()),
			output_info: Some(T::Out::type_info()),
		}
	}

	/// The handler [`TypeInfo`].
	pub fn handler_info(&self) -> &'static TypeInfo { self.handler_info }
	/// The input [`TypeInfo`], if available.
	pub fn input_info(&self) -> Option<&'static TypeInfo> { self.input_info }
	/// The output [`TypeInfo`], if available.
	pub fn output_info(&self) -> Option<&'static TypeInfo> { self.output_info }

	/// A description from the handler's doc comments, if available.
	pub fn description(&self) -> Option<&str> {
		cfg_if! {
			if #[cfg(feature = "reflect")] {
				self.handler_info.docs()
			} else {
				None
			}
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
