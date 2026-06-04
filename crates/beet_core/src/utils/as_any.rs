use alloc::boxed::Box;

/// A trait for converting a type to `Any`, allowing for downcasting.
pub trait AsAny {
	/// Casts self to a reference of type `&dyn Any`.
	fn as_any(&self) -> &dyn core::any::Any;
	/// Casts self to a mutable reference of type `&mut dyn Any`.
	fn as_any_mut(&mut self) -> &mut dyn core::any::Any;
	/// Casts self to a boxed value of type `Box<dyn Any>`.
	fn as_any_boxed(self: Box<Self>) -> Box<dyn core::any::Any>;
}
