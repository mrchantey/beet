use crate::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

/// Very simple implementation of signals used for testing and demos
pub fn signal<T: 'static + Send + Clone>(value: T) -> (Getter<T>, Setter<T>) {
	let signal = Signal::new(value);
	let signal_handle = Arena::insert(signal);

	(Getter::new(signal_handle), Setter::new(signal_handle))
}

thread_local! {
	static EFFECT_CALLBACK: Mutex<Option<Arc<Mutex<dyn FnMut() + Send>>>> = Mutex::new(None);
}

/// Very simple implementation of effects used for testing and demos
pub fn effect<F>(callback: F)
where
	F: 'static + Send + Sync + FnMut(),
{
	let callback = Arc::new(Mutex::new(callback));
	EFFECT_CALLBACK
		.with(|current| *current.lock().unwrap() = Some(callback.clone()));
	callback.lock().unwrap()();
	EFFECT_CALLBACK.with(|current| *current.lock().unwrap() = None);
}


/// an absolute minimal implementation of a signal
/// for testing of the reactive abstraction and use as an example
/// for integrations of fuller libraries
pub struct Signal<T> {
	value: Mutex<T>,
	subscribers: Mutex<Vec<Arc<Mutex<dyn FnMut() + Send>>>>,
}

impl<T: Clone + Send> Signal<T> {
	fn new(value: T) -> Self {
		Signal {
			value: Mutex::new(value),
			subscribers: Mutex::new(Vec::new()),
		}
	}

	fn subscribe(&self, callback: Arc<Mutex<dyn FnMut() + Send>>) {
		self.subscribers.lock().unwrap().push(callback);
	}
}

/// A Copy type that provides read access to a signal
pub struct Getter<T> {
	handle: ArenaHandle<Signal<T>>,
}
impl<T> Getter<T> {
	pub fn new(handle: ArenaHandle<Signal<T>>) -> Self { Getter { handle } }
}

impl<T> Copy for Getter<T> {}

impl<T> Clone for Getter<T> {
	fn clone(&self) -> Self {
		Getter {
			handle: self.handle.clone(),
		}
	}
}


impl<T: 'static + Send + Clone> Getter<T> {
	pub fn get(&self) -> T {
		// First, register the callback if we're in an effect
		EFFECT_CALLBACK.with(|current| {
			if let Some(callback) = current.lock().unwrap().clone() {
				self.handle.with(|signal| signal.subscribe(callback));
			}
		});
		// Then get the value - this needs to be separate to avoid deadlock
		self.handle
			.with(|signal| signal.value.lock().unwrap().clone())
	}
}


#[cfg(feature = "nightly")]
impl<T: 'static + Send + Clone> std::ops::Fn<()> for Getter<T> {
	extern "rust-call" fn call(&self, _args: ()) -> Self::Output { self.get() }
}

#[cfg(feature = "nightly")]
impl<T: 'static + Send + Clone> std::ops::FnMut<()> for Getter<T> {
	extern "rust-call" fn call_mut(&mut self, args: ()) -> Self::Output {
		self.call(args)
	}
}

#[cfg(feature = "nightly")]
impl<T: 'static + Send + Clone> std::ops::FnOnce<()> for Getter<T> {
	type Output = T;
	extern "rust-call" fn call_once(self, args: ()) -> Self::Output {
		self.call(args)
	}
}

/// A Copy type that provides write access to a signal
pub struct Setter<T> {
	getter: Getter<T>,
	handle: ArenaHandle<Signal<T>>,
}
impl<T> Setter<T> {
	pub fn new(handle: ArenaHandle<Signal<T>>) -> Self {
		Setter {
			getter: Getter::new(handle),
			handle,
		}
	}
}

impl<T> Copy for Setter<T> {}

impl<T> Clone for Setter<T> {
	fn clone(&self) -> Self {
		Setter {
			getter: self.getter,
			handle: self.handle,
		}
	}
}

impl<T: 'static + Send + Clone> Setter<T> {
	pub fn map(&self, updater: impl FnOnce(T) -> T) {
		self.set(updater(self.getter.get()));
	}
	pub fn update(&self, updater: impl FnOnce(&mut T)) {
		let mut value = self.getter.get();
		updater(&mut value);
		self.set(value);
	}

	pub fn set(&self, new_val: T) {
		// First, extract the callbacks outside of the arena lock
		let callbacks = self.handle.with(|signal| {
			*signal.value.lock().unwrap() = new_val;
			signal.subscribers.lock().unwrap().clone()
		});

		// Now execute callbacks without holding any arena locks
		for callback in callbacks.iter() {
			callback.lock().unwrap()();
		}
	}
}

impl<T: 'static + Send + Clone> Setter<Vec<T>> {
	pub fn push(&self, new_val: T) { self.update(|vec| vec.push(new_val)); }
}

#[cfg(feature = "nightly")]
impl<T: 'static + Send + Clone> std::ops::Fn<(T,)> for Setter<T> {
	extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
		self.set(args.0)
	}
}

#[cfg(feature = "nightly")]
impl<T: 'static + Send + Clone> std::ops::FnMut<(T,)> for Setter<T> {
	extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
		self.call(args)
	}
}

#[cfg(feature = "nightly")]
impl<T: 'static + Send + Clone> std::ops::FnOnce<(T,)> for Setter<T> {
	type Output = ();
	extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
		self.call(args)
	}
}


#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn signals() {
		let (get, set) = signal(7);
		assert_eq!(get.get(), 7);
		set.set(10);
		assert_eq!(get.get(), 10);
	}
	#[test]
	fn effects() {
		let (get, set) = signal(0);
		let effect_called = Arc::new(Mutex::new(0));
		let effect_called_clone = effect_called.clone();

		effect(move || {
			get.get(); // subscribe to changes
			*effect_called_clone.lock().unwrap() += 1;
		});

		assert_eq!(get.get(), 0);
		assert_eq!(*effect_called.lock().unwrap(), 1);

		set.set(1);
		assert_eq!(get.get(), 1);
		assert_eq!(*effect_called.lock().unwrap(), 2);

		set.set(2);
		assert_eq!(get.get(), 2);
		assert_eq!(*effect_called.lock().unwrap(), 3);
	}
}
