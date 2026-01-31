use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;

/// Entry in the arena.
struct ArenaEntry {
	object: Box<dyn Any + Send>,
}

/// A global, thread-safe arena for storing `Send` objects with copyable handles.
///
/// Objects are stored in a lazily-initialized global instance and accessed through
/// [`ArenaHandle`] values. This enables passing around lightweight, copyable
/// references without lifetime constraints.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// let handle = Arena::insert(42u32);
/// handle.with(|val| assert_eq!(*val, 42));
/// let removed = handle.remove();
/// assert_eq!(removed, 42);
/// ```
pub struct Arena {
	objects: Arc<Mutex<HashMap<usize, ArenaEntry>>>,
	next_id: Arc<Mutex<usize>>,
}

impl Arena {
	fn new() -> Self {
		Self {
			objects: Arc::new(Mutex::new(HashMap::new())),
			next_id: Arc::new(Mutex::new(0)),
		}
	}

	fn get_global() -> &'static Arena {
		static ARENA: LazyLock<Arena> = LazyLock::new(|| Arena::new());
		&ARENA
	}

	/// Inserts an object into the global arena and returns a handle to it.
	pub fn insert<T: Send + 'static>(object: T) -> ArenaHandle<T> {
		Self::get_global().insert_impl(object)
	}

	/// Stores an object and returns a handle to it.
	fn insert_impl<T: Send + 'static>(&self, object: T) -> ArenaHandle<T> {
		let mut objects = self.objects.lock().unwrap();
		let mut next_id = self.next_id.lock().unwrap();

		let id = *next_id;
		*next_id += 1;

		let entry = ArenaEntry {
			object: Box::new(object),
		};

		objects.insert(id, entry);

		ArenaHandle {
			id,
			_phantom: std::marker::PhantomData,
		}
	}

	/// Manually removes an object from the arena.
	fn remove_impl<T: 'static>(&self, handle: &ArenaHandle<T>) -> Option<T> {
		let mut objects = self.objects.lock().unwrap();
		objects
			.remove(&handle.id)
			.and_then(|entry| entry.object.downcast().ok())
			.map(|boxed| *boxed)
	}

	/// Returns the number of objects stored in the global arena.
	pub fn len() -> usize { Self::get_global().objects.lock().unwrap().len() }

	/// Removes all objects from the global arena, invalidating all handles.
	pub fn clear() { Self::get_global().objects.lock().unwrap().clear(); }

	/// Returns `true` if the global arena contains no objects.
	pub fn is_empty() -> bool {
		Self::get_global().objects.lock().unwrap().is_empty()
	}

	/// Executes a function with a reference to the object in this arena instance.
	///
	/// # Panics
	///
	/// Panics if the object has been removed or the type doesn't match.
	pub fn with_ref<T: 'static, R>(
		&self,
		handle: &ArenaHandle<T>,
		func: impl FnOnce(&T) -> R,
	) -> R {
		let objects = self.objects.lock().unwrap();
		let obj = objects
			.get(&handle.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.expect(PANIC_MSG);
		func(obj)
	}

	/// Executes a function with a mutable reference to the object in this arena instance.
	///
	/// # Panics
	///
	/// Panics if the object has been removed or the type doesn't match.
	pub fn with_mut<T: 'static, R>(
		&self,
		handle: &ArenaHandle<T>,
		func: impl FnOnce(&mut T) -> R,
	) -> R {
		let mut objects = self.objects.lock().unwrap();
		let obj = objects
			.get_mut(&handle.id)
			.and_then(|entry| entry.object.downcast_mut::<T>())
			.expect(PANIC_MSG);
		func(obj)
	}

	/// Returns a clone of a cloneable object stored in this arena instance.
	///
	/// # Panics
	///
	/// Panics if the object has been removed or the type doesn't match.
	pub fn get_cloned<T: Clone + 'static>(&self, handle: &ArenaHandle<T>) -> T {
		let objects = self.objects.lock().unwrap();
		objects
			.get(&handle.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.cloned()
			.expect(PANIC_MSG)
	}
}

