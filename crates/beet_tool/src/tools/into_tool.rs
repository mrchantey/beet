use crate::prelude::*;
use bevy::reflect::Typed;
// use std::sync::Arc;




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

pub trait DefaultTool<Input, Output> {
	fn default_tool() -> Tool<Input, Output>;
}
