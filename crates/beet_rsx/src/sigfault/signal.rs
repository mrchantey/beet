use std::sync::Arc;
use std::sync::Mutex;

/// an absolute minimal implementation of a signal
/// for testing of the reactive abstraction and use as an example
/// for integrations of fuller libraries
struct Signal<T> {
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

	fn get_value(&self) -> T { self.value.lock().unwrap().clone() }

	fn set_value(&self, new_val: T) {
		*self.value.lock().unwrap() = new_val;
		for callback in self.subscribers.lock().unwrap().iter() {
			callback.lock().unwrap()();
		}
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

/// Very simple implementation of signals used for testing and demos
pub fn signal<T: Clone + Send + 'static>(
	value: T,
) -> (impl Fn() -> T + Clone, impl Fn(T) + Clone) {
	let signal = Arc::new(Signal::new(value));
	let signal_getter = signal.clone();
	let signal_setter = signal.clone();

	(
		move || {
			EFFECT_CALLBACK.with(|current| {
				if let Some(callback) = current.lock().unwrap().clone() {
					signal_getter.subscribe(callback);
				}
			});
			signal_getter.get_value()
		},
		move |new_val| signal_setter.set_value(new_val),
	)
}

#[cfg(test)]
mod test {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn test_signals_and_effects() {
		let (get_count, set_count) = signal(0);
		let effect_called = Arc::new(Mutex::new(0));
		let effect_called_clone = effect_called.clone();

		let get_count2 = get_count.clone();
		effect(move || {
			get_count2(); // subscribe to changes
			*effect_called_clone.lock().unwrap() += 1;
		});

		expect(get_count()).to_be(0);
		expect(*effect_called.lock().unwrap()).to_be(1);

		set_count(1);
		expect(get_count()).to_be(1);
		expect(*effect_called.lock().unwrap()).to_be(2);

		set_count(2);
		expect(get_count()).to_be(2);
		expect(*effect_called.lock().unwrap()).to_be(3);
	}
}
