use crate::arena::Getter;

/// A utility struct containing a `'static + Send + Clone + FnOnce()`
pub struct CloneFunc<In, Out>(pub Box<dyn CloneFuncTrait<In, Out>>);

impl<In, Out> CloneFunc<In, Out>
where
	In: 'static,
	Out: 'static,
{
	/// Creates a new [`CloneFunc`] from a cloneable function.
	pub fn new(func: impl CloneFuncTrait<In, Out>) -> Self {
		Self(Box::new(func))
	}
	/// Calls the wrapped function with the given input.
	pub fn call_func(&self, input: In) -> Out { self.0.call_func(input) }
}

impl<In, Out> Clone for CloneFunc<In, Out>
where
	In: 'static,
	Out: 'static,
{
	fn clone(&self) -> Self { Self(self.0.clone_func_box_clone()) }
}

/// Trait for cloneable function objects.
pub trait CloneFuncTrait<In, Out>: 'static + Send + Sync {
	/// Clones the function into a boxed trait object.
	fn clone_func_box_clone(&self) -> Box<dyn CloneFuncTrait<In, Out>>;
	/// Calls the function with the given input.
	fn call_func(&self, input: In) -> Out;
}

impl<In, Out, F> CloneFuncTrait<In, Out> for F
where
	F: 'static + Send + Sync + Clone + FnOnce(In) -> Out,
{
	fn clone_func_box_clone(&self) -> Box<dyn CloneFuncTrait<In, Out>> {
		Box::new(self.clone())
	}
	fn call_func(&self, input: In) -> Out { (self.clone())(input) }
}

impl<Out> CloneFuncTrait<(), Out> for Getter<Out>
where
	Out: 'static + Send + Clone,
{
	fn clone_func_box_clone(&self) -> Box<dyn CloneFuncTrait<(), Out>> {
		Box::new(self.clone())
	}
	fn call_func(&self, _: ()) -> Out { self.get() }
}



#[cfg(feature = "nightly")]
impl<In, Out> std::ops::FnOnce<(In,)> for CloneFunc<In, Out>
where
	In: 'static,
	Out: 'static,
{
	type Output = Out;
	extern "rust-call" fn call_once(self, args: (In,)) -> Self::Output {
		self.0.call_func(args.0)
	}
}

#[cfg(feature = "nightly")]
impl<In, Out> std::ops::FnMut<(In,)> for CloneFunc<In, Out>
where
	In: 'static,
	Out: 'static,
{
	extern "rust-call" fn call_mut(&mut self, args: (In,)) -> Self::Output {
		self.0.call_func(args.0)
	}
}

#[cfg(feature = "nightly")]
impl<In, Out> std::ops::Fn<(In,)> for CloneFunc<In, Out>
where
	In: 'static,
	Out: 'static,
{
	extern "rust-call" fn call(&self, args: (In,)) -> Self::Output {
		self.0.call_func(args.0)
	}
}
