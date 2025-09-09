use std::sync::Arc;
use std::sync::Mutex;




pub fn mock_trigger() -> MockFunc<(), (), fn(())> {
	fn func(_: ()) {}
	MockFunc::new(func)
}
pub fn mock_bucket<T>() -> MockFunc<T, T, fn(val: T) -> T> {
	fn func<T>(val: T) -> T { val }
	MockFunc::new(func)
}
pub fn mock_func<I, O, F: Fn(I) -> O>(func: F) -> MockFunc<I, O, F> {
	MockFunc::new(func)
}



#[cfg(feature = "nightly")]
impl<I, O, F: Fn(I) -> O> FnOnce<(I,)> for MockFunc<I, O, F> {
	type Output = ();
	extern "rust-call" fn call_once(self, args: (I,)) -> () {
		MockFunc::call(&self, args.0);
	}
}
#[cfg(feature = "nightly")]
impl<I, O, F: Fn(I) -> O> FnMut<(I,)> for MockFunc<I, O, F> {
	extern "rust-call" fn call_mut(&mut self, args: (I,)) -> () {
		MockFunc::call(self, args.0);
	}
}
#[cfg(feature = "nightly")]
impl<I, O, F: Fn(I) -> O> Fn<(I,)> for MockFunc<I, O, F> {
	extern "rust-call" fn call(&self, args: (I,)) -> () {
		MockFunc::call(self, args.0);
	}
}


#[derive(Debug, Clone)]
pub struct MockFunc<I, O, F> {
	pub called: Arc<Mutex<Vec<O>>>,
	pub func: F,
	pub _phantom: std::marker::PhantomData<I>,
}



impl<I, O, F: Fn(I) -> O> MockFunc<I, O, F> {
	pub fn new(func: F) -> Self {
		Self {
			called: Default::default(),
			func,
			_phantom: std::marker::PhantomData,
		}
	}
	pub fn call(&self, input: I) {
		let output = (self.func)(input);
		self.called.lock().unwrap().push(output);
	}
}
impl<I: Default, O, F: Fn(I) -> O> MockFunc<I, O, F> {
	pub fn call0(&self) {
		let output = (self.func)(I::default());
		self.called.lock().unwrap().push(output);
	}
}

impl<I, O: Clone, F: Fn(I) -> O> MockFunc<I, O, F> {
	pub fn call_and_get(&self, input: I) -> O {
		let output = (self.func)(input);
		self.called.lock().unwrap().push(output.clone());
		output
	}
}
