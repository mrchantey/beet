//! Simple reactive signal primitives for testing and demos.
//!
//! This module provides a minimal signal implementation useful for testing
//! reactive abstractions or as a reference for integrating fuller reactive
//! libraries.

use crate::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;


/// Creates a read-only [`Getter`] for the given value.
///
/// This is a convenience function when you only need to read a signal.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// let get = getter(42);
/// assert_eq!(get.get(), 42);
/// ```
pub fn getter<T: 'static + Send + Clone>(value: T) -> Getter<T> {
	signal(value).0
}

/// Creates a reactive signal with separate [`Getter`] and [`Setter`] handles.
///
/// Signals provide a simple reactive primitive where reading a value inside
/// an [`effect`] automatically subscribes to future updates.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// let (get, set) = signal(10);
/// assert_eq!(get.get(), 10);
/// set.set(20);
/// assert_eq!(get.get(), 20);
/// ```
pub fn signal<T: 'static + Send + Clone>(value: T) -> (Getter<T>, Setter<T>) {
	let signal = Signal::new(value);
	let signal_handle = Arena::insert(signal);

	(Getter::new(signal_handle), Setter::new(signal_handle))
}

thread_local! {
	static EFFECT_CALLBACK: Mutex<Option<Arc<Mutex<dyn FnMut() + Send>>>> = Mutex::new(None);
}

/// Runs a callback that automatically re-executes when any signals it reads change.
///
/// When a signal is read inside the callback, the effect subscribes to that signal.
/// Future updates to the signal will trigger the callback again.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// # use std::sync::{Arc, Mutex};
/// let (get, set) = signal(0);
/// let count = Arc::new(Mutex::new(0));
/// let count_clone = count.clone();
///
/// effect(move || {
///     get.get(); // subscribe to changes
///     *count_clone.lock().unwrap() += 1;
/// });
///
/// assert_eq!(*count.lock().unwrap(), 1); // initial run
/// set.set(1);
/// assert_eq!(*count.lock().unwrap(), 2); // re-run on change
/// ```
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


/// A minimal signal implementation for testing reactive abstractions.
///
/// This is intentionally simple and serves as an example for integrating
/// more complete reactive libraries.
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

/// A copyable handle providing read access to a signal.
///
/// When read inside an [`effect`], the getter automatically subscribes
/// to the signal so the effect re-runs on changes.
pub struct Getter<T> {
	handle: ArenaHandle<Signal<T>>,
}

impl<T> Getter<T> {
	/// Creates a new getter from an arena handle.
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
	/// Returns a clone of the current signal value.
	///
	/// If called inside an [`effect`], this getter subscribes to the signal.
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

/// A copyable handle providing write access to a signal.
///
/// Changes made through a setter notify all subscribed effects.
pub struct Setter<T> {
	getter: Getter<T>,
	handle: ArenaHandle<Signal<T>>,
}

impl<T> Setter<T> {
	/// Creates a new setter from an arena handle.
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
	/// Applies a function to transform the current value.
	pub fn map(&self, updater: impl FnOnce(T) -> T) {
		self.set(updater(self.getter.get()));
	}

	/// Mutates the current value in place.
	pub fn update(&self, updater: impl FnOnce(&mut T)) {
		let mut value = self.getter.get();
		updater(&mut value);
		self.set(value);
	}

	/// Replaces the signal value and notifies all subscribers.
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
	/// Pushes a value onto the signal's vector and notifies subscribers.
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
	use crate::prelude::*;
	use std::sync::Arc;
	use std::sync::Mutex;

	#[test]
	fn signals() {
		let (get, set) = signal(7);
		get.get().xpect_eq(7);
		set.set(10);
		get.get().xpect_eq(10);
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

		get.get().xpect_eq(0);
		(*effect_called.lock().unwrap()).xpect_eq(1);

		set.set(1);
		get.get().xpect_eq(1);
		(*effect_called.lock().unwrap()).xpect_eq(2);

		set.set(2);
		get.get().xpect_eq(2);
		(*effect_called.lock().unwrap()).xpect_eq(3);
	}
}
