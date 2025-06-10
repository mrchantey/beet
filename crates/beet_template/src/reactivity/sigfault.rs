use std::sync::Arc;
use std::sync::Mutex;
use sweet::prelude::Arena;
use sweet::prelude::ArenaHandle;

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


/// A Copy type that provides read access to a signal
pub struct Getter<T> {
	handle: ArenaHandle<Signal<T>>,
}
impl<T> Copy for Getter<T> {}

#[cfg(feature = "nightly")]
impl<T: Clone + Send + 'static> std::ops::Fn<()> for Getter<T> {
	extern "rust-call" fn call(&self, _args: ()) -> Self::Output { self.get() }
}

#[cfg(feature = "nightly")]
impl<T: Clone + Send + 'static> std::ops::FnMut<()> for Getter<T> {
	extern "rust-call" fn call_mut(&mut self, args: ()) -> Self::Output {
		self.call(args)
	}
}

#[cfg(feature = "nightly")]
impl<T: Clone + Send + 'static> std::ops::FnOnce<()> for Getter<T> {
	type Output = T;
	extern "rust-call" fn call_once(self, args: ()) -> Self::Output {
		self.call(args)
	}
}

impl<T> Clone for Getter<T> {
	fn clone(&self) -> Self {
		Getter {
			handle: self.handle.clone(),
		}
	}
}


impl<T: Clone + Send + 'static> Getter<T> {
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

/// A Copy type that provides write access to a signal
pub struct Setter<T> {
	handle: ArenaHandle<Signal<T>>,
}
impl<T> Copy for Setter<T> {}

#[cfg(feature = "nightly")]
impl<T: Clone + Send + 'static> std::ops::Fn<(T,)> for Setter<T> {
	extern "rust-call" fn call(&self, args: (T,)) -> Self::Output {
		self.set(args.0)
	}
}

#[cfg(feature = "nightly")]
impl<T: Clone + Send + 'static> std::ops::FnMut<(T,)> for Setter<T> {
	extern "rust-call" fn call_mut(&mut self, args: (T,)) -> Self::Output {
		self.call(args)
	}
}

#[cfg(feature = "nightly")]
impl<T: Clone + Send + 'static> std::ops::FnOnce<(T,)> for Setter<T> {
	type Output = ();
	extern "rust-call" fn call_once(self, args: (T,)) -> Self::Output {
		self.call(args)
	}
}
impl<T> Clone for Setter<T> {
	fn clone(&self) -> Self {
		Setter {
			handle: self.handle.clone(),
		}
	}
}

impl<T: Clone + Send + 'static> Setter<T> {
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

/// Very simple implementation of signals used for testing and demos
pub fn signal<T: Clone + Send + 'static>(value: T) -> (Getter<T>, Setter<T>) {
	let signal = Signal::new(value);
	let signal_handle = Arena::insert(signal);

	(
		Getter {
			handle: signal_handle,
		},
		Setter {
			handle: signal_handle,
		},
	)
}


#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn signals() {
		let (get, set) = signal(7);
		expect(get()).to_be(7);
		set(10);
		expect(get()).to_be(10);
	}
	#[test]
	fn effects() {
		let (get, set) = signal(0);
		let effect_called = Arc::new(Mutex::new(0));
		let effect_called_clone = effect_called.clone();

		effect(move || {
			get(); // subscribe to changes
			*effect_called_clone.lock().unwrap() += 1;
		});

		expect(get()).to_be(0);
		expect(*effect_called.lock().unwrap()).to_be(1);

		set(1);
		expect(get()).to_be(1);
		expect(*effect_called.lock().unwrap()).to_be(2);

		set(2);
		expect(get()).to_be(2);
		expect(*effect_called.lock().unwrap()).to_be(3);
	}
}
