use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
	static EFFECT_CALLBACK: RefCell<Option<Rc<RefCell<dyn FnMut()>>>> = RefCell::new(None);
}

/// an absolute minimal implementation of a signal
/// for testing of the reactive abstraction and use as an example
/// for integrations of fuller libraries
// https://www.freecodecamp.org/news/learn-javascript-reactivity-build-signals-from-scratch/
struct Signal<T> {
	value: RefCell<T>,
	subscribers: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>,
}

impl<T: Clone> Signal<T> {
	fn new(value: T) -> Self {
		Signal {
			value: RefCell::new(value),
			subscribers: RefCell::new(Vec::new()),
		}
	}

	fn subscribe(&self, callback: Rc<RefCell<dyn FnMut()>>) {
		self.subscribers.borrow_mut().push(callback);
	}

	fn get_value(&self) -> T { self.value.borrow().clone() }

	fn set_value(&self, new_val: T) {
		*self.value.borrow_mut() = new_val;
		for callback in self.subscribers.borrow().iter() {
			callback.borrow_mut()();
		}
	}
}

/// Very simple implementation of effects used for testing and demos
pub fn effect<F>(callback: F)
where
	F: FnMut() + 'static,
{
	let callback = Rc::new(RefCell::new(callback));
	EFFECT_CALLBACK
		.with(|current| *current.borrow_mut() = Some(callback.clone()));
	callback.borrow_mut()();
	EFFECT_CALLBACK.with(|current| *current.borrow_mut() = None);
}

/// Very simple implementation of signals used for testing and demos
pub fn signal<T: Clone + 'static>(
	value: T,
) -> (impl Fn() -> T + Clone, impl Fn(T) + Clone) {
	let signal = Rc::new(Signal::new(value));
	let signal_getter = signal.clone();
	let signal_setter = signal.clone();

	(
		move || {
			EFFECT_CALLBACK.with(|current| {
				if let Some(callback) = current.borrow().clone() {
					signal_getter.subscribe(callback);
				}
			});
			signal_getter.get_value()
		},
		move |new_val| signal_setter.set_value(new_val),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[test]
	fn test_signals_and_effects() {
		let (get_count, set_count) = signal(0);
		let effect_called = Rc::new(RefCell::new(0));
		let effect_called_clone = effect_called.clone();

		let get_count2 = get_count.clone();
		effect(move || {
			get_count2(); // subscribe to changes
			*effect_called_clone.borrow_mut() += 1;
		});

		expect(get_count()).to_be(0);
		expect(*effect_called.borrow()).to_be(1);

		set_count(1);
		expect(get_count()).to_be(1);
		expect(*effect_called.borrow()).to_be(2);

		set_count(2);
		expect(get_count()).to_be(2);
		expect(*effect_called.borrow()).to_be(3);
	}
}