const PANIC_MSG: &str = r#"
Object does not exist in the Arena.
It may have been manually removed by another handle.
"#;

/// A copyable handle providing type-safe access to an object in the [`Arena`].
///
/// Handles are lightweight and can be freely copied. All copies refer to the
/// same underlying object; mutations through one handle are visible to all others.
///
/// # Warning
///
/// When [`remove`](Self::remove) is called, all other handles to the same object
/// become invalid. Accessing an invalid handle will panic.
pub struct ArenaHandle<T> {
	id: usize,
	_phantom: std::marker::PhantomData<T>,
}

impl<T> Copy for ArenaHandle<T> {}

impl<T> Clone for ArenaHandle<T> {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			_phantom: std::marker::PhantomData,
		}
	}
}

impl<T: Clone + 'static> ArenaHandle<T> {
	/// Returns a clone of the object.
	///
	/// # Panics
	///
	/// Panics if the object has been removed.
	pub fn get_cloned(&self) -> T {
		let objects = Arena::get_global().objects.lock().unwrap();
		objects
			.get(&self.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.cloned()
			.expect(PANIC_MSG)
	}
}

impl<T: 'static> ArenaHandle<T> {
	/// Executes a function with a reference to the object.
	///
	/// # Panics
	///
	/// Panics if the object has been removed.
	pub fn with<R>(&self, func: impl FnOnce(&T) -> R) -> R {
		let objects = Arena::get_global().objects.lock().unwrap();
		let obj = objects
			.get(&self.id)
			.and_then(|entry| entry.object.downcast_ref::<T>())
			.expect(PANIC_MSG);
		func(obj)
	}

	/// Executes a function with a mutable reference to the object.
	///
	/// # Panics
	///
	/// Panics if the object has been removed.
	pub fn with_mut<R>(&self, func: impl FnOnce(&mut T) -> R) -> R {
		let mut objects = Arena::get_global().objects.lock().unwrap();
		let obj = objects
			.get_mut(&self.id)
			.and_then(|entry| entry.object.downcast_mut::<T>())
			.expect(PANIC_MSG);
		func(obj)
	}

	/// Removes the object from the arena and returns it.
	///
	/// After removal, all handles to this object become invalid.
	///
	/// # Panics
	///
	/// Panics if the object has already been removed.
	pub fn remove(self) -> T {
		Arena::get_global().remove_impl(&self).expect(PANIC_MSG)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::prelude::*;

	#[derive(Debug, Clone)]
	struct Counter {
		value: i32,
		name: String,
	}

	impl Counter {
		fn new(name: String, initial_value: i32) -> Self {
			Self {
				value: initial_value,
				name,
			}
		}

		fn increment(&mut self) { self.value += 1; }

		fn get_value(&self) -> i32 { self.value }

		fn get_name(&self) -> &str { &self.name }
	}

	impl Drop for Counter {
		fn drop(&mut self) {
			// println!("Counter '{}' is being dropped", self.name);
		}
	}

	#[test]
	fn handle_is_copy() {
		fn assert_copy<T: Copy>() {}
		fn assert_send<T: Send>() {}
		assert_copy::<ArenaHandle<i32>>();
		assert_send::<ArenaHandle<i32>>();
	}

	#[test]
	fn basic_arena_operations() {
		let arena = Arena::new();

		let counter_handle =
			arena.insert_impl(Counter::new("test".to_string(), 42));
		let string_handle = arena.insert_impl("Hello, World!".to_string());
		let number_handle = arena.insert_impl(123i32);

		arena.objects.lock().unwrap().len().xpect_eq(3);

		arena.with_ref(&counter_handle, |counter| {
			counter.get_value().xpect_eq(42);
			counter.get_name().xpect_eq("test");
		});

		arena.with_ref(&string_handle, |string| {
			string.as_str().xpect_eq("Hello, World!");
		});

		arena.with_ref(&number_handle, |number| {
			(*number).xpect_eq(123);
		});

		arena.with_mut(&counter_handle, |counter| {
			counter.increment();
			counter.get_value().xpect_eq(43);
		});

		let cloned_counter = arena.get_cloned(&counter_handle);
		cloned_counter.get_value().xpect_eq(43);

		let _removed_counter =
			arena.remove_impl(&counter_handle).expect(PANIC_MSG);
		let _removed_string =
			arena.remove_impl(&string_handle).expect(PANIC_MSG);
		let _removed_number =
			arena.remove_impl(&number_handle).expect(PANIC_MSG);

		arena.objects.lock().unwrap().len().xpect_eq(0);
	}

	#[test]
	fn copy_handles() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 100));

		let handle2 = handle1.clone();

		arena.with_ref(&handle1, |counter| {
			counter.get_value().xpect_eq(100);
		});

		arena.with_ref(&handle2, |counter| {
			counter.get_value().xpect_eq(100);
		});

		arena.with_mut(&handle1, |counter| {
			counter.increment();
		});

		arena.with_ref(&handle2, |counter| {
			counter.get_value().xpect_eq(101);
		});

		let removed = arena.remove_impl(&handle1).expect(PANIC_MSG);
		removed.get_value().xpect_eq(101);
		arena.objects.lock().unwrap().len().xpect_eq(0);
	}

	#[test]
	#[should_panic]
	fn panic_on_invalid_handle_access() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone();

		let _removed = arena.remove_impl(&handle2).expect(PANIC_MSG);
		arena.objects.lock().unwrap().len().xpect_eq(0);

		arena.with_ref(&handle1, |_| {});
	}

	#[test]
	#[should_panic]
	fn panic_on_invalid_handle_with_mut() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone();

		let _removed = arena.remove_impl(&handle2).expect(PANIC_MSG);
		arena.objects.lock().unwrap().len().xpect_eq(0);

		// handle1 should now panic when accessed mutably
		arena.with_mut(&handle1, |_| {});
	}

	#[test]
	#[should_panic]
	fn panic_on_invalid_handle_remove() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("test".to_string(), 42));
		let handle2 = handle1.clone();

		let _removed = arena.remove_impl(&handle2).expect(PANIC_MSG);
		arena.objects.lock().unwrap().len().xpect_eq(0);

		// handle1 should now panic when trying to remove again
		let _removed2 = arena.remove_impl(&handle1).expect(PANIC_MSG);
	}

	#[test]
	fn multiple_objects() {
		let arena = Arena::new();

		let handle1 = arena.insert_impl(Counter::new("first".to_string(), 1));
		let handle2 = arena.insert_impl(Counter::new("second".to_string(), 2));
		let handle3 = arena.insert_impl("string".to_string());

		arena.objects.lock().unwrap().len().xpect_eq(3);

		// All handles should work independently
		arena.with_ref(&handle1, |counter| {
			counter.get_value().xpect_eq(1);
		});
		arena.with_ref(&handle2, |counter| {
			counter.get_value().xpect_eq(2);
		});
		arena.with_ref(&handle3, |string| {
			string.as_str().xpect_eq("string");
		});

		// Remove objects individually
		let _removed1 = arena.remove_impl(&handle1).expect(PANIC_MSG);
		arena.objects.lock().unwrap().len().xpect_eq(2);

		let _removed2 = arena.remove_impl(&handle2).expect(PANIC_MSG);
		arena.objects.lock().unwrap().len().xpect_eq(1);

		let _removed3 = arena.remove_impl(&handle3).expect(PANIC_MSG);
		arena.objects.lock().unwrap().len().xpect_eq(0);
	}

	#[test]
	fn clear_functionality() {
		let arena = Arena::new();

		let _handle1 = arena.insert_impl(Counter::new("test1".to_string(), 1));
		let _handle2 = arena.insert_impl(Counter::new("test2".to_string(), 2));
		let _handle3 = arena.insert_impl("string".to_string());

		arena.objects.lock().unwrap().len().xpect_eq(3);

		arena.objects.lock().unwrap().clear();
		arena.objects.lock().unwrap().len().xpect_eq(0);
		arena.objects.lock().unwrap().is_empty().xpect_true();
	}
}
