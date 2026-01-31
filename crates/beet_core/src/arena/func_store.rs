use crate::prelude::*;

/// A copyable handle around a function stored in a global [`Arena`] that records
/// each call's output.
///
/// Each `FuncStore` contains:
/// - `func`: the function to call, stored in a [`Store`]
/// - `called`: a `Store<Vec<O>>` where every call's output is pushed in order
///
/// The handle itself is `Copy` and `Clone`, so you can pass it around cheaply.
/// All handles refer to the same underlying function and calls vector; mutating
/// through one handle is visible to all others.
///
/// # Warning
///
/// This handle owns two [`Store`] values (`func` and `called`) which allocate
/// inside the global [`Arena`]. In long-running applications, call `remove()`
/// on both stores when the function is no longer needed to avoid leaking memory.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use beet_core::prelude::*;
/// let adder = FuncStore::new(|n: u32| n + 1);
/// adder.call(1);
/// adder.call(2);
/// assert_eq!(adder.called.len(), 2);
/// assert_eq!(adder.called.get(), vec![2, 3]);
/// // Clean up in long-running apps
/// adder.called.remove();
/// adder.func.remove();
/// ```
///
/// Using `call0` with default input:
///
/// ```
/// # use beet_core::prelude::*;
/// let make_ten = FuncStore::new(|n: u32| n + 10);
/// make_ten.call0(); // uses u32::default() == 0
/// assert_eq!(make_ten.called.get(), vec![10]);
/// make_ten.called.remove();
/// make_ten.func.remove();
/// ```
#[derive(Debug)]
pub struct FuncStore<I, O, F>
where
	O: 'static,
	F: 'static,
{
	/// Captured outputs of each invocation, in call order.
	pub called: Store<Vec<O>>,
	/// The stored function to invoke.
	pub func: Store<F>,
	/// Holds the input type parameter without storing a value.
	pub _phantom: std::marker::PhantomData<I>,
}

impl<I, O, F> Copy for FuncStore<I, O, F>
where
	O: 'static,
	F: 'static,
{
}

impl<I, O, F> Clone for FuncStore<I, O, F>
where
	O: 'static,
	F: 'static,
{
	fn clone(&self) -> Self {
		Self {
			called: self.called.clone(),
			func: self.func.clone(),
			_phantom: std::marker::PhantomData,
		}
	}
}

impl<I, O, F> FuncStore<I, O, F>
where
	O: 'static + Send,
	F: 'static + Send + Fn(I) -> O,
{
	/// Creates a new `FuncStore` by inserting the function into the [`Arena`].
	///
	/// All copies of the handle refer to the same function and `called` vector.
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let fs = FuncStore::<u32, u32, _>::new(|n| n * 2);
	/// fs.call(3);
	/// assert_eq!(fs.called.get(), vec![6]);
	/// // Clean up
	/// fs.called.remove();
	/// fs.func.remove();
	/// ```
	pub fn new(func: F) -> Self {
		Self {
			called: Store::default(),
			func: Store::new(func),
			_phantom: std::marker::PhantomData,
		}
	}

	/// Calls the stored function with `input` and pushes the output to `called`.
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let doubler = FuncStore::new(|n: u32| n * 2);
	/// doubler.call(4);
	/// assert_eq!(doubler.called.get(), vec![8]);
	/// // Clean up
	/// doubler.called.remove();
	/// doubler.func.remove();
	/// ```
	pub fn call(&self, input: I) {
		let output = self.func.with(|func| func(input));
		self.called.push(output);
	}
}

impl<I, O, F> FuncStore<I, O, F>
where
	I: Default,
	O: 'static + Send,
	F: Send + Fn(I) -> O,
{
	/// Calls the stored function with `I::default()` and pushes the output to `called`.
	///
	/// Useful for nullary-style functions expressed with a defaultable input type.
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let make_five = FuncStore::new(|n: u32| n + 5);
	/// make_five.call0(); // uses u32::default() == 0
	/// assert_eq!(make_five.called.get(), vec![5]);
	/// // Clean up
	/// make_five.called.remove();
	/// make_five.func.remove();
	/// ```
	pub fn call0(&self) {
		let output = self.func.with(|func| func(Default::default()));
		self.called.push(output);
	}
}

impl<I, O, F> FuncStore<I, O, F>
where
	O: 'static + Send + Clone,
	F: Send + Fn(I) -> O,
{
	/// Calls the stored function with `input`, records the output, and returns it.
	///
	/// This avoids borrowing from the internal vector and is convenient when you
	/// need both the side-effect (recording) and the return value.
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let triple = FuncStore::new(|n: u32| n * 3);
	/// let got = triple.call_and_get(3);
	/// assert_eq!(got, 9);
	/// assert_eq!(triple.called.get(), vec![9]);
	/// // Clean up
	/// triple.called.remove();
	/// triple.func.remove();
	/// ```
	pub fn call_and_get(&self, input: I) -> O {
		let output = self.func.with(|func| func(input));
		self.called.push(output.clone());
		output
	}
}


/// When the `nightly` feature is enabled, `FuncStore` can be called like a
/// regular function/closure. Each invocation records the output just like
/// [`FuncStore::call`].
#[cfg(feature = "nightly")]
impl<I, O, F> FnOnce<(I,)> for FuncStore<I, O, F>
where
	O: Send,
	F: Send + Fn(I) -> O,
{
	type Output = ();
	extern "rust-call" fn call_once(self, args: (I,)) -> () {
		FuncStore::call(&self, args.0);
	}
}

/// See the `FnOnce` impl notes above.
#[cfg(feature = "nightly")]
impl<I, O, F> FnMut<(I,)> for FuncStore<I, O, F>
where
	O: Send,
	F: Send + Fn(I) -> O,
{
	extern "rust-call" fn call_mut(&mut self, args: (I,)) -> () {
		FuncStore::call(self, args.0);
	}
}

/// See the `FnOnce` impl notes above.
#[cfg(feature = "nightly")]
impl<I, O, F> Fn<(I,)> for FuncStore<I, O, F>
where
	O: Send,
	F: Send + Fn(I) -> O,
{
	extern "rust-call" fn call(&self, args: (I,)) -> () {
		FuncStore::call(self, args.0);
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn calls_and_records() {
		let func_store = FuncStore::new(|n: u32| n + 1);

		func_store.call(1);
		func_store.call(2);

		func_store.called.len().xpect_eq(2);
		let outputs = func_store.called.get();
		outputs.xpect_eq(vec![2, 3]);
	}

	#[test]
	fn multiple_calls() {
		let func_store = FuncStore::new(|v| v);

		func_store.call(4);
		func_store.call(5);

		let outputs = func_store.called.get();
		outputs.xpect_eq(vec![4, 5]);
	}

	#[test]
	fn call0_uses_default_input() {
		let func_store = FuncStore::new(|n: u32| n + 5);

		func_store.call0();

		let outputs = func_store.called.get();
		outputs.xpect_eq(vec![5]);
	}

	#[test]
	fn call_and_get_returns_and_records() {
		let func_store = FuncStore::new(|n: u32| n * 2);

		let first = func_store.call_and_get(3);
		let second = func_store.call_and_get(4);

		first.xpect_eq(6);
		second.xpect_eq(8);

		let outputs = func_store.called.get();
		outputs.xpect_eq(vec![6, 8]);
	}
}
